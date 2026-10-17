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
//! ```
//! use oxirush_aper::{AsnType, Decode, Encode, Error, Octets, Reader, Writer};
//!
//! // Sample ::= SEQUENCE { id INTEGER (0..65535), payload OCTET STRING OPTIONAL, ... }
//! #[derive(Debug, PartialEq)]
//! struct Sample<'a> {
//!     id: u16,
//!     payload: Option<Octets<'a>>,
//! }
//!
//! impl AsnType for Sample<'_> {
//!     const NAME: &'static str = "Sample";
//! }
//!
//! impl<'a> Decode<'a> for Sample<'a> {
//!     fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
//!         // The extension bit, then a bit for the member that may be absent
//!         let extended = reader.bit()?;
//!         let has_payload = reader.bit()?;
//!         let id = reader.unsigned(0, 65535)? as u16;
//!         let payload = match has_payload {
//!             true => Some(reader.octet_string(0, oxirush_aper::UNBOUNDED)?),
//!             false => None,
//!         };
//!         if extended {
//!             reader.skip_additions()?;
//!         }
//!         Ok(Self { id, payload })
//!     }
//! }
//!
//! impl Encode for Sample<'_> {
//!     fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
//!         writer.bit(false);
//!         writer.bit(self.payload.is_some());
//!         writer.unsigned(self.id.into(), 0, 65535)?;
//!         if let Some(payload) = &self.payload {
//!             writer.octet_string(payload, 0, oxirush_aper::UNBOUNDED)?;
//!         }
//!         Ok(())
//!     }
//! }
//!
//! // The payload of what is decoded is a slice of the octets
//! let octets = [0x40, 0x12, 0x34, 0x02, 0xca, 0xfe];
//! let sample: Sample = oxirush_aper::decode(&octets)?;
//! assert_eq!(sample.id, 0x1234);
//! assert!(sample.payload.as_ref().is_some_and(|payload| payload.is_borrowed()));
//! assert_eq!(oxirush_aper::encode(&sample)?, octets);
//! # Ok::<(), Error>(())
//! ```
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
pub const UNBOUNDED: usize = usize::MAX;

/// A type of an ASN.1 module.
pub trait AsnType {
    /// The name that ASN.1 gives the type, as `AMF-UE-NGAP-ID`: an error names the
    /// type by it.
    const NAME: &'static str;
}

/// A type whose values are read in aligned PER. `'a` is how long the octets live that a
/// value borrows from.
pub trait Decode<'a>: AsnType + Sized {
    /// The value at the reader, which is moved after it.
    fn decode(reader: &mut Reader<'a>) -> Result<Self, Error>;
}

/// A type whose values are written in aligned PER.
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
