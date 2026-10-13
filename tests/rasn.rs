//! The fields of the crate against the aligned PER codec of rasn, another
//! implementation: the same value is written to the same octets by both, alone and
//! after one bit, which takes it off the octet boundary, and each reads what the other
//! wrote.
//!
//! Where rasn 0.29 does not follow ITU-T X.691 (02/2021), the case says so and gives
//! the clause, and the octets of the clause are in `tests/fields.rs`.

use oxirush_aper::{Bits, ObjectIdentifier, Reader, UNBOUNDED, Writer};
use rasn::prelude::*;

/// A value after one bit: what follows a BOOLEAN in a SEQUENCE starts within an octet.
#[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
struct After<T> {
    flag: bool,
    value: T,
}

fn written(write: impl FnOnce(&mut Writer<'_>)) -> Vec<u8> {
    let mut octets = Vec::new();
    write(&mut Writer::new(&mut octets));
    octets
}

/// Where a value is compared with rasn.
#[derive(Clone, Copy, PartialEq)]
enum Where {
    /// Alone, and after a bit.
    Anywhere,
    /// Alone, and after a bit for what rasn writes: rasn 0.29 does not read it there.
    WrittenAnywhere,
    /// Alone: rasn 0.29 does not follow X.691 after a bit.
    Alone,
    /// What rasn writes, alone and after a bit: rasn 0.29 does not read it.
    Written,
}

/// `theirs` is written by rasn as `write` writes it, alone and after a bit, and `read`
/// says that the reader gives the value back from these octets.
#[track_caller]
fn same<T>(theirs: &T, write: impl Fn(&mut Writer<'_>), read: impl Fn(&mut Reader<'_>) -> bool)
where
    T: Encode + Decode + Clone + PartialEq + core::fmt::Debug,
{
    same_where(theirs, write, read, Where::Anywhere)
}

#[track_caller]
fn same_where<T>(
    theirs: &T,
    write: impl Fn(&mut Writer<'_>),
    read: impl Fn(&mut Reader<'_>) -> bool,
    compared: Where,
) where
    T: Encode + Decode + Clone + PartialEq + core::fmt::Debug,
{
    // Alone: the fields of the value. Where it has none, rasn writes no octet, and
    // the complete encoding is one zero octet (X.691 §11.1.4), which
    // `oxirush_aper::encode` writes.
    let ours = written(&write);
    assert_eq!(rasn::aper::encode(theirs).unwrap(), ours, "{theirs:?}");
    assert!(read(&mut Reader::new(&ours)), "{theirs:?} from {ours:02x?}");
    if compared != Where::Written {
        assert_eq!(&rasn::aper::decode::<T>(&ours).unwrap(), theirs);
    }
    if compared == Where::Alone {
        return;
    }
    // After a bit.
    let ours = written(|w| {
        w.bit(true);
        write(w);
    });
    let after = After {
        flag: true,
        value: theirs.clone(),
    };
    assert_eq!(
        rasn::aper::encode(&after).unwrap(),
        ours,
        "after a bit: {theirs:?}"
    );
    let mut reader = Reader::new(&ours);
    assert!(reader.bit().unwrap());
    assert!(read(&mut reader), "after a bit: {theirs:?}");
    if compared == Where::Anywhere {
        assert_eq!(rasn::aper::decode::<After<T>>(&ours).unwrap(), after);
    }
}

/// Numbers around the bounds of a constraint, and between them.
fn numbers(lower: i128, upper: i128) -> Vec<i128> {
    let mut numbers = vec![lower, upper, lower + (upper - lower) / 2];
    for step in [
        1,
        2,
        127,
        128,
        255,
        256,
        65535,
        65536,
        1 << 24,
        1 << 32,
        1 << 40,
    ] {
        numbers.extend([lower + step, upper - step]);
    }
    numbers.retain(|number| (lower..=upper).contains(number));
    numbers.sort_unstable();
    numbers.dedup();
    numbers
}

macro_rules! constrained {
    ($($name:ident: $native:ty = $constraint:literal, $lower:literal, $upper:literal;)*) => {$({
        #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
        #[rasn(delegate, value($constraint))]
        struct $name($native);
        let (lower, upper): (i128, i128) = ($lower, $upper);
        for number in numbers(lower, upper) {
            if lower < 0 {
                same(
                    &$name(number as $native),
                    |w| w.signed(number as i64, lower as i64, upper as i64).unwrap(),
                    |r| r.signed(lower as i64, upper as i64).unwrap() == number as i64,
                );
            } else {
                same(
                    &$name(number as $native),
                    |w| w.unsigned(number as u64, lower as u64, upper as u64).unwrap(),
                    |r| r.unsigned(lower as u64, upper as u64).unwrap() == number as u64,
                );
            }
        }
    })*};
}

#[test]
fn constrained_whole_numbers() {
    constrained! {
        One: u8 = "5..=5", 5, 5;
        Bit: u8 = "0..=1", 0, 1;
        Three: u8 = "0..=7", 0, 7;
        Hundred: u8 = "0..=100", 0, 100;
        Most: u8 = "0..=254", 0, 254;
        Octet: u8 = "0..=255", 0, 255;
        OctetFromOne: u16 = "1..=256", 1, 256;
        Over: u16 = "0..=256", 0, 256;
        Twelve: u16 = "0..=4095", 0, 4095;
        Two: u16 = "0..=65535", 0, 65535;
        TwoFromOne: u32 = "1..=65536", 1, 65536;
        Seventeen: u32 = "0..=65536", 0, 65536;
        High: u32 = "4096..=131071", 4096, 131071;
        Three24: u32 = "0..=16777215", 0, 16777215;
        Four: u32 = "0..=4294967295", 0, 4294967295;
        Ten: u64 = "0..=10000000000", 0, 10000000000;
        Five: u64 = "0..=1099511627775", 0, 1099511627775;
        Eight: u64 = "0..=18446744073709551615", 0, 18446744073709551615;
        Small: i8 = "-127..=127", -127, 127;
        Signed: i32 = "-640000..=640000", -640000, 640000;
        Negative: i16 = "-420..=10000", -420, 10000;
    }
}

#[test]
fn whole_numbers_with_an_extension_marker_and_without_an_upper_bound() {
    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(delegate, value("0..=255", extensible))]
    struct Octet(Integer);
    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(delegate, value("0..=4000000000000", extensible))]
    struct BitRate(Integer);
    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(delegate, value("-420..=10000", extensible))]
    struct Offset(Integer);
    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(delegate, value("0.."))]
    struct Natural(Integer);
    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(delegate, value("-5.."))]
    struct Above(Integer);

    for number in [
        0u64,
        1,
        127,
        128,
        255,
        256,
        65535,
        65536,
        1 << 40,
        u64::MAX >> 1,
        u64::MAX,
    ] {
        same(
            &Octet(Integer::from(number)),
            |w| w.unsigned_extensible(number, 0, 255).unwrap(),
            |r| r.unsigned_extensible(0, 255).unwrap() == number,
        );
        same(
            &BitRate(Integer::from(number)),
            |w| w.unsigned_extensible(number, 0, 4000000000000).unwrap(),
            |r| r.unsigned_extensible(0, 4000000000000).unwrap() == number,
        );
        same(
            &Natural(Integer::from(number)),
            |w| w.semi_constrained_unsigned(number, 0).unwrap(),
            |r| r.semi_constrained_unsigned(0).unwrap() == number,
        );
    }
    for number in [
        i64::MIN,
        -(1 << 40),
        -65537,
        -32769,
        -32768,
        -421,
        -420,
        -129,
        -128,
        -1,
        0,
        1,
        127,
        128,
        10000,
        10001,
        32767,
        32768,
        1 << 40,
        i64::MAX,
    ] {
        same(
            &Offset(Integer::from(number)),
            |w| w.signed_extensible(number, -420, 10000).unwrap(),
            |r| r.signed_extensible(-420, 10000).unwrap() == number,
        );
        same(
            &Integer::from(number),
            |w| w.unconstrained(number),
            |r| r.unconstrained().unwrap() == number,
        );
        if number >= -5 {
            same(
                &Above(Integer::from(number)),
                |w| w.semi_constrained(number, -5).unwrap(),
                |r| r.semi_constrained(-5).unwrap() == number,
            );
        }
    }
}

