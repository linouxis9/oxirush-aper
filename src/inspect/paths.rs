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

//! The values of a view by their paths, and the IEs of a tree by their names, as the
//! family of the views of an application protocol on the engine of
//! `oxirush_utils::tree`.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use oxirush_utils::tree::{self, Edit, Family, Key, Listed, Next, Place, Root, Step};
use serde_json::{Map, Value, json};

use super::{Kind, Member, Protocol, Sent, Shape, same_name, type_name};
use crate::{Error, ErrorKind};

/// The member of a view that holds the message of the PDU.
const MESSAGE: &str = "message";
/// The member of a message that holds its IEs.
const IES: &str = "protocolIEs";
/// The member that marks an IE or a transfer whose value an edit changed: its octets are
/// no longer those of its value.
pub const EDITED: &str = "_edited";
/// The members that hold the value of an IE or of a transfer.
const VALUES: [&str; 3] = ["value", "extensionValue", "decoded"];
/// How deep a tree that is written is read.
const WRITTEN: usize = 256;

// ── The errors ──────────────────────────────────────────────────────────────

/// The kinds of the errors of the crate, by the code that the engine carries for them.
const KINDS: [ErrorKind; 16] = [
    ErrorKind::Decode,
    ErrorKind::Encode,
    ErrorKind::WrongMessage,
    ErrorKind::MissingIe,
    ErrorKind::UndecodableIe,
    ErrorKind::Argument,
    ErrorKind::UnknownMessage,
    ErrorKind::UnknownIe,
    ErrorKind::Path,
    ErrorKind::NoSuchMember,
    ErrorKind::NothingSelected,
    ErrorKind::NotOpened,
    ErrorKind::Value,
    ErrorKind::Form,
    ErrorKind::Edit,
    ErrorKind::Limit,
];

/// The code that the engine carries for an error of this kind.
fn code(kind: ErrorKind) -> u16 {
    let at = KINDS.iter().position(|known| *known == kind);
    at.map_or(u16::MAX, |at| at as u16)
}

/// Why a segment selects nothing at a place, which another place may make up for: the
/// path is refused when no place has what it selects.
fn fault(kind: ErrorKind, reason: impl Into<String>) -> tree::Error {
    let engine = match kind {
        ErrorKind::Path => tree::ErrorKind::Path,
        ErrorKind::Limit => tree::ErrorKind::Limit,
        _ => tree::ErrorKind::NoSuchMember,
    };
    tree::Error::new(engine, "", reason).with_code(code(kind))
}

/// Why a path is refused at a place, whatever the others have: the value there cannot
/// be read, or the edit cannot be done.
fn refusal(kind: ErrorKind, reason: impl Into<String>) -> tree::Error {
    tree::Error::new(tree::ErrorKind::Refused, "", reason).with_code(code(kind))
}

/// The same error, with the reason that `reword` makes of the one that it has.
fn reworded(error: tree::Error, reword: impl FnOnce(&str) -> String) -> tree::Error {
    let again = tree::Error::new(error.kind(), error.at(), reword(error.reason()));
    match error.code() {
        Some(code) => again.with_code(code),
        None => again,
    }
}

/// The error of the crate that an error of the engine is: of the kind that the family
/// gave it, and otherwise of the kind that says the same.
fn error(error: tree::Error) -> Error {
    let kind = match error.code() {
        Some(code) => KINDS.get(usize::from(code)).copied(),
        None => None,
    };
    let kind = kind.unwrap_or(match error.kind() {
        tree::ErrorKind::Path => ErrorKind::Path,
        tree::ErrorKind::Limit => ErrorKind::Limit,
        tree::ErrorKind::NothingSelected => ErrorKind::NothingSelected,
        tree::ErrorKind::Value => ErrorKind::Value,
        tree::ErrorKind::Refused => ErrorKind::Edit,
        _ => ErrorKind::NoSuchMember,
    });
    Error::new(kind, error.to_string())
}

// ── What the tables say ─────────────────────────────────────────────────────

/// The position that a segment is: a number as decimal writes it, without a sign and
/// without a zero before it.
fn position(part: &str) -> Option<usize> {
    tree::position(part)
}

