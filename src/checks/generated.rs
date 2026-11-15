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

//! The tree of a message that the tables of a protocol make from a string of octets,
//! which say each choice: whether an optional member or IE is there, which alternative
//! or name, which number, how many entries and which octets. A tree that they make is
//! a value of its message within the root of each constraint, as a sender of this
//! version of the specification writes it, in the form that the tree of the PDU that
//! it writes has: the tests and the fuzz targets of the protocols write it, encode and
//! decode it, and compare the tree that they read back with it.
//!
//! The IEs of a message and of a transfer are those of its object set, in its order: a
//! mandatory one always, another by a choice. An IE that a value holds alone, and one
//! of an extension container, is any IE of one type, as the trees take them. A list has
//! at most four entries more than its lower bound, octets and text at most 64 and 32
//! more; below a depth of 24 a value has its mandatory members alone, a CHOICE its
//! first alternative and an IE that a value holds alone no other value.

use alloc::string::String;
use alloc::vec::Vec;

use oxirush_utils::tree::{self as utils, Map, Value};

use crate::inspect::schema::{Field, Kind, Text};
use crate::inspect::{Form, Message, Protocol};
use crate::{Criticality, Presence, UNBOUNDED, json};

/// The depth below which a value has its mandatory members alone, a CHOICE its first
/// alternative and an IE that a value holds alone no other value.
const DEPTH: usize = 24;

/// The choices that a string of octets says, one octet or more for each, which are
/// zero when the octets end: the first alternative, the lower bound, nothing optional.
pub struct Choices<'o> {
    octets: &'o [u8],
}

impl<'o> Choices<'o> {
    /// The choices that `octets` say.
    pub fn new(octets: &'o [u8]) -> Self {
        Self { octets }
    }

    /// The next octet.
    pub fn octet(&mut self) -> u8 {
        match self.octets.split_first() {
            Some((octet, rest)) => {
                self.octets = rest;
                *octet
            }
            None => 0,
        }
    }

    /// Whether something optional is there.
    pub fn bit(&mut self) -> bool {
        self.octet() & 1 == 1
    }

    /// A number below `count`, which is not zero.
    pub fn below(&mut self, count: u64) -> u64 {
        let mut number = 0u64;
        let mut range = 1u64;
        while range < count {
            number = number << 8 | u64::from(self.octet());
            range = range.saturating_mul(256);
        }
        number % count
    }

    /// A number within `lower..=upper`: a bound, next to a bound, or any.
    pub fn within(&mut self, lower: u64, upper: u64) -> u64 {
        match self.octet() % 5 {
            0 => lower,
            1 => upper,
            2 => lower.saturating_add(1).min(upper),
            3 => upper.saturating_sub(1).max(lower),
            _ => match upper - lower {
                u64::MAX => self.below(u64::MAX),
                span => lower + self.below(span + 1),
            },
        }
    }

    /// A signed number within `lower..=upper`, as [`within`](Self::within).
    fn within_signed(&mut self, lower: i64, upper: i64) -> i64 {
        let offset = self.within(0, upper.abs_diff(lower));
        lower.wrapping_add_unsigned(offset)
    }

    /// A size within `lower..=upper`, of at most `more` above the lower bound.
    fn size(&mut self, lower: usize, upper: usize, more: usize) -> usize {
        let upper = upper.min(lower.saturating_add(more));
        self.within(lower as u64, upper as u64) as usize
    }

    /// `count` octets.
    fn octets(&mut self, count: usize) -> Vec<u8> {
        (0..count).map(|_| self.octet()).collect()
    }
}

/// `count` octets that `seed` gives, the same each time: none but zeros for 0, which
/// make the smallest tree of each message.
pub fn octets(seed: u64, count: usize) -> Vec<u8> {
    // SplitMix64.
    let mut state = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    (0..count)
        .map(|_| match seed {
            0 => 0,
            _ => {
                state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
                let mut z = state;
                z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
                z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
                (z ^ (z >> 31)) as u8
            }
        })
        .collect()
}