#[test]
fn enumerated_and_choice() {
    #[derive(AsnType, Debug, Clone, Copy, Decode, Encode, PartialEq)]
    #[rasn(enumerated)]
    enum Three {
        A = 0,
        B = 1,
        C = 2,
    }
    #[derive(AsnType, Debug, Clone, Copy, Decode, Encode, PartialEq)]
    #[rasn(enumerated)]
    #[non_exhaustive]
    enum Extended {
        A = 0,
        B = 1,
        C = 2,
        #[rasn(extension_addition)]
        D = 3,
        #[rasn(extension_addition)]
        E = 4,
    }
    for (index, value) in [Three::A, Three::B, Three::C].into_iter().enumerate() {
        same(
            &value,
            |w| w.index(index as u64, 3, false),
            |r| r.enumerated(3, 0, false).unwrap() == index as u64,
        );
    }
    let extended = [
        Extended::A,
        Extended::B,
        Extended::C,
        Extended::D,
        Extended::E,
    ];
    for (index, value) in extended.into_iter().enumerate() {
        same(
            &value,
            |w| w.index(index as u64, 3, true),
            |r| r.enumerated(3, 2, true).unwrap() == index as u64,
        );
    }

    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(choice, automatic_tags)]
    enum Closed {
        #[rasn(value("0..=255"))]
        Number(u8),
        Flag(bool),
        Nothing(()),
    }
    same(
        &Closed::Number(7),
        |w| {
            w.index(0, 3, false);
            w.unsigned(7, 0, 255).unwrap();
        },
        |r| r.choice(3, 0, false).unwrap() == 0 && r.unsigned(0, 255).unwrap() == 7,
    );
    same(
        &Closed::Flag(true),
        |w| {
            w.index(1, 3, false);
            w.bit(true);
        },
        |r| r.choice(3, 0, false).unwrap() == 1 && r.bit().unwrap(),
    );
    same(
        &Closed::Nothing(()),
        |w| w.index(2, 3, false),
        |r| r.choice(3, 0, false).unwrap() == 2,
    );

    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(choice, automatic_tags)]
    #[non_exhaustive]
    enum Open {
        #[rasn(value("0..=255"))]
        Number(u8),
        Flag(bool),
        #[rasn(extension_addition, value("0..=65535"))]
        Later(u16),
        #[rasn(extension_addition)]
        Last(bool),
    }
    same(
        &Open::Flag(true),
        |w| {
            w.index(1, 2, true);
            w.bit(true);
        },
        |r| r.choice(2, 2, true).unwrap() == 1 && r.bit().unwrap(),
    );
    // An addition is in an open type (X.691 §23.8).
    same(
        &Open::Later(0x1234),
        |w| {
            w.index(2, 2, true);
            let start = w.begin_open_type();
            w.unsigned(0x1234, 0, 65535).unwrap();
            w.end_open_type(start);
        },
        |r| {
            let alternative = r.choice(2, 2, true).unwrap();
            let value = r.open_type().unwrap();
            alternative == 2 && Reader::new(&value).unsigned(0, 65535).unwrap() == 0x1234
        },
    );
    same(
        &Open::Last(true),
        |w| {
            w.index(3, 2, true);
            let start = w.begin_open_type();
            w.bit(true);
            w.end_open_type(start);
        },
        |r| {
            let alternative = r.choice(2, 2, true).unwrap();
            let value = r.open_type().unwrap();
            alternative == 3 && Reader::new(&value).bit().unwrap()
        },
    );
}

