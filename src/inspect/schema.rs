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

//! The types of a protocol as one table, which the generator writes from the ASN.1, and
//! a walker that reads the PER octets of a value of one of them as its tree and writes a
//! tree as its octets, with the reader and the writer that the typed bindings use, in the
//! same order and with the same errors.
//!
//! The tree of a value is the one that the typed value of the bindings has: a SEQUENCE
//! is an object of its members, by the names that ASN.1 gives them, in their order; a
//! CHOICE is its alternative by name with its value; an ENUMERATED is the name of its
//! value; a list is a list; a well-known type is in its readable [`Form`].

use alloc::borrow::Cow;
use alloc::vec::Vec;

use oxirush_utils::tree::{self as utils, Map, Reading, Refused, Value};

use crate::json::{self, Characters, Form};
use crate::{Error, ObjectIdentifier, Reader, UNBOUNDED, Writer};

/// A type of the table: the name that ASN.1 gives it, and what it is.
#[derive(Debug)]
pub struct Type {
    /// The name that ASN.1 gives the type, which is its `AsnType::NAME`.
    pub name: &'static str,
    /// The name of the type in the bindings, which the tables of the paths use.
    pub rust: &'static str,
    /// What the type is.
    pub kind: Kind,
}

/// What a type of the table is.
#[derive(Debug)]
pub enum Kind {
    /// A SEQUENCE: the names that ASN.1 gives its members, and the members in the same
    /// order. A later version adds members after an extension marker, which are skipped.
    Sequence {
        names: &'static [&'static str],
        members: &'static [Member],
        extensible: bool,
    },
    /// A CHOICE: the names of its alternatives and their fields in the same order, of
    /// which the first `root` are in its root and the others are additions.
    Choice {
        names: &'static [&'static str],
        alternatives: &'static [Field],
        root: u64,
        extensible: bool,
    },
    /// An ENUMERATED: the names of its values in the order of their indexes in PER, of
    /// which the first `root` are in its root.
    Enumerated {
        names: &'static [&'static str],
        root: u64,
        extensible: bool,
    },
    /// A name for a field: its readable form for a well-known type, and whether it
    /// holds a key, of which a refusal shows nothing.
    Alias {
        field: Field,
        form: Option<Form>,
        secret: bool,
    },
}

/// A member of a SEQUENCE: the name of its field in the bindings, which an error of the
/// writer names, its field, and whether a value may be without it.
#[derive(Debug)]
pub struct Member {
    pub field: &'static str,
    pub ty: Field,
    pub optional: bool,
}

/// The alphabet of a character string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Text {
    Printable,
    Visible,
    Utf8,
}

