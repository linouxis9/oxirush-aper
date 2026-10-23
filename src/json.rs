//! The values of generated bindings as trees of `oxirush_utils::tree`: a [`Value`] is
//! written and read, and no text.
//!
//! A value has the form of the JSON Encoding Rules, ITU-T X.697 (02/2021), but for what
//! follows, and but for the well-known types of 3GPP that have a readable [`Form`], as
//! `"208-93"` for a PLMN identity, which their trees show and are read in as well. A
//! string of decimal digits is never read as octets there: octets whose hexadecimal has
//! no letter are written after `0x`. In the JSON Encoding Rules such a string is octets,
//! and a view that another implementation wrote in them is read as a view of the crate
//! has it through `inspect::view_of_jer`.
//!
//! | ASN.1 | JSON |
//! | --- | --- |
//! | `BOOLEAN` | `true` or `false` |
//! | `INTEGER` | A number |
//! | `ENUMERATED` | The name of the value, a string |
//! | `BIT STRING` of one size | Its octets in hexadecimal, the last one padded with zero bits |
//! | `BIT STRING` whose size varies | `{"value": HEXADECIMAL, "length": BITS}`. Where the type gives one size and an extension marker, a string of that size is its octets in hexadecimal, and one of another size has its length: X.697 §24.3 has every string of such a type with its length |
//! | `OCTET STRING` | Its octets in hexadecimal |
//! | `NULL` | `null`, which is a value: an alternative of the type `NULL` is `{"pLMNWide": null}` |
//! | `OBJECT IDENTIFIER` | Its arcs in decimal, with a full stop between them |
//! | A character string | A string |
//! | `SEQUENCE` | An object with a member for each component, by the name that ASN.1 gives it, in the order of the components, and none for a component that is absent |
//! | `SEQUENCE OF` | A list |
//! | `CHOICE` | An object with one member: the name of the alternative, with its value |
//! | An open type | Its octets in hexadecimal: X.697 §41 has the value that the octets hold, which the views of the bindings show beside them |
//!
//! Hexadecimal is written in lower case and read in either, and the members of an
//! object keep the order in which they are written: the forms are those of
//! `oxirush_utils::tree`, whose `Value` keeps the order of its members by itself.
//!
//! A type has its tree through [`Tree`], which generated code implements with
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
    Map, Members, Read, Reading, Refused, Tree, Value, alternative, hex, octets, same_name, signed,
    text, unhex, unsigned,
};

use core::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use oxirush_utils::identity::Plmn;

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

// ── The readable forms ──────────────────────────────────────────────────────