impl Protocol {
    /// The name that ASN.1 gives the IE of an identifier, after `id-`.
    pub fn ie_name_of(&self, id: u16) -> Option<&'static str> {
        let at = self.ie_names.binary_search_by_key(&id, |(known, _)| *known);
        at.ok().map(|at| self.ie_names[at].1)
    }

    /// The name that ASN.1 gives the IE of an identifier, or `@id=` and its number.
    fn ie_name(&self, id: u16) -> String {
        match self.ie_name_of(id) {
            Some(name) => name.to_string(),
            None => format!("@id={id}"),
        }
    }

    /// The identifier of an entry of a list of IEs: its number, or the name of its IE as
    /// an edit may write it.
    fn entry_id(&self, entry: &Value) -> Option<u64> {
        match entry.get("id")? {
            Value::String(name) => (self.ie_id)(name).map(u64::from),
            id => id.as_u64(),
        }
    }

    /// Whether a segment selects among the IEs of a message: a name, a position,
    /// `@id=N`, `*`, or `-` for their end.
    fn selects_ies(&self, part: &str) -> bool {
        matches!(part, "*" | "-")
            || part.starts_with("@id=")
            || position(part).is_some()
            || (self.ie_id)(part).is_some()
    }

    /// The message of a direction and a procedure code, or of a name.
    fn sent(&self, of: impl Fn(&'static str, u8, &'static str) -> bool) -> Option<Sent> {
        let mut messages = self.messages.iter();
        let message = messages.find(|(direction, code, name, ..)| of(direction, *code, name));
        message.map(|(_, _, name, _, ies, ty)| Sent {
            name,
            ty: type_name(ty),
            ies,
        })
    }

    /// The shape of a view: that of a PDU, of the message that its direction and its
    /// procedure code say.
    fn pdu(&self, view: &Value) -> Shape {
        let code = view.get("procedure_code").and_then(Value::as_u64);
        let direction = view.get("direction").and_then(Value::as_str);
        Shape::Pdu(
            self.sent(|of, procedure, _| {
                Some(of) == direction && Some(u64::from(procedure)) == code
            }),
        )
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
        if self.carried.iter().any(|(carrier, ..)| *carrier == id) {
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
    pub fn members(&self, shape: Shape, entry: Option<&Map<String, Value>>) -> Option<Vec<Member>> {
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
                false => self.ie_value(of("_original_id").or(of("id")).or(id), extended),
            };
            vec![
                ("id", Shape::Plain, false),
                ("criticality", Shape::Plain, false),
                (value, decoded, true),
                (raw, Shape::Plain, true),
                ("_original_id", Shape::Plain, true),
                ("_ie_name", Shape::Plain, true),
                ("_decode_error", Shape::Plain, true),
                (EDITED, Shape::Plain, true),
            ]
        };
        Some(match shape {
            Shape::Unknown | Shape::Undecoded => return None,
            Shape::Pdu(message) => vec![
                ("procedure_code", Shape::Plain, false),
                ("direction", Shape::Plain, false),
                ("criticality", Shape::Plain, false),
                (
                    MESSAGE,
                    message.map_or(Shape::Unknown, Shape::Message),
                    false,
                ),
                ("_raw_message", Shape::Plain, true),
            ],
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
            Shape::Named(name) => match (self.kind_of)(name) {
                Some(Kind::Sequence(members) | Kind::Choice(members)) => {
                    // An extension container extends the type that has it.
                    let held = |(member, shape, optional): &Member| match shape {
                        Shape::Extensions(_) => (*member, Shape::Extensions(name), *optional),
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
                    (EDITED, Shape::Plain, true),
                ]
            }
            // Each type that the octets may carry, by the name that ASN.1 gives it.
            Shape::Carrier(id) => {
                let carried = self.carried.iter().filter(|(carrier, ..)| *carrier == id);
                carried
                    .map(|(_, name, ty)| (*name, Shape::Transfer(type_name(ty)), true))
                    .collect()
            }
            Shape::Plain | Shape::List(_) | Shape::Ies(_) | Shape::Extensions(_) => Vec::new(),
        })
    }

    /// The identifier of the IE that `entry` is, when a value of `shape` is an IE: an
    /// entry of a list of IEs, or an IE that a value holds alone.
    fn ie_of(&self, entry: &Value, shape: Shape) -> Option<u16> {
        match shape {
            Shape::Ie(_) | Shape::Extension(..) => {
                self.entry_id(entry).and_then(|id| id.try_into().ok())
            }
            _ => None,
        }
    }

    /// Whether `part`, a segment under the IE `id`, names that IE, and so stays on it: a
    /// path may say which IE an entry is. A member of its value that is named as the IE
    /// is that member, as a path had it before an IE could be named so, unless the
    /// segment spells the name as ASN.1 spells that of the IE. `extended` is the name of
    /// the type that the container of the IE extends, when it is an extension.
    pub fn stays_on(&self, id: u16, extended: &str, part: &str) -> bool {
        if (self.ie_id)(part) != Some(id) {
            return false;
        }
        let value = match self.resolved(self.ie_value(Some(id), extended)) {
            Shape::Transfer(contained) if !contained.is_empty() => Shape::Named(contained),
            value => value,
        };
        let of_the_value = self.members(value, None).unwrap_or_default();
        member(&of_the_value, part).is_none() || self.ie_name(id) == part
    }

    /// Whether a member of some value has the name `part` and another type than the
    /// value of the IE `id`, which extends the type that the bindings name `extended`
    /// when it is an extension: where an IE is not known, such a segment is not taken
    /// for its name.
    fn names_another_type(&self, part: &str, id: u16, extended: &str) -> bool {
        // A type that is another one names it a few times at most.
        let settled = |mut shape: Shape| {
            for _ in 0..16 {
                match shape {
                    Shape::Named(name) => match (self.kind_of)(name) {
                        Some(Kind::Is(is)) => shape = *is,
                        _ => break,
                    },
                    _ => break,
                }
            }
            shape
        };
        let of_the_ie = settled(self.ie_value(Some(id), extended));
        self.types.iter().any(|(_, kind)| match kind {
            Kind::Sequence(members) | Kind::Choice(members) => (members.iter())
                .any(|(name, shape, _)| same_name(name, part) && settled(*shape) != of_the_ie),
            Kind::Is(_) => false,
        })
    }

    /// The IE that a segment stays on when it names the IE that a value of `shape` is,
    /// for a value that a view does not have: a path may say which IE an entry is. An
    /// entry whose IE is not known is the one that the segment names, unless some value
    /// has a member of that name and of another type, which the segment may be.
    pub fn named_ie(&self, shape: Shape, part: &str, named: bool) -> Option<Shape> {
        let id = (self.ie_id)(part).filter(|_| named)?;
        let (known, extension) = match shape {
            Shape::Ie(known) => (known, None),
            Shape::Extension(known, extended) => (known, Some(extended)),
            _ => return None,
        };
        let extended = extended(shape);
        let stays = match known {
            Some(known) => known == id && self.stays_on(id, extended, part),
            None => !self.names_another_type(part, id, extended),
        };
        stays.then_some(match extension {
            Some(extended) => Shape::Extension(Some(id), extended),
            None => Shape::Ie(Some(id)),
        })
    }

    /// Why a segment that names an IE selects nothing under another IE, `this`: which IE
    /// that one is, then `reason`, why its value has no such member either. `None` for a
    /// segment that names no IE, or the IE itself.
    fn another_ie(
        &self,
        this: Option<u16>,
        part: &str,
        named: bool,
        reason: &tree::Error,
    ) -> Option<tree::Error> {
        let other = (self.ie_id)(part).filter(|_| named)?;
        let this = this.filter(|this| *this != other)?;
        let (this, other) = (self.ie_name(this), self.ie_name(other));
        Some(reworded(reason.clone(), |reason| {
            format!("this IE is {this}, not {other}: {reason}")
        }))
    }

    /// Why a segment selects nothing in a value of `shape`: the type has no such member.
    fn no_member(&self, shape: Shape, members: &[Member], part: &str) -> tree::Error {
        // The root stands for the IEs of a message, which a private message has none of.
        if let (Shape::Message(message), IES) = (shape, part) {
            let has: Vec<_> = members.iter().map(|(name, ..)| *name).collect();
            return fault(
                ErrorKind::NoSuchMember,
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
            Shape::Message(message) => message.name.to_string(),
            Shape::Pdu(_) => "a view".to_string(),
            Shape::Bits => "a bit string".to_string(),
            Shape::Transfer(_) => "a transfer".to_string(),
            Shape::Ie(_) | Shape::Extension(..) => "an IE".to_string(),
            _ => "this value".to_string(),
        };
        let names = names(shape, members);
        fault(
            ErrorKind::NoSuchMember,
            match names.is_empty() {
                true => format!("{part:?} is not a member of {of}, which has none"),
                false => format!(
                    "{part:?} is not a member of {of}, which has {}",
                    names.join(", ")
                ),
            },
        )
    }

    /// The criticality that ASN.1 assigns to the IE `id` in a container of `shape`,
    /// where the tables have it: that of the object set of a message for its IEs, and
    /// that of the single containers for an IE that a value holds alone or as an entry
    /// of a list. They have none for the IEs of a message that a transfer contains, nor
    /// for those of an extension container.
    fn assigned(&self, container: Shape, id: u16) -> Option<&'static str> {
        match container {
            Shape::Ies(Some(message)) => {
                let mut messages = self.message_ie_criticalities.iter();
                let (_, ies) = messages.find(|(name, _)| same_name(name, message.name))?;
                let ie = ies.iter().find(|(known, _)| *known == id);
                ie.map(|(_, criticality)| *criticality)
            }
            Shape::Ie(_) | Shape::List(Shape::Ie(_)) => {
                let mut items = self.item_criticalities.iter();
                items
                    .find(|(known, _)| *known == id)
                    .map(|(_, criticality)| *criticality)
            }
            _ => None,
        }
    }

    /// The message that ASN.1 names `name`, taken as a path takes a name.
    fn message_of(&self, name: &str) -> Result<Sent, Error> {
        let known = self.sent(|_, _, known| same_name(known, name));
        known.ok_or_else(|| {
            Error::new(
                ErrorKind::UnknownMessage,
                format!("{name:?} is not a message of {}", self.name),
            )
        })
    }

    /// The identifier of the IE that `name` names, as ASN.1 names it after `id-`.
    fn ie_named(&self, name: &str) -> Result<u16, Error> {
        (self.ie_id)(name).ok_or_else(|| {
            Error::new(
                ErrorKind::UnknownIe,
                format!("{name:?} is not an IE of {}", self.name),
            )
        })
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
fn member<'m>(members: &'m [Member], part: &str) -> Option<&'m Member> {
    let exact = members.iter().find(|(name, ..)| *name == part);
    exact.or_else(|| {
        let mut named = members.iter().filter(|(name, ..)| !name.starts_with('_'));
        named.find(|(name, ..)| same_name(name, part))
    })
}

