//! The values of generated bindings as JSON, and JSON as values of the bindings: a
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
//! | `SEQUENCE` | An object with a member for each component, by the name that ASN.1 gives it, and none for a component that is absent |
//! | `SEQUENCE OF` | A list |
//! | `CHOICE` | An object with one member: the name of the alternative, with its value |
//! | An open type | Its octets in hexadecimal: X.697 §41 has the value that the octets hold, which the views of the bindings show beside them |
//!
//! Hexadecimal is written in upper case and read in either.
//!
//! A type has the two forms through [`Json`], which generated code implements with the
//! functions of this module: the names of the members of a SEQUENCE are a table, and a
//! constraint is the arguments of a call. Reading is strict, as nothing reads its
//! result back to compare: a member that the type does not have, a second alternative
//! of a `CHOICE`, padding bits that are not zero and an integer outside the constraint
//! of its type are each refused, with the names on the way to the value.

use alloc::borrow::Cow;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use serde_json::{Map, Number, Value};

use crate::reader::Alphabet;
use crate::{AsnType, BitString, Bits, Error, ErrorKind, ObjectIdentifier, Octets};

/// A type whose values have a JSON form.
pub trait Json: Sized {
    /// The JSON form of the value.
    fn to_json(&self) -> Value;

    /// The value that `value` writes. What is not a value of the type is refused in
    /// `reading`, which has the reason and the names on the way to it.
    fn from_json(value: &Value, reading: &mut Reading) -> Result<Self, Refused>;
}

/// JSON that is not a value of its type: the [`Reading`] has why.
#[derive(Debug)]
pub struct Refused;

/// The JSON form of `value`.
pub fn to_value<T: Json>(value: &T) -> Value {
    value.to_json()
}

/// The value of the type `T` that `value` writes.
///
/// # Errors
///
/// An error of the kind `Value`, or `NoSuchMember` for a name that the type does not
/// have, which says where the value is not one of its type, by the names that ASN.1
/// gives, and why.
pub fn from_value<T: Json + AsnType>(value: &Value) -> Result<T, Error> {
    let mut reading = Reading::new();
    match T::from_json(value, &mut reading) {
        Ok(value) => Ok(value),
        Err(Refused) => Err(reading.refused(T::NAME)),
    }
}

// ── Hexadecimal ─────────────────────────────────────────────────────────────

/// Octets in hexadecimal, in upper case.
pub fn hex(octets: &[u8]) -> String {
    // A digit is its half octet after `0`, and seven further for one after `9`, which
    // are the characters between `9` and `A`. The loop has no table and no branch, so
    // the compiler writes it for several octets at once.
    let digit = |half: u8| half + b'0' + u8::from(half > 9) * (b'A' - b'9' - 1);
    let mut text = vec![0; octets.len() * 2];
    let (pairs, _) = text.as_chunks_mut::<2>();
    for (pair, octet) in pairs.iter_mut().zip(octets) {
        *pair = [digit(octet >> 4), digit(octet & 15)];
    }
    // The digits are ASCII.
    String::from_utf8(text).unwrap_or_default()
}

/// The octets that `text` writes in hexadecimal, two digits for each, in either case.
pub fn unhex(text: &str) -> Option<Vec<u8>> {
    /// What is no digit: a value that no half octet has.
    const NONE: u8 = 0x80;
    // The value of a digit, without a branch that leaves the loop: what is no digit
    // is told after it, so that the compiler writes the loop for several octets at
    // once. A letter is taken in lower case, which differs from upper case by one bit.
    let digit = |digit: u8| {
        let (number, letter) = (digit.wrapping_sub(b'0'), (digit | 0x20).wrapping_sub(b'a'));
        match (number < 10, letter < 6) {
            (true, _) => number,
            (false, true) => letter + 10,
            (false, false) => NONE,
        }
    };
    let (pairs, rest) = text.as_bytes().as_chunks::<2>();
    if !rest.is_empty() {
        return None;
    }
    let mut octets = vec![0; pairs.len()];
    let mut other = 0;
    for (octet, [high, low]) in octets.iter_mut().zip(pairs) {
        let (high, low) = (digit(*high), digit(*low));
        other |= high | low;
        *octet = high << 4 | low & 15;
    }
    (other & NONE == 0).then_some(octets)
}

/// The start of what is written, which says which value it is.
fn written(value: &Value) -> String {
    let text = value.to_string();
    match text.char_indices().nth(40) {
        Some((end, _)) => format!("{}…", &text[..end]),
        None => text,
    }
}