/// A field of a value, with the constraints that PER reads it with: a lower and an
/// upper bound, and whether the constraint has an extension marker. A size without an
/// upper bound has [`UNBOUNDED`].
#[derive(Clone, Copy, Debug)]
pub enum Field {
    Boolean,
    Null,
    /// A whole number of a constraint whose lower bound is not below zero.
    Unsigned(u64, u64, bool),
    /// A whole number of a constraint whose lower bound is below zero.
    Signed(i64, i64, bool),
    /// An OCTET STRING of one size below 64K.
    OctetArray(usize),
    /// Another OCTET STRING, of a size constraint.
    Octets(usize, usize, bool),
    /// An open type, which is its octets.
    Open,
    /// A BIT STRING of 64 bits at most, without an extension marker.
    Bits(usize, usize),
    /// Another BIT STRING.
    BitString(usize, usize, bool),
    /// A character string of an alphabet, of a size constraint.
    Text(Text, usize, usize, bool),
    ObjectIdentifier,
    /// A type of the table, by its position.
    Named(u16),
    /// A list of a field, of a size constraint.
    List(&'static Field, usize, usize),
}

/// The position of the type that ASN.1 names `name` in `types`, which are in the order
/// of their names.
pub fn position(types: &[Type], name: &str) -> Option<usize> {
    types.binary_search_by(|ty| ty.name.cmp(name)).ok()
}

// ── From octets to a tree ───────────────────────────────────────────────────

/// The tree of the value of the type `ty` of `types` that `octets` are, which are its
/// complete encoding, as `oxirush_aper::decode` reads them.
///
/// # Errors
///
/// Octets that are not a value of the type, with the error of the typed decoder.
pub fn tree(types: &'static [Type], ty: usize, octets: &[u8]) -> Result<Value, Error> {
    let name = types[ty].name;
    if octets.is_empty() {
        return Err(Error::decode("there are no octets").decoding(name));
    }
    let mut reader = Reader::new(octets);
    let read = read_type(types, ty, &mut reader).and_then(|value| {
        reader.finish()?;
        Ok(value)
    });
    read.map_err(|error| error.decoding(name))
}

/// The tree of a value of the type `ty`, read from `reader`.
fn read_type(types: &'static [Type], ty: usize, reader: &mut Reader<'_>) -> Result<Value, Error> {
    match &types[ty].kind {
        Kind::Sequence {
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
                let value = read_field(types, &member.ty, reader)
                    .map_err(|error| error.in_field(member.field))?;
                tree.push((*name).into(), value);
            }
            if extended {
                reader.skip_additions()?;
            }
            Ok(Value::Object(tree))
        }
        Kind::Choice {
            names,
            alternatives,
            root,
            extensible,
        } => {
            let additions = alternatives.len() as u64 - root;
            let index = reader.index(*root, additions, *extensible)?;
            // The last alternative is the one of every other index.
            let index = (index as usize).min(alternatives.len() - 1);
            let value = match index < *root as usize {
                true => read_field(types, &alternatives[index], reader)?,
                // An addition is in an open type (X.691 §23.8).
                false => {
                    let octets = reader.open_type()?;
                    read_field(types, &alternatives[index], &mut Reader::new(&octets))?
                }
            };
            Ok(utils::alternative(names[index], value))
        }
        Kind::Enumerated {
            names,
            root,
            extensible,
        } => {
            let additions = names.len() as u64 - root;
            let index = reader.index(*root, additions, *extensible)?;
            Ok(utils::text(names[index as usize]))
        }
        Kind::Alias { field, form, .. } => {
            let jer = read_field(types, field, reader)?;
            Ok(match form {
                Some(form) => json::readable(*form, jer),
                None => jer,
            })
        }
    }
}

/// The tree of a field, read from `reader`.
pub(crate) fn read_field(
    types: &'static [Type],
    field: &Field,
    reader: &mut Reader<'_>,
) -> Result<Value, Error> {
    Ok(match *field {
        Field::Boolean => Value::Bool(reader.bit()?),
        Field::Null => Value::Null,
        Field::Unsigned(lower, upper, extensible) => {
            utils::unsigned(reader.unsigned(lower, upper, extensible)?)
        }
        Field::Signed(lower, upper, extensible) => {
            utils::signed(reader.signed(lower, upper, extensible)?)
        }
        Field::OctetArray(size) => {
            let octets = match size > 2 {
                true => Cow::Borrowed(reader.octets(size)?),
                false => Cow::Owned(
                    (0..size)
                        .map(|_| reader.bits(8).map(|octet| octet as u8))
                        .collect::<Result<Vec<u8>, Error>>()?,
                ),
            };
            utils::octets(&octets)
        }
        Field::Octets(lower, upper, extensible) => {
            utils::octets(&reader.octet_string(lower, upper, extensible)?)
        }
        Field::Open => utils::octets(&reader.open_type()?),
        Field::Bits(lower, upper) => json::bits(reader.bits_sized(lower, upper)?, lower, upper),
        Field::BitString(lower, upper, extensible) => json::bit_string(
            &reader.bit_string(lower, upper, extensible)?,
            lower,
            upper,
            extensible,
        ),
        Field::Text(of, lower, upper, extensible) => {
            let text = match of {
                Text::Printable => reader.printable_string(lower, upper, extensible)?,
                Text::Visible => reader.visible_string(lower, upper, extensible)?,
                // A size constraint on a UTF8String is not PER-visible (X.691 §10.3.7).
                Text::Utf8 => reader.utf8_string()?,
            };
            utils::text(&text)
        }
        Field::ObjectIdentifier => json::object_identifier(&reader.object_identifier()?),
        Field::Named(ty) => read_type(types, ty.into(), reader)?,
        Field::List(of, lower, upper) => {
            let mut list = Vec::new();
            match upper {
                // A count below 64K is a constrained whole number (X.691 §20.6).
                upper if upper < 65536 => {
                    let count = reader.size(lower, upper)?;
                    list = reader.list(count);
                    for _ in 0..count {
                        list.push(read_field(types, of, reader)?);
                    }
                }
                // Otherwise an unconstrained one, in fragments, checked as the typed
                // decoder checks it.
                upper => {
                    let (mut count, mut more) = reader.length()?;
                    loop {
                        let size = list.len().saturating_add(count);
                        if !more || (upper != UNBOUNDED && size > upper) {
                            Reader::check_size(size, lower, upper)?;
                        }
                        for _ in 0..count {
                            list.push(read_field(types, of, reader)?);
                        }
                        if !more {
                            break;
                        }
                        (count, more) = reader.length_after(count)?;
                    }
                }
            }
            Value::Array(list)
        }
    })
}