/// The readable form of the values of a well-known type, which its tree shows, and in
/// which it is read as well as in the form of the JSON Encoding Rules: generated code
/// writes its tree with [`readable`] and reads it with [`read_readable`].
///
/// More forms may come with a later version: a `match` leaves the others to `_`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
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
    pub fn read(self, jer: &Value) -> Option<Value> {
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
                let octets = <[u8; 3]>::try_from(unhex(jer.as_str()?)?).ok()?;
                let plmn = Plmn::from_ap_octets(octets).ok()?;
                Some(oxirush_utils::tree!(format!(
                    "{}-{}",
                    plmn.mcc(),
                    plmn.mnc()
                )))
            }
            Self::Digits => {
                let octets = unhex(jer.as_str()?)?;
                let nibbles: Vec<u8> = octets.iter().flat_map(|o| [o & 15, o >> 4]).collect();
                // A filler takes the place of a last digit that is not there.
                let nibbles = nibbles.strip_suffix(&[15]).unwrap_or(&nibbles);
                digits(nibbles).map(Value::from)
            }
            Self::Number(bits) => {
                let (text, octets) = (jer.as_str()?, bits.div_ceil(8));
                let number = u64::from_str_radix(text, 16).ok()?;
                (text.len() as u32 == octets * 2)
                    .then(|| oxirush_utils::tree!(number >> (octets * 8 - bits)))
            }
            Self::Address => {
                let octets = unhex(jer.get("value")?.as_str()?)?;
                let v4 = |octets: &[u8]| Some(Ipv4Addr::from(<[u8; 4]>::try_from(octets).ok()?));
                let v6 = |octets: &[u8]| Some(Ipv6Addr::from(<[u8; 16]>::try_from(octets).ok()?));
                Some(oxirush_utils::tree!(
                    match (jer.get("length")?.as_u64()?, octets.len()) {
                        (32, 4) => v4(&octets)?.to_string(),
                        (128, 16) => v6(&octets)?.to_string(),
                        (160, 20) => format!("{},{}", v4(&octets[..4])?, v6(&octets[4..])?),
                        _ => return None,
                    }
                ))
            }
        }
    }

    /// The value as its ASN.1 type has it, from its readable form. A value in another
    /// form is kept as it is: the form of the type is still taken, and the codec refuses
    /// what is neither. A string of decimal digits is never read as octets: octets whose
    /// hexadecimal has no letter are written after `0x`, as `"0x134001"`.
    ///
    /// # Errors
    ///
    /// Why what is written in the readable form is no value of it, and a string of
    /// decimal digits that a PLMN identity or a number would read in two ways, which the
    /// error names.
    pub fn write(self, shown: &Value) -> Result<Value, String> {
        let number = |bits: u32, number: u64| {
            if bits < 64 && number >> bits != 0 {
                return Err(format!("{number} does not fit in {bits} bits"));
            }
            let octets = bits.div_ceil(8);
            let width = octets as usize * 2;
            Ok(oxirush_utils::tree!(format!(
                "{:0width$x}",
                number << (octets * 8 - bits)
            )))
        };
        let Some(text) = shown.as_str() else {
            return match (self, shown.as_u64()) {
                (Self::Number(bits), Some(value)) => number(bits, value),
                _ => Ok(shown.clone()),
            };
        };
        let bcd = |digits: &str| digits.bytes().map(|digit| digit & 15).collect::<Vec<_>>();
        let all_digits = |text: &str| !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit());
        // Octets in hexadecimal after `0x`, as they are written where digits would be read
        // in another way.
        let hexadecimal = |octets: &str| {
            unhex(octets)
                .map(|octets| Value::String(hex(&octets)))
                .ok_or_else(|| {
                    format!("{text}: octets in hexadecimal after 0x, two digits for each")
                })
        };
        match self {
            Self::Plmn if text.contains('-') => {
                let (mcc, mnc) = text.split_once('-').unwrap_or_default();
                // The three digits of an MCC, and the two or three of an MNC.
                let plmn = (all_digits(mcc) && all_digits(mnc))
                    .then(|| Plmn::new(mcc, mnc).ok())
                    .flatten();
                let plmn =
                    plmn.ok_or_else(|| format!("{text}: a PLMN identity is MCC-MNC, as 208-93"))?;
                Ok(Value::String(hex(&plmn.to_ap_octets())))
            }
            Self::Plmn | Self::Digits | Self::Number(_) if text.starts_with("0x") => match self {
                Self::Number(bits) => {
                    let value = u64::from_str_radix(&text[2..], 16).map_err(|error| {
                        format!("{text} is not a number in hexadecimal: {error}")
                    })?;
                    number(bits, value)
                }
                Self::Plmn if text.len() != 8 => Err(format!(
                    "{text}: the octets of a PLMN identity are three, six hexadecimal digits \
                     after 0x"
                )),
                _ => hexadecimal(&text[2..]),
            },
            // Six digits are the digits of an MCC and an MNC of three, and three octets.
            Self::Plmn if all_digits(text) && text.len() == 6 => {
                let (mcc, mnc) = text.split_at(3);
                let octets = unhex(text).and_then(|octets| <[u8; 3]>::try_from(octets).ok());
                let other = octets.and_then(|octets| Plmn::from_ap_octets(octets).ok());
                let other = other
                    .map(|other| format!(", which are {other}"))
                    .unwrap_or_default();
                Err(format!(
                    "{text} is read in two ways, as the PLMN identity {mcc}-{mnc} and as the \
                     octets 0x{text}{other}: write \"{mcc}-{mnc}\" or \"0x{text}\""
                ))
            }
            Self::Plmn if all_digits(text) && text.len() == 5 => {
                let (mcc, mnc) = text.split_at(3);
                Err(format!(
                    "{text}: a PLMN identity is MCC-MNC: write \"{mcc}-{mnc}\""
                ))
            }
            // Without a hyphen it is its three octets: what is neither says both forms,
            // where the codec would say that it is no octets.
            Self::Plmn if text.len() != 6 || !text.bytes().all(|b| b.is_ascii_hexdigit()) => {
                Err(format!(
                    "{text}: a PLMN identity is MCC-MNC, as 208-93, or its three octets in \
                     hexadecimal"
                ))
            }
            Self::Digits if all_digits(text) => {
                let pairs = bcd(text);
                let pairs = pairs.chunks(2);
                let octets = pairs.map(|pair| pair.get(1).copied().unwrap_or(15) << 4 | pair[0]);
                Ok(Value::String(hex(&octets.collect::<Vec<_>>())))
            }
            // A number in quotes, which the octets of its type in hexadecimal would be too.
            Self::Number(bits) if all_digits(text) => {
                let octets = bits.div_ceil(8);
                let as_octets = (text.len() == octets as usize * 2)
                    .then(|| u64::from_str_radix(text, 16).ok())
                    .flatten()
                    .map(|read| read >> (octets * 8 - bits));
                // The number as JSON writes it: without the zeros before its first digit.
                let number = text.trim_start_matches('0');
                let number = if number.is_empty() { "0" } else { number };
                Err(match as_octets {
                    Some(other) if other.to_string() != number => format!(
                        "\"{text}\" is read in two ways, as the number {number} and as the \
                         octets 0x{text}, which are the number {other}: write the number \
                         without quotes, {number} or {other}"
                    ),
                    _ => format!("\"{text}\" is a number in quotes: write {number} without quotes"),
                })
            }
            Self::Address if text.contains(['.', ':']) => {
                let mut octets = Vec::new();
                for part in text.split(',') {
                    match (octets.len(), part.trim().parse()) {
                        (0, Ok(IpAddr::V4(address))) => octets.extend(address.octets()),
                        (0 | 4, Ok(IpAddr::V6(address))) => octets.extend(address.octets()),
                        _ => {
                            return Err(format!(
                                "{text}: a transport layer address is an IPv4 address, an IPv6 \
                                 address, or the two with a comma"
                            ));
                        }
                    }
                }
                Ok(oxirush_utils::tree!({"value": hex(&octets), "length": octets.len() * 8}))
            }
            _ => Ok(shown.clone()),
        }
    }
}