// ── From a value to JSON ────────────────────────────────────────────────────

/// The JSON form of a number without a sign.
pub fn unsigned(value: u64) -> Value {
    Value::Number(Number::from(value))
}

/// The JSON form of a number with a sign.
pub fn signed(value: i64) -> Value {
    Value::Number(Number::from(value))
}

/// The JSON form of octets: hexadecimal.
pub fn octets(octets: &[u8]) -> Value {
    Value::String(hex(octets))
}

/// The JSON form of a short bit string of the size constraint `lower..=upper`: its
/// octets when its type gives it one size, and its octets with its length otherwise.
pub fn bits(value: Bits, lower: usize, upper: usize) -> Value {
    let text = hex(&value.to_vec());
    match lower == upper {
        true => Value::String(text),
        false => with_length(text, value.len()),
    }
}

/// The JSON form of a bit string of the size constraint `lower..=upper`, which has an
/// extension marker when it is `extensible`: its octets when it has the one size that
/// its type gives it, and its octets with its length otherwise. A string of a type of
/// one size and no marker is its octets, whatever its size.
pub fn bit_string(value: &BitString<'_>, lower: usize, upper: usize, extensible: bool) -> Value {
    let text = hex(value.as_octets());
    match lower == upper && (value.len() == lower || !extensible) {
        true => Value::String(text),
        false => with_length(text, value.len()),
    }
}

fn with_length(text: String, length: usize) -> Value {
    let mut members = Map::new();
    members.insert("value".into(), Value::String(text));
    members.insert("length".into(), Value::Number(Number::from(length)));
    Value::Object(members)
}

/// The JSON form of a character string.
pub fn text(text: &str) -> Value {
    Value::String(text.to_string())
}

/// The JSON form of an object identifier: its arcs in decimal, with a full stop
/// between them.
pub fn object_identifier(identifier: &ObjectIdentifier) -> Value {
    Value::String(identifier.to_string())
}

/// The JSON form of a CHOICE: an object with one member, the name of the alternative
/// with its value.
pub fn alternative(name: &str, value: Value) -> Value {
    let mut members = Map::new();
    members.insert(name.to_string(), value);
    Value::Object(members)
}

/// The JSON form of a SEQUENCE, to which each member that is there is added by the
/// name that ASN.1 gives it.
#[derive(Default)]
pub struct Members(Map<String, Value>);

impl Members {
    /// A SEQUENCE without a member yet.
    pub fn new() -> Self {
        Self(Map::new())
    }

    /// A member that is there.
    pub fn put(&mut self, name: &str, value: Value) {
        self.0.insert(name.to_string(), value);
    }

    /// A member that may be absent: nothing is written for one that is.
    pub fn put_optional(&mut self, name: &str, value: Option<Value>) {
        if let Some(value) = value {
            self.put(name, value);
        }
    }

    /// The JSON form.
    pub fn finish(self) -> Value {
        Value::Object(self.0)
    }
}

// ── From JSON to a value ────────────────────────────────────────────────────

/// What a value that is read is named by, in what holds it.
#[derive(Clone, Copy)]
enum Name {
    /// A member of a SEQUENCE or the alternative of a CHOICE, by the name that ASN.1
    /// gives it.
    Member(&'static str),
    /// An entry of a list, by its position.
    Entry(usize),
}

/// The reading of one JSON value as a value of the bindings: the names on the way to
/// what is being read, and why the JSON is refused, once it is.
pub struct Reading {
    at: Vec<Name>,
    /// The first fault: its kind, the names on the way to it, and what is wrong there.
    refusal: Option<(ErrorKind, Vec<String>, String)>,
}

/// The members of a SEQUENCE that is being read.
pub struct Read<'v> {
    members: &'v Map<String, Value>,
    names: &'static [&'static str],
}

impl Reading {
    fn new() -> Self {
        Self {
            at: Vec::new(),
            refusal: None,
        }
    }

    /// The error of a value that is refused, for the type that ASN.1 names `name`.
    fn refused(self, name: &str) -> Error {
        let (kind, at, why) = self.refusal.unwrap_or_else(|| {
            let why = "it is refused without a reason".to_string();
            (ErrorKind::Value, Vec::new(), why)
        });
        let what = match at.is_empty() {
            true => format!("this is not a value of {name}"),
            false => format!("this is not a value of {name}, at {}", at.join("/")),
        };
        Error::new(kind, what).because(why)
    }

