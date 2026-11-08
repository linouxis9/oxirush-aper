/*
   OxiRush
   Copyright 2025-2026 Valentin D'Emmanuele

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

   http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

//! What the tables of an application protocol say of the values of its trees, the errors
//! of their paths, and the IEs of a tree by their names: what the paths of `trees` and
//! the codec of `views` read.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use oxirush_utils::tree::{self, Edit};
use oxirush_utils::tree::{Map, Value};

use super::views::LOSSY;
use super::{Kind, Member, Protocol, Sent, Shape, foreign, registered, same_name, type_name};
use crate::{Direction, Error, ErrorKind};

/// The member of a message that holds its IEs.
const IES: &str = "protocolIEs";
/// How deep a tree that is written is read.
const WRITTEN: usize = 256;

// ── The errors ──────────────────────────────────────────────────────────────

/// The kinds of the errors of the crate, by the code that the engine carries for them.
const KINDS: [ErrorKind; 10] = [
    ErrorKind::Decode,
    ErrorKind::Encode,
    ErrorKind::WrongMessage,
    ErrorKind::MissingIe,
    ErrorKind::UndecodableIe,
    ErrorKind::Argument,
    ErrorKind::UnknownMessage,
    ErrorKind::UnknownIe,
    ErrorKind::Path,
    ErrorKind::Value,
];

/// The code that the engine carries for an error of this kind.
fn code(kind: ErrorKind) -> u16 {
    let at = KINDS.iter().position(|known| *known == kind);
    at.map_or(u16::MAX, |at| at as u16)
}

/// Why a segment selects nothing at a place, which another place may make up for: the
/// path is refused when no place has what it selects.
pub(super) fn fault(kind: ErrorKind, reason: impl Into<String>) -> tree::Error {
    tree::Error::new(tree::ErrorKind::NoSuchMember, "", reason).with_code(code(kind))
}

/// Why a path is refused at a place, whatever the others have: the value there cannot
/// be read, or the edit cannot be done.
pub(super) fn refusal(kind: ErrorKind, reason: impl Into<String>) -> tree::Error {
    tree::Error::new(tree::ErrorKind::Refused, "", reason).with_code(code(kind))
}

/// The error of the crate that an error of the engine is: of the kind that the family
/// gave it, and otherwise of the kind that says the same.
pub(super) fn error(error: tree::Error) -> Error {
    let kind = match error.code() {
        Some(code) => KINDS.get(usize::from(code)).copied(),
        None => None,
    };
    let kind = kind.unwrap_or(match error.kind() {
        tree::ErrorKind::Value | tree::ErrorKind::Refused => ErrorKind::Value,
        _ => ErrorKind::Path,
    });
    Error::new(kind, error.to_string())
}

// ── What the tables say ─────────────────────────────────────────────────────

/// The position that a segment is: a number as decimal writes it, without a sign and
/// without a zero before it.
pub(super) fn position(part: &str) -> Option<usize> {
    tree::position(part)
}

impl Protocol {
    /// The name that ASN.1 gives the IE of an identifier, after `id-`.
    pub fn ie_name_of(&self, id: u16) -> Option<&'static str> {
        let at = self.ie_names.binary_search_by_key(&id, |(known, _)| *known);
        at.ok().map(|at| self.ie_names[at].1)
    }

    /// The message of a direction and a procedure code, or of a name.
    pub(crate) fn sent(&self, of: impl Fn(Direction, u8, &'static str) -> bool) -> Option<Sent> {
        let mut messages = self.messages.iter();
        let message = messages.find(|(set, ..)| of(set.direction, set.procedure_code, set.name));
        message.map(|(set, _, ty)| Sent {
            name: set.name,
            ty: type_name(ty),
            ies: set.ies.unwrap_or_default(),
        })
    }

    /// The names of the values of the ENUMERATED that the bindings name `name`.
    pub fn enumeration(&self, name: &str) -> Option<&'static [&'static str]> {
        let at = self
            .enumerations
            .binary_search_by(|(known, _)| (*known).cmp(name));
        at.ok().map(|at| self.enumerations[at].1)
    }

    /// `shape`, or what the type that it names is.
    pub fn resolved(&self, mut shape: Shape) -> Shape {
        // A type names another one a few times at most.
        for _ in 0..16 {
            shape = match shape {
                Shape::Named(name) => match (self.kind_of)(name) {
                    Some(Kind::Is(is)) => *is,
                    Some(_) => return shape,
                    None => return Shape::Plain,
                },
                shape => return shape,
            };
        }
        Shape::Unknown
    }

    /// The shape of the value of the IE of an identifier, in the extension container of
    /// the type that the bindings name `extended` when it is in one.
    pub fn ie_value(&self, id: Option<u16>, extended: &str) -> Shape {
        let Some(id) = id else {
            return Shape::Unknown;
        };
        if self.carries(id) {
            return Shape::Carrier(id);
        }
        // An IE that has the type which the type that it extends gives it.
        let mut extensions = self.extension_types.iter();
        if let Some((.., ty)) = extensions.find(|(known, of, _)| *known == id && *of == extended) {
            return Shape::Named(type_name(ty));
        }
        match self.ie_types.iter().find(|(known, ..)| *known == id) {
            Some((_, _, contained)) if !contained.is_empty() => {
                Shape::Transfer(type_name(contained))
            }
            Some((_, ty, _)) => Shape::Named(type_name(ty)),
            // An identifier that is unknown, or one of an extension where it extends no
            // type that gives it one.
            None => Shape::Unknown,
        }
    }

    /// The members that a value of `shape` can have, which is `entry` in a view; `None`
    /// for a shape whose members are those that the view has.
    pub fn members(&self, shape: Shape, entry: Option<&Map>) -> Option<Vec<Member>> {
        let has = |member: &str| entry.is_some_and(|entry| entry.contains_key(member));
        // The entry of an IE in a view says which IE its value was decoded as.
        let ie = |id: Option<u16>, extended: &str, value: &'static str, raw: &'static str| {
            let of = |member: &str| {
                let id = entry
                    .and_then(|entry| entry.get(member))
                    .and_then(Value::as_u64);
                id.and_then(|id| u16::try_from(id).ok())
            };
            let decoded = match has("_decode_error") {
                true => Shape::Undecoded,
                false => self.ie_value(of("id").or(id), extended),
            };
            vec![
                ("id", Shape::Plain, false),
                ("criticality", Shape::Plain, false),
                (value, decoded, true),
                (raw, Shape::Plain, true),
                ("_decode_error", Shape::Plain, true),
                (LOSSY, Shape::Plain, true),
            ]
        };
        Some(match shape {
            // The members of a type of another protocol are those that its tables give.
            // The members of a tree of a PDU are those that the trees give it.
            Shape::Unknown | Shape::Undecoded | Shape::Foreign(..) | Shape::Pdu(_) => return None,
            // The IEs of the message of a PDU are those of its object set.
            Shape::Message(message) => {
                let mut members = self.members(Shape::Named(message.ty), entry)?;
                for (name, shape, _) in &mut members {
                    if *name == IES {
                        *shape = Shape::Ies(Some(message));
                    }
                }
                members
            }
            // Its IEs are each of the type that its identifier gives it.
            Shape::AnyMessage => vec![(IES, Shape::Ies(None), false)],
            Shape::Named(name) => match (self.kind_of)(name) {
                Some(Kind::Sequence(members) | Kind::Choice(members)) => {
                    // An extension container extends the type that has it, and the IEs
                    // of a transfer are those of its object set.
                    let held = |(member, shape, optional): &Member| match shape {
                        Shape::Extensions(_) => (*member, Shape::Extensions(name), *optional),
                        Shape::Ies(None) if *member == IES => {
                            (*member, Shape::Ies(self.transfer(name)), *optional)
                        }
                        shape => (*member, *shape, *optional),
                    };
                    members.iter().map(held).collect()
                }
                Some(Kind::Is(is)) => return self.members(self.resolved(*is), entry),
                None => Vec::new(),
            },
            Shape::Bits => vec![
                ("value", Shape::Plain, false),
                ("length", Shape::Plain, false),
            ],
            Shape::Ie(id) => ie(id, "", "value", "_raw_value"),
            Shape::Extension(id, extended) => ie(id, extended, "extensionValue", "_raw_value"),
            Shape::Transfer(contained) => {
                let decoded = match contained.is_empty() {
                    true => Shape::Unknown,
                    false => Shape::Named(contained),
                };
                vec![
                    ("decoded", decoded, true),
                    ("_raw_transfer", Shape::Plain, true),
                    ("_decode_error", Shape::Plain, true),
                    (LOSSY, Shape::Plain, true),
                ]
            }
            // Each type that the octets may carry, by the name that ASN.1 gives it: one
            // of this protocol, or one of another, which its tables read.
            Shape::Carrier(id) => {
                let carried = self.carried.iter().filter(|(carrier, ..)| *carrier == id);
                let own =
                    carried.map(|(_, name, ty)| (*name, Shape::Transfer(type_name(ty)), true));
                let foreign = self.foreign.iter().filter(|(carrier, ..)| *carrier == id);
                let other = foreign
                    .map(|(_, name, protocol)| (*name, Shape::Foreign(name, protocol), true));
                own.chain(other).collect()
            }
            Shape::Plain | Shape::Null | Shape::List(_) | Shape::Ies(_) | Shape::Extensions(_) => {
                Vec::new()
            }
        })
    }

    /// Why a segment selects nothing in a value of `shape`: the type has no such member.
    pub(super) fn no_member(&self, shape: Shape, members: &[Member], part: &str) -> tree::Error {
        // The root stands for the IEs of a message, which a private message has none of.
        if let (Shape::Message(message), IES) = (shape, part) {
            let has: Vec<_> = members.iter().map(|(name, ..)| *name).collect();
            return fault(
                ErrorKind::Path,
                format!(
                    "a message {} has no IEs under /{}: it has {}",
                    message.name,
                    self.root,
                    has.join(", ")
                ),
            );
        }
        let of = match shape {
            Shape::Named(name) => name.to_string(),
            Shape::Pdu(_) => "the tree of a PDU".to_string(),
            Shape::Bits => "a bit string".to_string(),
            Shape::Transfer(_) => "a transfer".to_string(),
            Shape::Ie(_) | Shape::Extension(..) => "an IE".to_string(),
            _ => "this value".to_string(),
        };
        let names = names(shape, members);
        fault(
            ErrorKind::Path,
            match names.is_empty() {
                true => format!("{part:?} is not a member of {of}, which has none"),
                false => format!(
                    "{part:?} is not a member of {of}, which has {}",
                    names.join(", ")
                ),
            },
        )
    }

    /// The transfer of protocol IEs whose type the bindings name `name`, as a message
    /// of its object set: `None` for a type that is no such transfer.
    fn transfer(&self, name: &str) -> Option<Sent> {
        let mut transfers = self.transfers.iter();
        let (asn, ty, ies) = transfers.find(|(_, ty, _)| *ty == name)?;
        Some(Sent { name: asn, ty, ies })
    }

    /// The criticality that ASN.1 assigns to the IE `id` in a container of `shape`,
    /// where the tables have it: that of the object set of a message, or of a transfer,
    /// for its IEs, and that of the single containers for an IE that a value holds alone
    /// or as an entry of a list. They have none for the IEs of an extension container.
    fn assigned(&self, container: Shape, id: u16) -> Option<&'static str> {
        match container {
            Shape::Ies(Some(message)) => {
                let ie = message.ies.iter().find(|(known, ..)| *known == id);
                ie.map(|(_, criticality, _)| criticality.name())
            }
            Shape::Ie(_) | Shape::List(Shape::Ie(_)) => {
                let mut items = self.item_criticalities.iter();
                items
                    .find(|(known, _)| *known == id)
                    .map(|(_, criticality)| criticality.name())
            }
            _ => None,
        }
    }

    /// The message that ASN.1 names `name`, taken as a path takes a name.
    pub(super) fn message_of(&self, name: &str) -> Result<Sent, Error> {
        let known = self.sent(|_, _, known| same_name(known, name));
        known.ok_or_else(|| {
            Error::new(
                ErrorKind::UnknownMessage,
                format!(
                    "{name:?} is not a message of {}{}",
                    self.name,
                    self.nearest_message(name)
                ),
            )
        })
    }

    /// The identifier of the IE that `name` names, as ASN.1 names it after `id-`.
    fn ie_named(&self, name: &str) -> Result<u16, Error> {
        (self.ie_id)(name).ok_or_else(|| {
            Error::new(
                ErrorKind::UnknownIe,
                format!(
                    "{name:?} is not an IE of {}{}",
                    self.name,
                    self.nearest_ie(name)
                ),
            )
        })
    }

    /// What an error says of `name`, which no IE of the protocol has: the names of the
    /// IEs that are nearest to it, if some are near.
    pub(crate) fn nearest_ie(&self, name: &str) -> String {
        let names: Vec<&str> = self.ie_names.iter().map(|(_, name)| *name).collect();
        nearest_names(&names, name)
    }

    /// What an error says of `name`, which no message of the protocol has: the names of
    /// the messages that are nearest to it, if some are near.
    pub(crate) fn nearest_message(&self, name: &str) -> String {
        let names: Vec<&str> = self.messages.iter().map(|(set, ..)| set.name).collect();
        nearest_names(&names, name)
    }
}

/// `: the nearest name is X`, or `: the nearest names are X and Y`, of the names that are
/// near `name`; nothing when none is.
fn nearest_names(names: &[&str], name: &str) -> String {
    match oxirush_utils::tree::nearest(names, name, 3)[..] {
        [] => String::new(),
        [one] => format!(": the nearest name is {one}"),
        [ref near @ .., last] => format!(": the nearest names are {} and {last}", near.join(", ")),
    }
}

/// The name of the type that the extension container of `shape`, or of an IE of
/// `shape`, extends; no name for another shape, and where the tables do not say.
pub fn extended(shape: Shape) -> &'static str {
    match shape {
        Shape::Extensions(extended) | Shape::Extension(_, extended) => extended,
        _ => "",
    }
}

/// The member of `members` that a segment names. A member that starts with `_` is named
/// as it is.
pub(super) fn member<'m>(members: &'m [Member], part: &str) -> Option<&'m Member> {
    let exact = members.iter().find(|(name, ..)| *name == part);
    exact.or_else(|| {
        let mut named = members.iter().filter(|(name, ..)| !name.starts_with('_'));
        named.find(|(name, ..)| same_name(name, part))
    })
}

/// Whether a value of `shape` is an IE or a transfer: it has `octets`.
fn has_octets(shape: Shape) -> bool {
    matches!(
        shape,
        Shape::Ie(_) | Shape::Extension(..) | Shape::Transfer(_)
    )
}

/// The member that holds the value of an IE or of a transfer of `shape`, which a path
/// may leave out: a segment that names no member of the IE or of the transfer is taken
/// in that value.
fn value_member(shape: Shape) -> Option<&'static str> {
    match shape {
        Shape::Ie(_) => Some("value"),
        Shape::Extension(..) => Some("extensionValue"),
        Shape::Transfer(_) => Some("decoded"),
        _ => None,
    }
}

/// The members of a value of `shape` that a path names: those that its type has, and
/// the `octets` of an IE or of a transfer.
fn names(shape: Shape, members: &[Member]) -> Vec<&'static str> {
    let mut names: Vec<_> = (members.iter().map(|(name, ..)| *name))
        .filter(|name| shown(name) && *name != "_decode_error" && *name != LOSSY)
        .collect();
    if has_octets(shape) {
        names.push("octets");
    }
    names
}

/// Why a segment that is no position selects no entry of a list.
pub(super) fn no_position(part: &str) -> tree::Error {
    fault(
        ErrorKind::Path,
        format!(
            "{part:?} is not a position in a list: an entry goes by its number, and \"*\" is \
             each of them"
        ),
    )
}

/// Why a segment that starts with `@id=` selects no IE.
pub(super) fn not_an_id(part: &str) -> tree::Error {
    refusal(
        ErrorKind::Path,
        format!(
            "{part:?} is not the identifier of an IE: \"@id=\" is followed by a number up to \
             65535"
        ),
    )
}

/// The identifier that a segment `@id=N` gives, or the IE that it names when IEs are
/// selected by name; `None` for a segment that is neither.
pub(super) fn segment_id(
    protocol: &Protocol,
    part: &str,
    named: bool,
) -> Result<Option<u16>, tree::Error> {
    match part.strip_prefix("@id=") {
        Some(id) => id.parse::<u16>().map(Some).map_err(|_| not_an_id(part)),
        None => Ok((protocol.ie_id)(part).filter(|_| named)),
    }
}

/// The identifier that a segment gives in the entry of an IE that a value holds alone,
/// where no segment is a position: `@id=N`, the name of the IE, or its identifier in
/// digits, as a tree writes an IE that has no name and `from_tree` reads it.
pub(super) fn held_id(protocol: &Protocol, part: &str) -> Result<Option<u16>, tree::Error> {
    match part.parse::<u16>() {
        Ok(id) => Ok(Some(id)),
        Err(_) => segment_id(protocol, part, true),
    }
}

/// The shape of an entry of a list of `shape`.
pub(super) fn element(shape: Shape) -> Shape {
    match shape {
        Shape::Ies(_) | Shape::List(Shape::Ie(_)) => Shape::Ie(None),
        Shape::Extensions(extended) => Shape::Extension(None, extended),
        Shape::List(Shape::Extension(_, extended)) => Shape::Extension(None, extended),
        Shape::List(of) => *of,
        _ => Shape::Unknown,
    }
}

/// The shape of the IE of an identifier in a list of `shape`; `None` for a list whose
/// entries are no IEs.
pub(super) fn element_of(shape: Shape, id: u16) -> Option<Shape> {
    match element(shape) {
        Shape::Ie(_) => Some(Shape::Ie(Some(id))),
        Shape::Extension(_, extended) => Some(Shape::Extension(Some(id), extended)),
        Shape::Unknown => Some(Shape::Unknown),
        _ => None,
    }
}

/// Whether a member is one of a value, and not what a view keeps of what was received:
/// but why a value is its octets, or has no place for all of them, which is why a tree
/// has those octets in its place.
fn shown(name: &str) -> bool {
    !name.starts_with('_') || name == "_decode_error" || name == LOSSY
}

/// The member of an entry that holds the octets it was received as.
fn received_member(entry: &Map) -> Option<&'static str> {
    ["_raw_value", "_raw_transfer"]
        .into_iter()
        .find(|member| entry.contains_key(member))
}

// ── The protocol of a place ─────────────────────────────────────────────────

/// The protocol whose tables read a place, by its name, where it is another than the one
/// of the tree: in the type of another protocol that the octets of an IE carry, which is
/// registered.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Owner(Option<&'static str>);

impl Owner {
    /// The tables of `protocol`, which read the places under this one.
    pub(super) fn of(protocol: &'static Protocol) -> Self {
        Self(Some(protocol.name))
    }

    /// The protocol of a place of a tree of `protocol`.
    pub(super) fn or(self, protocol: &'static Protocol) -> &'static Protocol {
        self.0.and_then(registered).unwrap_or(protocol)
    }
}

/// Why an edit at the member `name` of a value is refused: an alternative of a CHOICE
/// and a member that its type always has are not taken out, and insert adds to a list.
pub(super) fn refuse(
    edit: Edit,
    name: &str,
    alternative: bool,
    optional: bool,
) -> Result<(), tree::Error> {
    let refused = |reason: String| Err(refusal(ErrorKind::Value, reason));
    match edit {
        Edit::Remove => {
            if alternative {
                return refused(format!(
                    "{name} is not taken out: a CHOICE has one alternative, set another in \
                     its place"
                ));
            }
            if !optional {
                return refused(format!("{name} is not taken out: its type always has it"));
            }
            Ok(())
        }
        Edit::Insert => refused("insert adds to a list".to_string()),
        _ => Ok(()),
    }
}

// ── Edits ───────────────────────────────────────────────────────────────────

/// Write the entry of an IE as a view has it: the `octets` that a tree writes in the place
/// of its value are what it is sent as.
pub(super) fn written_ie(entry: &mut Map) {
    if entry.get("id").is_some_and(Value::is_number)
        && let Some(octets) = entry.shift_remove("octets")
    {
        entry.insert("_raw_value".into(), octets);
    }
}

// ── Trees ───────────────────────────────────────────────────────────────────

/// Whether a transfer has a member of this name in a view: its `decoded` value, its
/// `octets`, or what a view keeps of what was received. Where a transfer is written, in
/// a tree or in a view, an object that has none of them is its decoded value.
pub fn of_a_transfer(name: &str) -> bool {
    name.starts_with('_') || ["decoded", "octets"].contains(&name)
}

/// The members that a value of a view can have, each with its shape.
pub enum Members {
    /// Those of a SEQUENCE or of a CHOICE, as the list of the types has them, and the
    /// name of that type: an extension container among them extends it.
    Listed(&'static [Member], &'static str),
    /// Those of another value, as an IE or a transfer.
    Made(Vec<Member>),
}

impl Protocol {
    /// The members that `entry`, a value of `shape` in a view, can have, each with its
    /// shape; none for a shape that the tables do not describe.
    pub fn member_shapes(&self, shape: Shape, entry: &Map) -> Members {
        let shape = match self.resolved(shape) {
            // A transfer that is written as its decoded value itself.
            Shape::Transfer(contained)
                if !contained.is_empty() && !entry.keys().any(|name| of_a_transfer(name)) =>
            {
                self.resolved(Shape::Named(contained))
            }
            shape => shape,
        };
        // The IEs of a transfer are those of its object set, which `members` gives.
        if let Shape::Named(name) = shape
            && let Some(Kind::Sequence(members) | Kind::Choice(members)) = (self.kind_of)(name)
            && !(matches!(members, [(IES, Shape::Ies(None), _)]) && self.transfer(name).is_some())
        {
            return Members::Listed(members, name);
        }
        Members::Made(self.members(shape, Some(entry)).unwrap_or_default())
    }

    /// The shape of each entry of a list of `shape`.
    pub fn entry_shape(&self, shape: Shape) -> Shape {
        element(self.resolved(shape))
    }

    /// The shape of the message of a direction and a procedure code in a view, where
    /// the tables have it.
    pub fn message_shape(&self, direction: Direction, code: u8) -> Shape {
        let message = self.sent(|of, procedure, _| procedure == code && of == direction);
        message.map_or(Shape::AnyMessage, Shape::Message)
    }

    /// The shape of octets that carry the type which ASN.1 names `name`, once a view
    /// shows them so: the transfer of that type.
    pub fn carried_shape(&self, name: &str) -> Shape {
        let mut carried = self.carried.iter();
        let carried = carried.find(|(_, known, _)| same_name(known, name));
        carried.map_or(Shape::Unknown, |(.., ty)| Shape::Transfer(type_name(ty)))
    }

    /// `value`, a value of `shape` of a view, as a tree has it: a transfer as its
    /// decoded value, or as its `octets` when it has none, and each IE of a container as
    /// [`ie_by_name`](Self::ie_by_name) shows it. The view is taken: what the tree keeps
    /// of it is moved.
    pub(crate) fn by_name(&self, value: Value, shape: Shape) -> Result<Value, Error> {
        let shape = self.resolved(shape);
        // A type of another protocol, as its tables show it.
        if let Shape::Foreign(name, protocol) = shape {
            let other = foreign(name, protocol)?;
            return other.by_name(value, other.carried_shape(name));
        }
        match value {
            Value::Object(mut entry) => match shape {
                Shape::Transfer(_) => {
                    let members = self.members(shape, Some(&entry)).unwrap_or_default();
                    let of = member(&members, "decoded").map(|(_, of, _)| *of);
                    let decoded = entry.shift_remove("decoded");
                    // What the typed value has no place for is kept in the octets.
                    let decoded = decoded.filter(|_| !entry.contains_key(LOSSY));
                    match (decoded, of) {
                        (Some(decoded), Some(of)) => self.by_name(decoded, of),
                        _ => {
                            let octets =
                                received_member(&entry).and_then(|raw| entry.shift_remove(raw));
                            Ok(octets_alone(octets.unwrap_or_default()))
                        }
                    }
                }
                // An IE that a value holds alone.
                Shape::Ie(_) | Shape::Extension(..) => self.ie_by_name(entry, shape, shape),
                _ => {
                    let members = self.member_shapes(shape, &entry);
                    for (name, value) in entry.iter_mut() {
                        *value =
                            self.by_name(core::mem::take(value), member_shape(&members, name))?;
                    }
                    Ok(Value::Object(entry))
                }
            },
            Value::Array(mut entries) => {
                let of = element(shape);
                for entry in &mut entries {
                    *entry = match (of, core::mem::take(entry)) {
                        (Shape::Ie(_) | Shape::Extension(..), Value::Object(entry)) => {
                            self.ie_by_name(entry, of, shape)?
                        }
                        (Shape::Ie(_) | Shape::Extension(..), _) => return Err(no_identifier()),
                        (of, entry) => self.by_name(entry, of)?,
                    };
                }
                Ok(Value::Array(entries))
            }
            plain => Ok(plain),
        }
    }

    /// The entry of an IE of `shape` in a container of `container`, as its name with its
    /// value: the name that ASN.1 gives the IE, or its identifier in digits when it has
    /// none. One that did not decode, or that contains a type and octets that are not
    /// of that type, has the octets that it was received as, as `{"octets": "…"}`. Its
    /// `criticality` is beside its name when it is not the one that the specification
    /// assigns it there, and where the tables do not have that one.
    fn ie_by_name(&self, mut entry: Map, shape: Shape, container: Shape) -> Result<Value, Error> {
        let id = match entry.get("id") {
            Some(Value::String(name)) => (self.ie_id)(name),
            Some(id) => id.as_u64().and_then(|id| u16::try_from(id).ok()),
            None => None,
        };
        let id = id.ok_or_else(no_identifier)?;
        let name = self
            .ie_name_of(id)
            .map_or_else(|| id.to_string(), str::to_string);
        let field = value_member(shape).unwrap_or("value");
        let value = entry.shift_remove(field).unwrap_or_default();
        // An IE that did not decode, or whose typed value has no place for what it was
        // received with, is its octets.
        let undecoded = entry.contains_key("_decode_error")
            || entry.contains_key(LOSSY)
            || value.get("_decode_error").is_some()
            || value.get(LOSSY).is_some();
        let value = match undecoded {
            true => octets_alone(entry.shift_remove("_raw_value").unwrap_or_default()),
            false => self.by_name(value, self.ie_value(Some(id), extended(shape)))?,
        };
        let criticality = entry.shift_remove("criticality").unwrap_or_default();
        let mut shown = Map::new();
        shown.insert(name, value);
        if criticality.as_str() != self.assigned(container, id) {
            shown.insert("criticality".into(), criticality);
        }
        Ok(Value::Object(shown))
    }

    /// `value`, a value of `shape` of a tree, as a view has it: each IE of a container
    /// as the entry that [`entry_of`](Self::entry_of) gives it. A transfer is its decoded
    /// value, or its `octets` alone.
    pub(crate) fn written(
        &self,
        value: &Value,
        shape: Shape,
        depth: usize,
    ) -> Result<Value, Error> {
        if depth > WRITTEN {
            return Err(Error::new(
                ErrorKind::Value,
                format!("a written tree nests deeper than {WRITTEN}"),
            ));
        }
        let shape = self.resolved(shape);
        // A type of another protocol, as its tables write it.
        if let Shape::Foreign(name, protocol) = shape {
            let other = foreign(name, protocol)?;
            return other.written(value, other.carried_shape(name), depth);
        }
        match value {
            Value::Object(entry) => match shape {
                // A transfer as its decoded value, or as its octets alone, which the codec of
                // the view takes as they are.
                Shape::Transfer(_) => {
                    let members = self.members(shape, None).unwrap_or_default();
                    let decoded = member(&members, "decoded");
                    let decoded = decoded.map_or(Shape::Unknown, |(_, of, _)| *of);
                    self.written(value, decoded, depth + 1)
                }
                // An IE that a value holds alone.
                Shape::Ie(_) | Shape::Extension(..) => self.entry_of(value, shape, shape, depth),
                _ => {
                    let members = self.member_shapes(shape, entry);
                    let listed = entry.iter().map(|(name, value)| {
                        let of = member_shape(&members, name);
                        Ok((name.clone(), self.written(value, of, depth + 1)?))
                    });
                    Ok(Value::Object(listed.collect::<Result<_, Error>>()?))
                }
            },
            Value::Array(entries) => {
                let listed = entries.iter().map(|entry| match element(shape) {
                    of @ (Shape::Ie(_) | Shape::Extension(..)) => {
                        self.entry_of(entry, of, shape, depth)
                    }
                    of => self.written(entry, of, depth + 1),
                });
                Ok(Value::Array(listed.collect::<Result<_, Error>>()?))
            }
            plain => Ok(plain.clone()),
        }
    }

    /// The entry that a view has for an IE of `shape` in a container of `container`,
    /// which a tree writes as its name with its value: the name that ASN.1 gives the IE
    /// or its identifier in digits, and `{"octets": "…"}` for the octets that it is sent
    /// as. Its `criticality` is the one beside its name, or the one that the
    /// specification assigns it there.
    fn entry_of(
        &self,
        ie: &Value,
        shape: Shape,
        container: Shape,
        depth: usize,
    ) -> Result<Value, Error> {
        const IE: &str = "each entry of a container is one IE: its name with its value";
        let field = value_member(shape).unwrap_or("value");
        let entry = ie
            .as_object()
            .ok_or_else(|| Error::new(ErrorKind::Value, IE))?;
        let mut members = entry.iter().filter(|(member, _)| *member != "criticality");
        let (Some((name, value)), None) = (members.next(), members.next()) else {
            return Err(Error::new(ErrorKind::Value, IE));
        };
        let id = match name.parse::<u16>() {
            Ok(id) => id,
            Err(_) => self.ie_named(name)?,
        };
        let criticality = match (
            entry.get("criticality"),
            self.assigned(container, id),
            container,
        ) {
            (Some(criticality), ..) => criticality.clone(),
            (None, Some(criticality), _) => oxirush_utils::tree!(criticality),
            (None, None, Shape::Ies(Some(message))) => {
                return Err(Error::new(
                    ErrorKind::Value,
                    format!(
                        "{} has no IE {name}: one that is sent in it says its \"criticality\"",
                        message.name
                    ),
                ));
            }
            (None, None, _) => {
                return Err(Error::new(
                    ErrorKind::Value,
                    format!("{name} says its \"criticality\" here: the tables have none for it"),
                ));
            }
        };
        let mut listed = Map::new();
        listed.insert("id".into(), id.into());
        listed.insert("criticality".into(), criticality);
        let octets = value.as_object().filter(|value| value.len() == 1);
        match octets.and_then(|value| value.get("octets")) {
            Some(octets) => listed.insert("octets".into(), octets.clone()),
            None => {
                let value =
                    self.written(value, self.ie_value(Some(id), extended(shape)), depth + 1)?;
                listed.insert(field.into(), value)
            }
        };
        Ok(Value::Object(listed))
    }
}

impl Protocol {
    /// `value`, a value of `shape` that another implementation wrote in the JSON Encoding
    /// Rules (ITU-T X.697), as a tree has it: each value of a well-known type in its
    /// readable form, where that form gives the same value back. In those rules a string
    /// of decimal digits is octets, which a tree reads in two ways or as digits: the tests
    /// read the values of their reference with this.
    pub fn from_jer(&self, value: &mut Value, shape: Shape) {
        self.of_jer(value, shape, 0);
    }

    /// `value`, a value of `shape` in the JSON Encoding Rules, as a tree has it.
    fn of_jer(&self, value: &mut Value, mut shape: Shape, depth: usize) {
        if depth > WRITTEN {
            return;
        }
        // A type that another names, as far as a well-known type, which has a form.
        for _ in 0..16 {
            let Shape::Named(name) = shape else {
                break;
            };
            if let Some(form) = (self.type_form)(name) {
                *value = crate::json::readable(form, value.clone());
                return;
            }
            match (self.kind_of)(name) {
                Some(Kind::Is(is)) => shape = *is,
                _ => break,
            }
        }
        match value {
            Value::Object(object) => {
                let members = self.member_shapes(shape, object);
                for (key, child) in object.iter_mut() {
                    self.of_jer(child, member_shape(&members, key), depth + 1);
                }
                // A transfer is the value that it contains, which a view has as its
                // decoded value.
                if let Shape::Transfer(contained) = self.resolved(shape)
                    && !contained.is_empty()
                    && !object.keys().any(|name| of_a_transfer(name))
                {
                    let decoded = core::mem::take(object);
                    object.insert("decoded".into(), Value::Object(decoded));
                }
            }
            Value::Array(entries) => {
                let entry = self.entry_shape(shape);
                for child in entries {
                    self.of_jer(child, entry, depth + 1);
                }
            }
            _ => {}
        }
    }
}

/// The shape of the member that `name` names among `members`, which
/// [`member_shapes`](Protocol::member_shapes) gave.
pub fn member_shape(members: &Members, name: &str) -> Shape {
    let (members, of) = match members {
        Members::Listed(members, of) => (*members, Some(*of)),
        Members::Made(members) => (&members[..], None),
    };
    match (member(members, name), of) {
        // An extension container extends the type that has it.
        (Some((_, Shape::Extensions(_), _)), Some(of)) => Shape::Extensions(of),
        (Some((_, shape, _)), _) => *shape,
        (None, _) => Shape::Unknown,
    }
}

/// What a tree has for an IE or a transfer that has no value: the octets that it was
/// received as.
fn octets_alone(octets: Value) -> Value {
    let mut alone = Map::new();
    alone.insert("octets".into(), octets);
    Value::Object(alone)
}

/// An entry of a container that has no identifier.
fn no_identifier() -> Error {
    Error::new(ErrorKind::Value, "an IE has its identifier")
}

/// The IEs of the message of `protocol` that ASN.1 names `message`, which a view has as
/// `ies`, as a tree has them: each as its name with its value, and so are the IEs that
/// a value holds, to any depth.
///
/// # Errors
///
/// An entry that has no identifier.
pub fn named_ies(protocol: &'static Protocol, message: &str, ies: Value) -> Result<Value, Error> {
    let message = protocol.sent(|_, _, name| same_name(name, message));
    protocol.by_name(ies, Shape::Ies(message))
}

/// The entries that a view has for the IEs of the message of `protocol` that ASN.1 names
/// `message`, which `ies` has as a tree has them: as [`named_ies`] shows them.
///
/// # Errors
///
/// An entry that is not one IE, the name of no IE, a criticality that is not written
/// where no table has it, and a tree that nests deeper than 256.
pub fn listed_ies(protocol: &'static Protocol, message: &str, ies: &Value) -> Result<Value, Error> {
    let message = protocol.sent(|_, _, name| same_name(name, message));
    protocol.written(ies, Shape::Ies(message), 0)
}
