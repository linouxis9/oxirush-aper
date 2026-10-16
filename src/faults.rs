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

//! What a reader takes although ITU-T X.691 gives it another form.

use core::fmt;

/// An encoding that X.691 gives one form for, in another: the value is whole, and a
/// sender by the clause writes it otherwise. A reader takes it, as the encoder of a
/// peer may write it, and notes it: see [`Reader::faults`](crate::Reader::faults).
///
/// What announces something that is not there is no fault, and does not decode: an
/// extension bit without an addition, or a length that the octets do not have.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Fault {
    /// A length of 127 or less in the two octets of a length from 128 to 16383: it is
    /// one octet (§11.9.3.6).
    LengthInTwoOctets,
    /// A fragment of less than four times 16K items before another fragment: a
    /// fragment is the largest that the items allow (§11.9.3.8.1).
    SmallFragment,
    /// A normally small number of 63 or less in the form of 64 and more: it is six
    /// bits after a zero bit (§11.6.1).
    SmallNumberAsLarge,
    /// A whole number in more octets than it needs: octets that only extend it before
    /// the least that hold it (§11.3.6, §11.4.6).
    LongNumber,
    /// Padding bits that are not zero, before an octet-aligned field or at the end of
    /// a complete encoding (§11.1.4).
    Padding,
    /// An arc of an OBJECT IDENTIFIER whose first octet holds nothing: the octets of
    /// an arc are the fewest that hold it, so the first is not 0x80 (ITU-T X.690
    /// §8.19.2, which X.691 §24 takes).
    LongArc,
}

impl Fault {
    /// The clause that gives the encoding its one form, as `11.9.3.6`, of the
    /// recommendation that [`standard`](Self::standard) names.
    pub const fn clause(self) -> &'static str {
        match self {
            Self::LengthInTwoOctets => "11.9.3.6",
            Self::SmallFragment => "11.9.3.8.1",
            Self::SmallNumberAsLarge => "11.6.1",
            Self::LongNumber => "11.3.6, 11.4.6",
            Self::Padding => "11.1.4",
            Self::LongArc => "8.19.2",
        }
    }

    /// The recommendation of ITU-T (02/2021) that has the clause: `X.691`, and
    /// `X.690` for the arcs of an object identifier, whose contents octets are those
    /// of the Basic Encoding Rules.
    pub const fn standard(self) -> &'static str {
        match self {
            Self::LongArc => "X.690",
            _ => "X.691",
        }
    }
}

impl fmt::Display for Fault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match self {
            Self::LengthInTwoOctets => "a length of 127 or less is in two octets",
            Self::SmallFragment => "a fragment before another is not the largest",
            Self::SmallNumberAsLarge => "a number of 63 or less is in the form of a larger one",
            Self::LongNumber => "a whole number is in more octets than it needs",
            Self::Padding => "padding bits are not zero",
            Self::LongArc => {
                "an arc of an object identifier starts with an octet that holds nothing"
            }
        };
        write!(formatter, "{what} ({} §{})", self.standard(), self.clause())
    }
}

/// A fault that a reader noted, and where it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Noted {
    /// What is in another form than the one of its clause.
    pub fault: Fault,
    /// The bit of the octets of the reader at which it is: where a length, a number or
    /// a fragment starts, and the first bit of a padding.
    pub bit: usize,
}

impl fmt::Display for Noted {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "at bit {}, {}", self.bit, self.fault)
    }
}

/// The faults of one reading, in the order of the octets. It is a list of [`Noted`],
/// which it derefs to: the first four, held in place without an allocation, and
/// [`count`](Self::count) says how many the reader met. It is empty for octets that
/// are written by the clauses.
///
/// ```
/// use oxirush_aper::{Fault, decode_with_faults};
///
/// // A BOOLEAN, and seven bits of padding of which the last is one
/// let (value, faults) = decode_with_faults::<bool>(&[0x81])?;
/// assert!(value);
/// assert_eq!(faults.len(), 1);
/// assert_eq!((faults[0].fault, faults[0].bit), (Fault::Padding, 1));
/// # Ok::<(), oxirush_aper::Error>(())
/// ```
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Faults {
    /// The first faults, then what fills the list to its size.
    first: [Noted; Self::KEPT],
    count: usize,
}

impl Faults {
    /// How many faults the list holds: the first of a reading.
    const KEPT: usize = 4;

    /// No fault.
    pub(crate) const fn new() -> Self {
        // The first of the faults, so that a list of none is zeros.
        let unused = Noted {
            fault: Fault::LengthInTwoOctets,
            bit: 0,
        };
        Self {
            first: [unused; Self::KEPT],
            count: 0,
        }
    }

    /// One more fault.
    pub(crate) fn push(&mut self, fault: Fault, bit: usize) {
        if let Some(place) = self.first.get_mut(self.count) {
            *place = Noted { fault, bit };
        }
        self.count = self.count.saturating_add(1);
    }

    /// How many faults the reader met: as many as the list has, or more than it keeps.
    pub const fn count(&self) -> usize {
        self.count
    }

    /// The faults that are kept: the first four.
    pub fn as_slice(&self) -> &[Noted] {
        &self.first[..self.count.min(Self::KEPT)]
    }
}

impl Default for Faults {
    fn default() -> Self {
        Self::new()
    }
}

impl core::ops::Deref for Faults {
    type Target = [Noted];

    fn deref(&self) -> &[Noted] {
        self.as_slice()
    }
}

impl fmt::Debug for Faults {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_slice().fmt(formatter)
    }
}