/// Octets that are not all the same.
fn some_octets(length: usize) -> Vec<u8> {
    (0..length).map(|at| (at * 7 + 3) as u8).collect()
}

#[test]
fn octet_strings() {
    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(delegate, size("1"))]
    struct One(FixedOctetString<1>);
    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(delegate, size("2"))]
    struct Two(FixedOctetString<2>);
    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(delegate, size("3"))]
    struct Three(FixedOctetString<3>);
    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(delegate, size("32"))]
    struct Key(FixedOctetString<32>);
    same(
        &One([0xab].into()),
        |w| w.octet_array(&[0xab]),
        |r| r.octet_array::<1>().unwrap() == [0xab],
    );
    same(
        &Two([0xab, 0xcd].into()),
        |w| w.octet_array(&[0xab, 0xcd]),
        |r| r.octet_array::<2>().unwrap() == [0xab, 0xcd],
    );
    same(
        &Three([1, 2, 3].into()),
        |w| w.octet_array(&[1, 2, 3]),
        |r| r.octet_array::<3>().unwrap() == [1, 2, 3],
    );
    let key: [u8; 32] = some_octets(32).try_into().unwrap();
    same(
        &Key(key.into()),
        |w| w.octet_array(&key),
        |r| r.octet_array::<32>().unwrap() == key,
    );

    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(delegate, size("1..=4"))]
    struct Short(OctetString);
    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(delegate, size("0..=3"))]
    struct FromNone(OctetString);
    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(delegate, size("1..=150"))]
    struct Name(OctetString);
    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(delegate, size("1..=1000"))]
    struct Long(OctetString);
    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(delegate, size("3", extensible))]
    struct Extensible(OctetString);
    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(delegate, size("1..=150", extensible))]
    struct ExtensibleName(OctetString);
    for length in 1..=4 {
        let octets = some_octets(length);
        same(
            &Short(octets.clone().into()),
            |w| w.octet_string(&octets, 1, 4).unwrap(),
            |r| r.octet_string(1, 4).unwrap() == octets,
        );
    }
    for length in 1..=3 {
        let octets = some_octets(length);
        same(
            &FromNone(octets.clone().into()),
            |w| w.octet_string(&octets, 0, 3).unwrap(),
            |r| r.octet_string(0, 3).unwrap() == octets,
        );
    }
    // No octet: nothing is added after the length, and no padding (X.691 §11.9.3.3).
    same(
        &FromNone(OctetString::from(Vec::new())),
        |w| w.octet_string(&[], 0, 3).unwrap(),
        |r| r.octet_string(0, 3).unwrap().is_empty(),
    );
    for length in [1, 2, 127, 128, 150] {
        let octets = some_octets(length);
        same(
            &Name(octets.clone().into()),
            |w| w.octet_string(&octets, 1, 150).unwrap(),
            |r| r.octet_string(1, 150).unwrap() == octets,
        );
        same(
            &ExtensibleName(octets.clone().into()),
            |w| w.octet_string_extensible(&octets, 1, 150).unwrap(),
            |r| r.octet_string_extensible(1, 150).unwrap() == octets,
        );
    }
    // A range of 256 sizes or more: the length is octet-aligned (X.691 §11.5.7.3),
    // which rasn 0.29 does not do when the string starts within an octet.
    for length in [1, 255, 256, 1000] {
        let octets = some_octets(length);
        same_where(
            &Long(octets.clone().into()),
            |w| w.octet_string(&octets, 1, 1000).unwrap(),
            |r| r.octet_string(1, 1000).unwrap() == octets,
            Where::Alone,
        );
    }
    for length in [0, 2, 3, 4, 200] {
        let octets = some_octets(length);
        // rasn 0.29 skips to the next octet before the extension bit of a string of
        // one size and not after it, where X.691 §17.3 has the bit, then the string as
        // without the marker, which is octet-aligned (§17.7): it writes that, and does
        // not read it back.
        same_where(
            &Extensible(octets.clone().into()),
            |w| w.octet_string_extensible(&octets, 3, 3).unwrap(),
            |r| r.octet_string_extensible(3, 3).unwrap() == octets,
            Where::Written,
        );
        same(
            &ExtensibleName(octets.clone().into()),
            |w| w.octet_string_extensible(&octets, 1, 150).unwrap(),
            |r| r.octet_string_extensible(1, 150).unwrap() == octets,
        );
    }
    // No size constraint: each form of the length, and fragments.
    for length in [
        0, 1, 127, 128, 16383, 16384, 16385, 32768, 65535, 65536, 65537, 70000,
    ] {
        let octets = some_octets(length);
        same(
            &OctetString::from(octets.clone()),
            |w| w.octet_string(&octets, 0, UNBOUNDED).unwrap(),
            |r| r.octet_string(0, UNBOUNDED).unwrap() == octets,
        );
        // An open type holds one octet at least.
        if length > 0 {
            same(
                &Any::new(octets.clone()),
                |w| w.open_type(&octets),
                |r| r.open_type().unwrap() == octets,
            );
        }
    }
}

