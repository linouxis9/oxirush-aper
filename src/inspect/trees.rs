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

//! The values of the tree of a PDU by their paths, as the family of the trees of an
//! application protocol on the engine of `oxirush_utils::tree`.
//!
//! The root is short for the IEs of the message. In a list of IEs, a name or a position
//! selects the value of an IE, as the IEs of a NAS message are selected, and its entry
//! for a removal or an insertion; after the value, `criticality` is the criticality
//! beside it, and the name of the IE may follow once. In a container of IEs, as a
//! transfer, a segment that names no member is taken in its list, and an IE that a value
//! holds alone goes by its name after the member that holds it. A value has the members
//! that its type gives it, read against the tables of the protocol.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use oxirush_utils::tree::{self, Edit, Family, Key, Listed, Map, Next, Place, Root, Step, Value};

use super::paths::{
    Owner, element, element_of, error, extended, fault, member, no_position, not_an_id, position,
    refusal, refuse, segment_id,
};
use super::{Kind, Member, Protocol, Sent, Shape, foreign, same_name};
use crate::{Error, ErrorKind};

/// The member of a tree that holds the IEs of its message.
const IES: &str = "ies";
/// The member of a container that holds its IEs.
const PROTOCOL_IES: &str = "protocolIEs";
/// The member of the entry of an IE that holds the criticality that it is sent with,
/// where it is not the one that the tables assign.
const CRITICALITY: &str = "criticality";
/// The member of a value that is the octets that it is sent as.
const OCTETS: &str = "octets";

/// What the family knows of a place of a tree.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct At {
    /// What the value there is, as its type has it.
    shape: Shape,
    /// The protocol whose tables have the type, where it is not that of the tree.
    owner: Owner,
    /// The IE whose value the place is, with the name of the type that its container
    /// extends: a segment that names the IE stays on it, once, and its criticality is
    /// beside it.
    ie: Option<(u16, &'static str)>,
    /// The segment that names the IE stayed on the place already.
    named: bool,
    /// The place is the entry of an IE in a list, which the name of the IE stands for:
    /// an IE that a value holds alone goes by its name after the member that holds it.
    listed: bool,
    /// Without a tree, the place is the value of an IE that a position or `*` selects in
    /// a list whose IEs the tables do not say, of a container that extends this type:
    /// the name of an IE after it says which IE it is.
    any_ie: Option<&'static str>,
    /// The place is an alternative of a CHOICE, which an edit puts in the place of the
    /// one there.
    alternative: bool,
}

impl At {
    /// A place of `shape` that the segment at this place selects.
    fn of(self, shape: Shape) -> Self {
        Self {
            shape,
            owner: self.owner,
            ..Self::default()
        }
    }

    /// The value of the IE `id` of a container that extends `extended`.
    fn value(self, protocol: &Protocol, id: Option<u16>, extended: &'static str) -> Self {
        Self {
            shape: protocol.ie_value(id, extended),
            owner: self.owner,
            ie: id.map(|id| (id, extended)),
            named: false,
            listed: false,
            any_ie: None,
            alternative: false,
        }
    }

    /// Whether `part` names the IE whose value the place is, and stays on it.
    fn names_the_ie(self, protocol: &Protocol, part: &str) -> bool {
        let id = self.ie.filter(|_| !self.named).map(|(id, _)| id);
        id.is_some_and(|id| (protocol.ie_id)(part) == Some(id))
    }

    /// The place itself, read by the tables of another protocol where the octets carry
    /// a type of it, or why that protocol cannot read it.
    fn owned(self) -> Result<Self, tree::Error> {
        let Shape::Foreign(name, protocol) = self.shape else {
            return Ok(self);
        };
        let other =
            foreign(name, protocol).map_err(|error| refusal(error.kind(), error.reason()))?;
        Ok(Self {
            shape: other.carried_shape(name),
            owner: Owner::of(other),
            ..self
        })
    }
}

/// What makes, of octets and of the name of a type that they may carry, the value that a
/// tree shows.
type Opener<'e> = &'e dyn Fn(Value, &str, Shape) -> Value;

/// The trees of a protocol, as a family of the engine: the rules of its paths.
struct Trees<'e> {
    protocol: &'static Protocol,
    /// The message that a path is read against where no tree is given.
    message: Option<Sent>,
    /// What makes, of octets and of the name of a type that they may carry, the value
    /// that a tree shows: for `open`.
    opens: Option<Opener<'e>>,
}

