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

//! The views and the trees of the PDUs of an application protocol: each IE and each
//! transfer of a message as its type is decoded, beside the octets that it was received
//! as, and the message that a view or a tree writes encoded again.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use oxirush_utils::identity::Plmn;
use serde_json::{Map, Value, json};

use super::paths::{EDITED, extended, member_shape, of_a_transfer, written_ie};
use super::{Protocol, Shape, Typed, same_name};
use crate::Error;
use crate::json::{self, hex};

/// How deep in a view an IE or a transfer is still decoded.
const NESTING: usize = 64;

/// The octets that `value` writes in hexadecimal.
fn unhex(value: &Value) -> Result<Vec<u8>, Error> {
    let octets = value.as_str().and_then(json::unhex);
    octets.ok_or_else(|| {
        // The start of what is written says which value it is.
        let written: String = value.to_string().chars().take(40).collect();
        Error::value(format!(
            "{written} is not octets: octets are written in hexadecimal, two digits for each"
        ))
    })
}

/// How a view and a tree show the values of a well-known type, and take them back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    /// A PLMN identity as its MCC and MNC: `"208-93"`.
    Plmn,
    /// The digits of an identity in TBCD, as an IMSI has them.
    Digits,
    /// An identifier or a counter of this many bits, as a number.
    Number(u32),
    /// A transport layer address: an IPv4 address, an IPv6 address, or both with a comma.
    Address,
}

impl Form {
    /// The readable form of a value as its ASN.1 type has it; `None` when it has none.
    fn read(self, jer: &Value) -> Option<Value> {
        // The decimal digits that these half octets are; none when one of them is no digit.
        let digits = |digits: &[u8]| {
            let text = digits
                .iter()
                .map(|digit| char::from_digit((*digit).into(), 10));
            text.collect::<Option<String>>()
        };
        match self {
            Self::Plmn => {
                // Octets that are no PLMN identity stay octets.
                let octets = <[u8; 3]>::try_from(unhex(jer).ok()?).ok()?;
                let plmn = Plmn::from_ap_octets(octets).ok()?;
                Some(json!(format!("{}-{}", plmn.mcc(), plmn.mnc())))
            }
            Self::Digits => {
                let octets = unhex(jer).ok()?;
                let nibbles: Vec<u8> = octets.iter().flat_map(|o| [o & 15, o >> 4]).collect();
                // A filler takes the place of a last digit that is not there.
                let nibbles = nibbles.strip_suffix(&[15]).unwrap_or(&nibbles);
                digits(nibbles).map(Value::from)
            }
            Self::Number(bits) => {
                let (text, octets) = (jer.as_str()?, bits.div_ceil(8));
                let number = u64::from_str_radix(text, 16).ok()?;
                (text.len() as u32 == octets * 2).then(|| json!(number >> (octets * 8 - bits)))
            }
            Self::Address => {
                let octets = unhex(jer.get("value")?).ok()?;
                let v4 = |octets: &[u8]| Some(Ipv4Addr::from(<[u8; 4]>::try_from(octets).ok()?));
                let v6 = |octets: &[u8]| Some(Ipv6Addr::from(<[u8; 16]>::try_from(octets).ok()?));
                Some(json!(match (jer.get("length")?.as_u64()?, octets.len()) {
                    (32, 4) => v4(&octets)?.to_string(),
                    (128, 16) => v6(&octets)?.to_string(),
                    (160, 20) => format!("{},{}", v4(&octets[..4])?, v6(&octets[4..])?),
                    _ => return None,
                }))
            }
        }
    }

