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

//! What the paths of a protocol know of its types, made from its table of the types:
//! each SEQUENCE and each CHOICE with the shape of each member, each type that is a list
//! or a bit string whose size varies, the names of the values of each ENUMERATED, and
//! the readable form of each well-known type.

use alloc::boxed::Box;
use alloc::vec::Vec;

use super::schema::{self, Field};
use super::{Form, Kind, Member, Protocol, Shape};

/// The containers of IEs, by the names of their types in the bindings, and the shape of
/// a value of each.
const CONTAINERS: [(&str, Shape); 4] = [
    ("ProtocolIEContainer", Shape::Ies(None)),
    ("ProtocolIEField", Shape::Ie(None)),
    ("ProtocolExtensionContainer", Shape::Extensions("")),
    ("ProtocolExtensionField", Shape::Extension(None, "")),
];

/// Each SEQUENCE and each CHOICE of `protocol` with its members, and each type that is a
/// list or a bit string whose size varies, by the names of the types in the bindings and
/// in their order: what `Protocol::kind_of` reads. The members and the shapes of the
/// lists are made once, for the life of the program, as a table of constants would be.
pub fn kinds(protocol: &Protocol) -> Vec<(&'static str, Kind)> {
    let mut kinds = Vec::new();
    for ty in protocol.schema {
        if CONTAINERS
            .iter()
            .any(|(container, _)| *container == ty.rust)
        {
            continue;
        }
        let kind = match &ty.kind {
            schema::Kind::Sequence { names, members, .. } => {
                let members = names.iter().zip(*members);
                let members = members.map(|(name, member)| {
                    let shape = member_shape(protocol, name, &member.ty, member.optional);
                    (*name, shape, member.optional)
                });
                Kind::Sequence(made(members.collect()))
            }
            schema::Kind::Choice {
                names,
                alternatives,
                ..
            } => {
                let alternatives = names.iter().zip(*alternatives);
                let alternatives = alternatives.map(|(name, field)| {
                    (*name, member_shape(protocol, name, field, false), false)
                });
                Kind::Choice(made(alternatives.collect()))
            }
            // A type that is another, where that one has members or is a bit string whose
            // size varies.
            schema::Kind::Alias { field, .. } => match inner(protocol, field) {
                Inner::Plain => continue,
                Inner::Other => Kind::Is(shape(protocol, field)),
            },
            schema::Kind::Enumerated { .. } => continue,
        };
        kinds.push((ty.rust, kind));
    }
    kinds.sort_by_key(|(name, _)| *name);
    kinds
}

/// The names of the values of each ENUMERATED of `types`, in the order of their indexes
/// in PER, by the names of the types in the bindings and in their order.
pub fn enumerations(
    types: &'static [schema::Type],
) -> Vec<(&'static str, &'static [&'static str])> {
    let enumerated = types.iter().filter_map(|ty| match &ty.kind {
        schema::Kind::Enumerated { names, .. } => Some((ty.rust, *names)),
        _ => None,
    });
    let mut enumerations: Vec<_> = enumerated.collect();
    enumerations.sort_by_key(|(name, _)| *name);
    enumerations
}

/// The readable form of each well-known type of `types`, by its name in the bindings.
pub fn forms(types: &'static [schema::Type]) -> impl Iterator<Item = (&'static str, Form)> {
    types.iter().filter_map(|ty| match ty.kind {
        schema::Kind::Alias {
            form: Some(form), ..
        } => Some((ty.rust, form)),
        _ => None,
    })
}

/// The members of a type, which are kept for the life of the program.
fn made(members: Vec<Member>) -> &'static [Member] {
    Box::leak(members.into_boxed_slice())
}

/// The shape of the member `name` of a field, which a value may be without when it is
/// `optional`: the octets of a transfer are the value that they contain, and a bit string
/// of one size that is always there has no members.
fn member_shape(protocol: &Protocol, name: &str, field: &Field, optional: bool) -> Shape {
    if protocol.transfer_fields.contains(&name) {
        // The type that the octets contain, or none where they have several.
        let typed = (protocol.transfer_typed)(name).ok();
        let contained = typed.map_or("", |typed| protocol.schema[(typed.row)()].rust);
        return Shape::Transfer(contained);
    }
    match shape(protocol, field) {
        Shape::Bits if one_size(field) && !optional => Shape::Plain,
        shape => shape,
    }
}

/// The shape of a value of a field.
fn shape(protocol: &Protocol, field: &Field) -> Shape {
    match *field {
        Field::Null => Shape::Null,
        Field::Bits(..) | Field::BitString(..) => Shape::Bits,
        Field::List(of, ..) => Shape::List(Box::leak(Box::new(shape(protocol, of)))),
        Field::Named(row) => {
            let ty = &protocol.schema[usize::from(row)];
            let container = CONTAINERS
                .iter()
                .find(|(container, _)| *container == ty.rust);
            match (container, &ty.kind) {
                (Some((_, shape)), _) => *shape,
                // A type that is a value without members, as an INTEGER or a bit string
                // of one size, has none; a well-known type has its name, which gives its
                // form.
                (
                    None,
                    schema::Kind::Alias {
                        field, form: None, ..
                    },
                ) => match inner(protocol, field) {
                    Inner::Plain => Shape::Plain,
                    Inner::Other => Shape::Named(ty.rust),
                },
                (None, _) => Shape::Named(ty.rust),
            }
        }
        _ => Shape::Plain,
    }
}

/// What a type that is the field `field` is: a value without members, or one that has
/// them or that is a bit string whose size varies.
enum Inner {
    Plain,
    Other,
}

/// What the field of a type that is another is.
fn inner(protocol: &Protocol, field: &Field) -> Inner {
    match *field {
        Field::Bits(..) | Field::BitString(..) if one_size(field) => Inner::Plain,
        Field::Null | Field::Bits(..) | Field::BitString(..) | Field::List(..) => Inner::Other,
        Field::Named(_) => match shape(protocol, field) {
            Shape::Plain => Inner::Plain,
            _ => Inner::Other,
        },
        _ => Inner::Plain,
    }
}

/// Whether a bit string has one size.
fn one_size(field: &Field) -> bool {
    match *field {
        Field::Bits(lower, upper) => lower == upper,
        Field::BitString(lower, upper, extensible) => lower == upper && !extensible,
        _ => false,
    }
}