/// The member of the entry of a view that a segment names, where no table says what
/// the entry has.
fn key<'e>(entry: &'e Map<String, Value>, part: &str) -> Option<&'e str> {
    let exact = entry.get_key_value(part).map(|(name, _)| name);
    let named = || {
        let mut names = entry.keys().filter(|name| !name.starts_with('_'));
        names.find(|name| same_name(name, part))
    };
    exact.or_else(named).map(String::as_str)
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
        .filter(|name| shown(name) && *name != "_decode_error")
        .collect();
    if has_octets(shape) {
        names.push("octets");
    }
    names
}

/// `reason`, why a segment selects nothing in the value of an IE or of a transfer of
/// `shape`, with the members that the IE or the transfer has itself: the segment may be
/// one of them, written wrong.
fn or_its_own(reason: tree::Error, shape: Shape, members: &[Member]) -> tree::Error {
    let of = match shape {
        Shape::Transfer(_) => "a transfer",
        _ => "an IE",
    };
    let own = names(shape, members).join(", ");
    reworded(reason, |reason| format!("{reason}; {of} itself has {own}"))
}

/// Why a segment that is no position selects no entry of a list.
fn no_position(part: &str) -> tree::Error {
    fault(
        ErrorKind::NoSuchMember,
        format!(
            "{part:?} is not a position in a list: an entry goes by its number, and \"*\" is \
             each of them"
        ),
    )
}

/// Why a segment that names an IE selects no entry of a list whose entries are no IEs.
fn no_ie_there(part: &str) -> tree::Error {
    fault(
        ErrorKind::NoSuchMember,
        format!("{part:?} selects an IE, and the entries of this list go by position"),
    )
}

/// Why a segment that starts with `@id=` selects no IE.
fn not_an_id(part: &str) -> tree::Error {
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
fn segment_id(protocol: &Protocol, part: &str, named: bool) -> Result<Option<u16>, tree::Error> {
    match part.strip_prefix("@id=") {
        Some(id) => id.parse::<u16>().map(Some).map_err(|_| not_an_id(part)),
        None => Ok((protocol.ie_id)(part).filter(|_| named)),
    }
}

/// The shape of an entry of a list of `shape`.
fn element(shape: Shape) -> Shape {
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
fn element_of(shape: Shape, id: u16) -> Option<Shape> {
    match element(shape) {
        Shape::Ie(_) => Some(Shape::Ie(Some(id))),
        Shape::Extension(_, extended) => Some(Shape::Extension(Some(id), extended)),
        Shape::Unknown => Some(Shape::Unknown),
        _ => None,
    }
}

/// Whether a member is one of a value, and not what a view keeps of what was received.
fn shown(name: &str) -> bool {
    !name.starts_with('_') || name == "_decode_error"
}

/// The octets that an IE or a transfer was received as.
fn received(entry: &Map<String, Value>) -> Option<(&'static str, &Value)> {
    let member = received_member(entry)?;
    Some((member, &entry[member]))
}

/// The member of an entry that holds the octets it was received as.
fn received_member(entry: &Map<String, Value>) -> Option<&'static str> {
    ["_raw_value", "_raw_transfer"]
        .into_iter()
        .find(|member| entry.contains_key(*member))
}

// ── The family of the views ─────────────────────────────────────────────────

/// What the family knows of a place of a view.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct At {
    /// What the value there is, as its type has it.
    shape: Shape,
    /// Whether the IEs on the way are selected by their names: the path starts with the
    /// root.
    named: bool,
    /// How many segments of the path were taken to the place: the root is short for
    /// two, and the segment after it selects among the IEs of the message.
    taken: u8,
    /// The place is an alternative of a CHOICE, which an edit puts in the place of the
    /// one there.
    alternative: bool,
    /// The place is the octets that an IE or a transfer is sent as, which an edit sets
    /// in the place of its value.
    octets: bool,
}

impl At {
    /// A place of `shape` that the segment at this place selects.
    fn of(self, shape: Shape) -> Self {
        Self {
            shape,
            named: self.named,
            taken: self.taken.saturating_add(1),
            ..Self::default()
        }
    }

    /// A place of `shape` in which the segment at this place is taken, as if the name
    /// of the place stood before it.
    fn inside(self, shape: Shape) -> Self {
        Self {
            shape,
            named: self.named,
            taken: self.taken,
            ..Self::default()
        }
    }
}

/// What makes, of octets and of the name of a type that they may carry, the value that a
/// view shows.
type Opener<'e> = &'e dyn Fn(Value, &str) -> Value;

/// The views of a protocol, as a family of the engine: the rules of its paths.
struct Views<'e> {
    protocol: &'static Protocol,
    /// The message that a path is read against where no view is given.
    message: Option<Sent>,
    /// Whether what an edit sets is text, which the octets of an IE are.
    written_text: bool,
    /// What makes, of octets and of the name of a type that they may carry, the value
    /// that a view shows: for `open`.
    opens: Option<Opener<'e>>,
}

impl<'e> Views<'e> {
    fn new(protocol: &'static Protocol) -> Self {
        Self {
            protocol,
            message: None,
            written_text: false,
            opens: None,
        }
    }
}