    /// The value as its ASN.1 type has it, from its readable form. A value in another
    /// form is kept as it is: the form of the type is still taken, and the codec refuses
    /// what is neither.
    fn write(self, shown: &Value) -> Result<Value, Error> {
        let number = |bits: u32, number: u64| {
            if bits < 64 && number >> bits != 0 {
                return Err(Error::value(format!(
                    "{number} does not fit in {bits} bits"
                )));
            }
            let octets = bits.div_ceil(8);
            let width = octets as usize * 2;
            Ok(json!(format!("{:0width$x}", number << (octets * 8 - bits))))
        };
        let Some(text) = shown.as_str() else {
            return match (self, shown.as_u64()) {
                (Self::Number(bits), Some(value)) => number(bits, value),
                _ => Ok(shown.clone()),
            };
        };
        let bcd = |digits: &str| digits.bytes().map(|digit| digit & 15).collect::<Vec<_>>();
        let all_digits = |text: &str| text.bytes().all(|b| b.is_ascii_digit());
        match self {
            Self::Plmn if text.contains('-') => {
                let (mcc, mnc) = text.split_once('-').unwrap_or_default();
                // The three digits of an MCC, and the two or three of an MNC.
                let plmn = (all_digits(mcc) && all_digits(mnc))
                    .then(|| Plmn::new(mcc, mnc).ok())
                    .flatten();
                let plmn = plmn.ok_or_else(|| {
                    Error::value(format!("{text}: a PLMN identity is MCC-MNC, as 208-93"))
                })?;
                Ok(Value::String(hex(&plmn.to_ap_octets())))
            }
            // Without a hyphen it is its three octets: what is neither says both forms,
            // where the codec would say that it is no octets.
            Self::Plmn if text.len() != 6 || !text.bytes().all(|b| b.is_ascii_hexdigit()) => {
                Err(Error::value(format!(
                    "{text}: a PLMN identity is MCC-MNC, as 208-93, or its three octets in \
                     hexadecimal"
                )))
            }
            Self::Digits if all_digits(text) => {
                let pairs = bcd(text);
                let pairs = pairs.chunks(2);
                let octets = pairs.map(|pair| pair.get(1).copied().unwrap_or(15) << 4 | pair[0]);
                Ok(Value::String(hex(&octets.collect::<Vec<_>>())))
            }
            Self::Number(bits) if text.starts_with("0x") => {
                let value = u64::from_str_radix(&text[2..], 16).map_err(|error| {
                    Error::value(format!("{text} is not a number in hexadecimal: {error}"))
                })?;
                number(bits, value)
            }
            Self::Address if text.contains(['.', ':']) => {
                let mut octets = Vec::new();
                for part in text.split(',') {
                    match (octets.len(), part.trim().parse()) {
                        (0, Ok(IpAddr::V4(address))) => octets.extend(address.octets()),
                        (0 | 4, Ok(IpAddr::V6(address))) => octets.extend(address.octets()),
                        _ => {
                            return Err(Error::value(format!(
                                "{text}: a transport layer address is an IPv4 address, an IPv6 \
                                 address, or the two with a comma"
                            )));
                        }
                    }
                }
                Ok(json!({"value": hex(&octets), "length": octets.len() * 8}))
            }
            _ => Ok(shown.clone()),
        }
    }
}

/// Show `value`, or each value of a list, in the readable form that it has: only a value
/// that the form gives back as it was.
fn show(form: Form, value: &mut Value) {
    match value {
        Value::Array(values) => values.iter_mut().for_each(|value| show(form, value)),
        value => {
            let shown = form.read(value);
            if let Some(shown) =
                shown.filter(|shown| form.write(shown).is_ok_and(|jer| jer == *value))
            {
                *value = shown;
            }
        }
    }
}

/// `value`, or each value of a list, as its ASN.1 type has it again.
fn unshow(form: Form, value: &mut Value) -> Result<(), Error> {
    match value {
        Value::Array(values) => values.iter_mut().try_for_each(|value| unshow(form, value)),
        value => {
            *value = form.write(value)?;
            Ok(())
        }
    }
}

/// The value that the octets `raw` contain, beside them, or why they did not decode.
fn contained(raw: Value, typed: Result<Typed, Error>) -> Value {
    let decoded = typed.and_then(|typed| (typed.decode)(&unhex(&raw)?));
    let mut transfer = Map::new();
    transfer.insert("_raw_transfer".into(), raw);
    match decoded {
        Ok(decoded) => transfer.insert("decoded".into(), decoded),
        Err(error) => transfer.insert("_decode_error".into(), error.to_string().into()),
    };
    Value::Object(transfer)
}