impl<'e> Trees<'e> {
    fn new(protocol: &'static Protocol) -> Self {
        Self {
            protocol,
            message: None,
            opens: None,
        }
    }
}

/// The IE that the entry of a list of IEs holds: the name that the entry gives it, its
/// value and its identifier, which a name of the protocol or digits give.
fn held<'v>(protocol: &Protocol, entry: &'v Value) -> Option<(&'v str, &'v Value, Option<u16>)> {
    held_in(protocol, entry.as_object()?)
}

/// The same of the members of the entry.
fn held_in<'v>(protocol: &Protocol, members: &'v Map) -> Option<(&'v str, &'v Value, Option<u16>)> {
    let mut named = members.iter().filter(|(name, _)| *name != CRITICALITY);
    let (Some((name, value)), None) = (named.next(), named.next()) else {
        return None;
    };
    let id = name.parse::<u16>().ok().or_else(|| (protocol.ie_id)(name));
    Some((name.as_str(), value, id))
}

/// Whether a value is written as the octets that it is sent as, alone.
fn octets_alone(entry: &Map) -> bool {
    entry.len() == 1 && entry.contains_key(OCTETS)
}

/// The members of the tree of a PDU, whose message is `message` when the tables have it.
fn tree_members(message: Option<Sent>) -> Vec<Member> {
    vec![
        ("message", Shape::Plain, true),
        ("procedure_code", Shape::Plain, true),
        ("direction", Shape::Plain, true),
        (CRITICALITY, Shape::Plain, true),
        (IES, Shape::Ies(message), true),
        (OCTETS, Shape::Plain, true),
    ]
}

impl Family for Trees<'_> {
    type Shape = At;

    fn root(&self) -> Root<'_> {
        Root::new(self.protocol.root, &[IES], false)
    }

    fn start(&self, tree: &Value, _rooted: bool) -> At {
        let message = self.message.or_else(|| {
            let name = tree.get("message").and_then(Value::as_str)?;
            self.protocol.sent(|_, _, known| known == name)
        });
        At {
            shape: Shape::Pdu(message),
            ..At::default()
        }
    }

    fn step<'v>(
        &self,
        place: &Place<'v, At>,
        part: &str,
        next: &mut Next<'v, At>,
    ) -> Result<Step<At>, tree::Error> {
        let taken = self
            .take(place, part, next)
            .map_err(|reason| self.another_ie(place, part, reason));
        // An edit is done at each place that its path selects, or at none: a place that
        // cannot take it refuses the edit, where a selection takes what the others have.
        match (taken, next.edit()) {
            (Err(reason), Some(edit))
                if edit != Edit::Open && reason.kind() != tree::ErrorKind::Refused =>
            {
                let again = refusal(ErrorKind::Path, reason.reason());
                Err(match reason.code() {
                    Some(code) => again.with_code(code),
                    None => again,
                })
            }
            (taken, _) => taken.map(|()| Step::Taken),
        }
    }

    fn member(&self, at: &At, name: &str, _value: Option<&Value>) -> At {
        let at = &at.owned().unwrap_or(*at);
        let protocol = at.owner.or(self.protocol);
        match at.shape {
            Shape::Pdu(message) => {
                let known = tree_members(message);
                at.of(member(&known, name).map_or(Shape::Plain, |(_, of, _)| *of))
            }
            // The entry of an IE in a list: its value, and the criticality beside it.
            Shape::Ie(id) | Shape::Extension(id, _) if name != CRITICALITY => {
                let id = id.or_else(|| (protocol.ie_id)(name));
                at.value(protocol, id, extended(at.shape))
            }
            Shape::Ie(_) | Shape::Extension(..) => at.of(Shape::Plain),
            shape => {
                let shape = self.as_written(protocol, shape);
                let members = match shape {
                    Shape::Pdu(message) => tree_members(message),
                    shape => protocol.members(shape, None).unwrap_or_default(),
                };
                match member(&members, name) {
                    Some((_, of, _)) => at.of(*of),
                    None => at.of(Shape::Unknown),
                }
            }
        }
    }

    fn entry(&self, at: &At, value: Option<&Value>) -> At {
        let at = &at.owned().unwrap_or(*at);
        let protocol = at.owner.or(self.protocol);
        let shape = protocol.resolved(at.shape);
        match element(shape) {
            Shape::Ie(_) | Shape::Extension(..) => {
                let id = value.and_then(|entry| held(protocol, entry)?.2);
                let of = element_of(shape, id.unwrap_or(u16::MAX)).unwrap_or(Shape::Unknown);
                At {
                    listed: true,
                    ..at.of(of)
                }
            }
            of => at.of(of),
        }
    }

    fn null_is_value(&self, at: &At) -> bool {
        at.owner.or(self.protocol).resolved(at.shape) == Shape::Null
    }

    fn listed(&self, at: &At, name: &str) -> Listed {
        match (at.shape, name) {
            // The IEs of the message go under the root, and nothing else of the tree.
            (Shape::Pdu(_), IES) => Listed::Root,
            (Shape::Pdu(_), _) => Listed::Hidden,
            // An IE goes by its name, which is that of its entry in a list, with its value,
            // and its criticality after it.
            (Shape::Ie(_) | Shape::Extension(..), CRITICALITY) => Listed::Member,
            (Shape::Ie(_) | Shape::Extension(..), _) if at.listed => Listed::Through,
            // The IEs of a container go in its place.
            (_, PROTOCOL_IES) => Listed::Through,
            _ => Listed::Member,
        }
    }

    fn entry_names(&self, at: &At, list: &[Value]) -> Option<Vec<Option<String>>> {
        let at = &at.owned().unwrap_or(*at);
        let protocol = at.owner.or(self.protocol);
        if !matches!(
            element(protocol.resolved(at.shape)),
            Shape::Ie(_) | Shape::Extension(..)
        ) {
            return None;
        }
        // The list is read once, whatever its length: the names are counted for all of
        // its IEs, not for each.
        let names: Vec<Option<&str>> = list
            .iter()
            .map(|entry| held(protocol, entry).map(|(name, ..)| name))
            .collect();
        let mut times: BTreeMap<&str, usize> = BTreeMap::new();
        for name in names.iter().flatten() {
            *times.entry(name).or_default() += 1;
        }
        let named = |name: &Option<&str>| {
            let name = (*name)?;
            // An IE that has no name is written by its identifier, which is no segment.
            let alone = times[name] == 1 && name.parse::<u16>().is_err();
            alone.then(|| name.to_string())
        };
        Some(names.iter().map(named).collect())
    }

    fn edited(&self, tree: &mut Value, keys: &[Key], at: &At, edit: Edit) {
        // A CHOICE has one alternative: the one that is set takes the place of the one
        // there.
        if edit != Edit::Set || !at.alternative {
            return;
        }
        let Some((Key::Name(name), before)) = keys.split_last() else {
            return;
        };
        let mut holder = &mut *tree;
        for key in before {
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
        if let Value::Object(entry) = holder {
            entry.retain(|member, _| member == name);
        }
    }
}

