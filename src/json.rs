//! The values of generated bindings as trees of `oxirush_utils::tree`: a
//! [`serde_json::Value`] is written and read, and no text.
//!
//! A value has the form of the JSON Encoding Rules, ITU-T X.697 (02/2021), but for what
//! follows.
//!
//! | ASN.1 | JSON |
//! | --- | --- |
//! | `BOOLEAN` | `true` or `false` |
//! | `INTEGER` | A number |
//! | `ENUMERATED` | The name of the value, a string |
//! | `BIT STRING` of one size | Its octets in hexadecimal, the last one padded with zero bits |
//! | `BIT STRING` whose size varies | `{"value": HEXADECIMAL, "length": BITS}`. Where the type gives one size and an extension marker, a string of that size is its octets in hexadecimal, and one of another size has its length: X.697 §24.3 has every string of such a type with its length |
//! | `OCTET STRING` | Its octets in hexadecimal |
//! | `NULL` | `null` |
//! | `OBJECT IDENTIFIER` | Its arcs in decimal, with a full stop between them |
//! | A character string | A string |
//! | `SEQUENCE` | An object with a member for each component, by the name that ASN.1 gives it, in the order of the components, and none for a component that is absent |
//! | `SEQUENCE OF` | A list |
//! | `CHOICE` | An object with one member: the name of the alternative, with its value |
//! | An open type | Its octets in hexadecimal: X.697 §41 has the value that the octets hold, which the views of the bindings show beside them |
//!
//! Hexadecimal is written in lower case and read in either, and the members of an
//! object keep the order in which they are written: the forms are those of
//! `oxirush_utils::tree`, whose feature `tree` turns on `serde_json/preserve_order`.
//!
//! A type has the two forms through [`Tree`], which generated code implements with
//! the functions of this module and the [`Reading`] of `oxirush-utils`: the names of
//! the members of a SEQUENCE are a table, and a constraint is the arguments of a call.
//! Reading is strict, as nothing reads its result back to compare: a member that the
//! type does not have, a second alternative of a `CHOICE`, padding bits that are not
//! zero and an integer outside the constraint of its type are each refused, with the
//! names on the way to the value. A name is read however it is written, as the trees
//! read every name.

use alloc::borrow::Cow;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use oxirush_utils::tree::{self as utils, ErrorKind as Fault};
pub use oxirush_utils::tree::{
    Members, Read, Reading, Refused, Tree, alternative, hex, octets, same_name, signed, text,
    unhex, unsigned,
};
use serde_json::Value;

use crate::reader::Alphabet;
use crate::{BitString, Bits, Error, ErrorKind, ObjectIdentifier, Octets};

/// The tree of `value`.
pub fn to_value<T: Tree>(value: &T) -> Value {
    value.to_tree()
}

/// The value of the type `T` that `value` writes.
///
/// # Errors
///
/// An error of the kind `Value`, or `NoSuchMember` for a name that the type does not
/// have, which says why the value is not one of its type and where, by the names that
/// ASN.1 gives on the way to it.
pub fn from_value<T: Tree>(value: &Value) -> Result<T, Error> {
    utils::from_value(value).map_err(|error| {
        let kind = match error.kind() {
            Fault::NoSuchMember => ErrorKind::NoSuchMember,
            _ => ErrorKind::Value,
        };
        Error::new(kind, error.to_string())
    })
}

// ── From a value to JSON ────────────────────────────────────────────────────

/// The JSON form of a short bit string of the size constraint `lower..=upper`: its
/// octets when its type gives it one size, and its octets with its length otherwise.
pub fn bits(value: Bits, lower: usize, upper: usize) -> Value {
    let octets = value.to_vec();
    match lower == upper {
        true => utils::octets(&octets),
        false => utils::bits(&octets, value.len()),
    }
}

/// The JSON form of a bit string of the size constraint `lower..=upper`, which has an
/// extension marker when it is `extensible`: its octets when it has the one size that
/// its type gives it, and its octets with its length otherwise. A string of a type of
/// one size and no marker is its octets, whatever its size.
pub fn bit_string(value: &BitString<'_>, lower: usize, upper: usize, extensible: bool) -> Value {
    match lower == upper && (value.len() == lower || !extensible) {
        true => utils::octets(value.as_octets()),
        false => utils::bits(value.as_octets(), value.len()),
    }
}

/// The JSON form of an object identifier: its arcs in decimal, with a full stop
/// between them.
pub fn object_identifier(identifier: &ObjectIdentifier) -> Value {
    Value::String(identifier.to_string())
}

// ── From JSON to a value ────────────────────────────────────────────────────

/// The start of what is written, which says which value it is.
fn written(value: &Value) -> String {
    let text = value.to_string();
    match text.char_indices().nth(40) {
        Some((end, _)) => format!("{}…", &text[..end]),
        None => text,
    }
}

/// Refuse a value that is not written as its type is: `expected` says what is
/// expected, where `found` is written.
fn mismatch(reading: &mut Reading, expected: &str, found: &Value) -> Refused {
    let why = format!("{expected}, and {} is written", written(found));
    reading.refuse(Fault::Value, why)
}

/// Octets in hexadecimal: an OCTET STRING whose size varies, and an open type.
///
/// # Errors
///
/// What is no octets in hexadecimal, refused in `reading`.
pub fn read_octets(reading: &mut Reading, value: &Value) -> Result<Octets<'static>, Refused> {
    reading.octets(value).map(Octets::owned)
}