/// Why what is deeper than the inspection goes keeps its octets.
fn too_deep<T>() -> Result<T, Error> {
    Err(Error::limit(format!(
        "inspection nesting exceeds {NESTING}"
    )))
}

/// A view whose direction is none of the three.
fn no_direction() -> Error {
    Error::form(
        "the direction of a view is InitiatingMessage, SuccessfulOutcome or UnsuccessfulOutcome",
    )
}

impl Protocol {
    /// The octets `raw` as the transfer that they are when they carry the type that
    /// ASN.1 names `name`: its decoded value beside them, or why they are not of that
    /// type.
    pub(crate) fn opened(&self, raw: Value, name: &str) -> Value {
        let typed = (self.carried_typed)(name);
        let carries_none = || Error::no_such_member(format!("octets carry no {name}"));
        let mut transfer = contained(raw, typed.ok_or_else(carries_none));
        self.expand(&mut transfer, 0, self.carried_shape(name));
        transfer
    }

    /// The octets of the IE `id`, which is written as a type that they may carry: one
    /// member, the name of that type with its value, which is its octets by now.
    fn carried_octets(&self, written: &Value, id: u16) -> Result<Value, Error> {
        let carried = self.carried.iter().filter(|(carrier, ..)| *carrier == id);
        let names: Vec<_> = carried.map(|(_, name, _)| *name).collect();
        let mut members = written.as_object().into_iter().flatten();
        match (members.next(), members.next()) {
            (Some((name, octets)), None) if names.iter().any(|known| same_name(known, name)) => {
                Ok(octets.clone())
            }
            _ => Err(Error::value(format!(
                "IE {id} is written as its octets, or as {{NAME: VALUE}} for a type that they \
                 carry: {}",
                names.join(", ")
            ))),
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
    fn message_typed(&self, direction: &str, code: u8) -> Typed {
        let mut messages = self.messages.iter();
        let message = messages.find(|(of, procedure, ..)| *procedure == code && *of == direction);
        message.map_or_else(self.any_message, |(_, _, _, typed, ..)| typed())
    }

    /// Decode the IEs and the transfers of `value`, a value of `shape` which is `depth`
    /// deep in its view. What is deeper than the inspection goes keeps its octets, with
    /// why.
    fn expand(&self, value: &mut Value, depth: usize, shape: Shape) {
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
                let raw = raw.take();
                let decoded = match deep {
                    true => too_deep(),
                    false => {
                        unhex(&raw).and_then(|bytes| (self.ie_type(id, shape)?.decode)(&bytes))
                    }
                };
                match decoded {
                    Ok(decoded) => {
                        // An OCTET STRING (CONTAINING ...) has the form of a transfer.
                        let mut decoded = match (self.ie_contents)(id) {
                            Some(contents) => contained(decoded, Ok(contents)),
                            None => decoded,
                        };
                        if let Some(form) = (self.ie_form)(id) {
                            show(form, &mut decoded);
                        }
                        object.insert(field.into(), decoded);
                    }
                    // The value stays the octets that it was received as.
                    Err(error) => {
                        object.insert(field.into(), raw.clone());
                        object.insert("_decode_error".into(), error.to_string().into());
                    }
                }
                object.insert("_raw_value".into(), raw);
                object.insert("_original_id".into(), id.into());
                if let Some(name) = self.ie_name_of(id) {
                    object.insert("_ie_name".into(), name.into());
                }
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
                        *child = contained(child.take(), typed);
                    }
                    self.expand(child, depth + 1, member_shape(&members, key));
                    if let Some(form) = (self.member_form)(key) {
                        show(form, child);
                    }
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
            return Err(Error::limit(format!(
                "inspection nesting exceeds {}",
                4 * NESTING
            )));
        }
        if let Some(object) = value.as_object_mut() {
            // An IE may be written with its name as `id`, and with its `octets`.
            written_ie(self, object)?;
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
                if let Some(form) = (self.member_form)(key) {
                    unshow(form, child)?;
                }
                if let Some(raw) = child.get("_raw_transfer").cloned() {
                    self.received_transfer(child, key, id, raw)?;
                } else if let Some(members) = child.as_object() {
                    // What was not received is encoded from the value written.
                    let transfer = match key.as_str() {
                        "value" | "extensionValue" => id.and_then(self.ie_contents),
                        key if self.transfer_fields.contains(&key) => {
                            Some((self.transfer_typed)(key)?)
                        }
                        // A type that the octets of an IE may carry, by its name.
                        key => (self.carried_typed)(key),
                    };
                    if let Some(transfer) = transfer {
                        // The decoded value, as the object itself when it has no member
                        // of a transfer or as its `decoded` member, or the `octets`.
                        let alone = |member| members.get(member).filter(|_| members.len() == 1);
                        let wire = if !members.keys().any(|name| of_a_transfer(name)) {
                            Value::String(hex(&(transfer.encode)(child)?))
                        } else if let Some(written) = alone("decoded") {
                            Value::String(hex(&(transfer.encode)(written)?))
                        } else if let Some(octets) = alone("octets") {
                            octets.clone()
                        } else {
                            return Err(Error::form(format!(
                                "{key} is written with its decoded value alone, or as its octets"
                            )));
                        };
                        *child = wire;
                    }
                }
            }
            let raw = object.shift_remove("_raw_value");
            let original_id = object.shift_remove("_original_id");
            let undecoded = object.shift_remove("_decode_error").is_some();
            object.shift_remove("_ie_name");
            object.shift_remove(EDITED);
            if let Some(id) = id {
                let wire =
                    self.ie_octets(object, member, shape, id, raw, original_id, undecoded)?;
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

    /// The octets of a transfer of the member `key`, which was received as `raw`: those
    /// received for a value that was not edited, and its edited value encoded.
    fn received_transfer(
        &self,
        child: &mut Value,
        key: &str,
        id: Option<u16>,
        raw: Value,
    ) -> Result<(), Error> {
        // Nothing that is written is left out: a transfer has its decoded value beside
        // the octets that it was received as, and no other.
        let known = ["_raw_transfer", "decoded", "_decode_error", EDITED];
        let mut members = child.as_object().into_iter().flatten();
        if let Some((other, _)) = members.find(|(name, _)| !known.contains(&name.as_str())) {
            return Err(Error::no_such_member(format!(
                "{key} has no member {other:?}: a transfer has its decoded value"
            )));
        }
        let Some(edited) = child.get("decoded") else {
            *child = raw;
            return Ok(());
        };
        let bytes = unhex(&raw)?;
        // An IE's identifier gives the type of what it contains, and the name of a
        // transfer field the type of the transfer.
        let transfer = match key {
            "value" | "extensionValue" => id.and_then(self.ie_contents).ok_or_else(|| {
                Error::form("an IE that contains no type has no decoded value beside its octets")
            })?,
            key => match (self.carried_typed)(key) {
                Some(carried) => carried,
                None => (self.transfer_typed)(key)?,
            },
        };
        let original = (transfer.decode)(&bytes)?;
        if &original == edited {
            *child = raw;
        } else {
            if (transfer.encode)(&original)? != bytes {
                return Err(Error::edit(format!(
                    "{key} does not encode back to the octets received"
                )));
            }
            *child = Value::String(hex(&(transfer.encode)(edited)?));
        }
        Ok(())
    }

    /// The octets of the IE `id`, whose entry is `object` in a container of `shape`
    /// that the member `member` holds: those that it was received as, `raw`, for a
    /// value that was not edited, and its value encoded otherwise.
    #[allow(clippy::too_many_arguments)]
    fn ie_octets(
        &self,
        object: &mut Map<String, Value>,
        member: &str,
        shape: Shape,
        id: u16,
        raw: Option<Value>,
        original_id: Option<Value>,
        undecoded: bool,
    ) -> Result<Value, Error> {
        // The value of an IE of an extension container has another name.
        let field = match member.starts_with("iE-Extension") {
            true => "extensionValue",
            false => "value",
        };
        // Octets that are written as a type that they may carry.
        if let Some(written) = object.get_mut(field).filter(|value| value.is_object())
            && self.carried.iter().any(|(carrier, ..)| *carrier == id)
        {
            *written = self.carried_octets(written, id)?;
        }
        let typed = |id: u16| {
            let octets = format!("give its octets as _raw_value, without {field}");
            self.ie_type(id, shape)
                .map_err(|error| error.reworded(|what| format!("{what}: {octets}")))
        };
        if !undecoded
            && let Some(form) = (self.ie_form)(id)
            && let Some(value) = object.get_mut(field)
        {
            unshow(form, value)?;
        }
        Ok(match (raw, object.get(field)) {
            // The octets alone are sent as they are.
            (Some(raw), None) => raw,
            // The IE was shown as its octets.
            (_, Some(edited)) if undecoded => {
                if !edited.is_string() {
                    return Err(Error::edit(format!(
                        "IE {id} did not decode: its value is its octets in hexadecimal"
                    )));
                }
                edited.clone()
            }
            (Some(raw), Some(edited)) => {
                // The identifier that the octets were decoded as, when a view has one:
                // without it they are those of the identifier of the entry.
                let original_id = match &original_id {
                    Some(original) => (original.as_u64())
                        .and_then(|v| u16::try_from(v).ok())
                        .ok_or_else(|| {
                            Error::form(
                                "_original_id is the identifier that the octets were decoded \
                                 as, a number",
                            )
                        })?,
                    None => id,
                };
                let bytes = unhex(&raw)?;
                let received = self.ie_type(original_id, shape)?;
                let original = (received.decode)(&bytes)?;
                if &original == edited && id == original_id {
                    raw
                } else if (received.encode)(&original)? != bytes {
                    return Err(Error::edit(format!(
                        "IE {original_id} does not encode back to the octets received; \
                         replace its octets instead"
                    )));
                } else {
                    Value::String(hex(&(typed(id)?.encode)(edited)?))
                }
            }
            // An IE that was not received.
            (None, Some(new)) => Value::String(hex(&(typed(id)?.encode)(new)?)),
            (None, None) => {
                return Err(Error::form(format!(
                    "IE {id} has no {field} and no _raw_value"
                )));
            }
        })
    }

    /// The name that ASN.1 gives the message of a direction and a procedure code.
    fn message_name_of(&self, direction: &str, code: u8) -> Option<&'static str> {
        let mut messages = self.messages.iter();
        let message = messages.find(|(of, procedure, ..)| *procedure == code && *of == direction);
        message.map(|(_, _, name, ..)| *name)
    }

    /// The criticality that ASN.1 assigns to the procedure of the message that it names
    /// `name`.
    fn message_criticality(&self, name: &str) -> Option<&'static str> {
        let mut messages = self.messages.iter();
        let (_, code, ..) = messages.find(|(_, _, known, ..)| same_name(known, name))?;
        let mut procedures = self.procedure_criticalities.iter();
        let procedure = procedures.find(|(known, _)| known == code);
        procedure.map(|(_, criticality)| *criticality)
    }
}

