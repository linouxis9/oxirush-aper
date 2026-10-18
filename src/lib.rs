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

//! Aligned PER (ITU-T X.691, the ALIGNED variant of BASIC-PER) for generated bindings
//! of 3GPP-style ASN.1: a reader, a writer, and the types of the values.
//!
//! ```rust
//! use oxirush_aper::{Reader, Writer};
//!
//! fn main() -> Result<(), oxirush_aper::Error> {
//!     let mut octets = Vec::new();
//!     let mut writer = Writer::new(&mut octets);
//!     // INTEGER (0..7): three bits
//!     writer.unsigned(5, 0, 7)?;
//!     // INTEGER (0..255, ...): the extension bit, then one octet, aligned
//!     writer.unsigned_extensible(200, 0, 255)?;
//!     // ENUMERATED with 6 root values and an extension marker: the fourth
//!     writer.index(3, 6, true);
//!     // SIZE (1..256) of a SEQUENCE OF: its count, then its items
//!     writer.size(2, 1, 256)?;
//!     writer.unsigned(1, 0, 1)?;
//!     writer.unsigned(0, 0, 1)?;
//!     println!("{octets:02x?}");
//!
//!     let mut reader = Reader::new(&octets);
//!     println!("{}", reader.unsigned(0, 7)?);
//!     println!("{}", reader.unsigned_extensible(0, 255)?);
//!     println!("{}", reader.enumerated(6, 0, true)?);
//!     println!("{} items: {} {}", reader.size(1, 256)?, reader.bit()?, reader.bit()?);
//!
//!     println!("{}", Writer::new(&mut Vec::new()).unsigned(8, 0, 7).unwrap_err());
//!     Ok(())
//! }
//! ```
//!
//! ```text
//! [a0, c8, 30, 01, 80]
//! 5
//! 200
//! 3
//! 2 items: true false
//! the value does not encode: 8 is outside its constraint (0..7)
//! ```
//!
//! # Tasks
//!
//! The [README](https://github.com/linouxis9/oxirush-aper#readme) has an example of
//! each, with what it prints.
//!
//! | Task | Items |
//! |------|-------|
//! | Write the codec of a type | [`AsnType`], [`Decode`], [`Encode`], [`decode`], [`encode`], [`encode_into`] |
//! | Read and write the fields | [`Reader`], [`Writer`], [`UNBOUNDED`] |
//! | Bit strings | [`Bits`], [`BitString`] |
//! | An open type | [`Reader::open_type`], [`Writer::begin_open_type`], [`Writer::end_open_type`], [`Octets`] |
//! | Extension additions | [`Reader::additions`], [`Additions`], [`Writer::additions`], [`Reader::skip_additions`] |
//! | What the reader accepts | [`decode_with_faults`], [`decode_strict`], [`Fault`], [`Reader::noting`] |
//! | Check a PDU against the object set of its message | [`check`] |
//! | The macros and the readers of a protocol crate | [`protocol`] |
//!
//! A [`Reader`] and a [`Writer`] have a method for each field of X.691 that the two
//! protocols of the family use. They work on octets where the encoding is aligned and
//! on bits where it is not, the bounds of a constraint are their arguments, and a
//! reader allocates nothing but the lists and the strings in fragments of what it
//! reads.
//!
//! The values are native numbers, arrays for an OCTET STRING of one size, [`Octets`]
//! for the others and for an open type, [`Bits`] for a BIT STRING of 64 bits at most,
//! [`BitString`] for a longer one, `Cow<str>` for a character string and
//! [`ObjectIdentifier`].
//!
//! # The OxiRush family
//!
//! - [oxirush-utils](https://docs.rs/oxirush-utils): the 3GPP identities, and the JSON tree with its paths that the others share.
//! - [oxirush-aper](https://docs.rs/oxirush-aper): aligned PER, the runtime of the NGAP and S1AP codecs.
//! - [oxirush-nas](https://docs.rs/oxirush-nas): the NAS messages of 5GS and EPS.
//! - [oxirush-security](https://docs.rs/oxirush-security): NAS security, AKA and the derivation of keys.
//! - [oxirush-ngap](https://docs.rs/oxirush-ngap): NGAP, between a gNB and an AMF.
//! - [oxirush-s1ap](https://docs.rs/oxirush-s1ap): S1AP, between an eNB and an MME.
//! - [oxirush-gtp-u](https://docs.rs/oxirush-gtp-u): GTP-U tunnels, with a test UPF.
//! - [oxirush-sctp](https://docs.rs/oxirush-sctp): SCTP sockets, which carry NGAP and S1AP.

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod bits;
pub mod check;
mod error;
mod faults;
// The plumbing of the generated crates, hidden from the documentation: it changes with
// them, and oxirush-ngap and oxirush-s1ap depend on the one version of this crate that is
// released with theirs (`=0.1.0`), so that no other version of it reaches their builds.
#[cfg(feature = "inspect")]
#[doc(hidden)]
pub mod inspect;
#[cfg(feature = "inspect")]
pub mod json;
mod octets;
mod oid;
pub mod protocol;
mod reader;
mod writer;

