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

//! What the fuzz targets do with the octets that they are given: the programs of
//! `fuzz_targets/` call these functions.
//!
//! `fields` reads fields that the octets choose, with constraints that they choose,
//! and writes back what it read: the writer gives every value one encoding, which the
//! reader reads as the same value. `value` reads a value of one type with the crate
//! and with rasn, another implementation of aligned PER: what one reads the other
//! reads, as the same value, which both write as the same octets.

mod value;

pub use value::value;

use oxirush_aper::{BitString, Bits, ObjectIdentifier, Reader, UNBOUNDED, Writer};

/// A field that was read, with the constraint that it was read with.
#[derive(Debug, Clone, PartialEq)]
enum Field {
    Bit(bool),
    Bits(u64, usize),
    Align,
    Unsigned(u64, u64, u64),
    Signed(i64, i64, i64),
    UnsignedExtensible(u64, u64, u64),
    SignedExtensible(i64, i64, i64),
    Unconstrained(i64),
    SemiConstrained(i64, i64),
    SemiConstrainedUnsigned(u64, u64),
    Small(u64),
    Size(usize, usize, usize),
    Index(u64, u64, u64, bool),
    Array3([u8; 3]),
    Array2([u8; 2]),
    Octets(Vec<u8>, usize, usize),
    OctetsExtensible(Vec<u8>, usize, usize),
    Open(Vec<u8>),
    BitsSized(Bits, usize, usize),
    BitString(BitString<'static>, usize, usize),
    BitStringExtensible(BitString<'static>, usize, usize),
    Text(u8, bool, String, usize, usize),
    Utf8(String),
    Object(ObjectIdentifier),
}

/// The bounds of a constraint on a number without a sign, which two octets choose.
fn unsigned_bounds(a: u8, b: u8) -> (u64, u64) {
    const LOWER: [u64; 4] = [0, 1, 100, 1 << 32];
    const RANGE: [u64; 16] = [
        0,
        1,
        2,
        7,
        15,
        100,
        254,
        255,
        256,
        65534,
        65535,
        65536,
        1 << 24,
        u32::MAX as u64,
        (1 << 40) - 1,
        u64::MAX,
    ];
    let lower = LOWER[usize::from(a) % 4];
    let range = RANGE[usize::from(b) % 16];
    (lower, lower.saturating_add(range))
}

/// The bounds of a constraint on a number with a sign.
fn signed_bounds(a: u8, b: u8) -> (i64, i64) {
    const LOWER: [i64; 6] = [i64::MIN, -(1 << 40), -32768, -8, -1, 0];
    let (_, range) = unsigned_bounds(0, b);
    let lower = LOWER[usize::from(a) % 6];
    let upper = i128::from(lower) + i128::from(range);
    (lower, upper.min(i64::MAX.into()) as i64)
}

/// The bounds of a size constraint.
fn size_bounds(a: u8, b: u8) -> (usize, usize) {
    const LOWER: [usize; 4] = [0, 1, 3, 22];
    const RANGE: [usize; 8] = [0, 1, 7, 10, 149, 255, 256, 65000];
    let lower = LOWER[usize::from(a) % 4];
    (lower, lower + RANGE[usize::from(b) % 8])
}

/// The field that the octets `[code, a, b]` choose, read at the reader; `None` for
/// what is not read.
fn read(reader: &mut Reader<'_>, [code, a, b]: [u8; 3]) -> Option<Field> {
    let (lower, upper) = unsigned_bounds(a, b);
    let (low, high) = signed_bounds(a, b);
    let (least, most) = size_bounds(a, b);
    Some(match code % 30 {
        0 => Field::Bit(reader.bit().ok()?),
        1 => {
            let count = usize::from(a) % 65;
            Field::Bits(reader.bits(count).ok()?, count)
        }
        2 => {
            reader.align();
            Field::Align
        }
        3 => Field::Unsigned(reader.unsigned(lower, upper).ok()?, lower, upper),
        4 => Field::Signed(reader.signed(low, high).ok()?, low, high),
        5 => {
            let value = reader.unsigned_extensible(lower, upper).ok()?;
            Field::UnsignedExtensible(value, lower, upper)
        }
        6 => {
            let value = reader.signed_extensible(low, high).ok()?;
            Field::SignedExtensible(value, low, high)
        }
        7 => Field::Unconstrained(reader.unconstrained().ok()?),
        8 => Field::SemiConstrained(reader.semi_constrained(low).ok()?, low),
        9 => {
            let value = reader.semi_constrained_unsigned(lower).ok()?;
            Field::SemiConstrainedUnsigned(value, lower)
        }
        10 => Field::Small(reader.small().ok()?),
        11 => Field::Size(reader.size(least, most).ok()?, least, most),
        12 | 13 => {
            let root = u64::from(a) % 70 + 1;
            let additions = u64::from(b) % 70;
            let extensible = additions > 0;
            let index = match code % 30 {
                12 => reader.enumerated(root, additions, extensible),
                _ => reader.choice(root, additions, extensible),
            };
            Field::Index(index.ok()?, root, additions, extensible)
        }
        14 => Field::Array3(reader.octet_array().ok()?),
        15 => Field::Array2(reader.octet_array().ok()?),
        16 => {
            let octets = reader.octet_string(least, most).ok()?;
            Field::Octets(octets.into_vec(), least, most)
        }
        17 => {
            let octets = reader.octet_string(0, UNBOUNDED).ok()?;
            Field::Octets(octets.into_vec(), 0, UNBOUNDED)
        }
        18 => {
            let octets = reader.octet_string_extensible(least, most).ok()?;
            Field::OctetsExtensible(octets.into_vec(), least, most)
        }
        19 => Field::Open(reader.open_type().ok()?.into_vec()),
        20 => {
            let most = most.min(64);
            let least = least.min(most);
            Field::BitsSized(reader.bits_sized(least, most).ok()?, least, most)
        }
        21 => {
            let bits = reader.bit_string(least, most).ok()?;
            Field::BitString(bits.into_owned(), least, most)
        }
        22 => {
            let bits = reader.bit_string(0, UNBOUNDED).ok()?;
            Field::BitString(bits.into_owned(), 0, UNBOUNDED)
        }
        23 => {
            let bits = reader.bit_string_extensible(least, most).ok()?;
            Field::BitStringExtensible(bits.into_owned(), least, most)
        }
        24..=26 => {
            let (alphabet, extensible) = (code % 30 - 24, a & 0x80 != 0);
            let text = match (alphabet, extensible) {
                (0, false) => reader.printable_string(least, most),
                (0, true) => reader.printable_string_extensible(least, most),
                (1, false) => reader.visible_string(least, most),
                (1, true) => reader.visible_string_extensible(least, most),
                (_, false) => reader.ia5_string(least, most),
                (_, true) => reader.ia5_string_extensible(least, most),
            };
            Field::Text(alphabet, extensible, text.ok()?.into_owned(), least, most)
        }
        27 => Field::Utf8(reader.utf8_string().ok()?.into_owned()),
        28 => Field::Object(reader.object_identifier().ok()?),
        _ => {
            // The additions of a SEQUENCE are skipped, and nothing is written back
            // of what was not read.
            let _ = reader.skip_additions();
            return None;
        }
    })
}

/// Write a field as it was read. A value that was read is one that its constraint
/// has, so no writer refuses it.
fn write(writer: &mut Writer<'_>, field: &Field) {
    match field {
        Field::Bit(bit) => writer.bit(*bit),
        Field::Bits(value, count) => writer.bits(*value, *count),
        Field::Align => writer.align(),
        Field::Unsigned(value, lower, upper) => writer.unsigned(*value, *lower, *upper).unwrap(),
        Field::Signed(value, lower, upper) => writer.signed(*value, *lower, *upper).unwrap(),
        Field::UnsignedExtensible(value, lower, upper) => {
            writer.unsigned_extensible(*value, *lower, *upper)
        }
        Field::SignedExtensible(value, lower, upper) => {
            writer.signed_extensible(*value, *lower, *upper)
        }
        Field::Unconstrained(value) => writer.unconstrained(*value),
        Field::SemiConstrained(value, lower) => writer.semi_constrained(*value, *lower).unwrap(),
        Field::SemiConstrainedUnsigned(value, lower) => {
            writer.semi_constrained_unsigned(*value, *lower).unwrap()
        }
        Field::Small(value) => writer.small(*value),
        Field::Size(size, lower, upper) => writer.size(*size, *lower, *upper).unwrap(),
        Field::Index(index, root, _, extensible) => writer.index(*index, *root, *extensible),
        Field::Array3(octets) => writer.octet_array(octets),
        Field::Array2(octets) => writer.octet_array(octets),
        Field::Octets(octets, lower, upper) => writer.octet_string(octets, *lower, *upper).unwrap(),
        Field::OctetsExtensible(octets, lower, upper) => {
            writer.octet_string_extensible(octets, *lower, *upper)
        }
        Field::Open(octets) => writer.open_type(octets),
        Field::BitsSized(bits, lower, upper) => writer.bits_sized(*bits, *lower, *upper).unwrap(),
        Field::BitString(bits, lower, upper) => writer.bit_string(bits, *lower, *upper).unwrap(),
        Field::BitStringExtensible(bits, lower, upper) => {
            writer.bit_string_extensible(bits, *lower, *upper)
        }
        Field::Text(alphabet, extensible, text, lower, upper) => {
            let (lower, upper) = (*lower, *upper);
            match (alphabet, extensible) {
                (0, false) => writer.printable_string(text, lower, upper).unwrap(),
                (0, true) => writer
                    .printable_string_extensible(text, lower, upper)
                    .unwrap(),
                (1, false) => writer.visible_string(text, lower, upper).unwrap(),
                (1, true) => writer
                    .visible_string_extensible(text, lower, upper)
                    .unwrap(),
                (_, false) => writer.ia5_string(text, lower, upper).unwrap(),
                (_, true) => writer.ia5_string_extensible(text, lower, upper).unwrap(),
            }
        }
        Field::Utf8(text) => writer.utf8_string(text),
        Field::Object(identifier) => writer.object_identifier(identifier),
    }
}

/// Fields that the octets choose, read from the octets: no field makes the reader
/// panic, and the fields that were read are written and read again as the same.
///
/// The first octet says how many fields are read, 32 at most, and three octets say
/// each: which field, and its constraint. The rest is what they are read from.
pub fn fields(octets: &[u8]) {
    let Some((count, rest)) = octets.split_first() else {
        return;
    };
    let count = usize::from(*count) % 33;
    let Some((script, data)) = rest.split_at_checked(count * 3) else {
        return;
    };
    let (script, _) = script.as_chunks::<3>();

    // What the octets hold, up to the first field that they do not.
    let mut reader = Reader::new(data);
    let mut fields = Vec::new();
    let mut read_as = Vec::new();
    for step in script {
        let Some(field) = read(&mut reader, *step) else {
            break;
        };
        fields.push(field);
        read_as.push(*step);
    }
    assert!(reader.position() <= data.len() * 8);

    // Written, they are octets that hold the same fields, and that are written again
    // as the same octets: a value has one encoding.
    let mut written = Vec::new();
    let mut writer = Writer::new(&mut written);
    fields.iter().for_each(|field| write(&mut writer, field));
    let mut reader = Reader::new(&written);
    let again: Vec<_> = read_as
        .iter()
        .map(|step| read(&mut reader, *step))
        .collect();
    let fields: Vec<_> = fields.into_iter().map(Some).collect();
    assert_eq!(again, fields, "{written:02x?}");
    let mut rewritten = Vec::new();
    let mut writer = Writer::new(&mut rewritten);
    for field in again.iter().flatten() {
        write(&mut writer, field);
    }
    assert_eq!(rewritten, written);
}