/// The view of a PDU of `protocol` of a direction, a procedure code and a criticality,
/// whose message is the octets `raw`: its `procedure_code`, its `direction`, its
/// `criticality` and its `message`, each IE of which has its typed value and what it was
/// received as.
///
/// # Errors
///
/// The message does not decode.
pub fn view(
    protocol: &'static Protocol,
    direction: &'static str,
    code: u8,
    criticality: Value,
    raw: &[u8],
) -> Result<Value, Error> {
    let mut message = (protocol.message_typed(direction, code).decode)(raw)?;
    protocol.expand(&mut message, 0, protocol.message_shape(direction, code));
    let mut view = Map::new();
    view.insert("procedure_code".into(), code.into());
    view.insert("direction".into(), direction.into());
    view.insert("criticality".into(), criticality);
    view.insert("message".into(), message);
    view.insert("_raw_message".into(), hex(raw).into());
    Ok(Value::Object(view))
}

/// The tree of the PDU of `protocol` whose view is `view`: the name of its message and
/// its IEs by name, in their order, and its criticality when it is not the one of its
/// procedure. The view is taken: its IEs are moved into the tree, and not copied.
///
/// # Errors
///
/// A procedure that the protocol does not have in that direction, and a message without
/// a list of IEs of the protocol.
pub fn tree(protocol: &'static Protocol, mut view: Value) -> Result<Value, Error> {
    let direction = view["direction"].as_str().unwrap_or_default();
    let code = view["procedure_code"].as_u64().unwrap_or_default();
    let known = u8::try_from(code).ok();
    let name = known.and_then(|code| protocol.message_name_of(direction, code));
    let name = name.ok_or_else(|| {
        Error::unknown_message(format!(
            "{} has no {direction} of procedure {code}",
            protocol.name
        ))
    })?;
    let listed = view.pointer_mut("/message/protocolIEs").map(Value::take);
    let Some(listed) = listed.filter(Value::is_array) else {
        return Err(Error::limit(format!("a message {name} has no list of IEs")));
    };
    let mut tree = Map::new();
    tree.insert("message".into(), name.into());
    tree.insert("ies".into(), super::named_ies(protocol, name, listed)?);
    if view["criticality"].as_str() != protocol.message_criticality(name) {
        tree.insert("criticality".into(), view["criticality"].take());
    }
    Ok(Value::Object(tree))
}