impl Family for Views<'_> {
    type Shape = At;

    fn root(&self) -> Root<'_> {
        Root {
            name: self.protocol.root,
            stands_for: &[MESSAGE, IES],
            required: false,
        }
    }

    fn start(&self, tree: &Value, rooted: bool) -> At {
        let shape = match self.message {
            Some(message) => Shape::Pdu(Some(message)),
            None => self.protocol.pdu(tree),
        };
        At {
            shape,
            named: rooted,
            ..At::default()
        }
    }

    fn step<'v>(
        &self,
        place: &Place<'v, At>,
        part: &str,
        next: &mut Next<'v, At>,
    ) -> Result<Step<At>, tree::Error> {
        let taken = self.take(place, part, next);
        // An edit is done at each place that its path selects, or at none: a place that
        // cannot take it refuses the edit, where a selection takes what the others have.
        match (taken, next.edit()) {
            (Err(reason), Some(_)) if reason.kind() != tree::ErrorKind::Refused => {
                let again = refusal(ErrorKind::NoSuchMember, reason.reason());
                Err(match reason.code() {
                    Some(code) => again.with_code(code),
                    None => again,
                })
            }
            (taken, _) => taken.map(|()| Step::Taken),
        }
    }

    fn edited(&self, tree: &mut Value, keys: &[Key], at: &At, edit: Edit) {
        let Some((last, before)) = keys.split_last() else {
            return;
        };
        // Each value of an IE or of a transfer that the edit changed something under is
        // no longer what its octets say.
        let mark = |entry: &mut Map<String, Value>, name: &str| {
            if VALUES.contains(&name) && received_member(entry).is_some() {
                entry.insert(EDITED.into(), true.into());
            }
        };
        let mut holder = &mut *tree;
        for key in before {
            if let (Key::Name(name), Value::Object(entry)) = (key, &mut *holder) {
                mark(entry, name);
            }
            holder = match (key, holder) {
                (Key::Name(name), Value::Object(entry)) => match entry.get_mut(name.as_str()) {
                    Some(value) => value,
                    None => return,
                },
                (Key::Position(position), Value::Array(entries)) => {
                    match entries.get_mut(*position) {
                        Some(value) => value,
                        None => return,
                    }
                }
                _ => return,
            };
        }
        let (Key::Name(name), Value::Object(entry)) = (last, holder) else {
            return;
        };
        if edit != Edit::Set {
            return;
        }
        mark(entry, name);
        // The octets of an IE or of a transfer are sent in the place of its value.
        if at.octets {
            for value in VALUES.into_iter().chain(["_decode_error", EDITED]) {
                entry.shift_remove(value);
            }
        }
        // A CHOICE has one alternative.
        if at.alternative {
            entry.retain(|member, _| member == name);
        }
    }
}