/// The tree of a value of a well-known type, whose tree in the form of the JSON
/// Encoding Rules is `jer`: its readable form, where the form gives the value back as it
/// was, and `jer` otherwise, as octets that are no PLMN identity.
pub fn readable(form: Form, jer: Value) -> Value {
    match form.read(&jer) {
        Some(shown) if form.write(&shown).is_ok_and(|again| again == jer) => shown,
        _ => jer,
    }
}

/// The tree in the form of the JSON Encoding Rules of a value of a well-known type, which
/// `value` writes in its readable form or in that one.
///
/// # Errors
///
/// What is written in the readable form and is no value of it, refused in `reading`.
pub fn read_readable(reading: &mut Reading, form: Form, value: &Value) -> Result<Value, Refused> {
    form.write(value)
        .map_err(|why| reading.refuse(Fault::Value, why))
}

// ── From JSON to a value ────────────────────────────────────────────────────

/// Refuse a value that is not written as its type is: `expected` says what is
/// expected, where `found` is written, which the refusal shows the start of unless it is
/// [secret](Reading::secret).
fn mismatch(reading: &mut Reading, expected: &str, found: &Value) -> Refused {
    let why = format!("{expected}, and {} is written", reading.shown(found));
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
    let shown = |reading: &Reading| reading.shown(&Value::String(digits.into()));
    let Some(octets) = unhex(digits) else {
        let why = format!(
            "{} is not hexadecimal, two digits for each octet",
            shown(reading)
        );
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
            "the last {padding} bits of {} are after the {length} bits of the bit string, and \
             are not zero",
            shown(reading)
        );
        return Err(reading.refuse(Fault::Value, why));
    }
    Ok((octets, length))
}

/// The character strings, by the characters that each has.
///
/// More strings may come with a later version: a `match` leaves the others to `_`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
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