/// A bit string of rasn: the first `length` bits of octets.
fn their_bits(octets: &[u8], length: usize) -> BitString {
    let mut bits = BitString::from_slice(octets);
    bits.truncate(length);
    bits
}

/// The octets of a bit string of `length` bits that are not all the same.
fn some_bits(length: usize) -> Vec<u8> {
    let mut octets = some_octets(length.div_ceil(8));
    if let Some(last) = octets.last_mut() {
        *last &= 0xffu8
            .checked_shl((8 - length % 8) as u32 % 8)
            .unwrap_or(0xff);
        if !length.is_multiple_of(8) {
            *last &= 0xff << (8 - length % 8);
        }
    }
    octets
}

#[test]
fn bit_strings() {
    macro_rules! fixed {
        ($($name:ident = $size:literal, $length:literal;)*) => {$({
            #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
            #[rasn(delegate, size($size))]
            struct $name(BitString);
            let octets = some_bits($length);
            let long = oxirush_aper::BitString::new(octets.clone(), $length).unwrap();
            // rasn 0.29 reads a string of one size of more than 16 bits where it is,
            // without the padding that X.691 §16.10 has before it: what it writes is
            // compared.
            let compared = if $length <= 16 { Where::Anywhere } else { Where::WrittenAnywhere };
            same_where(
                &$name(their_bits(&octets, $length)),
                |w| w.bit_string(&long, $length, $length).unwrap(),
                |r| r.bit_string($length, $length).unwrap() == long,
                compared,
            );
            if $length <= 64 {
                let short = Bits::from_octets(&octets, $length).unwrap();
                same_where(
                    &$name(their_bits(&octets, $length)),
                    |w| w.bits_sized(short, $length, $length).unwrap(),
                    |r| r.bits_sized($length, $length).unwrap() == short,
                    compared,
                );
            }
        })*};
    }
    fixed! {
        B1 = "1", 1;
        B5 = "5", 5;
        B6 = "6", 6;
        B8 = "8", 8;
        B10 = "10", 10;
        B16 = "16", 16;
        B17 = "17", 17;
        B20 = "20", 20;
        B22 = "22", 22;
        B24 = "24", 24;
        B28 = "28", 28;
        B32 = "32", 32;
        B36 = "36", 36;
        B44 = "44", 44;
        B64 = "64", 64;
        B65 = "65", 65;
        B256 = "256", 256;
        B4096 = "4096", 4096;
    }

    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(delegate, size("22..=32"))]
    struct Identifier(BitString);
    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(delegate, size("1..=160", extensible))]
    struct Address(BitString);
    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(delegate, size("16", extensible))]
    struct Algorithms(BitString);
    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(delegate, size("0..=7"))]
    struct FromNone(BitString);
    for length in 22..=32 {
        let octets = some_bits(length);
        let short = Bits::from_octets(&octets, length).unwrap();
        let long = oxirush_aper::BitString::new(octets.clone(), length).unwrap();
        same(
            &Identifier(their_bits(&octets, length)),
            |w| w.bits_sized(short, 22, 32).unwrap(),
            |r| r.bits_sized(22, 32).unwrap() == short,
        );
        same(
            &Identifier(their_bits(&octets, length)),
            |w| w.bit_string(&long, 22, 32).unwrap(),
            |r| r.bit_string(22, 32).unwrap() == long,
        );
    }
    for length in 1..=7 {
        let octets = some_bits(length);
        let short = Bits::from_octets(&octets, length).unwrap();
        same(
            &FromNone(their_bits(&octets, length)),
            |w| w.bits_sized(short, 0, 7).unwrap(),
            |r| r.bits_sized(0, 7).unwrap() == short,
        );
    }
    for length in [1, 8, 32, 128, 159, 160, 161, 200, 0] {
        let octets = some_bits(length);
        let long = oxirush_aper::BitString::new(octets.clone(), length).unwrap();
        same(
            &Address(their_bits(&octets, length)),
            |w| w.bit_string_extensible(&long, 1, 160).unwrap(),
            |r| r.bit_string_extensible(1, 160).unwrap() == long,
        );
        same(
            &Algorithms(their_bits(&octets, length)),
            |w| w.bit_string_extensible(&long, 16, 16).unwrap(),
            |r| r.bit_string_extensible(16, 16).unwrap() == long,
        );
    }
    let sixteen = oxirush_aper::BitString::from([0xe0, 0x00]);
    same(
        &Algorithms(their_bits(&[0xe0, 0x00], 16)),
        |w| w.bit_string_extensible(&sixteen, 16, 16).unwrap(),
        |r| r.bit_string_extensible(16, 16).unwrap() == sixteen,
    );
    // No size constraint: each form of the length, and fragments.
    for length in [
        0, 1, 7, 8, 9, 127, 128, 16383, 16384, 16385, 20001, 65536, 131072, 131073,
    ] {
        let octets = some_bits(length);
        let long = oxirush_aper::BitString::new(octets.clone(), length).unwrap();
        same(
            &their_bits(&octets, length),
            |w| w.bit_string(&long, 0, UNBOUNDED).unwrap(),
            |r| r.bit_string(0, UNBOUNDED).unwrap() == long,
        );
    }
}

