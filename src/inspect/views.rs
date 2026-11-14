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

//! The trees of the PDUs of an application protocol, through their views: each IE and each
//! transfer of a message as its type is decoded, beside the octets that it was received
//! as, which the tree has where its value has no place for them, and the message that a
//! tree writes encoded again.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use oxirush_utils::tree::{Map, Reading, Refused, Value};

use super::paths::{extended, member_shape, of_a_transfer, written_ie};
use super::schema::{self, Field, Walk};
use super::{Protocol, Sent, Shape, Typed, foreign, same_name};
use crate::json::{self, hex};
use crate::{Direction, Error, ErrorKind, Reader, Writer};

/// How deep in a view an IE or a transfer is still decoded.
const NESTING: usize = 64;
/// The member of a view that marks an IE, a transfer or a message whose typed value
/// encodes to other octets than those received: what a later version of its type adds,
/// which the value has no place for, or another form than the canonical one. A tree has
/// its octets in the place of its value.
pub(crate) const LOSSY: &str = "_lossy";

/// The octets that `value` writes in hexadecimal.
fn unhex(value: &Value) -> Result<Vec<u8>, Error> {
    let octets = value.as_str().and_then(json::unhex);
    octets.ok_or_else(|| {
        // The start of what is written says which value it is.
        let written: String = value.to_string().chars().take(40).collect();
        Error::new(
            ErrorKind::Value,
            format!(
                "{written} is not octets: octets are written in hexadecimal, two digits for each"
            ),
        )
    })
}

/// The value that the octets `raw` contain, beside them, or why they did not decode.
fn contained(raw: Value, typed: Result<Typed, Error>) -> Value {
    // Octets in another form than the one of X.691 are refused, as by the tree.
    let decoded = typed.and_then(|typed| (typed.decode)(&unhex(&raw)?));
    let decoded = decoded.and_then(|(decoded, exact, refused)| match refused {
        Some(refused) => Err(refused),
        None => Ok((decoded, exact)),
    });
    let mut transfer = Map::new();
    transfer.insert("_raw_transfer".into(), raw);
    match decoded {
        Ok((decoded, exact)) => {
            transfer.insert("decoded".into(), decoded);
            if !exact {
                transfer.insert(LOSSY.into(), true.into());
            }
        }
        Err(error) => {
            transfer.insert("_decode_error".into(), error.to_string().into());
        }
    };
    Value::Object(transfer)
}

/// Why what is deeper than the inspection goes keeps its octets.
fn too_deep<T>() -> Result<T, Error> {
    Err(Error::new(
        ErrorKind::Value,
        format!("inspection nesting exceeds {NESTING}"),
    ))
}

/// A view whose direction is none of the three.
fn no_direction() -> Error {
    Error::new(
        ErrorKind::Value,
        "the direction of a view is InitiatingMessage, SuccessfulOutcome or UnsuccessfulOutcome",
    )
}

impl Protocol {
    /// The octets `raw` as the transfer that they are when they carry the type that
    /// ASN.1 names `name`: its decoded value beside them, or why they are not of that
    /// type.
    pub(crate) fn opened(&self, raw: Value, name: &str) -> Value {
        // A type of another protocol is opened by its tables, or says why it is not.
        if let Some((name, protocol)) = self.foreign_type(name) {
            return match foreign(name, protocol) {
                Ok(other) => other.opened(raw, name),
                Err(error) => contained(raw, Err(error)),
            };
        }
        let typed = (self.carried_typed)(name);
        let carries_none = || Error::new(ErrorKind::Path, format!("octets carry no {name}"));
        let mut transfer = contained(raw, typed.ok_or_else(carries_none));
        self.expand(&mut transfer, 0, self.carried_shape(name));
        transfer
    }

    /// The octets of the IE `id`, which is written as a type that they may carry: one
    /// member, the name of that type with its value, which is its octets by now.
    fn carried_octets(&self, written: &Value, id: u16) -> Result<Value, Error> {
        let carried = self
            .carried
            .iter()
            .map(|(carrier, name, _)| (carrier, name));
        let foreign = self
            .foreign
            .iter()
            .map(|(carrier, name, _)| (carrier, name));
        let carried = carried
            .chain(foreign)
            .filter(|(carrier, _)| **carrier == id);
        let names: Vec<_> = carried.map(|(_, name)| *name).collect();
        let mut members = written.as_object().into_iter().flatten();
        match (members.next(), members.next()) {
            (Some((name, octets)), None) if names.iter().any(|known| same_name(known, name)) => {
                Ok(octets.clone())
            }
            _ => Err(Error::new(
                ErrorKind::Value,
                format!(
                    "IE {id} is written as its octets, or as {{NAME: VALUE}} for a type that they \
                 carry: {}",
                    names.join(", ")
                ),
            )),
        }
    }

    /// The functions of the type of the IE `id` in a container of `shape`: the type of
    /// its identifier, or the one that the type which an extension container extends
    /// gives it.
    fn ie_type(&self, id: u16, shape: Shape) -> Result<Typed, Error> {
        match (self.extension_typed)(id, extended(shape)) {
            Some(typed) => Ok(typed),
            None => (self.ie_typed)(id),
        }
    }

    /// The functions of the type of the message of a direction and a procedure code. A
    /// procedure that the specification does not have in that direction has no type of
    /// its own: its message is read and written as what every message of the protocol
    /// but the private one is, a container of protocol IEs and nothing else.
    fn message_typed(&self, direction: Direction, code: u8) -> Typed {
        let mut messages = self.messages.iter();
        let message =
            messages.find(|(set, ..)| set.procedure_code == code && set.direction == direction);
        message.map_or_else(self.any_message, |(_, typed, _)| typed())
    }