impl Views<'_> {
    /// What `part` selects at `place`, by the rules of the views: [`Family::step`] with
    /// the errors of a selection, which a member that a segment is taken in words as its
    /// own.
    fn take<'v>(
        &self,
        place: &Place<'v, At>,
        part: &str,
        next: &mut Next<'v, At>,
    ) -> Result<(), tree::Error> {
        let at = *place.shape();
        // An identifier that is no number is no path, wherever it is.
        if let Some(id) = part.strip_prefix("@id=")
            && id.parse::<u16>().is_err()
        {
            return Err(not_an_id(part));
        }
        if at.named && at.taken == 2 && !self.protocol.selects_ies(part) {
            return Err(refusal(
                ErrorKind::UnknownIe,
                format!("{part:?} is not an IE of {}", self.protocol.name),
            ));
        }
        let shape = self.protocol.resolved(at.shape);
        match place.value() {
            Some(whole @ Value::Object(entry)) => {
                self.object(place, whole, entry, at, shape, part, next)?
            }
            Some(Value::Array(entries)) => self.list(entries, at, shape, part, next)?,
            // A value that is not there, by what its type says.
            Some(Value::Null) | None => {
                if next.edit().is_none() {
                    self.typed(at, shape, part, next)?;
                }
            }
            Some(value) => self.scalar(place, value, at, shape, part, next)?,
        }
        Ok(())
    }

    /// What `part` selects in an object of a view: a member of the value, the IEs of a
    /// message that it holds, the octets of an IE, or a member of its value.
    #[allow(clippy::too_many_arguments)]
    fn object<'v>(
        &self,
        place: &Place<'v, At>,
        whole: &'v Value,
        entry: &'v Map<String, Value>,
        at: At,
        shape: Shape,
        part: &str,
        next: &mut Next<'v, At>,
    ) -> Result<(), tree::Error> {
        let protocol = self.protocol;
        let (edit, last, named) = (next.edit(), next.is_last(), at.named);
        let members = protocol.members(shape, Some(entry));
        // The IEs of a message that the view contains are selected in its place.
        if named
            && protocol.selects_ies(part)
            && !entry.contains_key(part)
            && let Some(Value::Array(_)) = entry.get(IES)
        {
            let of = members.as_deref().and_then(|members| member(members, IES));
            let ies = at.inside(of.map_or(Shape::Ies(None), |(_, shape, _)| *shape));
            return next.through(place, IES, ies, |inside, next| {
                self.take(inside, part, next)
            });
        }
        if part == "*" {
            if edit.is_some() {
                return Err(refusal(
                    ErrorKind::NoSuchMember,
                    format!("{part:?} selects among the entries of a list, and this is no list"),
                ));
            }
            for (name, value) in entry.iter().filter(|(name, _)| shown(name)) {
                let known = members.as_deref().and_then(|members| member(members, name));
                let of = known.map_or(Shape::Unknown, |(_, shape, _)| *shape);
                next.member(name, value, at.of(of));
            }
            return Ok(());
        }
        // The value of an IE or of a transfer, in which a segment that names none of its
        // members is taken, as if the member that holds it stood before.
        let held = value_member(shape);
        if part.starts_with("@id=") && held.is_none() {
            return Err(fault(
                ErrorKind::NoSuchMember,
                format!("{part:?} selects among the IEs of a list, and this is no list"),
            ));
        }
        // `octets` is what an IE or a transfer was received as, and what an edit sends in
        // the place of its value.
        if same_name(part, "octets") {
            if let Some(edit) = edit {
                let raw = received_member(entry).or(match shape {
                    Shape::Ie(_) | Shape::Extension(..) => Some("_raw_value"),
                    Shape::Transfer(_) => Some("_raw_transfer"),
                    _ => None,
                });
                if let Some(raw) = raw.filter(|_| last) {
                    let octets = At {
                        octets: true,
                        ..at.of(Shape::Plain)
                    };
                    match (edit, self.written_text) {
                        (Edit::Set, true) => match entry.get(raw) {
                            Some(value) => next.member(raw, value, octets),
                            None => next.absent(raw, octets),
                        },
                        (Edit::Open, _) => {}
                        _ => {
                            return Err(refusal(ErrorKind::Edit, "octets are set, in hexadecimal"));
                        }
                    }
                    return Ok(());
                }
            } else if has_octets(shape) || received(entry).is_some() {
                if entry.contains_key(EDITED) {
                    return Err(refusal(
                        ErrorKind::Edit,
                        "the value was edited since these octets were received",
                    ));
                }
                match received(entry) {
                    Some((name, octets)) => next.member(name, octets, at.of(Shape::Plain)),
                    None => next.absent_once(part, at.of(Shape::Plain)),
                }
                return Ok(());
            }
        }
        let Some(members) = members else {
            // No table says what the value can have: it has what the view has.
            let name = key(entry, part);
            if let Some(edit) = edit
                && last
            {
                refuse(edit, name.unwrap_or(part), entry, false, true)?;
            }
            return match name {
                Some(name) => {
                    next.member(name, &entry[name], at.of(Shape::Unknown));
                    Ok(())
                }
                None if edit.is_some() && last => {
                    next.absent(part, at.of(Shape::Unknown));
                    Ok(())
                }
                None => Err(fault(
                    ErrorKind::NoSuchMember,
                    format!("{part:?} is not a member of this value"),
                )),
            };
        };
        let own = member(&members, part);
        // The name of the IE that the entry is stays on the entry: a path may say which
        // IE an entry is, before it goes into it.
        let this = protocol
            .ie_of(whole, shape)
            .filter(|_| named && own.is_none());
        if this.is_some_and(|this| protocol.stays_on(this, extended(shape), part)) {
            next.stay(place, at);
            return Ok(());
        }
        let held = held.and_then(|held| member(&members, held));
        let Some((name, of, optional)) = own.or(held).copied() else {
            return Err(protocol.no_member(shape, &members, part));
        };
        let value = match entry.get(name) {
            // An edit changes nothing in what did not decode.
            None if name == "decoded" && entry.contains_key("_decode_error") && edit.is_some() => {
                return Ok(());
            }
            None if name == "decoded" && entry.contains_key("_decode_error") => {
                let error = entry["_decode_error"].as_str().unwrap_or_default();
                return Err(refusal(
                    ErrorKind::UndecodableIe,
                    format!("the transfer did not decode: {error}"),
                ));
            }
            value => value,
        };
        if own.is_some() {
            let alternative = matches!(shape, Shape::Named(name) if matches!((protocol.kind_of)(name), Some(Kind::Choice(_))));
            if let Some(edit) = edit
                && last
            {
                refuse(edit, name, entry, alternative, optional)?;
            }
            let child = At {
                alternative,
                ..at.of(of)
            };
            match value {
                Some(value) => next.member(name, value, child),
                // A member that an edit sets is added; one that is not there is nothing
                // to go into.
                None if edit.is_some() => {
                    if last && edit == Some(Edit::Set) {
                        next.absent(name, child);
                    }
                }
                None => next.absent_once(name, child),
            }
            return Ok(());
        }
        let taken = next.through(place, name, at.inside(of), |inside, next| {
            self.take(inside, part, next)
        });
        taken.map_err(|reason| match reason.kind() {
            tree::ErrorKind::Refused => reason,
            _ => match protocol.another_ie(this, part, named, &reason) {
                Some(another) => another,
                None => or_its_own(reason, shape, &members),
            },
        })
    }

    /// What `part` selects in a list of a view: an entry by its position, an IE by its
    /// name or `@id=N`, each entry, or the end of the list for an insertion.
    fn list<'v>(
        &self,
        entries: &'v [Value],
        at: At,
        shape: Shape,
        part: &str,
        next: &mut Next<'v, At>,
    ) -> Result<(), tree::Error> {
        let protocol = self.protocol;
        let of = at.of(element(shape));
        if part == "*" {
            for (index, entry) in entries.iter().enumerate() {
                next.entry(index, entry, of);
            }
            return Ok(());
        }
        if part == "-" && next.edit() == Some(Edit::Insert) && next.is_last() {
            next.end(of);
            return Ok(());
        }
        if let Some(id) = segment_id(protocol, part, at.named)? {
            let Some(absent) = element_of(shape, id) else {
                return Err(no_ie_there(part));
            };
            let mut found = false;
            for (index, entry) in entries.iter().enumerate() {
                if protocol.entry_id(entry) == Some(u64::from(id)) {
                    next.entry(index, entry, of);
                    found = true;
                }
            }
            // An IE that is not there: what follows is still one of its paths.
            if !found && next.edit().is_none() {
                next.absent_once(part, at.of(absent));
            }
            return Ok(());
        }
        let Some(index) = position(part) else {
            return Err(no_position(part));
        };
        if let Some(entry) = entries.get(index) {
            next.entry(index, entry, of);
        }
        Ok(())
    }

    /// What `part` selects in a value of `shape` that a view does not have: the places
    /// of the values that it could have.
    fn typed(
        &self,
        at: At,
        shape: Shape,
        part: &str,
        next: &mut Next<'_, At>,
    ) -> Result<(), tree::Error> {
        let protocol = self.protocol;
        let named = at.named;
        match shape {
            Shape::Unknown | Shape::Undecoded => next.absent_once(part, at.of(Shape::Unknown)),
            Shape::List(_) | Shape::Ies(_) | Shape::Extensions(_) => {
                if let Some(id) = segment_id(protocol, part, named)? {
                    let Some(of) = element_of(shape, id) else {
                        return Err(no_ie_there(part));
                    };
                    // A name is that of an IE of the object set, where the tables have it.
                    if let Shape::Ies(Some(message)) = shape
                        && !part.starts_with("@id=")
                        && !message.ies.iter().any(|(known, _)| *known == id)
                    {
                        let ies: Vec<_> = (message.ies.iter())
                            .map(|(id, _)| protocol.ie_name(*id))
                            .collect();
                        return Err(fault(
                            ErrorKind::NoSuchMember,
                            format!(
                                "{part:?} is not an IE of {}, which has {}",
                                message.name,
                                ies.join(", ")
                            ),
                        ));
                    }
                    next.absent_once(part, at.of(of));
                } else if matches!(part, "*" | "-") || position(part).is_some() {
                    match shape {
                        Shape::Ies(Some(message)) => {
                            for (id, _) in message.ies {
                                next.absent_once(part, at.of(Shape::Ie(Some(*id))));
                            }
                        }
                        shape => next.absent_once(part, at.of(element(shape))),
                    }
                } else {
                    return Err(no_position(part));
                }
            }
            shape => {
                let members = protocol.members(shape, None).unwrap_or_default();
                // The IEs of a message that a value contains go in its place.
                if named
                    && protocol.selects_ies(part)
                    && member(&members, part).is_none()
                    && let Some((_, ies, _)) = member(&members, IES)
                {
                    return self.typed(at, *ies, part, next);
                }
                if part == "*" {
                    for (name, of, _) in members.iter().filter(|(name, ..)| shown(name)) {
                        next.absent_once(name, at.of(*of));
                    }
                } else if same_name(part, "octets") && has_octets(shape) {
                    next.absent_once(part, at.of(Shape::Plain));
                } else if let Some((name, of, _)) = member(&members, part) {
                    next.absent_once(name, at.of(*of));
                } else if let Some(ie) = protocol.named_ie(shape, part, named) {
                    // The name of the IE that the entry is stays on the entry.
                    next.absent_once(part, at.of(ie));
                } else if let Some((_, of, _)) =
                    value_member(shape).and_then(|held| member(&members, held))
                {
                    // What names no member of an IE or of a transfer is one of its value.
                    let this = match shape {
                        Shape::Ie(known) | Shape::Extension(known, _) => known,
                        _ => None,
                    };
                    let taken = self.typed(at, protocol.resolved(*of), part, next);
                    return taken.map_err(|reason| match reason.kind() {
                        tree::ErrorKind::Refused => reason,
                        _ => match protocol.another_ie(this, part, named, &reason) {
                            Some(another) => another,
                            None => or_its_own(reason, shape, &members),
                        },
                    });
                } else {
                    return Err(protocol.no_member(shape, &members, part));
                }
            }
        }
        Ok(())
    }

    /// What `part` selects in a value without members: nothing, but for a transfer that
    /// is its octets and octets that may carry a type.
    fn scalar<'v>(
        &self,
        place: &Place<'v, At>,
        value: &'v Value,
        at: At,
        shape: Shape,
        part: &str,
        next: &mut Next<'v, At>,
    ) -> Result<(), tree::Error> {
        let protocol = self.protocol;
        match shape {
            // An edit changes nothing in what did not decode.
            Shape::Undecoded if next.edit().is_some() => {}
            Shape::Undecoded => {
                return Err(refusal(ErrorKind::UndecodableIe, "the IE did not decode"));
            }
            // A value without members has none to select.
            _ if part == "*" => {}
            // A bit string that the view shows as it is usually written.
            Shape::Bits if ["value", "length"].contains(&part) => {}
            // A transfer that an edit replaced by its octets.
            Shape::Transfer(_) if same_name(part, "octets") => {
                next.stay(place, at.of(Shape::Plain));
            }
            Shape::Transfer(_) if same_name(part, "decoded") => {}
            // What is no member of a transfer is one of the decoded value, which a
            // transfer that is its octets has not.
            Shape::Transfer(contained) if !contained.is_empty() => {
                if next.edit().is_none() {
                    let taken = self.typed(at, Shape::Named(contained), part, next);
                    let own = protocol.members(shape, None).unwrap_or_default();
                    taken.map_err(|reason| match reason.kind() {
                        tree::ErrorKind::Refused => reason,
                        _ => or_its_own(reason, shape, &own),
                    })?;
                }
            }
            // Octets that may carry a type are that type once they are opened.
            Shape::Carrier(_) => {
                let carried = protocol.members(shape, None).unwrap_or_default();
                return match member(&carried, part) {
                    Some((name, ..)) => match (next.edit(), self.opens, value) {
                        (Some(Edit::Open), Some(decode), Value::String(_)) => {
                            let mut opened = Map::new();
                            opened.insert(name.to_string(), decode(value.clone(), name));
                            next.open(place, Value::Object(opened));
                            Ok(())
                        }
                        _ => Err(refusal(
                            ErrorKind::NotOpened,
                            format!("the octets are not shown as a {name}: open decodes them"),
                        )),
                    },
                    None => Err(protocol.no_member(shape, &carried, part)),
                };
            }
            Shape::Plain => return Err(protocol.no_member(shape, &[], part)),
            _ => {
                return Err(fault(
                    ErrorKind::NoSuchMember,
                    format!("{part:?} is not a member of this value, which has none"),
                ));
            }
        }
        Ok(())
    }
}