#[test]
fn character_strings_and_object_identifiers() {
    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(delegate, size("1..=150", extensible))]
    struct Name(PrintableString);
    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(delegate, size("1..=150", extensible))]
    struct VisibleName(VisibleString);
    for text in [
        "a",
        "amf1",
        "An AMF (5GC), named: x=y?",
        &"x".repeat(150),
        &"y".repeat(151),
    ] {
        same(
            &Name(PrintableString::try_from(text).unwrap()),
            |w| w.printable_string_extensible(text, 1, 150).unwrap(),
            |r| r.printable_string_extensible(1, 150).unwrap() == text,
        );
        same(
            &VisibleName(VisibleString::try_from(text).unwrap()),
            |w| w.visible_string_extensible(text, 1, 150).unwrap(),
            |r| r.visible_string_extensible(1, 150).unwrap() == text,
        );
    }
    for text in [
        "",
        "http://example.org/~user?a=b&c=d",
        &"z".repeat(300),
        &"w".repeat(20000),
    ] {
        same(
            &VisibleString::try_from(text).unwrap(),
            |w| w.visible_string(text, 0, UNBOUNDED).unwrap(),
            |r| r.visible_string(0, UNBOUNDED).unwrap() == text,
        );
        same(
            &Ia5String::try_from(text).unwrap(),
            |w| w.ia5_string(text, 0, UNBOUNDED).unwrap(),
            |r| r.ia5_string(0, UNBOUNDED).unwrap() == text,
        );
        same(
            &Utf8String::from(text),
            |w| w.utf8_string(text),
            |r| r.utf8_string().unwrap() == text,
        );
    }
    for text in ["né", "日本", "😀 and more"] {
        same(
            &Utf8String::from(text),
            |w| w.utf8_string(text),
            |r| r.utf8_string().unwrap() == text,
        );
    }
    for arcs in [
        vec![0u32, 0],
        vec![1, 2, 840, 113549],
        vec![2, 100, 3],
        vec![0, 4, 0, 1989, 0],
        vec![2, 999, 4294967295],
    ] {
        let ours = ObjectIdentifier::new(arcs.clone()).unwrap();
        same(
            &rasn::types::ObjectIdentifier::new(arcs).unwrap(),
            |w| w.object_identifier(&ours),
            |r| r.object_identifier().unwrap() == ours,
        );
    }
}