/// What the PDU that a view or a tree writes is made of: its direction, its procedure
/// code, its criticality as JSON, which the protocol reads as its own type, and the
/// octets of its message.
pub struct Parts {
    /// `InitiatingMessage`, `SuccessfulOutcome` or `UnsuccessfulOutcome`.
    pub direction: &'static str,
    /// The procedure code.
    pub code: u8,
    /// The criticality of the PDU, as JSON.
    pub criticality: Value,
    /// The octets of the message: those received for what was not edited, and what was
    /// written encoded.
    pub message: Vec<u8>,
}

/// The direction that a view or a tree names, as one of the three.
fn direction_of(direction: &str) -> Result<&'static str, Error> {
    const DIRECTIONS: [&str; 3] = [
        "InitiatingMessage",
        "SuccessfulOutcome",
        "UnsuccessfulOutcome",
    ];
    let known = DIRECTIONS.iter().find(|known| **known == direction);
    known.copied().ok_or_else(no_direction)
}

/// The octets of the message of a direction and a procedure code of `protocol` that
/// `message`, the message of a view, writes, which was received as the octets
/// `received` when the view has them: those received for a message that was not
/// edited.
fn message_octets(
    protocol: &'static Protocol,
    direction: &str,
    code: u8,
    mut message: Value,
    received: Option<&Value>,
) -> Result<Vec<u8>, Error> {
    protocol.collapse(&mut message, "", 0, protocol.message_shape(direction, code))?;
    let typed = protocol.message_typed(direction, code);
    let Some(original) = received else {
        return (typed.encode)(&message);
    };
    let original = unhex(original)?;
    let decoded = (typed.decode)(&original)?;
    if decoded == message {
        return Ok(original);
    }
    if (typed.encode)(&decoded)? != original {
        return Err(Error::edit(
            "the message does not encode back to the octets received",
        ));
    }
    (typed.encode)(&message)
}