use alloc::vec::Vec;

/// The examples of the README, which are compiled and run with the tests of the
/// documentation.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct Readme;

pub use bits::{BitString, Bits};
pub use error::{Criticality, Error, ErrorIe, ErrorKind};
pub use faults::{Fault, Faults, Noted};
pub use octets::Octets;
pub use oid::ObjectIdentifier;
pub use reader::{Additions, Reader};
pub use writer::Writer;

/// The upper bound of a size constraint that has none, as `SIZE (1..MAX)`.
///
/// ```
/// use oxirush_aper::{Reader, UNBOUNDED, Writer};
///
/// // OCTET STRING (SIZE (1..MAX)): the length of the octets, then the octets
/// let mut octets = Vec::new();
/// Writer::new(&mut octets).octet_string(&[0xca, 0xfe], 1, UNBOUNDED)?;
/// assert_eq!(octets, [0x02, 0xca, 0xfe]);
/// assert_eq!(Reader::new(&octets).octet_string(1, UNBOUNDED)?, [0xca, 0xfe]);
/// # Ok::<(), oxirush_aper::Error>(())
/// ```
pub const UNBOUNDED: usize = usize::MAX;

/// A type of an ASN.1 module. The example of [`Decode`] implements it.
pub trait AsnType {
    /// The name that ASN.1 gives the type, as `AMF-UE-NGAP-ID`: an error names the
    /// type by it.
    const NAME: &'static str;
}

/// A type whose values are read in aligned PER. `'a` is how long the octets live that a
/// value borrows from.
///
/// A type of a module implements [`AsnType`], `Decode` and [`Encode`]:
///
/// ```
/// use oxirush_aper::{AsnType, Decode, Encode, Error, Reader, Writer};
///
/// // AMF-UE-NGAP-ID ::= INTEGER (0..1099511627775)
/// #[derive(Debug, PartialEq)]
/// struct AmfUeNgapId(u64);
///
/// impl AsnType for AmfUeNgapId {
///     const NAME: &'static str = "AMF-UE-NGAP-ID";
/// }
///
/// impl Decode<'_> for AmfUeNgapId {
///     fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
///         Ok(Self(reader.unsigned(0, 1_099_511_627_775)?))
///     }
/// }
///
/// impl Encode for AmfUeNgapId {
///     fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
///         writer.unsigned(self.0, 0, 1_099_511_627_775)
///     }
/// }
///
/// // The length of the number in three bits, then its octet, aligned
/// let octets = oxirush_aper::encode(&AmfUeNgapId(42))?;
/// assert_eq!(octets, [0x00, 0x2a]);
/// assert_eq!(oxirush_aper::decode::<AmfUeNgapId>(&octets)?, AmfUeNgapId(42));
/// # Ok::<(), Error>(())
/// ```
pub trait Decode<'a>: AsnType + Sized {
    /// The value at the reader, which is moved after it.
    fn decode(reader: &mut Reader<'a>) -> Result<Self, Error>;
}

