//! One type, with the codec of the crate and with that of rasn.
//!
//! ```text
//! Kind ::= ENUMERATED { a, b, c, ..., d, e }
//!
//! Either ::= CHOICE {
//!     number  INTEGER (0..255),
//!     octets  OCTET STRING,
//!     nothing NULL,
//!     ...
//! }
//!
//! Sample ::= SEQUENCE {
//!     small   INTEGER (0..100),
//!     word    INTEGER (0..65535),
//!     signed  INTEGER (-8..7),
//!     rate    INTEGER (0..4000000000000, ...),
//!     kind    Kind,
//!     either  Either,
//!     name    PrintableString (SIZE (1..150, ...)),
//!     set     BIT STRING (SIZE (10)),
//!     payload OCTET STRING OPTIONAL,
//!     fixed   OCTET STRING (SIZE (3)),
//!     short   OCTET STRING (SIZE (1..4)),
//!     open    ANY
//! }
//! ```
//!
//! The members are of the constructs that rasn 0.29 reads and writes as ITU-T X.691
//! has them wherever they start: `tests/rasn.rs` of the crate says which are not, each
//! with its clause, and compares the writers for those.

use oxirush_aper::{AsnType, Bits, Decode, Encode, Error, Octets, Reader, UNBOUNDED, Writer};

/// The type with the codec of rasn.
mod theirs {
    use rasn::prelude::*;

    #[derive(AsnType, Debug, Clone, Copy, Decode, Encode, PartialEq)]
    #[rasn(enumerated)]
    #[non_exhaustive]
    pub enum Kind {
        A = 0,
        B = 1,
        C = 2,
        #[rasn(extension_addition)]
        D = 3,
        #[rasn(extension_addition)]
        E = 4,
    }

    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(choice, automatic_tags)]
    #[non_exhaustive]
    pub enum Either {
        #[rasn(value("0..=255"))]
        Number(u8),
        Octets(OctetString),
        Nothing(()),
    }

    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(automatic_tags)]
    pub struct Sample {
        #[rasn(value("0..=100"))]
        pub small: u8,
        #[rasn(value("0..=65535"))]
        pub word: u16,
        #[rasn(value("-8..=7"))]
        pub signed: i8,
        #[rasn(value("0..=4000000000000", extensible))]
        pub rate: Integer,
        pub kind: Kind,
        pub either: Either,
        #[rasn(size("1..=150", extensible))]
        pub name: PrintableString,
        #[rasn(size("10"))]
        pub set: BitString,
        pub payload: Option<OctetString>,
        #[rasn(size("3"))]
        pub fixed: FixedOctetString<3>,
        #[rasn(size("1..=4"))]
        pub short: OctetString,
        pub open: Any,
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Either<'a> {
    Number(u8),
    Octets(Octets<'a>),
    Nothing,
}

#[derive(Debug, Clone, PartialEq)]
struct Sample<'a> {
    small: u8,
    word: u16,
    signed: i8,
    rate: u64,
    kind: u64,
    either: Either<'a>,
    name: std::borrow::Cow<'a, str>,
    set: Bits,
    payload: Option<Octets<'a>>,
    fixed: [u8; 3],
    short: Octets<'a>,
    open: Octets<'a>,
}

impl AsnType for Sample<'_> {
    const NAME: &'static str = "Sample";
}

impl<'a> Decode<'a> for Sample<'a> {
    fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
        let has_payload = reader.bit()?;
        let small = reader.unsigned(0, 100)? as u8;
        let word = reader.unsigned(0, 65535)? as u16;
        let signed = reader.signed(-8, 7)? as i8;
        let rate = reader.unsigned_extensible(0, 4_000_000_000_000)?;
        let kind = reader.enumerated(3, 2, true)?;
        let either = match reader.choice(3, 0, true)? {
            0 => Either::Number(reader.unsigned(0, 255)? as u8),
            1 => Either::Octets(reader.octet_string(0, UNBOUNDED)?),
            _ => Either::Nothing,
        };
        let name = reader.printable_string_extensible(1, 150)?;
        let set = reader.bits_sized(10, 10)?;
        let payload = match has_payload {
            true => Some(reader.octet_string(0, UNBOUNDED)?),
            false => None,
        };
        let fixed = reader.octet_array()?;
        let short = reader.octet_string(1, 4)?;
        let open = reader.open_type()?;
        Ok(Self {
            small,
            word,
            signed,
            rate,
            kind,
            either,
            name,
            set,
            payload,
            fixed,
            short,
            open,
        })
    }
}