    /// Decode the IEs and the transfers of `value`, a value of `shape` which is `depth`
    /// deep in its view. What is deeper than the inspection goes keeps its octets, with
    /// why.
    fn expand(&self, value: &mut Value, depth: usize, shape: Shape) {
        // A type of another protocol, by its tables.
        if let Shape::Foreign(name, protocol) = shape {
            if let Ok(other) = foreign(name, protocol) {
                other.expand(value, depth, other.carried_shape(name));
            }
            return;
        }
        let deep = depth > NESTING;
        if let Some(object) = value.as_object_mut() {
            let field = if object.contains_key("value") {
                "value"
            } else {
                "extensionValue"
            };
            if let Some(id) = object
                .get("id")
                .and_then(Value::as_u64)
                .and_then(|v| u16::try_from(v).ok())
                && let Some(raw) = object.get_mut(field).filter(|v| v.is_string())
            {
                let raw = core::mem::take(raw);
                let decoded = match deep {
                    true => too_deep(),
                    false => {
                        unhex(&raw).and_then(|bytes| (self.ie_type(id, shape)?.decode)(&bytes))
                    }
                };
                // A value in another form than the one of X.691 is refused, as the tree
                // refuses it: the IE is its octets, with why.
                let decoded = decoded.and_then(|(decoded, exact, refused)| match refused {
                    Some(refused) => Err(refused),
                    None => Ok((decoded, exact)),
                });
                match decoded {
                    Ok((decoded, exact)) => {
                        if !exact {
                            object.insert(LOSSY.into(), true.into());
                        }
                        // An OCTET STRING (CONTAINING ...) has the form of a transfer.
                        let decoded = match (self.ie_contents)(id) {
                            Some(contents) => contained(decoded, Ok(contents)),
                            None => decoded,
                        };
                        object.insert(field.into(), decoded);
                    }
                    // The value stays the octets that it was received as.
                    Err(error) => {
                        object.insert(field.into(), raw.clone());
                        object.insert("_decode_error".into(), error.to_string().into());
                    }
                }
                object.insert("_raw_value".into(), raw);
            }
            // What each member is, once the entry of an IE says what it was decoded as.
            let members = self.member_shapes(shape, object);
            for (key, child) in object {
                if !key.starts_with('_') {
                    if self.transfer_fields.contains(&key) && child.is_string() {
                        let typed = match deep {
                            true => too_deep(),
                            false => (self.transfer_typed)(key),
                        };
                        *child = contained(core::mem::take(child), typed);
                    }
                    self.expand(child, depth + 1, member_shape(&members, key));
                }
            }
        } else if let Some(array) = value.as_array_mut() {
            for child in array {
                self.expand(child, depth + 1, self.entry_shape(shape));
            }
        }
    }

    /// Encode again what `value`, a value of `shape`, has decoded: `member` is the name
    /// of the member that holds it, or the list that it is in.
    fn collapse(
        &'static self,
        value: &mut Value,
        member: &str,
        depth: usize,
        shape: Shape,
    ) -> Result<(), Error> {
        // What is decoded at the limit of the inspection has values of its own under it.
        if depth > 4 * NESTING {
            return Err(Error::new(
                ErrorKind::Value,
                format!("inspection nesting exceeds {}", 4 * NESTING),
            ));
        }
        // A type of another protocol, by its tables.
        if let Shape::Foreign(name, protocol) = shape {
            let other = foreign(name, protocol)?;
            return other.collapse(value, member, depth, other.carried_shape(name));
        }
        if let Some(object) = value.as_object_mut() {
            // An IE may be written as its `octets`.
            written_ie(object);
            let id = object
                .get("id")
                .and_then(Value::as_u64)
                .and_then(|v| u16::try_from(v).ok());
            let members = self.member_shapes(shape, object);
            for (key, child) in object.iter_mut() {
                if key.starts_with('_') {
                    continue;
                }
                self.collapse(child, key, depth + 1, member_shape(&members, key))?;
                if let Some(members) = child.as_object() {
                    // What was not received is encoded from the value written.
                    let transfer = match key {
                        "value" | "extensionValue" => id.and_then(self.ie_contents),
                        key if self.transfer_fields.contains(&key) => {
                            Some((self.transfer_typed)(key)?)
                        }
                        // A type that the octets of an IE may carry, by its name.
                        key => self.carried_or_foreign(key),
                    };
                    if let Some(transfer) = transfer {
                        // The decoded value, as the object itself when it has no member
                        // of a transfer, or the `octets` alone.
                        let octets = members.get("octets").filter(|_| members.len() == 1);
                        let wire = if !members.keys().any(of_a_transfer) {
                            Value::String(hex(&(transfer.encode)(child)?).into())
                        } else if let Some(octets) = octets {
                            octets.clone()
                        } else {
                            return Err(Error::new(
                                ErrorKind::Value,
                                format!(
                                    "{key} is written as its decoded value, or as its octets alone"
                                ),
                            ));
                        };
                        *child = wire;
                    }
                }
            }
            let raw = object.shift_remove("_raw_value");
            if let Some(id) = id {
                let wire = self.ie_octets(object, member, shape, id, raw)?;
                let field = match member.starts_with("iE-Extension") {
                    true => "extensionValue",
                    false => "value",
                };
                object.insert(field.into(), wire);
            }
        } else if let Some(array) = value.as_array_mut() {
            for child in array {
                self.collapse(child, member, depth + 1, self.entry_shape(shape))?;
            }
        }
        Ok(())
    }