/// A type whose values are written in aligned PER. The example of [`Decode`]
/// implements it.
pub trait Encode: AsnType {
    /// Write the value.
    fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error>;
}

/// NULL: no bit.
impl AsnType for () {
    const NAME: &'static str = "NULL";
}

impl<'a> Decode<'a> for () {
    fn decode(_: &mut Reader<'a>) -> Result<Self, Error> {
        Ok(())
    }
}

impl Encode for () {
    fn encode(&self, _: &mut Writer<'_>) -> Result<(), Error> {
        Ok(())
    }
}

/// BOOLEAN: one bit.
impl AsnType for bool {
    const NAME: &'static str = "BOOLEAN";
}

impl<'a> Decode<'a> for bool {
    fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
        reader.bit()
    }
}

impl Encode for bool {
    fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
        writer.bit(*self);
        Ok(())
    }
}

/// The value of the type `T` that `octets` are the complete encoding of (X.691 §11.1):
/// one value, whole, and no octet after it. What the value holds of octets and of
/// strings is borrowed from `octets`.
///
/// ```
/// use oxirush_aper::Octets;
///
/// // An OCTET STRING without a constraint, whose octets stay in the input
/// let input = [0x02, 0xca, 0xfe];
/// let octets: Octets = oxirush_aper::decode(&input)?;
/// assert_eq!(octets.as_borrowed(), Some(&input[1..]));
///
/// // Octets that end early, and an octet after the value
/// assert!(oxirush_aper::decode::<Octets>(&input[..2]).is_err());
/// assert!(oxirush_aper::decode::<Octets>(&[0x01, 0xca, 0xfe]).is_err());
/// # Ok::<(), oxirush_aper::Error>(())
/// ```
///
/// # Errors
///
/// An error of the kind `Decode`, which names the type, says why, and gives the field
/// where the decoding stopped.
pub fn decode<'a, T: Decode<'a>>(octets: &'a [u8]) -> Result<T, Error> {
    if octets.is_empty() {
        return Err(Error::decode("there are no octets").decoding(T::NAME));
    }
    let mut reader = Reader::new(octets);
    match T::decode(&mut reader) {
        Ok(value) => match reader.finish() {
            Ok(()) => Ok(value),
            Err(error) => Err(error.decoding(T::NAME)),
        },
        Err(error) => Err(error.decoding(T::NAME)),
    }
}

/// The value of the type `T` that `octets` are the complete encoding of, as [`decode`]
/// reads it, and what of the encoding is in another form than the one that X.691 gives
/// it: the [`Faults`] of the reader, with the padding of the end among them. They are
/// none for octets that are written by the clauses.
///
/// A value in such a form is whole, and the encoder of a peer may write it: a receiver
/// reads it, a tool that tests the peer says that its encoder is not canonical, and
/// [`decode_strict`] refuses it. What a value holds as the octets of an open type is not
/// read here, and has its faults where it is decoded.
///
/// ```
/// use oxirush_aper::{Fault, Octets, decode_with_faults};
///
/// // An OCTET STRING of one octet, with its length in two octets
/// let (octets, faults) = decode_with_faults::<Octets>(&[0x80, 0x01, 0xaa])?;
/// assert_eq!(octets, [0xaa]);
/// assert_eq!(faults[0].fault, Fault::LengthInTwoOctets);
/// assert_eq!(faults[0].to_string(), "at bit 0, a length of 127 or less is in two octets (X.691 §11.9.3.6)");
/// # Ok::<(), oxirush_aper::Error>(())
/// ```
///
/// # Errors
///
/// Those of [`decode`].
pub fn decode_with_faults<'a, T: Decode<'a>>(octets: &'a [u8]) -> Result<(T, Faults), Error> {
    if octets.is_empty() {
        return Err(Error::decode("there are no octets").decoding(T::NAME));
    }
    let mut reader = Reader::noting(octets);
    let value = match T::decode(&mut reader) {
        Ok(value) => value,
        Err(error) => return Err(error.decoding(T::NAME)),
    };
    // The bits after the value, up to the end of its last octet, are padding.
    reader.align();
    match reader.finish() {
        Ok(()) => Ok((value, *reader.faults())),
        Err(error) => Err(error.decoding(T::NAME)),
    }
}