/// The tree of a message of `protocol`, which the first choices pick among those that
/// have an object set; `None` for a protocol with none.
pub fn message(protocol: &'static Protocol, choices: &mut Choices<'_>) -> Option<Value> {
    let messages: Vec<&Message> = (protocol.messages.iter())
        .filter(|message| message.0.ies.is_some())
        .collect();
    let at = choices.below(messages.len().max(1) as u64) as usize;
    Some(tree(protocol, messages.get(at)?, choices))
}

/// The tree of `message` of `protocol`.
pub fn tree(protocol: &'static Protocol, message: &Message, choices: &mut Choices<'_>) -> Value {
    let mut made = Made { protocol, choices };
    let ies = made.ies(message.0.ies.unwrap_or_default(), 0);
    let mut tree = Map::new();
    tree.push("message".into(), utils::text(message.0.name));
    tree.push("ies".into(), ies);
    Value::Object(tree)
}

/// A tree while it is made.
struct Made<'m, 'o> {
    protocol: &'static Protocol,
    choices: &'m mut Choices<'o>,
}

impl Made<'_, '_> {
    /// The IEs of an object set, as a container of a message or of a transfer has them:
    /// each by its name, with the criticality that the set assigns it.
    fn ies(&mut self, set: &[(u16, Criticality, Presence)], depth: usize) -> Value {
        let mut ies = Vec::new();
        for (id, _, presence) in set {
            if *presence != Presence::Mandatory && (depth > DEPTH || !self.choices.bit()) {
                continue;
            }
            if let Some(value) = self.ie(*id, depth) {
                ies.push(entry(self.name(*id), value, None));
            }
        }
        Value::Array(ies)
    }

    /// An IE that a value holds alone, or one of an extension container of the type
    /// `extended`: an extension that the type gives a type, or any IE of one type. The
    /// criticality is written where no table assigns it.
    fn single(&mut self, extended: Option<&str>, depth: usize) -> Value {
        let protocol = self.protocol;
        let extensions: Vec<u16> = match extended {
            Some(extended) => (protocol.extension_types.iter())
                .filter(|(_, of, _)| *of == extended)
                .map(|(id, ..)| *id)
                .collect(),
            None => Vec::new(),
        };
        // Below the depth, an IE whose value holds no other value.
        let one_type = |id: &u16| match (protocol.ie_typed)(*id) {
            Ok(typed) => depth <= DEPTH || flat(&protocol.schema[(typed.row)()].kind),
            Err(_) => false,
        };
        let id = match (extensions.is_empty() || depth > DEPTH, self.choices.bit()) {
            (false, true) => extensions[self.choices.below(extensions.len() as u64) as usize],
            _ => {
                let ids: Vec<u16> = (protocol.ie_types.iter())
                    .map(|(id, ..)| *id)
                    .filter(one_type)
                    .collect();
                ids[self.choices.below(ids.len() as u64) as usize]
            }
        };
        let typed = match extensions.contains(&id) {
            true => extended.and_then(|extended| (protocol.extension_typed)(id, extended)),
            false => None,
        };
        let value = match typed {
            Some(typed) => self.row((typed.row)(), depth),
            None => self.ie(id, depth).unwrap_or(Value::Null),
        };
        let assigned = (protocol.item_criticalities.iter()).any(|(item, _)| *item == id);
        let criticality = match extended.is_none() && assigned {
            true => None,
            false => Some(self.criticality()),
        };
        entry(self.name(id), value, criticality)
    }

    /// The value of the IE `id`, or what its octets contain for an IE of octets that
    /// contain a type; `None` for an IE of several types. Octets that may carry a type
    /// are octets, as a tree shows them.
    fn ie(&mut self, id: u16, depth: usize) -> Option<Value> {
        let protocol = self.protocol;
        let row = match (protocol.ie_contents)(id) {
            Some(contents) => (contents.row)(),
            None => ((protocol.ie_typed)(id).ok()?.row)(),
        };
        Some(self.row(row, depth + 1))
    }