/// Why an edit at the member `name` of `entry` is refused: what holds the value of an IE
/// or of a transfer, an alternative of a CHOICE and a member that its type always has
/// are not taken out, and insert adds to a list.
fn refuse(
    edit: Edit,
    name: &str,
    entry: &Map<String, Value>,
    alternative: bool,
    optional: bool,
) -> Result<(), tree::Error> {
    let refused = |reason: String| Err(refusal(ErrorKind::Edit, reason));
    match edit {
        Edit::Remove => {
            // Without the value the octets received would be sent, and the edit would
            // change nothing.
            let value = match name {
                "value" | "extensionValue" => entry.contains_key("id"),
                "decoded" => entry.contains_key("_raw_transfer"),
                _ => false,
            };
            if value {
                return refused(format!(
                    "{name} is not taken out: remove what has it, or set its octets"
                ));
            }
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
        Edit::Set | Edit::Open => Ok(()),
    }
}

// ── The calls ───────────────────────────────────────────────────────────────

/// The values at `path` in `view`, a view of a PDU of `protocol`, in the order of the
/// view.
///
/// An IE that the message does not have selects nothing, and so does a member that its
/// type can have and the value does not have: an OPTIONAL member that is absent, or
/// another alternative of a CHOICE. A name that the type of the value cannot have is an
/// error, which says the members that it has. `*` is each entry of a list or each member
/// of a value, without those that start with `_`; after it, a segment is an error when
/// none of the values can have it.
///
/// A segment that names no member of an IE is taken in its value, and one that names no
/// member of a transfer in its decoded value, as if `value` or `decoded` stood before
/// it. A name that neither has is an error, which says the members of both.
///
/// After an IE, a segment that is the name of that IE stays on it: a path may say which
/// IE an entry is, as `0/Cause` for the first IE of a list, and one that ends so selects
/// the IE. The name of another IE, where the value has no such member, is an error that
/// says which IE the entry is.
///
/// The `octets` of an IE or of a transfer are what it was received as: those of one whose
/// value [`set`], [`remove`] or [`insert`] edited since are an error, as the view has yet
/// to give it others. So is a path into a value that did not decode.
///
/// # Errors
///
/// What the path cannot select, as said above, and a path that is not well formed.
pub fn select<'v>(
    protocol: &'static Protocol,
    view: &'v Value,
    path: &str,
) -> Result<Vec<&'v Value>, Error> {
    tree::select_in(&Views::new(protocol), view, path).map_err(error)
}

/// Whether `path` can select a value in a message of `protocol` that ASN.1 names
/// `message`: what [`select`] says of a path without a view to select in.
///
/// The name is taken whatever its case and what is between its letters. Under the root,
/// the name of an IE has to be that of an IE of the object set of the message; under an
/// IE, each segment has to be a member that the type of the value has, through the
/// lists, the alternatives of a CHOICE and the decoded value of a transfer. A position
/// or `*` stands for each entry that a list could have, and `-` for the end of a list.
///
/// An IE of an extension container is read against the type that the type which the
/// container extends gives it, where its identifier alone does not say its type.
///
/// What is not known is not refused: the value of an IE selected by `@id=N` when the
/// identifier is unknown or has no type where it is, the entries of a container that is
/// not that of the message when they are selected by position, a segment under such an
/// IE, which may name a member of its value, and whether a position is within the bounds
/// of its list. Such an entry is the IE that the next segment names, and the path is
/// then read against the type of that IE: unless some value has a member of that name
/// and of another type, which the segment may be.
///
/// # Errors
///
/// A message that the protocol does not have, and what [`select`] refuses.
pub fn check_path(protocol: &'static Protocol, message: &str, path: &str) -> Result<(), Error> {
    let message = protocol.message_of(message)?;
    let views = Views {
        message: Some(message),
        ..Views::new(protocol)
    };
    let places = tree::places_in(&views, &Value::Null, path).map_err(error)?;
    if !places.is_empty() {
        return Ok(());
    }
    // The first segment after which nothing is selected: a value before it has no
    // members.
    let parts = tree::segments(path).map_err(error)?;
    for end in 1..=parts.len() {
        let prefix: String = parts[..end]
            .iter()
            .map(|part| format!("/{}", tree::escape(part)))
            .collect();
        let selected = tree::places_in(&views, &Value::Null, &prefix).map_err(error)?;
        if selected.is_empty() {
            return Err(Error::new(
                ErrorKind::NoSuchMember,
                format!(
                    "{path} selects nothing in a message {}: a value before {:?} has no members",
                    message.name,
                    parts[end - 1]
                ),
            ));
        }
    }
    Ok(())
}