    /// The octets of the IE `id`, whose entry is `object` in a container of `shape`
    /// that the member `member` holds: the octets that a tree writes in the place of its
    /// value, `raw`, or its value encoded.
    fn ie_octets(
        &self,
        object: &mut Map,
        member: &str,
        shape: Shape,
        id: u16,
        raw: Option<Value>,
    ) -> Result<Value, Error> {
        if let Some(raw) = raw {
            return Ok(raw);
        }
        // The value of an IE of an extension container has another name.
        let field = match member.starts_with("iE-Extension") {
            true => "extensionValue",
            false => "value",
        };
        // Octets that are written as a type that they may carry.
        if let Some(written) = object.get_mut(field).filter(|value| value.is_object())
            && self.carries(id)
        {
            *written = self.carried_octets(written, id)?;
        }
        let octets = "write its value as its octets, {\"octets\": \"…\"}";
        let typed = self.ie_type(id, shape);
        let typed = typed.map_err(|error| error.reworded(|what| format!("{what}: {octets}")))?;
        let value = object.get(field).unwrap_or(&Value::Null);
        Ok(Value::String(hex(&(typed.encode)(value)?).into()))
    }

    /// The name that ASN.1 gives the message of a direction and a procedure code.
    fn message_name_of(&self, direction: Direction, code: u8) -> Option<&'static str> {
        let mut messages = self.messages.iter();
        let message =
            messages.find(|(set, ..)| set.procedure_code == code && set.direction == direction);
        message.map(|(set, ..)| set.name)
    }

    /// The criticality that ASN.1 assigns to the procedure of the message that it names
    /// `name`.
    fn message_criticality(&self, name: &str) -> Option<&'static str> {
        let mut messages = self.messages.iter();
        let (set, ..) = messages.find(|(set, ..)| same_name(set.name, name))?;
        Some(set.criticality.name())
    }
}

/// The tree of the PDU of `protocol` of a direction, a procedure code and a criticality,
/// whose message is the octets `raw`: the name of its message and its IEs by name, in
/// their order, and its criticality when it is not the one of its procedure.
///
/// A procedure that the protocol does not have in that direction has its
/// `procedure_code` and its `direction` in the place of the name, and its criticality,
/// which the tables do not assign: its message is read as a container of IEs, which each
/// message of the protocol but the private one is. A message that has no list of IEs, as
/// the private one, one that does not decode, and one whose typed value has no place for
/// what it was received with, which a later version of its type adds, has its `octets` in
/// the place of its IEs. A PDU that decodes has a tree.
pub fn tree(
    protocol: &'static Protocol,
    direction: Direction,
    code: u8,
    criticality: Value,
    raw: &[u8],
) -> Value {
    // The message as a view has it, whose IEs are moved into the tree, and not copied. The
    // container of the message is read whatever its form: its IEs say theirs.
    // The message in one pass, with the table of the types, where it is a value that
    // writes its octets back: its IEs by name.
    let typed = protocol.message_typed(direction, code);
    let ies = (typed.shown)(raw).then(|| {
        let named = Named {
            protocol,
            rows: protocol.containers,
        };
        let sent = match protocol.message_shape(direction, code) {
            Shape::Message(sent) => Some(sent),
            _ => None,
        };
        let read = &mut Reader::new(raw);
        let message = named.row((typed.row)(), read, 0, sent, Within::Ie).ok()?;
        let Value::Object(mut message) = message else {
            return None;
        };
        message.shift_remove("protocolIEs")
    });
    tree_of(
        protocol,
        direction,
        code,
        criticality,
        ies.flatten(),
        || hex(raw).into(),
    )
}

/// The tree of the PDU of `protocol` of a direction, a procedure code and a criticality,
/// whose message has the entries `ies` of a view, or the octets that `octets` gives.
fn tree_of(
    protocol: &'static Protocol,
    direction: Direction,
    code: u8,
    criticality: Value,
    ies: Option<Value>,
    octets: impl FnOnce() -> Value,
) -> Value {
    let name = protocol.message_name_of(direction, code);
    let mut tree = Map::new();
    match name {
        Some(name) => {
            tree.insert("message".into(), name.into());
        }
        None => {
            tree.insert("procedure_code".into(), code.into());
            tree.insert("direction".into(), direction.name().into());
        }
    }
    let ies = ies.filter(Value::is_array);
    match ies {
        Some(ies) => tree.insert("ies".into(), ies),
        None => tree.insert("octets".into(), octets()),
    };
    let assigned = name.and_then(|name| protocol.message_criticality(name));
    if assigned.is_none() || criticality.as_str() != assigned {
        tree.insert("criticality".into(), criticality);
    }
    Value::Object(tree)
}

/// What the PDU that a view or a tree writes is made of: its direction, its procedure
/// code, its criticality as JSON, which the protocol reads as its own type, and the
/// octets of its message.
pub struct Parts {
    /// The direction of the PDU.
    pub direction: Direction,
    /// The procedure code.
    pub code: u8,
    /// The criticality of the PDU, as JSON.
    pub criticality: Value,
    /// The octets of the message: those received for what was not edited, and what was
    /// written encoded.
    pub message: Vec<u8>,
}

/// The direction that a view or a tree names, as one of the three.
fn direction_of(direction: &str) -> Result<Direction, Error> {
    Direction::named(direction).ok_or_else(no_direction)
}

/// The octets of the message of a direction and a procedure code of `protocol` that
/// `message`, the message of a view, writes.
fn message_octets(
    protocol: &'static Protocol,
    direction: Direction,
    code: u8,
    mut message: Value,
) -> Result<Vec<u8>, Error> {
    protocol.collapse(&mut message, "", 0, protocol.message_shape(direction, code))?;
    (protocol.message_typed(direction, code).encode)(&message)
}