// ── From a tree to octets ───────────────────────────────────────────────────

/// The octets of the value of the type `ty` of `types` that `value` writes: what the
/// typed value that the tree reads as encodes to, with the same errors. A tree that is
/// not a value of the type is refused first; then a value outside a constraint of the
/// type does not encode.
///
/// # Errors
///
/// A tree that is not a value of the type, and a value that does not encode.
pub fn write(types: &'static [Type], ty: usize, value: &Value) -> Result<Vec<u8>, Error> {
    let name = types[ty].name;
    let mut octets = Vec::new();
    let mut failed = None;
    let written = utils::read(value, name, |value, reading| {
        let mut writer = Writer::new(&mut octets);
        let mut walk = Walk {
            types,
            writer: &mut writer,
            failed: &mut failed,
        };
        walk.write_type(ty, value, reading)
    });
    written.map_err(json::refused)?;
    if let Some(error) = failed {
        return Err(error.encoding(name).outside_its_type(name));
    }
    if octets.is_empty() {
        octets.push(0);
    }
    Ok(octets)
}

/// A tree that is being written: the writer, and the first error of the writer, which
/// is told once the whole tree is read.
struct Walk<'w, 'o> {
    types: &'static [Type],
    writer: &'w mut Writer<'o>,
    failed: &'w mut Option<Error>,
}

impl Walk<'_, '_> {
    /// Keep the first error of the writer.
    fn wrote(&mut self, written: Result<(), Error>) {
        if let (Err(error), None) = (written, &self.failed) {
            *self.failed = Some(error);
        }
    }

    /// Write `value`, a tree of the type `ty`.
    fn write_type(
        &mut self,
        ty: usize,
        value: &Value,
        reading: &mut Reading,
    ) -> Result<(), Refused> {
        match &self.types[ty].kind {
            Kind::Sequence {
                names,
                members,
                extensible,
            } => {
                let read = reading.members(value, names)?;
                // Which optional members are there, whose bits come first.
                if *extensible {
                    self.writer.bit(false);
                }
                for (name, member) in names.iter().zip(*members) {
                    if member.optional {
                        let there = reading.optional(&read, name, |_, _| Ok(()))?;
                        self.writer.bit(there.is_some());
                    }
                }
                for (name, member) in names.iter().zip(*members) {
                    let before = self.failed.is_some();
                    let mut write = |value: &Value, reading: &mut Reading| {
                        self.write_field(&member.ty, value, reading)
                    };
                    match member.optional {
                        true => {
                            reading.optional(&read, name, &mut write)?;
                        }
                        false => reading.member(&read, name, &mut write)?,
                    }
                    if !before && let Some(error) = self.failed.take() {
                        *self.failed = Some(error.in_field(member.field));
                    }
                }
                Ok(())
            }
            Kind::Choice {
                names,
                alternatives,
                root,
                extensible,
            } => {
                let (index, value) = reading.alternative(value, names)?;
                self.writer.index(index as u64, *root, *extensible);
                let field = &alternatives[index];
                match index < *root as usize {
                    true => reading.within(names[index], value, |value, reading| {
                        self.write_field(field, value, reading)
                    }),
                    false => {
                        let start = self.writer.begin_open_type();
                        let written = reading.within(names[index], value, |value, reading| {
                            self.write_field(field, value, reading)
                        });
                        self.writer.end_open_type(start);
                        written
                    }
                }
            }
            Kind::Enumerated {
                names,
                root,
                extensible,
            } => {
                let index = reading.named(value, names)?;
                self.writer.index(index as u64, *root, *extensible);
                Ok(())
            }
            Kind::Alias {
                field,
                form,
                secret,
            } => {
                let shown;
                let value = match form {
                    Some(form) => {
                        shown = json::read_readable(reading, *form, value)?;
                        &shown
                    }
                    None => value,
                };
                match secret {
                    true => reading.secret(|reading| self.write_field(field, value, reading)),
                    false => self.write_field(field, value, reading),
                }
            }
        }
    }

