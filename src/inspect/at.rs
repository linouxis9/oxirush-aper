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

//! The type of the value at a path of the tree of a PDU, from the table of the types of
//! its protocol: what a tree takes there, the IEs that it may hold there, and the tree of
//! octets of that type.
//!
//! A path starts at an IE of a message, `/ngap/NAME`, and goes as the paths of a tree go:
//! a member of a SEQUENCE, an alternative of a CHOICE, the position of an entry of a list
//! (or `*`), and an IE by its name after what holds it: a transfer, an extension
//! container, or a value that holds one IE.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use oxirush_utils::tree::{self, Takes, Value, same_name};

use super::schema::{Field, Kind};
use super::views::Within;
use super::{Protocol, Sent};
use crate::{Criticality, Error, ErrorKind, Presence, Reader};

/// The IEs of an object set, each with its criticality and its presence.
pub type Ies = &'static [(u16, Criticality, Presence)];

/// The type of the value at a path.
#[derive(Clone, Copy, Debug)]
pub struct At {
    /// The name that ASN.1 gives the type, or what it is for a member of a type of no
    /// name: `INTEGER`, `OCTET STRING`, `SEQUENCE OF`.
    pub name: &'static str,
    /// The IE whose value it is, where the path ends at an IE.
    pub ie: Option<u16>,
    /// What its tree takes.
    pub takes: Takes,
    /// The IEs that it may hold, as its object set lists them: those of a transfer and
    /// those of an extension container. `None` for a value that holds no IEs, and for
    /// one that holds an IE of no set that the tables have.
    pub ies: Option<Ies>,
    /// The field of the value, and what holds it.
    field: Field,
    within: Within,
}

impl Protocol {
    /// The IEs that the extension container of the type that the bindings name `rust`
    /// may hold: its extension set.
    pub fn extension_set(&self, rust: &str) -> Option<Ies> {
        let sets = self.extension_sets;
        let at = sets
            .binary_search_by(|(known, _)| (*known).cmp(rust))
            .ok()?;
        Some(sets[at].1)
    }
}

/// The type of the value at `path` of the tree of a PDU of `protocol`, whose rows take
/// `rows` (`inspect::takes`). A member of a type of no name that is a list of a
/// type of no name, or a string of bits whose size varies, says what it takes in values
/// that are made for the call and kept.
///
/// # Errors
///
/// A path that does not start at an IE of the protocol, and a segment that the value
/// before it has no place for.
pub fn at(protocol: &'static Protocol, rows: &[&'static Takes], path: &str) -> Result<At, Error> {
    let refused = |why: String| Error::new(ErrorKind::Path, why);
    let segments = tree::segments(path).map_err(|error| refused(format!("{error}")))?;
    let root = protocol.root;
    let [first, ie, rest @ ..] = &segments[..] else {
        return Err(refused(format!("{path} is no path of an IE: /{root}/NAME")));
    };
    if !same_name(first, root) {
        return Err(refused(format!("{path} is no path of an IE: /{root}/NAME")));
    }
    let mut place = ie_place(protocol, ie, None, Within::Ie)?;
    for segment in rest {
        place = next(protocol, place, segment).map_err(|why| refused(format!("{path}: {why}")))?;
    }
    Ok(described(protocol, rows, place))
}

/// The tree of the value of the type at `at` that `octets` are, its complete encoding, as
/// the tree of a PDU shows it.
///
/// # Errors
///
/// Octets that are no value of the type.
pub fn tree_at(protocol: &'static Protocol, at: &At, octets: &[u8]) -> Result<Value, Error> {
    let mut reader = Reader::new(octets);
    let value = super::views::field_tree(protocol, &at.field, at.within, &mut reader)?;
    reader.finish()?;
    Ok(value)
}

/// A place of a path: the field of its value, what holds it, and the IE whose value it
/// is.
#[derive(Clone, Copy)]
struct Place {
    field: Field,
    within: Within,
    ie: Option<u16>,
}

/// The value of the IE `name`: what its octets contain for an IE of octets that contain
/// a type, and the type that the type `extended` gives an IE of its extension container.
/// `set` is the object set of what holds it, where the tables have one.
fn ie_place(
    protocol: &'static Protocol,
    name: &str,
    set: Option<Ies>,
    within: Within,
) -> Result<Place, Error> {
    let refused = |why: String| Error::new(ErrorKind::Path, why);
    let id = (protocol.ie_id)(name)
        .ok_or_else(|| refused(format!("{name:?} is no IE of {}", protocol.name)))?;
    if let Some(set) = set
        && !set.iter().any(|(known, ..)| *known == id)
    {
        let names: Vec<&str> = set
            .iter()
            .filter_map(|(id, ..)| protocol.ie_name_of(*id))
            .collect();
        return Err(refused(format!(
            "{name} is no IE that it holds, which are {}",
            listed(&names)
        )));
    }
    let extended = match within {
        Within::Extensions(extended) => (protocol.extension_typed)(id, extended),
        _ => None,
    };
    let typed = match (protocol.ie_contents)(id) {
        Some(contents) => contents,
        None => match extended {
            Some(typed) => typed,
            None => (protocol.ie_typed)(id)?,
        },
    };
    Ok(Place {
        field: Field::Named((typed.row)() as u16),
        within: Within::Ie,
        ie: Some(id),
    })
}