#[test]
fn sequences_with_members_that_may_be_absent_and_with_additions() {
    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(automatic_tags)]
    #[non_exhaustive]
    struct Sample {
        #[rasn(value("0..=65535"))]
        id: u16,
        flag: Option<bool>,
        payload: Option<OctetString>,
        #[rasn(extension_addition, value("0..=7"))]
        later: Option<u8>,
        #[rasn(extension_addition)]
        last: Option<OctetString>,
    }
    for (flag, payload, later, last) in [
        (None, None, None, None),
        (Some(true), None, None, None),
        (None, Some(vec![1, 2, 3]), None, None),
        (Some(false), Some(vec![]), Some(5), None),
        (Some(true), Some(vec![9; 200]), None, Some(vec![7; 300])),
        (None, None, Some(0), Some(vec![])),
    ] {
        let theirs = Sample {
            id: 0x1234,
            flag,
            payload: payload.clone().map(OctetString::from),
            later,
            last: last.clone().map(OctetString::from),
        };
        let extended = later.is_some() || last.is_some();
        same(
            &theirs,
            |w| {
                w.bit(extended);
                w.bit(flag.is_some());
                w.bit(payload.is_some());
                w.unsigned(0x1234, 0, 65535).unwrap();
                if let Some(flag) = flag {
                    w.bit(flag);
                }
                if let Some(payload) = &payload {
                    w.octet_string(payload, 0, UNBOUNDED).unwrap();
                }
                if extended {
                    w.additions(&[later.is_some(), last.is_some()]).unwrap();
                    if let Some(later) = later {
                        let start = w.begin_open_type();
                        w.unsigned(later.into(), 0, 7).unwrap();
                        w.end_open_type(start);
                    }
                    if let Some(last) = &last {
                        let start = w.begin_open_type();
                        w.octet_string(last, 0, UNBOUNDED).unwrap();
                        w.end_open_type(start);
                    }
                }
            },
            |r| {
                let extended = r.bit().unwrap();
                let (has_flag, has_payload) = (r.bit().unwrap(), r.bit().unwrap());
                let id = r.unsigned(0, 65535).unwrap();
                let read_flag = has_flag.then(|| r.bit().unwrap());
                let read_payload =
                    has_payload.then(|| r.octet_string(0, UNBOUNDED).unwrap().into_vec());
                let (mut read_later, mut read_last) = (None, None);
                if extended {
                    let mut additions = r.additions().unwrap();
                    if additions.present() {
                        let value = r.open_type().unwrap();
                        read_later = Some(Reader::new(&value).unsigned(0, 7).unwrap() as u8);
                    }
                    if additions.present() {
                        let value = r.open_type().unwrap();
                        let mut inner = Reader::new(&value);
                        read_last = Some(inner.octet_string(0, UNBOUNDED).unwrap().into_vec());
                    }
                    additions.skip_rest(r).unwrap();
                }
                id == 0x1234
                    && read_flag == flag
                    && read_payload == payload
                    && read_later == later
                    && read_last == last
            },
        );
    }
}

