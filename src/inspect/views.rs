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

use oxirush_utils::tree::{Map, Value};

use super::paths::{extended, member_shape, of_a_transfer, written_ie};
use super::{Protocol, Shape, Typed, foreign, same_name};
use crate::json::{self, hex};
use crate::{Direction, Error, ErrorKind};

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
                    if self.transfer_fields.contains(&key.as_str()) && child.is_string() {
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
                    let transfer = match key.as_str() {
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
                        let wire = if !members.keys().any(|name| of_a_transfer(name)) {
                            Value::String(hex(&(transfer.encode)(child)?))
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
        Ok(Value::String(hex(&(typed.encode)(value)?)))
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
    let decoded = (protocol.message_typed(direction, code).decode)(raw).ok();
    let ies = decoded
        .filter(|(_, exact, _)| *exact)
        .and_then(|(mut message, ..)| {
            protocol.expand(&mut message, 0, protocol.message_shape(direction, code));
            message.get_mut("protocolIEs").map(core::mem::take)
        });
    tree_of(protocol, direction, code, criticality, ies, || {
        hex(raw).into()
    })
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
    let sent = name.and_then(|name| protocol.sent(|_, _, known| known == name));
    let ies = ies
        .filter(Value::is_array)
        .and_then(|listed| protocol.by_name(listed, Shape::Ies(sent)).ok());
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
            let mut message = Map::new();
            let entries = protocol.written(listed, Shape::Ies(sent), 0)?;
            message.insert("protocolIEs".into(), entries);
            message_octets(protocol, direction, code, Value::Object(message))?
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
    let octets = match ies {
        Some(_) => Value::Null,
        None => hex(&message_octets(protocol, direction, code, message)?).into(),
    };
    let criticality = view["criticality"].clone();
    Ok(tree_of(protocol, direction, code, criticality, ies, || {
        octets
    }))
}