/// The parts of the PDU of `protocol` that a tree writes: a message by its name, or by
/// the `procedure_code` and the `direction` of a procedure that the protocol does not
/// have in that direction, with its IEs by name or its `octets`, as [`tree`] shows one.
///
/// # Errors
///
/// What is no tree, a name that is no message of the protocol, a procedure that the
/// protocol has written by its code, a criticality that the tables do not assign and the
/// tree does not write, and an IE that is not written as its type.
pub fn tree_parts(protocol: &'static Protocol, tree: &Value) -> Result<Parts, Error> {
    const FORM: &str = "a tree is {\"message\": NAME, \"ies\": [{NAME: VALUE}, …]}, with \
                        \"octets\" in the place of the IEs for a message as it is sent";
    let form = || Error::new(ErrorKind::Value, FORM);
    let written = tree.as_object().ok_or_else(form)?;
    const MEMBERS: [&str; 6] = [
        "message",
        "procedure_code",
        "direction",
        "ies",
        "octets",
        "criticality",
    ];
    if let Some(other) = written
        .keys()
        .find(|key| !MEMBERS.iter().any(|known| same_name(known, key)))
    {
        return Err(Error::new(
            ErrorKind::Value,
            format!("{other:?} is no member of a tree: {FORM}"),
        ));
    }
    // A member is read however its name is written, and is written once.
    for known in MEMBERS {
        let mut spellings = written.keys().filter(|key| same_name(known, key));
        if let (Some(first), Some(second)) = (spellings.next(), spellings.next()) {
            return Err(Error::new(
                ErrorKind::Value,
                format!(
                    "the member {known:?} of a tree is written twice, as {first:?} and as {second:?}"
                ),
            ));
        }
    }
    let member = |name: &str| {
        let mut members = written.iter();
        members
            .find(|(key, _)| same_name(key, name))
            .map(|(_, value)| value)
    };
    let (direction, code, name) = match member("message") {
        Some(name) => {
            let name = name.as_str().ok_or_else(form)?;
            let mut messages = protocol.messages.iter();
            let message = messages.find(|(set, ..)| same_name(set.name, name));
            let (set, ..) = message.ok_or_else(|| {
                let nearest = protocol.nearest_message(name);
                Error::new(
                    ErrorKind::UnknownMessage,
                    format!("{name:?} is not a message of {}{nearest}", protocol.name),
                )
            })?;
            if member("procedure_code").or(member("direction")).is_some() {
                return Err(Error::new(
                    ErrorKind::Value,
                    format!(
                        "{name} is named by its message alone: its procedure_code and its \
                     direction are those of its name"
                    ),
                ));
            }
            (set.direction, set.procedure_code, Some(set.name))
        }
        None => {
            const UNKNOWN: &str = "a procedure that the protocol does not have is written by \
                                   its procedure_code, a number from 0 to 255, and its \
                                   direction";
            let code = member("procedure_code").and_then(Value::as_u64);
            let code = code.and_then(|code| u8::try_from(code).ok());
            let direction = member("direction").and_then(Value::as_str);
            let (Some(code), Some(direction)) = (code, direction) else {
                return Err(Error::new(ErrorKind::Value, format!("{FORM}; {UNKNOWN}")));
            };
            let direction = direction_of(direction)?;
            if let Some(name) = protocol.message_name_of(direction, code) {
                return Err(Error::new(
                    ErrorKind::Value,
                    format!(
                        "the {direction} of procedure {code} is {name}: it is written as \
                     {{\"message\": \"{name}\"}}"
                    ),
                ));
            }
            (direction, code, None)
        }
    };
    let assigned = name.and_then(|name| protocol.message_criticality(name));
    let criticality = match (member("criticality"), assigned) {
        (Some(criticality), _) => criticality.clone(),
        (None, Some(assigned)) => assigned.into(),
        (None, None) => {
            return Err(Error::new(
                ErrorKind::Value,
                format!(
                    "the {direction} of procedure {code} says its \"criticality\": the tables \
                 have none for it"
                ),
            ));
        }
    };
    let message = match (member("ies"), member("octets")) {
        (Some(listed), None) if listed.is_array() => {
            let sent = name.and_then(|name| protocol.sent(|_, _, known| known == name));
            // The message in one pass, where the tree writes it; otherwise through its
            // view, which says why it does not.
            let written = Written::message(protocol, direction, code, sent, listed);
            let viewed = || {
                let mut message = Map::new();
                let entries = protocol.written(listed, Shape::Ies(sent), 0)?;
                message.insert("protocolIEs".into(), entries);
                message_octets(protocol, direction, code, Value::Object(message))
            };
            match written {
                Some(octets) => {
                    // The two ways write the same octets.
                    debug_assert_eq!(viewed().ok().as_ref(), Some(&octets), "{tree}");
                    octets
                }
                None => viewed()?,
            }
        }
        (None, Some(octets)) => unhex(octets)?,
        _ => return Err(form()),
    };
    Ok(Parts {
        direction,
        code,
        criticality,
        message,
    })
}

/// The tree of a view of a PDU of `protocol` whose values another implementation wrote in
/// the JSON Encoding Rules (ITU-T X.697), as pycrate writes them: the IEs of its message
/// with the values that [`Protocol::from_jer`] makes of theirs, or the octets of a message
/// without IEs.
///
/// # Errors
///
/// A view without its procedure code or its direction, and a message without IEs that
/// does not encode.
pub fn tree_of_jer(protocol: &'static Protocol, view: &Value) -> Result<Value, Error> {
    let code = view["procedure_code"]
        .as_u64()
        .and_then(|c| u8::try_from(c).ok())
        .ok_or_else(|| {
            Error::new(
                ErrorKind::Value,
                "the procedure_code of a view is a number from 0 to 255",
            )
        })?;
    let direction = direction_of(view["direction"].as_str().ok_or_else(no_direction)?)?;
    let shape = protocol.message_shape(direction, code);
    let mut message = view["message"].clone();
    protocol.from_jer(&mut message, shape);
    let ies = message.get("protocolIEs").cloned();
    // The IEs of the view, by name.
    let name = protocol.message_name_of(direction, code);
    let sent = name.and_then(|name| protocol.sent(|_, _, known| known == name));
    let named = ies
        .clone()
        .filter(Value::is_array)
        .and_then(|listed| protocol.by_name(listed, Shape::Ies(sent)).ok());
    let octets = match ies {
        Some(_) => Value::Null,
        None => hex(&message_octets(protocol, direction, code, message)?).into(),
    };
    let criticality = view["criticality"].clone();
    Ok(tree_of(
        protocol,
        direction,
        code,
        criticality,
        named,
        || octets,
    ))
}