/// The place after `segment`, at `place`.
fn next(protocol: &'static Protocol, place: Place, segment: &str) -> Result<Place, String> {
    let containers = protocol.containers;
    let row = |field: &Field| match *field {
        Field::Named(row) => Some(usize::from(row)),
        _ => None,
    };
    let position = segment == "*" || segment.parse::<usize>().is_ok();
    let error = |error: Error| error.to_string();
    match row(&place.field) {
        // The IEs of a container, after the position of one of them or by name.
        Some(at) if Some(at) == containers.ies || Some(at) == containers.extensions => {
            if position {
                return Ok(place);
            }
            let set = match place.within {
                Within::Ies(sent) => sent.map(|sent| sent.ies),
                Within::Extensions(extended) => protocol.extension_set(extended),
                Within::Ie => None,
            };
            ie_place(protocol, segment, set, place.within).map_err(error)
        }
        // An IE that a value holds alone, by its name.
        Some(at) if Some(at) == containers.field => {
            ie_place(protocol, segment, None, Within::Ie).map_err(error)
        }
        Some(at) => {
            let ty = &protocol.schema[at];
            match &ty.kind {
                Kind::Sequence { names, members, .. } => {
                    let found =
                        (names.iter().zip(*members)).find(|(name, _)| same_name(name, segment));
                    if let Some((name, member)) = found {
                        return Ok(self::member(protocol, ty.rust, name, &member.ty));
                    }
                    // The IEs of a transfer go by name after it.
                    if let Some(sent) = protocol.transfer(ty.rust) {
                        return ie_place(
                            protocol,
                            segment,
                            Some(sent.ies),
                            Within::Ies(Some(sent)),
                        )
                        .map_err(error);
                    }
                    Err(format!(
                        "{segment} is no member of {}, which has {}",
                        ty.name,
                        listed(names)
                    ))
                }
                Kind::Choice {
                    names,
                    alternatives,
                    ..
                } => {
                    let found = (names.iter().zip(*alternatives))
                        .find(|(name, _)| same_name(name, segment));
                    let (_, field) = found.ok_or_else(|| {
                        format!(
                            "{segment} is no alternative of {}, which has {}",
                            ty.name,
                            listed(names)
                        )
                    })?;
                    Ok(Place {
                        field: *field,
                        within: Within::Ie,
                        ie: None,
                    })
                }
                Kind::Alias {
                    field, form: None, ..
                } => next(
                    protocol,
                    Place {
                        field: *field,
                        ..place
                    },
                    segment,
                ),
                _ => Err(format!("{segment}: a value of {} has no members", ty.name)),
            }
        }
        None => match place.field {
            Field::List(of, ..) if position => Ok(Place {
                field: *of,
                within: Within::Ie,
                ie: None,
            }),
            Field::List(..) => Err(format!(
                "{segment} is no position of an entry of a list: 0, 1, … or *"
            )),
            _ => Err(format!(
                "{segment}: a value of {} has no members",
                named(&place.field)
            )),
        },
    }
}

/// The place of the member `name` of a value of the type that the bindings name `of`,
/// whose field is `field`: a transfer is the value that its octets contain, and the IEs
/// of a container those of the set of the transfer or of the extensions of the type.
fn member(protocol: &'static Protocol, of: &'static str, name: &str, field: &Field) -> Place {
    let containers = protocol.containers;
    let at = match *field {
        Field::Named(row) => Some(usize::from(row)),
        _ => None,
    };
    let within = match at {
        Some(at) if Some(at) == containers.ies => Within::Ies(protocol.transfer(of)),
        Some(at) if Some(at) == containers.extensions => Within::Extensions(of),
        _ => Within::Ie,
    };
    let field = match protocol.transfer_fields.contains(&name) {
        true => match (protocol.transfer_typed)(name) {
            Ok(typed) => Field::Named((typed.row)() as u16),
            Err(_) => *field,
        },
        false => *field,
    };
    Place {
        field,
        within,
        ie: None,
    }
}

/// What the value at `place` is.
fn described(protocol: &'static Protocol, rows: &[&'static Takes], place: Place) -> At {
    let row = match place.field {
        Field::Named(row) => Some(usize::from(row)),
        _ => None,
    };
    let (name, takes) = match row {
        Some(row) => (protocol.schema[row].name, *rows[row]),
        None => (
            named(&place.field),
            super::takes::field(protocol, rows, &place.field),
        ),
    };
    let ies = match place.within {
        Within::Ies(sent) => sent.map(|sent| sent.ies),
        Within::Extensions(extended) => protocol.extension_set(extended),
        // A transfer holds the IEs of its set.
        Within::Ie => row
            .and_then(|row| protocol.transfer(protocol.schema[row].rust))
            .map(|sent: Sent| sent.ies),
    };
    At {
        name,
        ie: place.ie,
        takes,
        ies,
        field: place.field,
        within: place.within,
    }
}

/// What a field of no name is, as ASN.1 writes it.
fn named(field: &Field) -> &'static str {
    match field {
        Field::Boolean => "BOOLEAN",
        Field::Null => "NULL",
        Field::Unsigned(..) | Field::Signed(..) => "INTEGER",
        Field::OctetArray(_) | Field::Octets(..) => "OCTET STRING",
        Field::Open => "an open type",
        Field::Bits(..) | Field::BitString(..) => "BIT STRING",
        Field::Text(..) => "a character string",
        Field::ObjectIdentifier => "OBJECT IDENTIFIER",
        Field::Named(_) => "a type",
        Field::List(..) => "SEQUENCE OF",
    }
}

/// Names with a comma between them.
fn listed(names: &[&str]) -> String {
    names.join(", ")
}