impl Trees<'_> {
    /// Why a segment that names another IE than the one whose value the place is selects
    /// nothing there: which IE that one is, then `reason`, why its value has no such
    /// member either.
    fn another_ie(&self, place: &Place<'_, At>, part: &str, reason: tree::Error) -> tree::Error {
        let at = place.shape();
        let protocol = at.owner.or(self.protocol);
        let (Some((this, _)), Some(other)) = (at.ie, (protocol.ie_id)(part)) else {
            return reason;
        };
        // A refusal is of a value that has the member.
        if reason.kind() == tree::ErrorKind::Refused {
            return reason;
        }
        let name = |id| protocol.ie_name_of(id).unwrap_or("?");
        if this == other {
            return reason;
        }
        let text = format!(
            "this IE is {}, not {}: {}",
            name(this),
            name(other),
            reason.reason()
        );
        let again = tree::Error::new(reason.kind(), "", text);
        match reason.code() {
            Some(code) => again.with_code(code),
            None => again,
        }
    }

    /// The shape of a value of `shape` as a tree writes it: a transfer as its decoded
    /// value. A transfer that is its octets is read before its shape is.
    fn as_written(&self, protocol: &Protocol, shape: Shape) -> Shape {
        match protocol.resolved(shape) {
            Shape::Transfer(contained) => protocol.resolved(Shape::Named(contained)),
            shape => shape,
        }
    }

    /// What `part` selects at `place`, by the rules of the trees.
    fn take<'v>(
        &self,
        place: &Place<'v, At>,
        part: &str,
        next: &mut Next<'v, At>,
    ) -> Result<(), tree::Error> {
        let at = place.shape().owned()?;
        let protocol = at.owner.or(self.protocol);
        // An identifier that is no number is no path, wherever it is.
        if let Some(id) = part.strip_prefix("@id=")
            && id.parse::<u16>().is_err()
        {
            return Err(not_an_id(part));
        }
        let shape = protocol.resolved(at.shape);
        // The criticality beside the value of an IE, where its type has no such member.
        if at.ie.is_some() && same_name(part, CRITICALITY) {
            let shape = self.as_written(protocol, shape);
            let members = protocol.members(shape, None).unwrap_or_default();
            if member(&members, part).is_none() {
                // The criticality of an IE that is not there selects nothing, and what
                // follows it is still a path.
                match place.value() {
                    None => next.absent_once(part, at.of(Shape::Plain)),
                    Some(_) => next.beside(place, CRITICALITY, at.of(Shape::Plain)),
                }
                return Ok(());
            }
        }
        match place.value() {
            Some(Value::Object(entry)) => self.object(place, entry, at, shape, part, next),
            Some(Value::Array(entries)) => self.list(place, entries, at, shape, part, next),
            // A value that is not there, by what its type says.
            Some(Value::Null) | None => match next.edit() {
                None => self.typed(at, shape, part, next),
                Some(_) => Ok(()),
            },
            Some(value) => self.scalar(place, value, at, shape, part, next),
        }
    }

    /// What `part` selects in an object of a tree: a member of the value, an IE of the
    /// container that it is, or its octets.
    fn object<'v>(
        &self,
        place: &Place<'v, At>,
        entry: &'v Map,
        at: At,
        shape: Shape,
        part: &str,
        next: &mut Next<'v, At>,
    ) -> Result<(), tree::Error> {
        let protocol = at.owner.or(self.protocol);
        let (edit, last) = (next.edit(), next.is_last());
        // A value that is its octets: one that did not decode, or that a tree writes as
        // it is sent.
        if octets_alone(entry)
            && (at.ie.is_some() || !matches!(shape, Shape::Unknown | Shape::Pdu(_)))
        {
            if same_name(part, OCTETS) {
                next.member(OCTETS, &entry[OCTETS], at.of(Shape::Plain));
                return Ok(());
            }
            if at.names_the_ie(protocol, part) {
                next.stay(place, At { named: true, ..at });
                return Ok(());
            }
            // Octets that were opened and are not of the type have nothing more to open.
            if edit == Some(Edit::Open) {
                return Ok(());
            }
            return Err(refusal(
                ErrorKind::UndecodableIe,
                format!(
                    "{part:?} is in no value here: the value is its octets, as it did not \
                     decode or is written as it is sent"
                ),
            ));
        }
        if let Shape::Ie(_) | Shape::Extension(..) = shape {
            return self.held_alone(entry, at, shape, part, next);
        }
        let shape = self.as_written(protocol, shape);
        let members = match shape {
            Shape::Pdu(message) => Some(tree_members(message)),
            Shape::Unknown | Shape::Undecoded => None,
            shape => protocol.members(shape, None),
        };
        let Some(members) = members else {
            // No table says what the value can have: it has what the tree has.
            let name = entry.keys().find(|name| same_name(name, part));
            return match name {
                Some(name) => {
                    next.member(name, &entry[name.as_str()], at.of(Shape::Unknown));
                    Ok(())
                }
                None if edit == Some(Edit::Set) && last => {
                    next.absent(part, at.of(Shape::Unknown));
                    Ok(())
                }
                None => Err(fault(
                    ErrorKind::Path,
                    format!("{part:?} is not a member of this value"),
                )),
            };
        };
        let own = member(&members, part);
        // The IE that the value is stays: a path may say which IE an entry is. A member
        // of that name as it is written is that member.
        if own.is_none_or(|(name, ..)| *name != part) && at.names_the_ie(protocol, part) {
            next.stay(place, At { named: true, ..at });
            return Ok(());
        }
        // The IEs of a container are selected in its place: a segment that names no
        // member of it is one of its IEs.
        if own.is_none()
            && let Some((name, ies, _)) = member(&members, PROTOCOL_IES).copied()
        {
            return next.through(place, name, at.of(ies), |inside, next| {
                self.take(inside, part, next)
            });
        }
        if part == "*" {
            if edit.is_some_and(|edit| edit != Edit::Open) {
                return Err(refusal(
                    ErrorKind::Path,
                    format!("{part:?} selects among the entries of a list, and this is no list"),
                ));
            }
            for (name, value) in entry {
                let known = member(&members, name);
                let of = known.map_or(Shape::Unknown, |(_, shape, _)| *shape);
                next.member(name, value, at.of(of));
            }
            return Ok(());
        }
        let Some((name, of, optional)) = own.copied() else {
            return Err(protocol.no_member(shape, &members, part));
        };
        let alternative = matches!(shape, Shape::Named(name) if matches!((protocol.kind_of)(name), Some(Kind::Choice(_))));
        if let Some(edit) = edit
            && last
        {
            refuse(edit, name, alternative, optional)?;
        }
        let child = At {
            alternative,
            ..at.of(of)
        };
        match entry.get(name) {
            Some(value) => next.member(name, value, child),
            // A member that is not there: a set at the end of a path adds it, and nothing
            // goes into it.
            None => next.absent_once(name, child),
        }
        Ok(())
    }

    /// What `part` selects in the entry of an IE that a value holds alone, `{NAME: VALUE}`
    /// with its criticality beside the name: the value by the name of the IE, and the
    /// criticality.
    fn held_alone<'v>(
        &self,
        entry: &'v Map,
        at: At,
        shape: Shape,
        part: &str,
        next: &mut Next<'v, At>,
    ) -> Result<(), tree::Error> {
        let protocol = at.owner.or(self.protocol);
        let held = held_in(protocol, entry).map(|(name, _, id)| (name, id));
        if part == "*" {
            for (name, value) in entry {
                let child = match name.as_str() {
                    CRITICALITY => at.of(Shape::Plain),
                    _ => at.value(
                        protocol,
                        held.as_ref().and_then(|(_, id)| *id),
                        extended(shape),
                    ),
                };
                next.member(name, value, child);
            }
            return Ok(());
        }
        if same_name(part, CRITICALITY) {
            match entry.get(CRITICALITY) {
                Some(value) => next.member(CRITICALITY, value, at.of(Shape::Plain)),
                None => next.absent(CRITICALITY, at.of(Shape::Plain)),
            }
            return Ok(());
        }
        let Some(id) = segment_id(protocol, part, true)? else {
            return Err(fault(
                ErrorKind::UnknownIe,
                format!(
                    "{part:?} is not an IE of {}{}",
                    protocol.name,
                    protocol.nearest_ie(part)
                ),
            ));
        };
        match held {
            Some((name, Some(known))) if known == id => {
                let value = at.value(protocol, Some(id), extended(shape));
                next.member(name, &entry[name], value);
            }
            Some((name, _)) => {
                return Err(fault(
                    ErrorKind::Path,
                    format!("{part:?} is not the IE here, which is {name}"),
                ));
            }
            // An entry without its IE: the IE that a set at the end of a path gives it.
            None if next.is_last() => {
                next.absent(part, at.value(protocol, Some(id), extended(shape)));
            }
            None => {}
        }
        Ok(())
    }

    /// What `part` selects in a list of a tree: an entry by its position, each entry, or
    /// the end of the list for an insertion; in a list of IEs, the value of an IE by its
    /// name or its position, and its entry for a removal or an insertion.
    fn list<'v>(
        &self,
        place: &Place<'v, At>,
        entries: &'v [Value],
        at: At,
        shape: Shape,
        part: &str,
        next: &mut Next<'v, At>,
    ) -> Result<(), tree::Error> {
        let protocol = at.owner.or(self.protocol);
        let of = element(shape);
        if part == "-" && next.edit() == Some(Edit::Insert) && next.is_last() {
            next.end(at.of(of));
            return Ok(());
        }
        if at.names_the_ie(protocol, part) {
            next.stay(place, At { named: true, ..at });
            return Ok(());
        }
        if !matches!(of, Shape::Ie(_) | Shape::Extension(..)) {
            if part == "*" {
                for (position, entry) in entries.iter().enumerate() {
                    next.entry(position, entry, at.of(of));
                }
                return Ok(());
            }
            if (protocol.ie_id)(part).is_some() || part.starts_with("@id=") {
                return Err(fault(
                    ErrorKind::Path,
                    format!("{part:?} selects an IE, and the entries of this list go by position"),
                ));
            }
            let Some(position) = position(part) else {
                return Err(no_position(part));
            };
            if let Some(entry) = entries.get(position) {
                next.entry(position, entry, at.of(of));
            }
            return Ok(());
        }
        // A removal or an insertion is of the entry of an IE; anything else of its value.
        let whole = matches!(next.edit(), Some(Edit::Remove | Edit::Insert)) && next.is_last();
        let extended = extended(of);
        let take = |position: usize, entry: &'v Value, next: &mut Next<'v, At>| {
            let held = held(protocol, entry);
            match (held, entry.as_object()) {
                (Some((_, _, id)), _) if whole => {
                    let of = element_of(shape, id.unwrap_or(u16::MAX)).unwrap_or(of);
                    next.entry(position, entry, at.of(of));
                }
                (Some((name, _, id)), Some(members)) => {
                    let value = at.value(protocol, id, extended);
                    next.entry_member(position, members, name, value);
                }
                // An entry that is no IE is taken as a whole.
                (None, _) | (_, None) => next.entry(position, entry, at.of(Shape::Unknown)),
            }
        };
        if part == "*" {
            for (position, entry) in entries.iter().enumerate() {
                take(position, entry, next);
            }
            return Ok(());
        }
        if let Some(position) = position(part) {
            if let Some(entry) = entries.get(position) {
                take(position, entry, next);
            }
            return Ok(());
        }
        let Some(id) = segment_id(protocol, part, true)? else {
            return Err(fault(
                ErrorKind::UnknownIe,
                format!(
                    "{part:?} is not an IE of {}{}",
                    protocol.name,
                    protocol.nearest_ie(part)
                ),
            ));
        };
        let mut found = false;
        for (position, entry) in entries.iter().enumerate() {
            if held(protocol, entry).is_some_and(|(.., held)| held == Some(id)) {
                take(position, entry, next);
                found = true;
            }
        }
        if !found {
            match next.edit() {
                // An IE that is not there: what follows is still one of its paths.
                None => next.absent_once(part, at.value(protocol, Some(id), extended)),
                Some(Edit::Set) if next.is_last() => {
                    let root = self.protocol.root;
                    return Err(refusal(
                        ErrorKind::Path,
                        format!(
                            "the message has no {part}: insert {{\"{part}\": …}} at /{root}/- \
                             to add it"
                        ),
                    ));
                }
                Some(_) => {}
            }
        }
        Ok(())
    }

    /// Whether the IE `id` is one of the object set of a list of IEs of `shape`, where the
    /// tables have it, which `part` names.
    fn in_the_set(
        &self,
        protocol: &Protocol,
        shape: Shape,
        id: u16,
        part: &str,
    ) -> Result<(), tree::Error> {
        let Shape::Ies(Some(message)) = shape else {
            return Ok(());
        };
        if part.starts_with("@id=") || message.ies.iter().any(|(known, ..)| *known == id) {
            return Ok(());
        }
        if message.ies.is_empty() {
            return Err(fault(
                ErrorKind::Path,
                format!(
                    "a message {} has no IEs: its tree has its octets",
                    message.name
                ),
            ));
        }
        let ies: Vec<_> = (message.ies.iter())
            .map(|(id, ..)| protocol.ie_name_of(*id).unwrap_or("?"))
            .collect();
        Err(fault(
            ErrorKind::Path,
            format!(
                "{part:?} is not an IE of {}, which has {}",
                message.name,
                ies.join(", ")
            ),
        ))
    }

    /// What `part` selects in a value of `shape` that a tree does not have: the places of
    /// the values that it could have.
    fn typed(
        &self,
        at: At,
        shape: Shape,
        part: &str,
        next: &mut Next<'_, At>,
    ) -> Result<(), tree::Error> {
        let protocol = at.owner.or(self.protocol);
        let of = element(shape);
        // An IE at a position, which the name after it says.
        if let Some(extended) = at.any_ie
            && let Some(id) = (protocol.ie_id)(part)
        {
            let named = At {
                named: true,
                ..at.value(protocol, Some(id), extended)
            };
            next.absent_once(part, named);
            return Ok(());
        }
        match shape {
            Shape::Unknown | Shape::Undecoded => next.absent_once(part, at.of(Shape::Unknown)),
            Shape::Ie(known) | Shape::Extension(known, _) => {
                if same_name(part, CRITICALITY) {
                    next.absent_once(part, at.of(Shape::Plain));
                } else if part == "*" {
                    // The value of the IE, of the type of the IE that the place has if it
                    // is one, and the criticality beside it, as in a tree.
                    let value = match known {
                        Some(id) => at.value(protocol, Some(id), extended(shape)),
                        None => at.of(Shape::Unknown),
                    };
                    next.absent_once(part, value);
                    next.absent_once(CRITICALITY, at.of(Shape::Plain));
                } else if let Some(id) = segment_id(protocol, part, true)? {
                    // Without a tree, an IE that a value holds alone is any IE: the walk
                    // meets none whose identifier a place gives.
                    next.absent_once(part, at.value(protocol, Some(id), extended(shape)));
                } else {
                    return Err(fault(
                        ErrorKind::UnknownIe,
                        format!(
                            "{part:?} is not an IE of {}{}",
                            protocol.name,
                            protocol.nearest_ie(part)
                        ),
                    ));
                }
            }
            Shape::List(_) | Shape::Ies(_) | Shape::Extensions(_)
                if at.names_the_ie(protocol, part) =>
            {
                next.absent_once(part, At { named: true, ..at });
            }
            Shape::List(_) | Shape::Ies(_) | Shape::Extensions(_) => {
                let extended = extended(of);
                let ies = matches!(of, Shape::Ie(_) | Shape::Extension(..));
                if ies && let Some(id) = segment_id(protocol, part, true)? {
                    self.in_the_set(protocol, shape, id, part)?;
                    next.absent_once(part, at.value(protocol, Some(id), extended));
                } else if matches!(part, "*" | "-") || position(part).is_some() {
                    match shape {
                        Shape::Ies(Some(message)) => {
                            for (id, ..) in message.ies {
                                next.absent_once(part, at.value(protocol, Some(*id), extended));
                            }
                        }
                        _ if ies => {
                            let any = At {
                                any_ie: Some(extended),
                                ..at.of(Shape::Unknown)
                            };
                            next.absent_once(part, any);
                        }
                        _ => next.absent_once(part, at.of(of)),
                    }
                } else if ies {
                    return Err(fault(
                        ErrorKind::UnknownIe,
                        format!(
                            "{part:?} is not an IE of {}{}",
                            protocol.name,
                            protocol.nearest_ie(part)
                        ),
                    ));
                } else if (protocol.ie_id)(part).is_some() || part.starts_with("@id=") {
                    return Err(fault(
                        ErrorKind::Path,
                        format!(
                            "{part:?} selects an IE, and the entries of this list go by position"
                        ),
                    ));
                } else {
                    return Err(no_position(part));
                }
            }
            shape => {
                let shape = self.as_written(protocol, shape);
                let members = match shape {
                    Shape::Pdu(message) => tree_members(message),
                    shape => protocol.members(shape, None).unwrap_or_default(),
                };
                let own = member(&members, part);
                if own.is_none_or(|(name, ..)| *name != part) && at.names_the_ie(protocol, part) {
                    next.absent_once(part, At { named: true, ..at });
                } else if own.is_none()
                    && let Some((_, ies, _)) = member(&members, PROTOCOL_IES)
                {
                    return self.typed(at.of(*ies), *ies, part, next);
                } else if part == "*" {
                    for (name, of, _) in &members {
                        next.absent_once(name, at.of(*of));
                    }
                } else if same_name(part, OCTETS) && at.ie.is_some() {
                    next.absent_once(part, at.of(Shape::Plain));
                } else if let Some((name, of, _)) = own {
                    next.absent_once(name, at.of(*of));
                } else {
                    return Err(protocol.no_member(shape, &members, part));
                }
            }
        }
        Ok(())
    }

    /// What `part` selects in a value without members: nothing, but octets that may carry
    /// a type, which are opened as that type.
    fn scalar<'v>(
        &self,
        place: &Place<'v, At>,
        value: &'v Value,
        at: At,
        shape: Shape,
        part: &str,
        next: &mut Next<'v, At>,
    ) -> Result<(), tree::Error> {
        let protocol = at.owner.or(self.protocol);
        if at.names_the_ie(protocol, part) {
            next.stay(place, At { named: true, ..at });
            return Ok(());
        }
        match shape {
            // Octets that may carry a type are that type once they are opened.
            Shape::Carrier(_) => {
                let carried = protocol.members(shape, None).unwrap_or_default();
                match member(&carried, part) {
                    Some((name, of, _)) => match (next.edit(), self.opens, value) {
                        (Some(Edit::Open), Some(decode), Value::String(_)) => {
                            let mut opened = Map::new();
                            opened.insert(name.to_string(), decode(value.clone(), name, *of));
                            next.open(place, Value::Object(opened));
                            Ok(())
                        }
                        _ => Err(refusal(
                            ErrorKind::Path,
                            format!("the octets are not shown as a {name}: open decodes them"),
                        )),
                    },
                    None => Err(protocol.no_member(shape, &carried, part)),
                }
            }
            // A value without members has none to select, nor a bit string written as its
            // octets.
            _ if part == "*" => Ok(()),
            Shape::Bits => {
                let members = protocol.members(shape, None).unwrap_or_default();
                match member(&members, part) {
                    Some(_) => Ok(()),
                    None => Err(protocol.no_member(shape, &members, part)),
                }
            }
            _ => Err(fault(
                ErrorKind::Path,
                format!("{part:?} is not a member of this value, which has none"),
            )),
        }
    }
}