/// The parts of the PDU of `protocol` that a tree writes: a message by its name and its
/// IEs by name, as [`tree`] shows one.
///
/// # Errors
///
/// What is no tree, a name that is no message of the protocol, and an IE that is not
/// written as its type.
pub fn tree_parts(protocol: &'static Protocol, tree: &Value) -> Result<Parts, Error> {
    const FORM: &str = "a tree is {\"message\": NAME, \"ies\": [{NAME: VALUE}, …]}";
    let form = || Error::form(FORM);
    let written = tree.as_object().ok_or_else(form)?;
    if let Some(other) = written
        .keys()
        .find(|key| !["message", "ies", "criticality"].contains(&key.as_str()))
    {
        return Err(Error::form(format!(
            "{other:?} is no member of a tree: {FORM}"
        )));
    }
    let name = written.get("message").and_then(Value::as_str);
    let name = name.ok_or_else(form)?;
    let mut messages = protocol.messages.iter();
    let message = messages.find(|(_, _, known, ..)| same_name(known, name));
    let (direction, code, name, ..) = message.ok_or_else(|| {
        Error::unknown_message(format!("{name:?} is not a message of {}", protocol.name))
    })?;
    let assigned = json!(protocol.message_criticality(name));
    let criticality = written.get("criticality").unwrap_or(&assigned).clone();
    let listed = written.get("ies").filter(|ies| ies.is_array());
    let mut message = Map::new();
    message.insert(
        "protocolIEs".into(),
        super::listed_ies(protocol, name, listed.ok_or_else(form)?)?,
    );
    let message = message_octets(protocol, direction, *code, Value::Object(message), None)?;
    Ok(Parts {
        direction,
        code: *code,
        criticality,
        message,
    })
}