#[test]
fn lists() {
    // A count below 64K is a constrained whole number, and the elements follow it
    // where it ends (X.691 §20.6). rasn 0.29 pads after the count when it reads, and
    // not when it writes: its writer is compared.
    #[derive(AsnType, Debug, Clone, Encode, PartialEq)]
    #[rasn(delegate, size("1..=8"))]
    struct Few(SequenceOf<bool>);
    #[derive(AsnType, Debug, Clone, Encode, PartialEq)]
    #[rasn(delegate, size("0..=65535"))]
    struct Many(SequenceOf<bool>);
    #[derive(AsnType, Debug, Clone, Encode, PartialEq)]
    struct AfterFew {
        flag: bool,
        value: Few,
    }
    for count in [1usize, 2, 7, 8] {
        let flags: Vec<bool> = (0..count).map(|at| at % 3 == 0).collect();
        let ours = written(|w| {
            w.bit(true);
            w.size(count, 1, 8).unwrap();
            flags.iter().for_each(|flag| w.bit(*flag));
        });
        let theirs = AfterFew {
            flag: true,
            value: Few(flags.clone()),
        };
        assert_eq!(rasn::aper::encode(&theirs).unwrap(), ours);
        let mut reader = Reader::new(&ours);
        assert!(reader.bit().unwrap());
        assert_eq!(reader.size(1, 8).unwrap(), count);
        let read: Vec<bool> = (0..count).map(|_| reader.bit().unwrap()).collect();
        assert_eq!(read, flags);
    }
    for count in [0usize, 1, 255, 256, 65535] {
        let flags: Vec<bool> = (0..count).map(|at| at % 3 == 0).collect();
        let ours = written(|w| {
            w.size(count, 0, 65535).unwrap();
            flags.iter().for_each(|flag| w.bit(*flag));
        });
        assert_eq!(rasn::aper::encode(&Many(flags.clone())).unwrap(), ours);
    }
    // No bound below 64K: an unconstrained count, in fragments from 16K elements.
    for count in [
        0usize, 1, 127, 128, 16383, 16384, 16385, 49153, 65536, 70000,
    ] {
        let flags: Vec<bool> = (0..count).map(|at| at % 3 == 0).collect();
        same(
            &flags,
            |w| {
                let mut left = &flags[..];
                loop {
                    let (count, more) = w.length(left.len());
                    left[..count].iter().for_each(|flag| w.bit(*flag));
                    left = &left[count..];
                    if !more {
                        break;
                    }
                }
            },
            |r| {
                let mut read = Vec::new();
                loop {
                    let (count, more) = r.length().unwrap();
                    read.extend((0..count).map(|_| r.bit().unwrap()));
                    if !more {
                        break;
                    }
                }
                read == flags
            },
        );
    }
}

#[test]
fn a_number_of_more_octets_than_64_bits_have_is_refused_whatever_it_holds() {
    #[derive(AsnType, Debug, Clone, Copy, Decode, Encode, PartialEq)]
    #[rasn(enumerated)]
    #[non_exhaustive]
    enum Extended {
        A = 0,
        B = 1,
        C = 2,
        #[rasn(extension_addition)]
        D = 3,
        #[rasn(extension_addition)]
        E = 4,
    }
    // The index of an addition above 63 is a semi-constrained number (X.691 §11.6.2),
    // which a writer gives its least octets (§11.3.6). Both readers take zero octets
    // before the number up to eight octets, and refuse nine, where rasn holds the
    // index in 64 bits as the crate does.
    for (octets, index) in [
        (&[0x80][..], Some(3)),
        (&[0xc0, 0x01, 0x00], Some(3)),
        (&[0xc0, 0x02, 0x00, 0x01], Some(4)),
        (&[0xc0, 0x80, 0x01, 0x00], Some(3)),
        (&[0xc0, 0x08, 0, 0, 0, 0, 0, 0, 0, 1], Some(4)),
        (&[0xc0, 0x09, 0, 0, 0, 0, 0, 0, 0, 0, 0], None),
        (&[0xc0, 0x0b, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], None),
    ] {
        let theirs = rasn::aper::decode::<Extended>(octets).ok();
        assert_eq!(theirs.map(|value| value as u64), index, "{octets:02x?}");
        let ours = Reader::new(octets).enumerated(3, 2, true).ok();
        assert_eq!(ours, index, "{octets:02x?}");
    }
}