/// A short bit string of the size constraint `lower..=upper`.
///
/// # Errors
///
/// What is no bit string of the constraint, refused in `reading`.
pub fn read_bits(
    reading: &mut Reading,
    value: &Value,
    lower: usize,
    upper: usize,
) -> Result<Bits, Refused> {
    let (octets, length) = bit_octets(reading, value, lower, upper, false)?;
    match Bits::from_octets(&octets, length) {
        Ok(bits) if (lower..=upper).contains(&length) => Ok(bits),
        _ => {
            let size = match lower == upper {
                true => lower.to_string(),
                false => format!("{lower}..{upper}"),
            };
            let why = format!("a size of {length} is outside its constraint ({size})");
            Err(reading.refuse(Fault::Value, why))
        }
    }
}

/// A bit string of the size constraint `lower..=upper`, which has an extension marker
/// when it is `extensible`. Its size is compared with the constraint where it is
/// encoded.
///
/// # Errors
///
/// What is no bit string of the type, refused in `reading`.
pub fn read_bit_string(
    reading: &mut Reading,
    value: &Value,
    lower: usize,
    upper: usize,
    extensible: bool,
) -> Result<BitString<'static>, Refused> {
    let (octets, length) = bit_octets(reading, value, lower, upper, extensible)?;
    BitString::new(octets, length).map_err(|error| reading.refuse(Fault::Value, error.to_string()))
}

/// The octets and the length of a bit string of a type of this size constraint.
fn bit_octets(
    reading: &mut Reading,
    value: &Value,
    lower: usize,
    upper: usize,
    extensible: bool,
) -> Result<(Vec<u8>, usize), Refused> {
    const WITH_LENGTH: &str = "a bit string whose size varies is expected, which is \
                               written as {\"value\": HEXADECIMAL, \"length\": BITS}";
    let one_size = (lower == upper).then_some(lower);
    let (digits, length) = match (value, one_size) {
        // A string of the one size of its type is its octets.
        (Value::String(digits), Some(size)) => (digits.as_str(), size),
        (Value::Object(members), size) if size.is_none() || extensible => {
            let digits = members.get("value").and_then(Value::as_str);
            let length = members.get("length").and_then(Value::as_u64);
            let length = length.and_then(|length| usize::try_from(length).ok());
            match digits.zip(length).filter(|_| members.len() == 2) {
                Some(read) => read,
                None => return Err(mismatch(reading, WITH_LENGTH, value)),
            }
        }
        (_, Some(size)) if !extensible => {
            let expected = format!("a bit string of {size} bits is expected, in hexadecimal");
            return Err(mismatch(reading, &expected, value));
        }
        (_, Some(size)) => {
            let expected = format!(
                "a bit string of {size} bits is expected, in hexadecimal, or one of another \
                 size as {{\"value\": HEXADECIMAL, \"length\": BITS}}"
            );
            return Err(mismatch(reading, &expected, value));
        }
        (_, None) => return Err(mismatch(reading, WITH_LENGTH, value)),
    };
    let Some(octets) = unhex(digits) else {
        let why = format!("{digits:?} is not hexadecimal, two digits for each octet");
        return Err(reading.refuse(Fault::Value, why));
    };
    if octets.len() != length.div_ceil(8) {
        let why = format!(
            "{} octets are written for a bit string of {length} bits, which has {}",
            octets.len(),
            length.div_ceil(8)
        );
        return Err(reading.refuse(Fault::Value, why));
    }
    let padding = octets.len() * 8 - length;
    if octets
        .last()
        .is_some_and(|last| last.trailing_zeros() < padding as u32)
    {
        let why = format!(
            "the last {padding} bits of {digits:?} are after the {length} bits of the bit string, \
             and are not zero"
        );
        return Err(reading.refuse(Fault::Value, why));
    }
    Ok((octets, length))
}

/// The character strings, by the characters that each has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Characters {
    /// A UTF8String: every character.
    Utf8,
    /// A PrintableString.
    Printable,
    /// A VisibleString.
    Visible,
    /// An IA5String.
    Ia5,
}

/// A character string of the characters `of`.
///
/// # Errors
///
/// What is no string, and a character that the string does not have, refused in
/// `reading`.
pub fn read_text(
    reading: &mut Reading,
    value: &Value,
    of: Characters,
) -> Result<Cow<'static, str>, Refused> {
    let text = Cow::Owned(reading.text(value)?.to_string());
    let (alphabet, name) = match of {
        Characters::Utf8 => return Ok(text),
        Characters::Printable => (Alphabet::Printable, "a PrintableString"),
        Characters::Visible => (Alphabet::Visible, "a VisibleString"),
        Characters::Ia5 => (Alphabet::Ia5, "an IA5String"),
    };
    if text.bytes().all(|code| alphabet.has(code)) {
        return Ok(text);
    }
    let why = format!("{text:?} has a character that {name} has not");
    Err(reading.refuse(Fault::Value, why))
}

/// An OBJECT IDENTIFIER.
///
/// # Errors
///
/// What is no object identifier, refused in `reading`.
pub fn read_object_identifier(
    reading: &mut Reading,
    value: &Value,
) -> Result<ObjectIdentifier, Refused> {
    const ARCS: &str = "an object identifier is expected, its arcs in decimal with a full stop \
                        between them";
    let identifier = value.as_str().and_then(|arcs| arcs.parse().ok());
    identifier.ok_or_else(|| mismatch(reading, ARCS, value))
}