// ── The tree of a PDU in one pass ───────────────────────────────────────────

/// What holds an IE, which gives its criticality and its type: the IEs of a message or
/// of a transfer, of this one when the tables have it, an IE alone or as an entry of a
/// list, or the IEs of an extension container of the type that the bindings give this
/// name.
#[derive(Clone, Copy)]
pub(super) enum Within {
    Ies(Option<Sent>),
    Ie,
    Extensions(&'static str),
}

impl Within {
    /// The shape that the paths give a container of this kind.
    fn shape(self) -> Shape {
        match self {
            Within::Ies(sent) => Shape::Ies(sent),
            Within::Ie => Shape::Ie(None),
            Within::Extensions(extended) => Shape::Extension(None, extended),
        }
    }
}

/// The reading of the tree of a PDU from its octets in one pass, with the table of the
/// types: the tree that the view of the PDU gives by name.
struct Named {
    protocol: &'static Protocol,
    rows: schema::Containers,
}

impl Named {
    /// The tree of a value of the row `row`, `depth` deep in the view of the PDU, read
    /// from `reader`. `sent` is the message whose IEs a container of the row holds.
    fn row(
        &self,
        row: usize,
        reader: &mut Reader<'_>,
        depth: usize,
        sent: Option<Sent>,
        within: Within,
    ) -> Result<Value, Error> {
        let types = self.protocol.schema;
        let ty = &types[row];
        match &ty.kind {
            schema::Kind::Sequence {
                names,
                members,
                extensible,
            } => {
                let extended = match extensible {
                    true => reader.bit()?,
                    false => false,
                };
                let optional = members.iter().filter(|member| member.optional).count();
                let present = match optional {
                    0 => 0,
                    count => reader.bits(count)?,
                };
                let mut tree = Map::with_capacity(members.len());
                let mut position = 0;
                for (name, member) in names.iter().zip(*members) {
                    if member.optional {
                        position += 1;
                        if present & (1 << (optional - position)) == 0 {
                            continue;
                        }
                    }
                    let value = self
                        .member(ty, name, &member.ty, reader, depth, sent)
                        .map_err(|error| error.in_field(member.field))?;
                    tree.push((*name).into(), value);
                }
                if extended {
                    reader.skip_additions()?;
                }
                Ok(Value::Object(tree))
            }
            schema::Kind::Choice {
                names,
                alternatives,
                root,
                extensible,
            } => {
                let additions = alternatives.len() as u64 - root;
                let index = reader.index(*root, additions, *extensible)?;
                let index = (index as usize).min(alternatives.len() - 1);
                let field = &alternatives[index];
                let value = match index < *root as usize {
                    true => self.field(field, reader, depth + 1, Within::Ie)?,
                    false => {
                        let octets = reader.open_type()?;
                        self.field(field, &mut Reader::new(&octets), depth + 1, Within::Ie)?
                    }
                };
                Ok(oxirush_utils::tree::alternative(names[index], value))
            }
            schema::Kind::Alias { field, form, .. } => match form {
                Some(_) => schema::read_field(types, &Field::Named(row as u16), reader),
                None => self.field(field, reader, depth, within),
            },
            schema::Kind::Enumerated { .. } => {
                schema::read_field(types, &Field::Named(row as u16), reader)
            }
        }
    }

    /// The tree of the member `name`, of a field, of a value of the type `of`.
    fn member(
        &self,
        of: &schema::Type,
        name: &str,
        field: &Field,
        reader: &mut Reader<'_>,
        depth: usize,
        sent: Option<Sent>,
    ) -> Result<Value, Error> {
        let rows = &self.rows;
        let within = match field {
            Field::Named(row) if Some(usize::from(*row)) == rows.ies => {
                // The IEs of a message, or of a transfer by the name of its type.
                Within::Ies(sent.or_else(|| self.protocol.transfer(of.rust)))
            }
            Field::Named(row) if Some(usize::from(*row)) == rows.extensions => {
                Within::Extensions(of.rust)
            }
            // The octets of a transfer, as the value that they contain.
            Field::Octets(lower, upper, extensible)
                if self.protocol.transfer_fields.contains(&name) =>
            {
                let octets = reader.octet_string(*lower, *upper, *extensible)?;
                let typed = match depth > NESTING {
                    true => too_deep(),
                    false => (self.protocol.transfer_typed)(name),
                };
                return Ok(self.contained(&octets, typed.ok(), depth + 1));
            }
            _ => Within::Ie,
        };
        self.field(field, reader, depth + 1, within)
    }

    /// The tree of a field, `depth` deep in the view.
    fn field(
        &self,
        field: &Field,
        reader: &mut Reader<'_>,
        depth: usize,
        within: Within,
    ) -> Result<Value, Error> {
        let rows = &self.rows;
        match *field {
            Field::Named(row)
                if Some(usize::from(row)) == rows.field
                    || Some(usize::from(row)) == rows.extension =>
            {
                self.ie(usize::from(row), reader, depth, within)
            }
            Field::Named(row) => self.row(row.into(), reader, depth, None, within),
            Field::List(of, lower, upper) if upper < 65536 => {
                let count = reader.size(lower, upper)?;
                let mut list = reader.list(count);
                for _ in 0..count {
                    list.push(self.field(of, reader, depth + 1, within)?);
                }
                Ok(Value::Array(list))
            }
            Field::List(of, lower, upper) => {
                let mut list = Vec::new();
                let (mut count, mut more) = reader.length()?;
                loop {
                    let size = list.len().saturating_add(count);
                    if !more || (upper != crate::UNBOUNDED && size > upper) {
                        Reader::check_size(size, lower, upper)?;
                    }
                    for _ in 0..count {
                        list.push(self.field(of, reader, depth + 1, within)?);
                    }
                    if !more {
                        break;
                    }
                    (count, more) = reader.length_after(count)?;
                }
                Ok(Value::Array(list))
            }
            ref field => schema::read_field(self.protocol.schema, field, reader),
        }
    }