    /// The name of the IE `id`.
    fn name(&self, id: u16) -> &'static str {
        let index = self.protocol.ie_index[usize::from(id)];
        self.protocol.ie_names[usize::from(index)].1
    }

    /// A criticality.
    fn criticality(&mut self) -> Criticality {
        [
            Criticality::Reject,
            Criticality::Ignore,
            Criticality::Notify,
        ][self.choices.below(3) as usize]
    }

    /// A value of the type of `row`.
    fn row(&mut self, row: usize, depth: usize) -> Value {
        let protocol = self.protocol;
        let ty = &protocol.schema[row];
        match &ty.kind {
            Kind::Sequence { names, members, .. } => {
                let transfer = protocol
                    .transfers
                    .iter()
                    .find(|(_, rust, _)| *rust == ty.rust);
                let mut tree = Map::with_capacity(members.len());
                for (name, member) in names.iter().zip(*members) {
                    if member.optional && (depth > DEPTH || !self.choices.bit()) {
                        continue;
                    }
                    let containers = protocol.containers;
                    let value = match member.ty {
                        Field::Named(at) if Some(usize::from(at)) == containers.ies => {
                            let set = transfer.map_or(&[][..], |(.., set)| set);
                            self.ies(set, depth + 1)
                        }
                        Field::Named(at) if Some(usize::from(at)) == containers.extensions => {
                            let count = self.choices.within(1, 2);
                            let extensions =
                                (0..count).map(|_| self.single(Some(ty.rust), depth + 1));
                            Value::Array(extensions.collect())
                        }
                        _ if protocol.transfer_fields.contains(name) => {
                            match (protocol.transfer_typed)(name) {
                                Ok(typed) => self.row((typed.row)(), depth + 1),
                                Err(_) => self.field(&member.ty, depth + 1),
                            }
                        }
                        ty => self.field(&ty, depth + 1),
                    };
                    tree.push((*name).into(), value);
                }
                Value::Object(tree)
            }
            Kind::Choice {
                names,
                alternatives,
                ..
            } => {
                let at = match depth > DEPTH {
                    true => 0,
                    false => self.choices.below(names.len() as u64) as usize,
                };
                utils::alternative(names[at], self.field(&alternatives[at], depth + 1))
            }
            Kind::Enumerated { names, .. } => {
                utils::text(names[self.choices.below(names.len() as u64) as usize])
            }
            Kind::Alias {
                field,
                form: Some(form),
                ..
            } => {
                // The value in the form of the JSON Encoding Rules, in its readable form
                // where it has one.
                let jer = match (form, field) {
                    (Form::Plmn | Form::Digits, Field::OctetArray(_) | Field::Octets(..))
                        if self.choices.bit() =>
                    {
                        self.digits(field)
                    }
                    _ => self.field(field, depth + 1),
                };
                json::readable(*form, jer)
            }
            Kind::Alias { field, .. } => self.field(field, depth + 1),
        }
    }

    /// The octets of `field` in which each half octet is a digit, as an identity in
    /// TBCD has them, the last one a filler or not.
    fn digits(&mut self, field: &Field) -> Value {
        let count = match *field {
            Field::OctetArray(size) => size,
            Field::Octets(lower, upper, _) => self.choices.size(lower, upper, 8),
            _ => 0,
        };
        let mut octets: Vec<u8> = (0..count)
            .map(|_| ((self.choices.octet() % 10) << 4) | (self.choices.octet() % 10))
            .collect();
        if let Some(last) = octets.last_mut()
            && self.choices.bit()
        {
            *last |= 0xf0;
        }
        utils::octets(&octets)
    }

    /// A value of `field`.
    fn field(&mut self, field: &Field, depth: usize) -> Value {
        let protocol = self.protocol;
        let containers = protocol.containers;
        match *field {
            Field::Boolean => Value::Bool(self.choices.bit()),
            Field::Null => Value::Null,
            Field::Unsigned(lower, upper, _) => utils::unsigned(self.choices.within(lower, upper)),
            Field::Signed(lower, upper, _) => {
                utils::signed(self.choices.within_signed(lower, upper))
            }
            Field::OctetArray(size) => utils::octets(&self.choices.octets(size)),
            Field::Octets(lower, upper, _) => {
                let size = self.choices.size(lower, upper, 64);
                utils::octets(&self.choices.octets(size))
            }
            Field::Open => {
                let size = self.choices.size(0, UNBOUNDED, 16);
                utils::octets(&self.choices.octets(size))
            }
            Field::Bits(lower, upper) | Field::BitString(lower, upper, _) => {
                let length = self.choices.size(lower, upper, 64);
                let mut octets = self.choices.octets(length.div_ceil(8));
                // The bits after the string in its last octet are zero.
                if let Some(last) = octets.last_mut()
                    && !length.is_multiple_of(8)
                {
                    *last &= 0xff << (8 - length % 8);
                }
                match lower == upper {
                    true => utils::octets(&octets),
                    false => utils::bits(&octets, length),
                }
            }
            Field::Text(of, lower, upper, _) => {
                let size = self.choices.size(lower, upper, 32);
                let text: String = (0..size).map(|_| self.character(of)).collect();
                utils::text(text)
            }
            Field::ObjectIdentifier => {
                let first = self.choices.below(3);
                let second = match first {
                    2 => self.choices.below(256),
                    _ => self.choices.below(40),
                };
                let mut text = alloc::format!("{first}.{second}");
                for _ in 0..self.choices.below(4) {
                    text.push_str(&alloc::format!(".{}", self.choices.within(0, 1 << 28)));
                }
                utils::text(text)
            }
            Field::Named(row) if Some(usize::from(row)) == containers.field => {
                self.single(None, depth + 1)
            }
            Field::Named(row) if Some(usize::from(row)) == containers.ies => {
                Value::Array(Vec::new())
            }
            Field::Named(row) => self.row(usize::from(row), depth),
            Field::List(of, lower, upper) => {
                let count = match depth > DEPTH {
                    true => lower,
                    false => self.choices.size(lower, upper, 4),
                };
                Value::Array((0..count).map(|_| self.field(of, depth + 1)).collect())
            }
        }
    }

    /// A character of the alphabet `of`.
    fn character(&mut self, of: Text) -> char {
        const PRINTABLE: &[u8] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 '()+,-./:=?";
        const OTHERS: [char; 4] = ['\u{e9}', '\u{20ac}', '\u{1f600}', '\u{7f}'];
        match of {
            Text::Printable => {
                char::from(PRINTABLE[self.choices.below(PRINTABLE.len() as u64) as usize])
            }
            Text::Visible => char::from(b' ' + self.choices.below(95) as u8),
            Text::Utf8 => match self.choices.below(8) {
                at @ 0..4 => OTHERS[at as usize],
                _ => char::from(b' ' + self.choices.below(95) as u8),
            },
        }
    }
}