    /// Write `value`, a tree of a field.
    fn write_field(
        &mut self,
        field: &Field,
        value: &Value,
        reading: &mut Reading,
    ) -> Result<(), Refused> {
        match *field {
            Field::Boolean => {
                let boolean = reading.boolean(value)?;
                self.writer.bit(boolean);
            }
            Field::Null => reading.null(value)?,
            Field::Unsigned(lower, upper, extensible) => {
                // A constraint with an extension marker holds what 64 bits hold.
                let (least, most) = match extensible {
                    true => (u64::MIN, u64::MAX),
                    false => (lower, upper),
                };
                let number = reading.unsigned(value, least, most)?;
                let written = self.writer.unsigned(number, lower, upper, extensible);
                self.wrote(written);
            }
            Field::Signed(lower, upper, extensible) => {
                let (least, most) = match extensible {
                    true => (i64::MIN, i64::MAX),
                    false => (lower, upper),
                };
                let number = reading.signed(value, least, most)?;
                let written = self.writer.signed(number, lower, upper, extensible);
                self.wrote(written);
            }
            Field::OctetArray(size) => {
                let octets = reading.octets_sized(value, size)?;
                match size > 2 {
                    true => self.writer.octets(&octets),
                    false => octets
                        .iter()
                        .for_each(|octet| self.writer.bits((*octet).into(), 8)),
                }
            }
            Field::Octets(lower, upper, extensible) => {
                let octets = reading.octets(value)?;
                let written = self.writer.octet_string(&octets, lower, upper, extensible);
                self.wrote(written);
            }
            Field::Open => {
                let octets = reading.octets(value)?;
                self.writer.open_type(&octets);
            }
            Field::Bits(lower, upper) => {
                let bits = json::read_bits(reading, value, lower, upper)?;
                let written = self.writer.bits_sized(bits, lower, upper);
                self.wrote(written);
            }
            Field::BitString(lower, upper, extensible) => {
                let bits = json::read_bit_string(reading, value, lower, upper, extensible)?;
                let written = self.writer.bit_string(&bits, lower, upper, extensible);
                self.wrote(written);
            }
            Field::Text(of, lower, upper, extensible) => {
                let characters = match of {
                    Text::Printable => Characters::Printable,
                    Text::Visible => Characters::Visible,
                    Text::Utf8 => Characters::Utf8,
                };
                let text: Cow<'_, str> = json::read_text(reading, value, characters)?;
                let written = match of {
                    Text::Printable => self
                        .writer
                        .printable_string(&text, lower, upper, extensible),
                    Text::Visible => self.writer.visible_string(&text, lower, upper, extensible),
                    Text::Utf8 => {
                        self.writer.utf8_string(&text);
                        Ok(())
                    }
                };
                self.wrote(written);
            }
            Field::ObjectIdentifier => {
                let identifier: ObjectIdentifier = json::read_object_identifier(reading, value)?;
                self.writer.object_identifier(&identifier);
            }
            Field::Named(ty) => self.write_type(ty.into(), value, reading)?,
            Field::List(of, lower, upper) => {
                let Some(entries) = value.as_array() else {
                    // As the reading of a list refuses it.
                    reading.list(value, |_, _| Ok(()))?;
                    return Ok(());
                };
                match upper {
                    upper if upper < 65536 => {
                        let written = self.writer.size(entries.len(), lower, upper);
                        self.wrote(written);
                        reading
                            .list(value, |entry, reading| self.write_field(of, entry, reading))?;
                    }
                    upper => {
                        let written = Writer::check_size(entries.len(), lower, upper);
                        self.wrote(written);
                        // The entries in fragments, each after its length: `left` is
                        // where the next length goes, and `more` says that one does.
                        let (mut left, mut more, mut position) = (0, true, 0);
                        reading.list(value, |entry, reading| {
                            if position == left {
                                let count;
                                (count, more) = self.writer.length(entries.len() - left);
                                left += count;
                            }
                            position += 1;
                            self.write_field(of, entry, reading)
                        })?;
                        // The length that ends a list of a whole number of fragments, or
                        // an empty one.
                        while more && position == left {
                            let count;
                            (count, more) = self.writer.length(entries.len() - left);
                            left += count;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