    /// The IE of an entry of a container of `within`, of the row `row`, `depth` deep in
    /// the view: its name with its value, or with its octets when they are no value that
    /// writes them back, and its criticality when it is not the one that is assigned.
    fn ie(
        &self,
        row: usize,
        reader: &mut Reader<'_>,
        depth: usize,
        within: Within,
    ) -> Result<Value, Error> {
        let types = self.protocol.schema;
        let schema::Kind::Sequence {
            members,
            extensible,
            ..
        } = &types[row].kind
        else {
            return Err(Error::new(ErrorKind::Value, "a field of IEs is a SEQUENCE"));
        };
        if *extensible && reader.bit()? {
            return Err(Error::new(
                ErrorKind::Value,
                "a field of IEs has no additions",
            ));
        }
        let [id, criticality, value] = members else {
            return Err(Error::new(
                ErrorKind::Value,
                "a field of IEs has three members",
            ));
        };
        let id = schema::read_field(types, &id.ty, reader)?;
        let criticality = schema::read_field(types, &criticality.ty, reader)?;
        let Field::Open = value.ty else {
            return Err(Error::new(
                ErrorKind::Value,
                "the value of an IE is an open type",
            ));
        };
        let octets = reader.open_type()?;
        let id = id.as_u64().and_then(|id| u16::try_from(id).ok());
        let id = id.ok_or_else(|| Error::new(ErrorKind::Value, "an IE has its identifier"))?;
        let protocol = self.protocol;
        let name = match protocol.ie_name_of(id) {
            Some(name) => name.into(),
            None => id.to_string().into(),
        };
        let extended = match within {
            Within::Extensions(extended) => extended,
            _ => "",
        };
        let typed = match depth > NESTING {
            true => None,
            false => match (protocol.extension_typed)(id, extended) {
                Some(typed) => Some(typed),
                None => (protocol.ie_typed)(id).ok(),
            },
        };
        let value = match typed {
            Some(typed) if (typed.shown)(&octets) => {
                let read = &mut Reader::new(&octets);
                match (protocol.ie_contents)(id) {
                    // An OCTET STRING (CONTAINING ...): the value that it contains, or the
                    // octets of the IE when they are no value that writes them back.
                    Some(contents) => {
                        let row = Field::Named((typed.row)() as u16);
                        let contained = schema::read_field(types, &row, read).ok();
                        let contained = contained.and_then(|value| json::unhex(value.as_str()?));
                        match contained {
                            Some(contained) if (contents.shown)(&contained) => {
                                let read = &mut Reader::new(&contained);
                                let value =
                                    self.row((contents.row)(), read, depth + 2, None, Within::Ie);
                                value.unwrap_or_else(|_| octets_alone(&octets))
                            }
                            _ => octets_alone(&octets),
                        }
                    }
                    None => {
                        let value = self.row((typed.row)(), read, depth + 1, None, Within::Ie);
                        value.unwrap_or_else(|_| octets_alone(&octets))
                    }
                }
            }
            _ => octets_alone(&octets),
        };
        let mut shown = Map::with_capacity(2);
        shown.push(name, value);
        if criticality.as_str() != protocol.assigned(within.shape(), id) {
            shown.push("criticality".into(), criticality);
        }
        Ok(Value::Object(shown))
    }

    /// The value that the octets of a transfer contain, `depth` deep in the view, or the
    /// octets when they are none that writes them back.
    fn contained(&self, octets: &[u8], typed: Option<Typed>, depth: usize) -> Value {
        match typed {
            Some(typed) if (typed.shown)(octets) => {
                let read = &mut Reader::new(octets);
                match self.row((typed.row)(), read, depth + 1, None, Within::Ie) {
                    Ok(value) => value,
                    Err(_) => octets_alone(octets),
                }
            }
            _ => octets_alone(octets),
        }
    }
}

/// What a tree has for an IE or a transfer that has no value: its octets.
fn octets_alone(octets: &[u8]) -> Value {
    let mut alone = Map::with_capacity(1);
    alone.push("octets".into(), hex(octets).into());
    Value::Object(alone)
}

// ── The message of a tree in one pass ───────────────────────────────────────

/// How deep the members of the values of a message are written in one pass: what is
/// deeper is written through the view of the tree, which says how deep it goes.
const ONE_PASS: usize = 64;

/// The writing of the message of a PDU from its tree in one pass, with the table of the
/// types: the IEs of each container by name, and each transfer as its decoded value. It
/// writes the octets that the view of the tree writes, for a tree that its view writes
/// as it is written; it refuses what else a tree may be, as a criticality that is not
/// written as its name, and the view then writes it or says why it does not.
pub(super) struct Written {
    protocol: &'static Protocol,
    rows: schema::Containers,
    /// The row of the type of the message, and the message whose IEs it holds.
    message: (usize, Option<Sent>),
    /// How deep the member that is being written is.
    depth: core::cell::Cell<usize>,
}

impl Written {
    /// The octets of the message of a direction and a procedure code of `protocol`,
    /// `sent` where the tables have it, whose IEs are `listed` in a tree; `None` for a
    /// tree that the view writes otherwise, or does not write.
    fn message(
        protocol: &'static Protocol,
        direction: Direction,
        code: u8,
        sent: Option<Sent>,
        listed: &Value,
    ) -> Option<Vec<u8>> {
        let types = protocol.schema;
        let row = (protocol.message_typed(direction, code).row)();
        // A message is a container of IEs, which a SEQUENCE has as its one member.
        let schema::Kind::Sequence {
            names: [name],
            members: [member],
            extensible,
        } = &types.get(row)?.kind
        else {
            return None;
        };
        let written = Written {
            protocol,
            rows: protocol.containers,
            message: (row, sent),
            depth: core::cell::Cell::new(0),
        };
        let mut octets = Vec::new();
        let mut failed = None;
        let mut writer = Writer::new(&mut octets);
        let wrote = oxirush_utils::tree::read(listed, name, |listed, reading| {
            let mut walk = Walk {
                types,
                writer: &mut writer,
                failed: &mut failed,
                pdu: Some(&written),
            };
            if *extensible {
                walk.writer.bit(false);
            }
            // The tree has no member of the message: its IEs stand for it.
            match member.optional {
                true => Err(refused(reading)),
                false => written.member(
                    &mut walk,
                    row,
                    (name, &member.ty),
                    &Value::Null,
                    listed,
                    reading,
                ),
            }
        });
        if wrote.is_err() || failed.is_some() {
            return None;
        }
        if octets.is_empty() {
            octets.push(0);
        }
        Some(octets)
    }