/// The names that the value at `path` can have in a message of `protocol` that ASN.1
/// names `message`, when its type is an ENUMERATED: the names of its values as the
/// specification spells them, in the order of its definition. A name is compared as
/// [`select`] compares one, whatever its case and whatever is between its letters. A
/// name that is not one of them is never the value of a message, so a caller can refuse
/// it before it compares one.
///
/// `None` for a value of another type, and for one whose type is not known: what
/// [`check_path`] does not refuse for that reason.
///
/// # Errors
///
/// What [`check_path`] refuses.
pub fn enumerated_at(
    protocol: &'static Protocol,
    message: &str,
    path: &str,
) -> Result<Option<&'static [&'static str]>, Error> {
    let message = protocol.message_of(message)?;
    let views = Views {
        message: Some(message),
        ..Views::new(protocol)
    };
    let places = tree::places_in(&views, &Value::Null, path).map_err(error)?;
    // The value is of one type, wherever the path finds it.
    let mut names = None;
    for place in places {
        let mut shape = place.shape().shape;
        // A type that is another one names it a few times at most.
        for _ in 0..16 {
            match shape {
                Shape::Named(name) => match (protocol.kind_of)(name) {
                    Some(Kind::Is(is)) => shape = *is,
                    _ => break,
                },
                _ => break,
            }
        }
        let Shape::Named(name) = shape else {
            return Ok(None);
        };
        match (protocol.enumeration(name), names) {
            (Some(of), None) => names = Some(of),
            (Some(of), Some(other)) if of == other => {}
            _ => return Ok(None),
        }
    }
    Ok(names)
}

// ── Paths ───────────────────────────────────────────────────────────────────

/// What the listing of the paths of a view knows of a place.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Listing {
    /// Another value.
    #[default]
    Other,
    /// The view of the PDU.
    View,
    /// The message of the PDU, whose IEs go under the root.
    Message,
    /// A list of IEs, which go by their names.
    Ies,
    /// An IE or a transfer that has the octets that it was received as and no value
    /// beside them: they are listed as its `octets`.
    Octets,
}

/// The paths of the views of a protocol.
struct Paths(&'static Protocol);

impl Paths {
    fn of(value: Option<&Value>) -> Listing {
        match value.and_then(Value::as_object) {
            Some(entry)
                if received(entry).is_some()
                    && !VALUES.iter().any(|value| entry.contains_key(*value)) =>
            {
                Listing::Octets
            }
            _ => Listing::Other,
        }
    }
}

impl Family for Paths {
    type Shape = Listing;

    fn root(&self) -> Root<'_> {
        Root {
            name: self.0.root,
            stands_for: &[MESSAGE, IES],
            required: false,
        }
    }

    fn start(&self, _tree: &Value, _rooted: bool) -> Listing {
        Listing::View
    }

    fn member(&self, shape: &Listing, name: &str, value: Option<&Value>) -> Listing {
        match (shape, name) {
            (Listing::View, MESSAGE) => Listing::Message,
            (_, IES) if value.is_some_and(Value::is_array) => Listing::Ies,
            _ => Self::of(value),
        }
    }

    fn entry(&self, _shape: &Listing, value: Option<&Value>) -> Listing {
        Self::of(value)
    }

    fn listed(&self, shape: &Listing, name: &str) -> Listed {
        match (shape, name) {
            (Listing::Octets, "_raw_value" | "_raw_transfer") => Listed::As("octets"),
            (_, name) if !shown(name) => Listed::Hidden,
            // The IEs of a message go by name: under the root for the message of the
            // PDU, and in the place of a message that the view contains.
            (Listing::Message, IES) => Listed::Root,
            (_, IES) => Listed::Through,
            _ => Listed::Member,
        }
    }

    fn entry_names(&self, shape: &Listing, list: &[Value]) -> Option<Vec<Option<String>>> {
        if *shape != Listing::Ies {
            return None;
        }
        // The list is read once, whatever its length: the identifiers are counted for
        // all of its IEs, not for each.
        let ids: Vec<Option<u64>> = list.iter().map(|ie| self.0.entry_id(ie)).collect();
        let mut times: BTreeMap<u64, usize> = BTreeMap::new();
        for id in ids.iter().flatten() {
            *times.entry(*id).or_default() += 1;
        }
        let name = |id: &Option<u64>| {
            let id = (*id)?;
            let known = u16::try_from(id).ok().filter(|_| times[&id] == 1)?;
            self.0.ie_name_of(known).map(str::to_string)
        };
        Some(ids.iter().map(name).collect())
    }
}

/// Each value of `view`, a view of a PDU of `protocol`, with the path that selects it,
/// in the order of the view. A list of plain values is one value. The IEs of the
/// message go by name under the root, and those of a message that the view contains by
/// name in its place; an IE goes by its position when it has no name or its identifier
/// is there twice.
///
/// The members that start with `_` repeat what was received and are left out, except
/// `_decode_error`. The `octets` of an IE or of a transfer are listed when it has no
/// value beside them: those of a value repeat it, and are selected all the same.
///
/// What is nested deeper than a path has segments, 64, is one value at the deepest
/// path, so every path that is listed is one that [`select`] takes.
pub fn paths(protocol: &'static Protocol, view: &Value) -> Vec<(String, Value)> {
    tree::paths_in(&Paths(protocol), view)
}

// ── Edits ───────────────────────────────────────────────────────────────────

/// Write the entry of an IE of `protocol` as a view has it: the name of the IE that its
/// `id` may be written as becomes its number, and its `octets` what it is sent as in
/// place of a value. An entry of a list of IEs has an `id`, which no member of an ASN.1
/// type is named beside a string or a number.
///
/// # Errors
///
/// An `id` that names no IE of the protocol, and an IE written with its value and its
/// octets.
pub fn written_ie(
    protocol: &'static Protocol,
    entry: &mut Map<String, Value>,
) -> Result<(), Error> {
    let id = match entry.get("id") {
        Some(Value::String(name)) => Some(json!(protocol.ie_named(name)?)),
        Some(Value::Number(_)) => None,
        _ => return Ok(()),
    };
    if let Some(id) = id {
        entry.insert("id".into(), id);
    }
    if let Some(octets) = entry.shift_remove("octets") {
        if VALUES.iter().any(|value| entry.contains_key(*value)) {
            return Err(Error::new(
                ErrorKind::Form,
                "an IE is written with its value or its octets, not both",
            ));
        }
        entry.insert("_raw_value".into(), octets);
    }
    Ok(())
}

/// The value of an edit, with the IE that it may be written as the view has it.
fn as_viewed(protocol: &'static Protocol, mut value: Value) -> Result<Value, Error> {
    if let Some(entry) = value.as_object_mut() {
        written_ie(protocol, entry)?;
    }
    Ok(value)
}