/// Checks each IE of a PDU of `protocol` alone, `ies` by their identifiers and octets in
/// the order of the message, whose tree `shown` shows them: the walker of the table of
/// its type reads the octets of each as a value that writes them back, exactly. A value
/// that holds no container of IEs, which the walker writes by their identifiers, is the
/// one that the tree shows. An IE of octets that contain a type is the value that they
/// contain in the tree, and is left to the PDU.
pub fn each_ie_by_its_type(protocol: &'static Protocol, ies: &[(u16, &[u8])], shown: &Value) {
    let entries = shown.get("ies").and_then(Value::as_array);
    for (nth, (id, octets)) in ies.iter().enumerate() {
        let Ok(typed) = (protocol.ie_typed)(*id) else {
            continue;
        };
        let decoded = (typed.decode)(octets);
        let Ok((value, exact, refused)) = decoded else {
            panic!("IE {id} does not read alone: {decoded:?}");
        };
        assert!(exact && refused.is_none(), "IE {id} is not exact: {value}");
        let again = (typed.encode)(&value);
        assert_eq!(again.as_deref(), Ok(*octets), "IE {id} from {value}");
        if (protocol.ie_contents)(*id).is_some() || holds_ies(protocol, (typed.row)()) {
            continue;
        }
        let entry = entries.and_then(|entries| entries.get(nth));
        let view = entry.and_then(|entry| entry.as_object()?.iter().next().map(|(_, value)| value));
        if let Some(view) = view {
            assert_eq!(view, &value, "IE {id}");
        }
    }
}