#[test]
fn what_one_refuses_the_other_refuses() {
    // Octets cut short and with each bit changed: a field is read by both, or by
    // neither.
    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(automatic_tags)]
    #[non_exhaustive]
    struct Sample {
        #[rasn(value("0..=100"))]
        small: u8,
        #[rasn(value("0..=4000000000000", extensible))]
        rate: Integer,
        #[rasn(size("1..=150", extensible))]
        name: PrintableString,
        payload: Option<OctetString>,
        #[rasn(size("3"))]
        fixed: FixedOctetString<3>,
    }
    type Members = (u8, u64, String, Option<Vec<u8>>, [u8; 3]);
    let ours = |octets: &[u8]| -> Option<Members> {
        let mut r = Reader::new(octets);
        let extended = r.bit().ok()?;
        let has_payload = r.bit().ok()?;
        let small = r.unsigned(0, 100).ok()? as u8;
        let rate = r.unsigned_extensible(0, 4000000000000).ok()?;
        let name = r.printable_string_extensible(1, 150).ok()?.into_owned();
        let payload = match has_payload {
            true => Some(r.octet_string(0, UNBOUNDED).ok()?.into_vec()),
            false => None,
        };
        let fixed = r.octet_array::<3>().ok()?;
        if extended {
            r.skip_additions().ok()?;
        }
        r.finish().ok()?;
        Some((small, rate, name, payload, fixed))
    };
    let sample = Sample {
        small: 42,
        rate: Integer::from(1_000_000_000u64),
        name: PrintableString::try_from("amf1").unwrap(),
        payload: Some(vec![1, 2, 3, 4, 5].into()),
        fixed: [7, 8, 9].into(),
    };
    let written = rasn::aper::encode(&sample).unwrap();
    let (mut both, mut neither) = (0, 0);
    let mut compare = |octets: &[u8]| {
        // A complete encoding: the value, and no octet after it.
        let theirs = rasn::aper::decode_with_remainder::<Sample>(octets).ok();
        let theirs = theirs.and_then(|(sample, rest)| rest.is_empty().then_some(sample));
        // The whole value, and no octet after it. rasn does not skip the additions of
        // a type that has none, which the bindings do themselves: an encoding with
        // its extension bit set is left out.
        if octets.first().is_some_and(|first| first & 0x80 != 0) {
            return;
        }
        let theirs = theirs.map(|sample| {
            let rate = u64::try_from(&sample.rate).ok();
            let payload = sample.payload.map(|payload| payload.to_vec());
            let name = String::from_utf8(sample.name.as_bytes().to_vec()).unwrap();
            let fixed: [u8; 3] = sample.fixed.as_ref().try_into().unwrap();
            (sample.small, rate, name, payload, fixed)
        });
        match (ours(octets), theirs) {
            (Some(ours), Some((small, Some(rate), name, payload, fixed))) => {
                assert_eq!(ours, (small, rate, name, payload, fixed), "{octets:02x?}");
                both += 1;
            }
            // A rate that the bindings do not hold: negative, or of more than 64 bits.
            (None, Some((_, None, ..))) => neither += 1,
            (None, None) => neither += 1,
            (ours, theirs) => panic!("{octets:02x?}: {ours:?} here, {theirs:?} by rasn"),
        }
    };
    compare(&written);
    for length in 0..written.len() {
        compare(&written[..length]);
    }
    for at in 0..written.len() {
        for bit in 0..8 {
            let mut changed = written.clone();
            changed[at] ^= 0x80 >> bit;
            compare(&changed);
        }
    }
    assert!(
        both > 50 && neither > 20,
        "{both} read by both, {neither} by neither"
    );
}