/// The value of the type `T` that `octets` are the complete encoding of, when they are
/// the one encoding that X.691 gives the value: what [`decode`] reads in another form,
/// a [`Fault`], is an error here. The counterpart of [`encode_strict`].
///
/// ```
/// use oxirush_aper::{Octets, decode, decode_strict};
///
/// // An OCTET STRING of one octet, whose length is in two octets
/// let input = [0x80, 0x01, 0xaa];
/// assert!(decode::<Octets>(&input).is_ok());
/// assert!(decode_strict::<Octets>(&input).is_err());
/// ```
///
/// # Errors
///
/// Those of [`decode`], and an error of the kind `Decode` that says the first fault,
/// its clause and the bit at which it is.
pub fn decode_strict<'a, T: Decode<'a>>(octets: &'a [u8]) -> Result<T, Error> {
    let (value, faults) = decode_with_faults(octets)?;
    match faults.first() {
        None => Ok(value),
        Some(noted) => {
            let why = alloc::format!(
                "{noted}: the value is read, and a sender by the clause writes it otherwise"
            );
            Err(Error::decode(why).decoding(T::NAME))
        }
    }
}

/// The complete encoding of `value` (X.691 §11.1): its fields, zero bits up to the end
/// of the last octet, and one zero octet for a value that has no field.
///
/// # Errors
///
/// An error of the kind `Encode`, which names the type and says why, as for a number
/// outside its constraint.
///
/// ```
/// use oxirush_aper::Octets;
///
/// // An OCTET STRING without a constraint: its length, then its octets
/// let octets = Octets::from(&[0xca, 0xfe][..]);
/// assert_eq!(oxirush_aper::encode(&octets)?, [0x02, 0xca, 0xfe]);
/// # Ok::<(), oxirush_aper::Error>(())
/// ```
pub fn encode<T: Encode>(value: &T) -> Result<Vec<u8>, Error> {
    let mut octets = Vec::new();
    encode_into(value, &mut octets)?;
    Ok(octets)
}

/// The complete encoding of `value`, as [`encode`] gives it, for a value that is one of
/// this version of its type: a number or a size outside the root of an extensible
/// constraint, which `encode` writes as an extension, is an error.
///
/// An extensible constraint, as `INTEGER (0..255, ...)` or `SIZE (1..150, ...)`, has a
/// receiver read what a later version of the type allows, so `encode` writes any value.
/// The sender of this version sends none of them: a name of no character is no AMF
/// name, though it encodes.
///
/// # Errors
///
/// An error of the kind `Encode`, which names the type, says why, and gives the field
/// that does not encode.
pub fn encode_strict<T: Encode>(value: &T) -> Result<Vec<u8>, Error> {
    let mut octets = Vec::new();
    match value.encode(&mut Writer::strict(&mut octets)) {
        Ok(()) => {
            if octets.is_empty() {
                octets.push(0);
            }
            Ok(octets)
        }
        Err(error) => Err(error.encoding(T::NAME)),
    }
}

/// Append the complete encoding of `value` to `octets`. After an error, `octets` are
/// as they were.
///
/// ```
/// // A BOOLEAN, one bit in an octet, then a NULL, no bit, which is one zero octet
/// let mut octets = vec![0xff];
/// oxirush_aper::encode_into(&true, &mut octets)?;
/// oxirush_aper::encode_into(&(), &mut octets)?;
/// assert_eq!(octets, [0xff, 0x80, 0x00]);
/// # Ok::<(), oxirush_aper::Error>(())
/// ```
pub fn encode_into<T: Encode>(value: &T, octets: &mut Vec<u8>) -> Result<(), Error> {
    let start = octets.len();
    match value.encode(&mut Writer::new(octets)) {
        Ok(()) => {
            if octets.len() == start {
                octets.push(0);
            }
            Ok(())
        }
        Err(error) => {
            octets.truncate(start);
            Err(error.encoding(T::NAME))
        }
    }
}