impl Encode for Sample<'_> {
    fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
        writer.bit(self.payload.is_some());
        writer.unsigned(self.small.into(), 0, 100)?;
        writer.unsigned(self.word.into(), 0, 65535)?;
        writer.signed(self.signed.into(), -8, 7)?;
        writer.unsigned_extensible(self.rate, 0, 4_000_000_000_000)?;
        writer.index(self.kind, 3, true);
        match &self.either {
            Either::Number(number) => {
                writer.index(0, 3, true);
                writer.unsigned((*number).into(), 0, 255)?;
            }
            Either::Octets(octets) => {
                writer.index(1, 3, true);
                writer.octet_string(octets, 0, UNBOUNDED)?;
            }
            Either::Nothing => writer.index(2, 3, true),
        }
        writer.printable_string_extensible(&self.name, 1, 150)?;
        writer.bits_sized(self.set, 10, 10)?;
        if let Some(payload) = &self.payload {
            writer.octet_string(payload, 0, UNBOUNDED)?;
        }
        writer.octet_array(&self.fixed);
        writer.octet_string(&self.short, 1, 4)?;
        writer.open_type(&self.open);
        Ok(())
    }
}

/// The value of rasn as the one of the crate; `None` for a rate that 64 bits do not
/// hold.
fn as_ours(theirs: &theirs::Sample) -> Option<Sample<'_>> {
    let set = theirs.set.as_raw_slice();
    Some(Sample {
        small: theirs.small,
        word: theirs.word,
        signed: theirs.signed,
        rate: u64::try_from(&theirs.rate).ok()?,
        kind: theirs.kind as u64,
        either: match &theirs.either {
            theirs::Either::Number(number) => Either::Number(*number),
            theirs::Either::Octets(octets) => Either::Octets(Octets::borrowed(octets)),
            theirs::Either::Nothing(()) => Either::Nothing,
        },
        name: String::from_utf8_lossy(theirs.name.as_bytes()),
        set: Bits::from_octets(set, theirs.set.len()).ok()?,
        payload: theirs.payload.as_deref().map(Octets::borrowed),
        fixed: theirs.fixed.as_ref().try_into().ok()?,
        short: Octets::borrowed(&theirs.short),
        open: Octets::borrowed(theirs.open.as_bytes()),
    })
}

/// The octets as a `Sample`, by the crate and by rasn: both read the same value, which
/// both write as the same octets, or neither reads one.
pub fn value(octets: &[u8]) {
    let ours = oxirush_aper::decode::<Sample>(octets);
    // A complete encoding: the value, and no octet after it.
    let theirs = rasn::aper::decode_with_remainder::<theirs::Sample>(octets).ok();
    let theirs = theirs.and_then(|(sample, rest)| rest.is_empty().then_some(sample));
    match (&ours, &theirs) {
        (Ok(ours), Some(theirs)) => {
            assert_eq!(Some(ours), as_ours(theirs).as_ref(), "{octets:02x?}");
            let written = oxirush_aper::encode(ours).unwrap();
            assert_eq!(rasn::aper::encode(theirs).unwrap(), written, "{ours:?}");
            assert_eq!(oxirush_aper::decode::<Sample>(&written).as_ref(), Ok(ours));
        }
        (Err(_), None) => {}
        // A rate that the crate does not hold, which has more octets than a number of
        // 64 bits or is below zero: rasn holds a number of any size.
        (Err(error), Some(_)) if error.to_string().contains("does not fit in 64 bits") => {}
        (ours, theirs) => panic!("{octets:02x?}: {ours:?} here, {theirs:?} by rasn"),
    }
}
