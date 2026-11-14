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

//! A perfect hash of the names of a table, which the generator writes beside the table:
//! a name is found in one probe, and then compared with the name in the slot that the
//! probe gives. The generator includes this file, so that the slots that it computes
//! are those that a lookup reads.
//!
//! A name hashes to one of the buckets, and with the displacement of its bucket to one
//! of the slots: the generator chooses for each bucket a displacement that puts each of
//! its names in a slot that no other name has.

/// The names of a table, their slots, and the displacement of each bucket.
#[derive(Debug)]
pub struct Hashed {
    /// The names, in the order of the table.
    pub keys: &'static [&'static str],
    /// The displacement of each bucket.
    pub displacements: &'static [u16],
    /// The entry of the table in each slot, or [`EMPTY`].
    pub slots: &'static [u16],
}

/// A slot without an entry.
pub const EMPTY: u16 = u16::MAX;

impl Hashed {
    /// The index in the table of the name `key`.
    #[inline]
    pub fn find(&self, key: &[u8]) -> Option<usize> {
        let hash = hash(key);
        let bucket = bucket(hash, self.displacements.len());
        let displacement = *self.displacements.get(bucket)?;
        let entry = self.slots[slot(hash, displacement, self.slots.len())];
        let found = self.keys.get(usize::from(entry))?;
        (found.as_bytes() == key).then_some(usize::from(entry))
    }
}

/// An odd number whose bits look random: the golden ratio.
const ODD: u64 = 0x9e37_79b9_7f4a_7c15;

/// The hash of the bytes of a name, eight at a time: each word is mixed in with one
/// multiplication, which carries its bits up to the high half that [`bucket`] and
/// [`slot`] read.
#[inline]
pub fn hash(bytes: &[u8]) -> u64 {
    let mix = |hash: u64, word: u64| (hash.rotate_left(26) ^ word).wrapping_mul(ODD);
    let (words, rest) = bytes.as_chunks::<8>();
    let mut hash = bytes.len() as u64;
    for word in words {
        hash = mix(hash, u64::from_le_bytes(*word));
    }
    // The last bytes, fewer than eight, as the low bytes of a word.
    let last = rest
        .iter()
        .rev()
        .fold(0, |last, byte| last << 8 | u64::from(*byte));
    mix(hash, last)
}

/// The bucket of a hash among `buckets`.
#[inline]
pub fn bucket(hash: u64, buckets: usize) -> usize {
    range(hash >> 32, buckets)
}

/// The slot of a hash among `slots`, with the displacement of its bucket.
#[inline]
pub fn slot(hash: u64, displacement: u16, slots: usize) -> usize {
    let displaced = (hash ^ u64::from(displacement).wrapping_mul(ODD)).wrapping_mul(ODD);
    range(displaced >> 32, slots)
}

/// A number below 2^32 scaled to `0..size`, without a division.
#[inline]
fn range(number: u64, size: usize) -> usize {
    // A table has fewer than 2^16 entries.
    ((number * size as u64) >> 32) as usize
}