/// Give the member or the list entry at `path` the value `value`, in `view`, a view of a
/// PDU of `protocol`. `null` takes an optional member out. An IE is written as
/// `{id, criticality, value}` with its typed value, or with its `octets` in hexadecimal
/// in place of `value`; its `id` is a number or the name of the IE. The `octets` of an
/// IE or of a transfer are set in place of its value. An alternative of a CHOICE takes
/// the place of the one that is there.
///
/// # Errors
///
/// A path that selects nothing, and the view is then as it was. So is a name that the
/// type of a value cannot have, a member that its type always has and that would be
/// taken out, and an edit that would send nothing else: the value of an IE or the
/// decoded value of a transfer is not taken out, as the octets received would go out in
/// its place.
pub fn set(
    protocol: &'static Protocol,
    view: &mut Value,
    path: &str,
    value: Value,
) -> Result<(), Error> {
    let value = as_viewed(protocol, value)?;
    let views = Views {
        written_text: value.is_string(),
        ..Views::new(protocol)
    };
    tree::set_in(&views, view, path, value).map_err(error)
}

/// Take the member or the list entries at `path` out of `view`, a view of a PDU of
/// `protocol`.
///
/// # Errors
///
/// A path that selects nothing, and a member that its type always has.
pub fn remove(protocol: &'static Protocol, view: &mut Value, path: &str) -> Result<(), Error> {
    tree::remove_in(&Views::new(protocol), view, path).map_err(error)
}

/// Add `value` to a list of `view`, a view of a PDU of `protocol`: before each entry
/// that `path` selects, or at the end for a path that ends with `-`. An IE is written as
/// [`set`] takes it.
///
/// # Errors
///
/// A path that selects nothing, and one that ends at a member.
pub fn insert(
    protocol: &'static Protocol,
    view: &mut Value,
    path: &str,
    value: Value,
) -> Result<(), Error> {
    let value = as_viewed(protocol, value)?;
    tree::insert_in(&Views::new(protocol), view, path, value).map_err(error)
}

/// Decode, in `view`, a view of a PDU of `protocol`, the octets that `path` takes as a
/// type that they may carry: those of an IE whose content the specification gives by
/// reference, where the segment after the IE or its `value` is the name that ASN.1 gives
/// one of these types. The IE then has, in place of its octets, the transfer of that type
/// under that name: its decoded value beside the octets, or why they are not of that
/// type. A path that goes through no such name changes nothing.
///
/// # Errors
///
/// What [`select`] refuses: the view is then as it was.
pub fn open(protocol: &'static Protocol, view: &mut Value, path: &str) -> Result<(), Error> {
    let opened = |raw: Value, name: &str| protocol.opened(raw, name);
    let views = Views {
        opens: Some(&opened),
        ..Views::new(protocol)
    };
    tree::open_in(&views, view, path).map_err(error)
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
    pub fn member_shapes(&self, shape: Shape, entry: &Map<String, Value>) -> Members {
        let shape = match self.resolved(shape) {
            // A transfer that is written as its decoded value itself.
            Shape::Transfer(contained)
                if !contained.is_empty() && !entry.keys().any(|name| of_a_transfer(name)) =>
            {
                self.resolved(Shape::Named(contained))
            }
            shape => shape,
        };
        if let Shape::Named(name) = shape
            && let Some(Kind::Sequence(members) | Kind::Choice(members)) = (self.kind_of)(name)
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
    pub fn message_shape(&self, direction: &str, code: u8) -> Shape {
        let message = self.sent(|of, procedure, _| procedure == code && of == direction);
        message.map_or(Shape::Unknown, Shape::Message)
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
    fn by_name(&self, value: Value, shape: Shape) -> Result<Value, Error> {
        let shape = self.resolved(shape);
        match value {
            Value::Object(mut entry) => match shape {
                Shape::Transfer(_) => {
                    let members = self.members(shape, Some(&entry)).unwrap_or_default();
                    let of = member(&members, "decoded").map(|(_, of, _)| *of);
                    match (entry.shift_remove("decoded"), of) {
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
                        *value = self.by_name(value.take(), member_shape(&members, name))?;
                    }
                    Ok(Value::Object(entry))
                }
            },
            Value::Array(mut entries) => {
                let of = element(shape);
                for entry in &mut entries {
                    *entry = match (of, entry.take()) {
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
    fn ie_by_name(
        &self,
        mut entry: Map<String, Value>,
        shape: Shape,
        container: Shape,
    ) -> Result<Value, Error> {
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
        let undecoded = entry.contains_key("_decode_error") || value.get("_decode_error").is_some();
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
    /// as the entry that [`entry_of`](Self::entry_of) gives it. A transfer keeps the form
    /// that it is written in, its decoded value alone or as its `decoded` member.
    fn written(&self, value: &Value, shape: Shape, depth: usize) -> Result<Value, Error> {
        if depth > WRITTEN {
            return Err(Error::new(
                ErrorKind::Limit,
                format!("a written tree nests deeper than {WRITTEN}"),
            ));
        }
        let shape = self.resolved(shape);
        match value {
            Value::Object(entry) => match shape {
                Shape::Transfer(_) => {
                    let members = self.members(shape, None).unwrap_or_default();
                    let decoded = member(&members, "decoded");
                    let decoded = decoded.map_or(Shape::Unknown, |(_, of, _)| *of);
                    if !entry.keys().any(|name| of_a_transfer(name)) {
                        return self.written(value, decoded, depth + 1);
                    }
                    let mut entry = entry.clone();
                    if let Some(value) = entry.get_mut("decoded") {
                        *value = self.written(value, decoded, depth + 1)?;
                    }
                    Ok(Value::Object(entry))
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
    /// specification assigns it there. An entry that is written as a view has it, with
    /// its `id`, stays as it is, and the IEs that its value holds are read the same way.
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
            .ok_or_else(|| Error::new(ErrorKind::Form, IE))?;
        if entry.contains_key("id") {
            let id = self.entry_id(ie).and_then(|id| u16::try_from(id).ok());
            let mut entry = entry.clone();
            if let Some(value) = entry.get_mut(field) {
                *value = self.written(value, self.ie_value(id, extended(shape)), depth + 1)?;
            }
            return Ok(Value::Object(entry));
        }
        let mut members = entry.iter().filter(|(member, _)| *member != "criticality");
        let (Some((name, value)), None) = (members.next(), members.next()) else {
            return Err(Error::new(ErrorKind::Form, IE));
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
            (None, Some(criticality), _) => json!(criticality),
            (None, None, Shape::Ies(Some(message))) => {
                return Err(Error::new(
                    ErrorKind::Form,
                    format!(
                        "{} has no IE {name}: one that is sent in it says its \"criticality\"",
                        message.name
                    ),
                ));
            }
            (None, None, _) => {
                return Err(Error::new(
                    ErrorKind::Form,
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
    Error::new(ErrorKind::Form, "an IE has its identifier")
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