    /// Refuse what is being read: `why`. The first refusal is the one that is told.
    pub fn refuse(&mut self, kind: ErrorKind, why: String) -> Refused {
        if self.refusal.is_none() {
            let segment = |name: &Name| match name {
                Name::Member(name) => name.to_string(),
                Name::Entry(position) => position.to_string(),
            };
            let at = self.at.iter().map(segment).collect();
            self.refusal = Some((kind, at, why));
        }
        Refused
    }

    /// Refuse a value that is not written as its type is: `expected` says what is
    /// expected, where `found` is written.
    fn mismatch(&mut self, expected: &str, found: &Value) -> Refused {
        let why = format!("{expected}, and {} is written", written(found));
        self.refuse(ErrorKind::Value, why)
    }

    // ── SEQUENCE, SEQUENCE OF and CHOICE ────────────────────────────────────

    /// The members of a SEQUENCE whose components ASN.1 names `names`, in their order.
    /// A member of another name is refused.
    pub fn members<'v>(
        &mut self,
        value: &'v Value,
        names: &'static [&'static str],
    ) -> Result<Read<'v>, Refused> {
        let Some(members) = value.as_object() else {
            return Err(self.mismatch("an object is expected", value));
        };
        let known = names.iter().filter(|name| members.contains_key(**name));
        if known.count() != members.len()
            && let Some(other) = members
                .keys()
                .find(|member| !names.contains(&member.as_str()))
        {
            let why = format!("{other:?} is not a member that its type has");
            return Err(self.refuse(ErrorKind::NoSuchMember, why));
        }
        Ok(Read { members, names })
    }

    /// The member at `position` among the names of a SEQUENCE, which its type always
    /// has: `read` reads its value.
    pub fn member<T>(
        &mut self,
        of: &Read<'_>,
        position: usize,
        read: impl FnOnce(&Value, &mut Self) -> Result<T, Refused>,
    ) -> Result<T, Refused> {
        let name = of.names[position];
        self.at.push(Name::Member(name));
        let Some(value) = of.members.get(name) else {
            return Err(self.refuse(ErrorKind::Value, "the member is missing".to_string()));
        };
        let value = read(value, self)?;
        self.at.pop();
        Ok(value)
    }

    /// The member at `position` among the names of a SEQUENCE, which may be absent.
    pub fn optional<T>(
        &mut self,
        of: &Read<'_>,
        position: usize,
        read: impl FnOnce(&Value, &mut Self) -> Result<T, Refused>,
    ) -> Result<Option<T>, Refused> {
        let name = of.names[position];
        let Some(value) = of.members.get(name) else {
            return Ok(None);
        };
        self.at.push(Name::Member(name));
        let value = read(value, self)?;
        self.at.pop();
        Ok(Some(value))
    }

    /// The entries of a list: `read` reads each.
    pub fn list<T>(
        &mut self,
        value: &Value,
        mut read: impl FnMut(&Value, &mut Self) -> Result<T, Refused>,
    ) -> Result<Vec<T>, Refused> {
        let Some(entries) = value.as_array() else {
            return Err(self.mismatch("a list is expected", value));
        };
        let mut values = Vec::with_capacity(entries.len());
        for (position, entry) in entries.iter().enumerate() {
            self.at.push(Name::Entry(position));
            values.push(read(entry, self)?);
            self.at.pop();
        }
        Ok(values)
    }

    /// The alternative of a CHOICE whose alternatives ASN.1 names `names`: its
    /// position among them, and its value, which the caller reads within
    /// [`within`](Self::within).
    pub fn alternative<'v>(
        &mut self,
        value: &'v Value,
        names: &'static [&'static str],
    ) -> Result<(usize, &'v Value), Refused> {
        const ONE: &str = "an object with one member is expected, the name of an alternative \
                           with its value";
        let mut members = value.as_object().into_iter().flatten();
        let (Some((written, held)), None) = (members.next(), members.next()) else {
            return Err(self.mismatch(ONE, value));
        };
        let Some(position) = names.iter().position(|known| known == written) else {
            let why = format!(
                "{written:?} is not an alternative that its type has: it has {}",
                names.join(", ")
            );
            return Err(self.refuse(ErrorKind::NoSuchMember, why));
        };
        Ok((position, held))
    }

    /// Read the value of the alternative that ASN.1 names `name`.
    pub fn within<T>(
        &mut self,
        name: &'static str,
        value: &Value,
        read: impl FnOnce(&Value, &mut Self) -> Result<T, Refused>,
    ) -> Result<T, Refused> {
        self.at.push(Name::Member(name));
        let value = read(value, self)?;
        self.at.pop();
        Ok(value)
    }

    // ── The other types ─────────────────────────────────────────────────────

    /// A BOOLEAN.
    pub fn boolean(&mut self, value: &Value) -> Result<bool, Refused> {
        match value.as_bool() {
            Some(read) => Ok(read),
            None => Err(self.mismatch("true or false is expected", value)),
        }
    }

    /// A NULL.
    pub fn null(&mut self, value: &Value) -> Result<(), Refused> {
        match value.is_null() {
            true => Ok(()),
            false => Err(self.mismatch("null is expected", value)),
        }
    }

    /// The name of a value of an ENUMERATED that has these names: the position of the
    /// name among them. A name is its letters and its digits, whatever their case and
    /// whatever is between them.
    pub fn enumerated(
        &mut self,
        value: &Value,
        names: &'static [&'static str],
    ) -> Result<usize, Refused> {
        const NAME: &str = "the name of a value of the ENUMERATED is expected";
        let Some(text) = value.as_str() else {
            return Err(self.mismatch(NAME, value));
        };
        let exact = names.iter().position(|known| *known == text);
        let found = exact.or_else(|| names.iter().position(|known| same_name(known, text)));
        found.ok_or_else(|| {
            let why = format!("{text:?} is no value of the ENUMERATED");
            self.refuse(ErrorKind::Value, why)
        })
    }

    /// An INTEGER of a type without a sign, whose constraint is `lower..=upper`: a
    /// number. A type whose constraint has an extension marker holds every number of
    /// 64 bits without a sign, and so has these as its bounds here.
    pub fn unsigned(&mut self, value: &Value, lower: u64, upper: u64) -> Result<u64, Refused> {
        let number = self.number(value)?;
        match u64::try_from(number) {
            Ok(number) if (lower..=upper).contains(&number) => Ok(number),
            _ => Err(self.outside(value, lower.into(), upper.into())),
        }
    }

    /// An INTEGER of a type with a sign, whose constraint is `lower..=upper`.
    pub fn signed(&mut self, value: &Value, lower: i64, upper: i64) -> Result<i64, Refused> {
        let number = self.number(value)?;
        match i64::try_from(number) {
            Ok(number) if (lower..=upper).contains(&number) => Ok(number),
            _ => Err(self.outside(value, lower.into(), upper.into())),
        }
    }

    /// The whole number that JSON writes: one that 64 bits hold, with a sign or without.
    fn number(&mut self, value: &Value) -> Result<i128, Refused> {
        let read = value.as_number().and_then(|number| {
            let signed = number.as_i64().map(i128::from);
            signed.or_else(|| number.as_u64().map(i128::from))
        });
        read.ok_or_else(|| self.mismatch("a number is expected", value))
    }

    fn outside(&mut self, value: &Value, lower: i128, upper: i128) -> Refused {
        let why = format!(
            "{} is outside its constraint ({lower}..{upper})",
            written(value)
        );
        self.refuse(ErrorKind::Value, why)
    }

    /// Octets in hexadecimal: an OCTET STRING, and an open type.
    pub fn octets(&mut self, value: &Value) -> Result<Octets<'static>, Refused> {
        const OCTETS: &str = "octets in hexadecimal, two digits for each, are expected";
        match value.as_str().and_then(unhex) {
            Some(octets) => Ok(Octets::owned(octets)),
            None => Err(self.mismatch(OCTETS, value)),
        }
    }

    /// An OCTET STRING of `N` octets.
    pub fn octet_array<const N: usize>(&mut self, value: &Value) -> Result<[u8; N], Refused> {
        let octets = self.octets(value)?;
        <[u8; N]>::try_from(&octets[..]).map_err(|_| {
            let why = format!(
                "{} octets are written for an OCTET STRING of {N}",
                octets.len()
            );
            self.refuse(ErrorKind::Value, why)
        })
    }

    /// A short bit string of the size constraint `lower..=upper`.
    pub fn bits(&mut self, value: &Value, lower: usize, upper: usize) -> Result<Bits, Refused> {
        let (octets, length) = self.bit_octets(value, lower, upper, false)?;
        match Bits::from_octets(&octets, length) {
            Ok(bits) if (lower..=upper).contains(&length) => Ok(bits),
            _ => {
                let size = match lower == upper {
                    true => lower.to_string(),
                    false => format!("{lower}..{upper}"),
                };
                let why = format!("a size of {length} is outside its constraint ({size})");
                Err(self.refuse(ErrorKind::Value, why))
            }
        }
    }

    /// A bit string of the size constraint `lower..=upper`, which has an extension
    /// marker when it is `extensible`. Its size is compared with the constraint where
    /// it is encoded.
    pub fn bit_string(
        &mut self,
        value: &Value,
        lower: usize,
        upper: usize,
        extensible: bool,
    ) -> Result<BitString<'static>, Refused> {
        let (octets, length) = self.bit_octets(value, lower, upper, extensible)?;
        BitString::new(octets, length).map_err(|error| {
            let why = error.to_string();
            self.refuse(ErrorKind::Value, why)
        })
    }

    /// The octets and the length of a bit string of a type of this size constraint.
    fn bit_octets(
        &mut self,
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
                    None => return Err(self.mismatch(WITH_LENGTH, value)),
                }
            }
            (_, Some(size)) if !extensible => {
                let expected = format!("a bit string of {size} bits is expected, in hexadecimal");
                return Err(self.mismatch(&expected, value));
            }
            (_, Some(size)) => {
                let expected = format!(
                    "a bit string of {size} bits is expected, in hexadecimal, or one of another \
                     size as {{\"value\": HEXADECIMAL, \"length\": BITS}}"
                );
                return Err(self.mismatch(&expected, value));
            }
            (_, None) => return Err(self.mismatch(WITH_LENGTH, value)),
        };
        let Some(octets) = unhex(digits) else {
            let why = format!("{digits:?} is not hexadecimal, two digits for each octet");
            return Err(self.refuse(ErrorKind::Value, why));
        };
        if octets.len() != length.div_ceil(8) {
            let why = format!(
                "{} octets are written for a bit string of {length} bits, which has {}",
                octets.len(),
                length.div_ceil(8)
            );
            return Err(self.refuse(ErrorKind::Value, why));
        }
        let padding = octets.len() * 8 - length;
        if octets
            .last()
            .is_some_and(|last| last.trailing_zeros() < padding as u32)
        {
            let why = format!(
                "the last {padding} bits of {digits:?} are after the {length} bits of the bit \
                 string, and are not zero"
            );
            return Err(self.refuse(ErrorKind::Value, why));
        }
        Ok((octets, length))
    }

    /// A UTF8String.
    pub fn utf8_string(&mut self, value: &Value) -> Result<Cow<'static, str>, Refused> {
        match value.as_str() {
            Some(text) => Ok(Cow::Owned(text.to_string())),
            None => Err(self.mismatch("a string is expected", value)),
        }
    }

    /// A PrintableString.
    pub fn printable_string(&mut self, value: &Value) -> Result<Cow<'static, str>, Refused> {
        self.known_multiplier(value, Alphabet::Printable, "a PrintableString")
    }

    /// A VisibleString.
    pub fn visible_string(&mut self, value: &Value) -> Result<Cow<'static, str>, Refused> {
        self.known_multiplier(value, Alphabet::Visible, "a VisibleString")
    }

    /// An IA5String.
    pub fn ia5_string(&mut self, value: &Value) -> Result<Cow<'static, str>, Refused> {
        self.known_multiplier(value, Alphabet::Ia5, "an IA5String")
    }

    fn known_multiplier(
        &mut self,
        value: &Value,
        alphabet: Alphabet,
        of: &str,
    ) -> Result<Cow<'static, str>, Refused> {
        let text = self.utf8_string(value)?;
        if text.bytes().all(|code| alphabet.has(code)) {
            return Ok(text);
        }
        let why = format!("{text:?} has a character that {of} has not");
        Err(self.refuse(ErrorKind::Value, why))
    }

    /// An OBJECT IDENTIFIER.
    pub fn object_identifier(&mut self, value: &Value) -> Result<ObjectIdentifier, Refused> {
        const ARCS: &str = "an object identifier is expected, its arcs in decimal with a full \
                            stop between them";
        let identifier = value.as_str().and_then(|arcs| arcs.parse().ok());
        identifier.ok_or_else(|| self.mismatch(ARCS, value))
    }
}

/// The letters and the digits of a name, in lower case.
fn letters(name: &str) -> impl Iterator<Item = u8> + '_ {
    let letters = name.bytes().filter(u8::is_ascii_alphanumeric);
    letters.map(|letter| letter.to_ascii_lowercase())
}

/// Whether two names are the same: they have the same letters and digits, whatever their
/// case and whatever is between them, as `user-inactivity` and `UserInactivity`. A name
/// without a letter or a digit is no name.
pub fn same_name(a: &str, b: &str) -> bool {
    letters(a).next().is_some() && letters(a).eq(letters(b))
}