/// The parts of the PDU of `protocol` that a view writes, as [`view`] gives one and the
/// edits leave it, with the octets received for what was not edited. A view that is
/// written by hand, without what a view keeps of what was received, is encoded from its
/// values.
///
/// # Errors
///
/// What is no view, and a value that does not encode as its type.
pub fn view_parts(protocol: &'static Protocol, view: &Value) -> Result<Parts, Error> {
    const MEMBERS: [&str; 5] = [
        "procedure_code",
        "direction",
        "criticality",
        "message",
        "_raw_message",
    ];
    const FORM: &str =
        "a view has its procedure_code, its direction, its criticality and its message";
    let members = view.as_object().ok_or_else(|| Error::form(FORM))?;
    if let Some(other) = members.keys().find(|key| !MEMBERS.contains(&key.as_str())) {
        return Err(Error::form(format!(
            "{other:?} is no member of a view: {FORM}"
        )));
    }
    let code = view["procedure_code"]
        .as_u64()
        .and_then(|c| u8::try_from(c).ok())
        .ok_or_else(|| Error::form("the procedure_code of a view is a number from 0 to 255"))?;
    let direction = direction_of(view["direction"].as_str().ok_or_else(no_direction)?)?;
    let message = view.get("message").ok_or_else(|| Error::form(FORM))?;
    let received = view.get("_raw_message");
    let message = message_octets(protocol, direction, code, message.clone(), received)?;
    Ok(Parts {
        direction,
        code,
        criticality: view["criticality"].clone(),
        message,
    })
}

/// Whether `view` is a view of the PDU of `protocol` of a direction and a procedure
/// code, where it says which message it is of.
///
/// # Errors
///
/// An error of the kind `Form` when the view is of another message.
pub fn of_the_pdu(
    protocol: &'static Protocol,
    view: &Value,
    direction: &str,
    code: u8,
) -> Result<(), Error> {
    let of = (view["direction"].as_str(), view["procedure_code"].as_u64());
    // A view that does not say which message it is of is refused as any view is.
    let (Some(of), Some(procedure)) = of else {
        return Ok(());
    };
    if (of, procedure) == (direction, u64::from(code)) {
        return Ok(());
    }
    let message = |direction: &str, code: u64| {
        let known = u8::try_from(code).ok();
        match known.and_then(|code| protocol.message_name_of(direction, code)) {
            Some(name) => name.to_string(),
            None => format!("the {direction} of procedure {code}"),
        }
    };
    Err(Error::form(format!(
        "this view is of {}, and the PDU is {}: with_view takes a view of the PDU itself, and \
         from_view any view",
        message(of, procedure),
        message(direction, code.into()),
    )))
}