// ── The calls ───────────────────────────────────────────────────────────────

/// The values at `path` in `tree`, the tree of a PDU of `protocol`, in the order of the
/// tree.
///
/// # Errors
///
/// A path that is not well formed, and a name that the value at its place cannot have.
pub fn select<'v>(
    protocol: &'static Protocol,
    tree: &'v Value,
    path: &str,
) -> Result<Vec<&'v Value>, Error> {
    tree::select_in(&Trees::new(protocol), tree, path).map_err(error)
}

/// Whether `path` can select a value in a message of `protocol` that ASN.1 names
/// `message`: what [`select`] says of a path without a tree to select in.
///
/// # Errors
///
/// A message that the protocol does not have, and what [`select`] refuses.
pub fn check_path(protocol: &'static Protocol, message: &str, path: &str) -> Result<(), Error> {
    let message = protocol.message_of(message)?;
    let trees = Trees {
        message: Some(message),
        ..Trees::new(protocol)
    };
    let places = tree::places_in(&trees, &Value::Null, path).map_err(error)?;
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
        let selected = tree::places_in(&trees, &Value::Null, &prefix).map_err(error)?;
        if selected.is_empty() {
            return Err(Error::new(
                ErrorKind::Path,
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
/// names `message`, when its type is an ENUMERATED; `None` for a value of another type,
/// and for one whose type is not known.
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
    let trees = Trees {
        message: Some(message),
        ..Trees::new(protocol)
    };
    let places = tree::places_in(&trees, &Value::Null, path).map_err(error)?;
    // The value is of one type, wherever the path finds it.
    let mut names = None;
    for place in places {
        let (mut shape, protocol) = (place.shape().shape, place.shape().owner.or(protocol));
        // A type that is another one names it a few times at most; an ENUMERATED is no
        // type of the list.
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

/// Each value of `tree`, the tree of a PDU of `protocol`, with the path that selects it,
/// in the order of the tree.
pub fn paths(protocol: &'static Protocol, tree: &Value) -> Vec<(String, Value)> {
    tree::paths_in(&Trees::new(protocol), tree)
}

/// Give the value at `path` in `tree`, the tree of a PDU of `protocol`, the value
/// `value`.
///
/// # Errors
///
/// A path that selects nothing, and the tree is then as it was.
pub fn set(
    protocol: &'static Protocol,
    tree: &mut Value,
    path: &str,
    value: Value,
) -> Result<(), Error> {
    tree::set_in(&Trees::new(protocol), tree, path, value).map_err(error)
}

/// Take the values at `path` out of `tree`, the tree of a PDU of `protocol`: an IE, a
/// member of a value or an entry of a list.
///
/// # Errors
///
/// A path that selects nothing, and a member that its type always has.
pub fn remove(protocol: &'static Protocol, tree: &mut Value, path: &str) -> Result<(), Error> {
    tree::remove_in(&Trees::new(protocol), tree, path).map_err(error)
}

/// Add `value` to a list of `tree`, the tree of a PDU of `protocol`: before each entry
/// that `path` selects, or at the end for a path that ends with `-`.
///
/// # Errors
///
/// A path that selects nothing, and one that ends at a member.
pub fn insert(
    protocol: &'static Protocol,
    tree: &mut Value,
    path: &str,
    value: Value,
) -> Result<(), Error> {
    tree::insert_in(&Trees::new(protocol), tree, path, value).map_err(error)
}

/// Decode, in `tree`, the tree of a PDU of `protocol`, the octets that `path` takes as a
/// type that they may carry: those of an IE whose content the specification gives by
/// reference, where the segment after the IE is the name that ASN.1 gives one of these
/// types. The IE then has, in place of its octets, that type under that name.
///
/// # Errors
///
/// What [`select`] refuses: the tree is then as it was.
pub fn open(protocol: &'static Protocol, tree: &mut Value, path: &str) -> Result<(), Error> {
    let opened = |raw: Value, name: &str, shape: Shape| {
        let opened = protocol.opened(raw.clone(), name);
        protocol.by_name(opened, shape).unwrap_or(raw)
    };
    let trees = Trees {
        opens: Some(&opened),
        ..Trees::new(protocol)
    };
    tree::open_in(&trees, tree, path).map_err(error)
}