    /// Whether the row `row` is that of the field of an IE.
    pub(super) fn is_ie(&self, row: u16) -> bool {
        let row = Some(usize::from(row));
        row == self.rows.field || row == self.rows.extension
    }

    /// Write `value`, the member `name` of `of_value`, a value of the row `of`, whose
    /// field is `field`: the IEs of a container, a transfer, or the field as the table
    /// has it.
    pub(super) fn member(
        &self,
        walk: &mut Walk<'_, '_>,
        of: usize,
        (name, field): (&'static str, &Field),
        of_value: &Value,
        value: &Value,
        reading: &mut Reading,
    ) -> Result<(), Refused> {
        let depth = self.depth.get();
        if depth > ONE_PASS {
            return Err(refused(reading));
        }
        self.depth.set(depth + 1);
        let protocol = self.protocol;
        let ty = &protocol.schema[of];
        let rows = &self.rows;
        let written = match *field {
            Field::Named(row) if Some(usize::from(row)) == rows.ies => {
                // The IEs of a message, or of a transfer by the name of its type.
                let sent = match of == self.message.0 {
                    true => self.message.1,
                    false => None,
                };
                let sent = sent.or_else(|| protocol.transfer(ty.rust));
                self.container(walk, row, value, reading, Within::Ies(sent))
            }
            Field::Named(row) if Some(usize::from(row)) == rows.extensions => {
                let within = Within::Extensions(ty.rust);
                self.container(walk, row, value, reading, within)
            }
            // A transfer that is written as the value that its octets contain, under the
            // name of its member as the view has it.
            Field::Octets(lower, upper, extensible)
                if value.is_object()
                    && protocol.transfer_fields.contains(&name)
                    && of_value.as_object().is_some_and(|of| of.contains_key(name)) =>
            {
                self.transfer(walk, name, (lower, upper, extensible), value, reading)
            }
            _ => walk.write_field(field, value, reading),
        };
        self.depth.set(depth);
        written
    }

    /// Write `value`, the IEs of a container of the row `row` that are within `within`.
    fn container(
        &self,
        walk: &mut Walk<'_, '_>,
        row: u16,
        value: &Value,
        reading: &mut Reading,
        within: Within,
    ) -> Result<(), Refused> {
        let schema::Kind::Alias {
            field: Field::List(Field::Named(field), lower, upper),
            ..
        } = self.protocol.schema[usize::from(row)].kind
        else {
            return Err(refused(reading));
        };
        let Some(entries) = value.as_array().filter(|_| upper < 65536) else {
            return Err(refused(reading));
        };
        let written = walk.writer.size(entries.len(), lower, upper);
        walk.wrote(written);
        let row = usize::from(*field);
        reading.list(value, |entry, reading| {
            self.ie(walk, row, entry, reading, within)
        })?;
        Ok(())
    }

    /// Write `entry`, the IE of a field of the row `row` within `within`, as a tree
    /// writes it: its name with its value, or with its `octets`, and its criticality
    /// beside them when it is not the one that the tables assign.
    pub(super) fn ie(
        &self,
        walk: &mut Walk<'_, '_>,
        row: usize,
        entry: &Value,
        reading: &mut Reading,
        within: Within,
    ) -> Result<(), Refused> {
        let protocol = self.protocol;
        let types = protocol.schema;
        let schema::Kind::Sequence {
            members: [id_field, criticality_field, _],
            extensible,
            ..
        } = &types[row].kind
        else {
            return Err(refused(reading));
        };
        let Some(written) = entry.as_object() else {
            return Err(refused(reading));
        };
        // The IE, by its name or its identifier in digits, as `entry_of` reads it.
        let mut members = written
            .iter()
            .filter(|(member, _)| *member != "criticality");
        let (Some((name, value)), None) = (members.next(), members.next()) else {
            return Err(refused(reading));
        };
        let id = match name.parse::<u16>() {
            Ok(id) => Some(id),
            Err(_) => (protocol.ie_id)(name),
        };
        let Some(id) = id else {
            return Err(refused(reading));
        };
        if *extensible {
            walk.writer.bit(false);
        }
        walk.write_field(&id_field.ty, &Value::from(id), reading)?;
        match written.get("criticality") {
            Some(criticality) => walk.write_field(&criticality_field.ty, criticality, reading)?,
            None => {
                let assigned = protocol.assigned(within.shape(), id);
                let assigned = assigned.ok_or_else(|| refused(reading))?;
                self.enumerated(walk, &criticality_field.ty, assigned, reading)?;
            }
        }
        // The octets of the IE as they are sent.
        let octets = value.as_object().filter(|value| value.len() == 1);
        if let Some(octets) = octets.and_then(|value| value.get("octets")) {
            return walk.write_field(&Field::Open, octets, reading);
        }
        let extended = match within {
            Within::Extensions(extended) => extended,
            _ => "",
        };
        let typed = match (protocol.extension_typed)(id, extended) {
            Some(typed) => typed,
            None => (protocol.ie_typed)(id).map_err(|_| refused(reading))?,
        };
        let row = (typed.row)();
        // The octets of a value that the value of the IE contains, which its type writes.
        let contained = match value.as_object() {
            // Octets that are written as a type that they carry, by its name.
            Some(carried) if protocol.carries(id) => {
                Some(self.carried(walk, id, carried, reading)?)
            }
            // An OCTET STRING (CONTAINING ...), as the value that it contains.
            Some(_) => match (protocol.ie_contents)(id) {
                Some(contents) => Some(self.contained(walk, (contents.row)(), value, reading)?),
                None => None,
            },
            None => None,
        };
        let contained = contained.map(|octets| Value::String(hex(&octets).into()));
        let start = walk.writer.begin_open_type();
        let written = walk.write_type(row, contained.as_ref().unwrap_or(value), reading);
        walk.writer.end_open_type(start);
        written
    }

    /// The octets of the IE `id` that `value` writes as a type that they may carry: one
    /// member, the name of that type with its value, of this protocol or of another
    /// that is registered.
    fn carried(
        &self,
        walk: &mut Walk<'_, '_>,
        id: u16,
        value: &Map,
        reading: &mut Reading,
    ) -> Result<Vec<u8>, Refused> {
        let protocol = self.protocol;
        let mut members = value.iter();
        let (Some((name, held)), None) = (members.next(), members.next()) else {
            return Err(refused(reading));
        };
        let own = protocol
            .carried
            .iter()
            .map(|(carrier, name, _)| (carrier, name));
        let other = protocol
            .foreign
            .iter()
            .map(|(carrier, name, _)| (carrier, name));
        let mut names = own.chain(other).filter(|(carrier, _)| **carrier == id);
        if !names.any(|(_, known)| same_name(known, name)) {
            return Err(refused(reading));
        }
        if !held.is_object() {
            return reading.octets(held);
        }
        if let Some(typed) = (protocol.carried_typed)(name) {
            return self.contained(walk, (typed.row)(), held, reading);
        }
        // A type of another protocol, by its tables.
        let other = protocol.foreign_type(name);
        let other = other.and_then(|(name, of)| Some((name, foreign(name, of).ok()?)));
        let Some((name, other)) = other else {
            return Err(refused(reading));
        };
        let typed = (other.carried_typed)(name).ok_or_else(|| refused(reading))?;
        let written = Written {
            protocol: other,
            rows: other.containers,
            message: (usize::MAX, None),
            depth: core::cell::Cell::new(self.depth.get()),
        };
        written.contained(walk, (typed.row)(), held, reading)
    }

    /// Write the octets of a transfer of the member `name`, an OCTET STRING of the
    /// size constraint `size`, that `value` writes: the value that they contain, or
    /// `{"octets": "…"}`.
    fn transfer(
        &self,
        walk: &mut Walk<'_, '_>,
        name: &str,
        (lower, upper, extensible): (usize, usize, bool),
        value: &Value,
        reading: &mut Reading,
    ) -> Result<(), Refused> {
        let typed = (self.protocol.transfer_typed)(name).map_err(|_| refused(reading))?;
        let octets = self.contained(walk, (typed.row)(), value, reading)?;
        let written = walk.writer.octet_string(&octets, lower, upper, extensible);
        walk.wrote(written);
        Ok(())
    }

    /// The octets of a value of the row `row` that `value` writes, which another value
    /// contains: the value itself, without a member of a transfer, or its `octets`.
    fn contained(
        &self,
        walk: &mut Walk<'_, '_>,
        row: usize,
        value: &Value,
        reading: &mut Reading,
    ) -> Result<Vec<u8>, Refused> {
        let Some(members) = value.as_object() else {
            return Err(refused(reading));
        };
        if members.keys().any(of_a_transfer) {
            let octets = members.get("octets").filter(|_| members.len() == 1);
            return match octets {
                Some(octets) => reading.octets(octets),
                None => Err(refused(reading)),
            };
        }
        let mut octets = Vec::new();
        let mut writer = Writer::new(&mut octets);
        let mut inner = Walk {
            types: self.protocol.schema,
            writer: &mut writer,
            failed: &mut *walk.failed,
            pdu: Some(self),
        };
        inner.write_type(row, value, reading)?;
        if octets.is_empty() {
            octets.push(0);
        }
        Ok(octets)
    }

    /// Write `name`, the name of a value of the ENUMERATED of `field`.
    fn enumerated(
        &self,
        walk: &mut Walk<'_, '_>,
        field: &Field,
        name: &str,
        reading: &mut Reading,
    ) -> Result<(), Refused> {
        let Field::Named(row) = *field else {
            return Err(refused(reading));
        };
        let schema::Kind::Enumerated {
            names,
            root,
            extensible,
        } = &self.protocol.schema[usize::from(row)].kind
        else {
            return Err(refused(reading));
        };
        let index = names.iter().position(|known| *known == name);
        let index = index.ok_or_else(|| refused(reading))?;
        walk.writer.index(index as u64, *root, *extensible);
        Ok(())
    }
}

/// What the writing in one pass leaves to the view of the tree, which says why.
fn refused(reading: &mut Reading) -> Refused {
    reading.refuse(
        oxirush_utils::tree::ErrorKind::Value,
        "written through the view",
    )
}