/// Whether a value of the type of `row` may hold a container of IEs, or a transfer.
fn holds_ies(protocol: &'static Protocol, row: usize) -> bool {
    let containers = protocol.containers;
    let ies = [
        containers.ies,
        containers.field,
        containers.extensions,
        containers.extension,
    ];
    let mut seen = alloc::vec![false; protocol.schema.len()];
    let mut rows = alloc::vec![row];
    while let Some(row) = rows.pop() {
        if core::mem::replace(&mut seen[row], true) {
            continue;
        }
        if ies.contains(&Some(row)) {
            return true;
        }
        let mut fields: Vec<&Field> = Vec::new();
        match &protocol.schema[row].kind {
            Kind::Sequence { names, members, .. } => {
                if names
                    .iter()
                    .any(|name| protocol.transfer_fields.contains(name))
                {
                    return true;
                }
                fields.extend(members.iter().map(|member| &member.ty));
            }
            Kind::Choice { alternatives, .. } => fields.extend(alternatives.iter()),
            Kind::Alias { field, .. } => fields.push(field),
            Kind::Enumerated { .. } => {}
        }
        while let Some(field) = fields.pop() {
            match field {
                Field::Named(at) => rows.push(usize::from(*at)),
                Field::List(of, ..) => fields.push(of),
                _ => {}
            }
        }
    }
    false
}

/// Whether a value of a type of this kind holds no other value.
fn flat(kind: &Kind) -> bool {
    match kind {
        Kind::Enumerated { .. } => true,
        Kind::Alias { field, .. } => !matches!(field, Field::Named(_) | Field::List(..)),
        _ => false,
    }
}

/// Where `read` is not `written` first, as a path of names and positions, with what each
/// has there; empty where they are the same.
pub fn difference(read: &Value, written: &Value) -> String {
    match (read, written) {
        (Value::Object(read), Value::Object(written)) => {
            fn names(map: &Map) -> Vec<&str> {
                map.iter().map(|(name, _)| name).collect()
            }
            if names(read) != names(written) {
                return alloc::format!(": {:?} where {:?} is written", names(read), names(written));
            }
            for ((name, read), (_, written)) in read.iter().zip(written.iter()) {
                let found = difference(read, written);
                if !found.is_empty() {
                    return alloc::format!("/{name}{found}");
                }
            }
            String::new()
        }
        (Value::Array(read), Value::Array(written)) if read.len() == written.len() => {
            for (at, (read, written)) in read.iter().zip(written).enumerate() {
                let found = difference(read, written);
                if !found.is_empty() {
                    return alloc::format!("/{at}{found}");
                }
            }
            String::new()
        }
        (read, written) if read == written => String::new(),
        (read, written) => alloc::format!(": {read} where {written} is written"),
    }
}

/// An entry of a container: the value of an IE by its name, with its criticality.
fn entry(name: &'static str, value: Value, criticality: Option<Criticality>) -> Value {
    let mut entry = Map::new();
    entry.push(name.into(), value);
    if let Some(criticality) = criticality {
        entry.push("criticality".into(), utils::text(criticality.name()));
    }
    Value::Object(entry)
}
