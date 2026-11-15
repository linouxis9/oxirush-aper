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

//! What the tree of a value of each type of a protocol takes, made from its table of the
//! types, as oxirush-utils says it ([`Takes`]): the members of a SEQUENCE, the
//! alternatives of a CHOICE, the names of an ENUMERATED, the readable form of a
//! well-known type, and the bounds of a number or the size of octets. The IEs of a
//! container are a message, written as the tree of a message writes its IEs; a transfer
//! is the value that its octets contain.

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;

use oxirush_utils::tree::Takes;

use super::Protocol;
use super::schema::{self, Field};
use crate::json::Form;

/// The containers of IEs, by the names of their types in the bindings.
const CONTAINERS: [&str; 4] = [
    "ProtocolIEContainer",
    "ProtocolIEField",
    "ProtocolExtensionContainer",
    "ProtocolExtensionField",
];

/// What the tree of a value of each type of `protocol` takes, by its row. A type that
/// holds itself, which no type of 3GPP does, takes octets where it holds itself.
pub fn takes(protocol: &Protocol) -> Vec<&'static Takes> {
    let mut made = Made {
        protocol,
        rows: vec![Row::Unmade; protocol.schema.len()],
    };
    (0..protocol.schema.len())
        .map(|row| made.row(row))
        .collect()
}

/// What a value of `field` of `protocol` takes, whose rows take `rows`. A list of a type of
/// no name, and a string of bits whose size varies, are made for the call and kept.
pub(super) fn field(protocol: &Protocol, rows: &[&'static Takes], field: &Field) -> Takes {
    let mut made = Made {
        protocol,
        rows: rows.iter().map(|takes| Row::Made(takes)).collect(),
    };
    *made.field(field)
}

/// A row while the table is made.
#[derive(Clone, Copy)]
enum Row {
    Unmade,
    Making,
    Made(&'static Takes),
}

/// What the rows of a protocol take, as far as it is made.
struct Made<'p> {
    protocol: &'p Protocol,
    rows: Vec<Row>,
}

/// A value of the table, kept for the life of the program, as a table of constants
/// would be.
fn kept(takes: Takes) -> &'static Takes {
    Box::leak(Box::new(takes))
}

impl Made<'_> {
    /// What a value of the type of `row` takes.
    fn row(&mut self, row: usize) -> &'static Takes {
        match self.rows[row] {
            Row::Made(takes) => return takes,
            Row::Making => return &Takes::Octets(None),
            Row::Unmade => self.rows[row] = Row::Making,
        }
        let ty = &self.protocol.schema[row];
        let takes = match &ty.kind {
            _ if CONTAINERS.contains(&ty.rust) => &Takes::Message,
            schema::Kind::Sequence { names, members, .. } => {
                let members: Vec<_> = (names.iter().zip(*members))
                    .map(|(name, member)| (*name, self.member(name, &member.ty)))
                    .collect();
                kept(Takes::Members(None, Box::leak(members.into_boxed_slice())))
            }
            schema::Kind::Choice {
                names,
                alternatives,
                ..
            } => {
                let alternatives: Vec<&'static Takes> = (names.iter().zip(*alternatives))
                    .map(|(name, field)| {
                        let alternative = match field {
                            Field::Null => &Takes::Named(&["null"]),
                            field => self.field(field),
                        };
                        let members = Box::leak(Box::new([(*name, alternative)]));
                        kept(Takes::Members(None, members))
                    })
                    .collect();
                kept(Takes::OneOf(Box::leak(alternatives.into_boxed_slice())))
            }
            schema::Kind::Enumerated { names, .. } => kept(Takes::Named(names)),
            schema::Kind::Alias { field, form, .. } => match form {
                Some(Form::Plmn) => &Takes::Plmn,
                Some(Form::Digits) => &Takes::Digits,
                Some(Form::Number(bits)) => kept(Takes::Number(most(*bits))),
                Some(Form::Address) => {
                    let jer = self.field(field);
                    let forms = Box::leak(Box::new([&Takes::Ipv4, &Takes::Ipv6, jer]));
                    kept(Takes::OneOf(forms))
                }
                _ => self.field(field),
            },
        };
        self.rows[row] = Row::Made(takes);
        takes
    }

    /// What the member `name` of a SEQUENCE takes: the value that the octets of a
    /// transfer contain, which a tree shows in their place.
    fn member(&mut self, name: &str, field: &Field) -> &'static Takes {
        if self.protocol.transfer_fields.contains(&name)
            && let Ok(typed) = (self.protocol.transfer_typed)(name)
        {
            return self.row((typed.row)());
        }
        self.field(field)
    }

    /// What a value of a field takes.
    fn field(&mut self, field: &Field) -> &'static Takes {
        match *field {
            Field::Boolean => &Takes::Flag,
            Field::Null => &Takes::Named(&["null"]),
            Field::Unsigned(0, upper, _) => kept(Takes::Number(upper)),
            Field::Unsigned(lower, upper, _) => {
                let bound = |number: u64| i64::try_from(number).unwrap_or(i64::MAX);
                kept(Takes::Signed(bound(lower), bound(upper)))
            }
            Field::Signed(lower, upper, _) => kept(Takes::Signed(lower, upper)),
            Field::OctetArray(size) => kept(Takes::Octets(Some(size))),
            Field::Octets(lower, upper, false) if lower == upper => {
                kept(Takes::Octets(Some(lower)))
            }
            Field::Octets(..) | Field::Open => &Takes::Octets(None),
            // A string of bits of one size is its octets, and one of a size that varies
            // its octets and its length (`json::bits`, `json::bit_string`).
            Field::Bits(lower, upper) | Field::BitString(lower, upper, false) if lower == upper => {
                kept(Takes::Octets(Some(lower.div_ceil(8))))
            }
            Field::Bits(_, upper) | Field::BitString(_, upper, _) => {
                let length = kept(Takes::Number(upper as u64));
                kept(Takes::Members(
                    Some("value"),
                    Box::leak(Box::new([
                        ("value", &Takes::Octets(None)),
                        ("length", length),
                    ])),
                ))
            }
            Field::Text(..) | Field::ObjectIdentifier => &Takes::Text,
            Field::Named(row) => self.row(usize::from(row)),
            Field::List(of, ..) => {
                let entry = self.field(of);
                kept(Takes::List(entry))
            }
        }
    }
}

/// The largest number of `bits` bits.
fn most(bits: u32) -> u64 {
    match bits {
        64.. => u64::MAX,
        bits => (1 << bits) - 1,
    }
}
