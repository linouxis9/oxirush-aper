//! Reading aligned PER: the fields of ITU-T X.691 (02/2021), from octets.

use alloc::borrow::Cow;
use alloc::string::String;
use alloc::vec::Vec;

use crate::{BitString, Bits, Error, Fault, Faults, ObjectIdentifier, Octets, UNBOUNDED};

/// How many items a fragment of 16K items has (X.691 §11.9.3.8).
const SIXTEEN_K: usize = 16384;

/// The most that a list reserves before its elements are read, in bytes: a count is
/// what a peer claims, and the elements are what it sent.
const RESERVED: usize = 4096;

/// A reader of aligned PER: the octets of an encoding, and how far they are read.
///
/// Each method reads one field of ITU-T X.691 and moves on. A field that is
/// octet-aligned skips the bits up to the next octet first. The bounds of a constraint
/// are arguments, which a caller writes as constants.
///
/// A value that is whole, in another form than the one that its clause gives it, is
/// read, as the encoder of a peer may write it: a length of 127 or less in two octets,
/// a fragment that is not the largest, a small number in the form of a larger one, a
/// whole number in more octets than it needs, and padding bits that are not zero. A
/// reader that [`noting`](Self::noting) made notes each, [`faults`](Self::faults) has
/// what it noted, and [`decode_strict`](crate::decode_strict) refuses it.
///
/// ```
/// use oxirush_aper::Reader;
///
/// // INTEGER (0..7), then INTEGER (0..65535), which is two octets after padding
/// let mut reader = Reader::new(&[0b1010_0000, 0x12, 0x34]);
/// assert_eq!(reader.unsigned(0, 7)?, 5);
/// assert_eq!(reader.unsigned(0, 65535)?, 0x1234);
/// # Ok::<(), oxirush_aper::Error>(())
/// ```
///
/// A value that holds octets borrows them from the octets of the reader, and lives as
/// long as they do.
#[derive(Clone, Debug)]
pub struct Reader<'a> {
    octets: &'a [u8],
    /// How many bits are read.
    at: usize,
    /// What was read in another form than the one of its clause, when the reader
    /// notes it.
    faults: Faults,
    noting: bool,
}

impl<'a> Reader<'a> {
    /// A reader at the first bit of `octets`.
    #[inline]
    pub fn new(octets: &'a [u8]) -> Self {
        // The position is a count of bits.
        let most = usize::MAX / 8;
        let octets = octets.get(..most).unwrap_or(octets);
        Self {
            octets,
            at: 0,
            faults: Faults::new(),
            noting: false,
        }
    }

    /// A reader at the first bit of `octets` that notes what it reads in another form
    /// than the one that its clause gives: see [`Fault`], and [`faults`](Self::faults).
    /// It reads what a reader of [`new`](Self::new) reads, which looks at no padding
    /// bit and notes nothing.
    ///
    /// ```
    /// use oxirush_aper::{Fault, Reader, UNBOUNDED};
    ///
    /// // An OCTET STRING of one octet, whose length is in two octets
    /// let mut reader = Reader::noting(&[0x80, 0x01, 0xaa]);
    /// assert_eq!(reader.octet_string(0, UNBOUNDED)?, [0xaa]);
    /// assert_eq!(reader.faults()[0].fault, Fault::LengthInTwoOctets);
    /// # Ok::<(), oxirush_aper::Error>(())
    /// ```
    #[inline]
    pub fn noting(octets: &'a [u8]) -> Self {
        let mut reader = Self::new(octets);
        reader.noting = true;
        reader
    }

    /// What a reader that [`noting`](Self::noting) made read so far in another form
    /// than the one that its clause gives, each with the bit of the octets at which it
    /// is. None for a reader of [`new`](Self::new).
    #[inline]
    pub fn faults(&self) -> &Faults {
        &self.faults
    }

    /// Note a fault at a bit, when the reader notes: no part of reading octets that
    /// are written by the clauses.
    #[cold]
    #[inline(never)]
    fn note(&mut self, fault: Fault, bit: usize) {
        if self.noting {
            self.faults.push(fault, bit);
        }
    }

    /// Note the bits up to the next octet boundary when one of them is not zero (X.691
    /// §11.1.4). They are not skipped here.
    #[inline]
    fn padding(&mut self) {
        let shift = self.at & 7;
        if self.noting
            && shift != 0
            && let Some(octet) = self.octets.get(self.at >> 3)
            && octet & (0xff >> shift) != 0
        {
            self.note(Fault::Padding, self.at);
        }
    }

    /// How many bits are read.
    #[inline]
    pub fn position(&self) -> usize {
        self.at
    }

    /// How many bits are left.
    #[inline]
    pub fn remaining(&self) -> usize {
        self.octets.len() * 8 - self.at
    }

    /// The octets that are left, from the next octet boundary.
    #[inline]
    pub fn rest(&self) -> &'a [u8] {
        &self.octets[self.at.div_ceil(8)..]
    }

    /// The end of a complete encoding (X.691 §11.1.4): no octet follows the value,
    /// whose last octet may have padding bits. An encoding of no bit is one zero octet.
    #[inline]
    pub fn finish(&self) -> Result<(), Error> {
        let rest = self.rest();
        if rest.is_empty() || (self.at == 0 && self.octets == [0]) {
            return Ok(());
        }
        Err(followed(rest.len()))
    }

    // ── Bits and octets ─────────────────────────────────────────────────────

    /// One bit: a BOOLEAN (X.691 §12), and the bit of an extension marker or of a
    /// member that may be absent.
    #[inline]
    pub fn bit(&mut self) -> Result<bool, Error> {
        let Some(octet) = self.octets.get(self.at >> 3) else {
            return Err(Error::ended());
        };
        let bit = octet & (0x80 >> (self.at & 7)) != 0;
        self.at += 1;
        Ok(bit)
    }

    /// The next `count` bits as a number, the first bit the most significant: 64 bits
    /// at most. They are not aligned.
    #[inline]
    pub fn bits(&mut self, count: usize) -> Result<u64, Error> {
        debug_assert!(count <= 64);
        let (first, shift) = (self.at >> 3, self.at & 7);
        if count > self.remaining() {
            return Err(Error::ended());
        }
        self.at += count;
        // Bits within one octet, as the index of an ENUMERATED and the bits of a
        // preamble are: the octet is there, as the bits are.
        if shift + count <= 8 {
            let octet = self.octets.get(first).copied().unwrap_or(0);
            let (wide, mask) = (u16::from(octet), (1u16 << count) - 1);
            return Ok(u64::from(wide >> (8 - shift - count) & mask));
        }
        // Eight octets hold the bits, unless they start late in the first one.
        if let Some(word) = self.octets.get(first..first + 8)
            && shift + count <= 64
        {
            let word = u64::from_be_bytes([
                word[0], word[1], word[2], word[3], word[4], word[5], word[6], word[7],
            ]);
            return Ok((word << shift) >> (64 - count));
        }
        Ok(self.bits_near_the_end(first, shift, count))
    }

    /// `count` bits from the bit `shift` of the octet `first`, which are there and are
    /// in more than one octet: those near the end of the octets, and 64 bits that
    /// start within an octet.
    fn bits_near_the_end(&self, first: usize, shift: usize, count: usize) -> u64 {
        let last = (first * 8 + shift + count).div_ceil(8);
        let held = &self.octets[first..last];
        // Nine octets at most: eight of them in a word, and the bits of the ninth.
        let (whole, rest) = held.split_at(held.len().min(8));
        let mut word = [0; 8];
        word[..whole.len()].copy_from_slice(whole);
        let word = u64::from_be_bytes(word);
        match rest.first() {
            None => (word << shift) >> (64 - count),
            // The 64 bits from `shift`, of which the last `shift` are in the ninth
            // octet, then the first `count` of them.
            Some(ninth) => (word << shift | u64::from(*ninth) >> (8 - shift)) >> (64 - count),
        }
    }

    /// Skip the bits up to the next octet boundary: the padding of an octet-aligned
    /// field, and that of the end of a complete encoding (X.691 §11.1.4). Bits of it
    /// that are not zero are noted.
    #[inline]
    pub fn align(&mut self) {
        self.padding();
        self.at = (self.at + 7) & !7;
    }

    /// One octet, octet-aligned.
    #[inline]
    pub fn octet(&mut self) -> Result<u8, Error> {
        self.padding();
        let at = (self.at + 7) >> 3;
        let Some(octet) = self.octets.get(at) else {
            return Err(Error::ended());
        };
        self.at = (at + 1) << 3;
        Ok(*octet)
    }

    /// The next `count` octets, octet-aligned.
    #[inline]
    pub fn octets(&mut self, count: usize) -> Result<&'a [u8], Error> {
        self.padding();
        let start = (self.at + 7) >> 3;
        let held = start
            .checked_add(count)
            .and_then(|end| self.octets.get(start..end));
        let Some(held) = held else {
            return Err(Error::ended());
        };
        self.at = (start + count) << 3;
        Ok(held)
    }

    /// The next `count` bits from an octet boundary, as the octets that hold them, the
    /// last one padded with zero bits. What follows them is not aligned.
    fn aligned_bits(&mut self, count: usize) -> Result<Octets<'a>, Error> {
        self.padding();
        let start = (self.at + 7) >> 3;
        let length = count.div_ceil(8);
        let held = start
            .checked_add(length)
            .and_then(|end| self.octets.get(start..end));
        let Some(held) = held else {
            return Err(Error::ended());
        };
        self.at = (start << 3) + count;
        let unused = length * 8 - count;
        match held.last() {
            // The bits after the last one are those of the next field, or padding
            // that is not zero.
            Some(last) if (last.trailing_zeros() as usize) < unused => {
                let mut owned = held.to_vec();
                owned[length - 1] &= 0xff << unused;
                Ok(Octets::owned(owned))
            }
            _ => Ok(Octets::borrowed(held)),
        }
    }

    // ── Whole numbers ───────────────────────────────────────────────────────

    /// A constrained whole number (X.691 §11.5) of `lower..=upper`, as an INTEGER of
    /// that constraint is encoded (§13.2.2): no bit for one value, a bit-field for a
    /// range of 255 values at most, one octet for 256, two octets up to 64K, and
    /// above that the number of its octets, then its octets.
    #[inline]
    pub fn unsigned(&mut self, lower: u64, upper: u64) -> Result<u64, Error> {
        debug_assert!(lower <= upper);
        let span = upper.wrapping_sub(lower);
        let offset = self.offset(span)?;
        if offset > span {
            let value = i128::from(lower) + i128::from(offset);
            return Err(Error::number_read_outside(
                value,
                lower.into(),
                upper.into(),
            ));
        }
        Ok(lower + offset)
    }

    /// A constrained whole number of `lower..=upper`, where the bounds may be negative.
    #[inline]
    pub fn signed(&mut self, lower: i64, upper: i64) -> Result<i64, Error> {
        debug_assert!(lower <= upper);
        let span = (upper as u64).wrapping_sub(lower as u64);
        let offset = self.offset(span)?;
        if offset > span {
            let value = i128::from(lower) + i128::from(offset);
            return Err(Error::number_read_outside(
                value,
                lower.into(),
                upper.into(),
            ));
        }
        Ok((lower as u64).wrapping_add(offset) as i64)
    }

    /// How far a constrained whole number is from its lower bound, for a constraint
    /// whose upper bound is `span` above the lower one. The field may hold more than
    /// `span`.
    #[inline]
    fn offset(&mut self, span: u64) -> Result<u64, Error> {
        match span {
            0 => Ok(0),
            1..=254 => self.bits(64 - span.leading_zeros() as usize),
            255 => self.octet().map(u64::from),
            256..=65535 => {
                let octets = self.octets(2)?;
                Ok(u64::from(u16::from_be_bytes([octets[0], octets[1]])))
            }
            _ => self.offset_with_length(span),
        }
    }

    /// The indefinite length case (X.691 §11.5.7.4, §13.2.6 a): the number of octets,
    /// which is constrained to what the range takes, then the octets.
    fn offset_with_length(&mut self, span: u64) -> Result<u64, Error> {
        let most = (64 - span.leading_zeros() as usize).div_ceil(8);
        let length_bits = usize::BITS as usize - (most - 1).leading_zeros() as usize;
        // The field of the length holds eight at most.
        let length = self.bits(length_bits)? as usize + 1;
        let octets = self.octets(length)?;
        // The least octets that hold the number (§11.3.6).
        if length > 1 && octets[0] == 0 {
            self.note(Fault::LongNumber, self.at - length * 8);
        }
        Ok(octets
            .iter()
            .fold(0, |number, octet| number << 8 | u64::from(*octet)))
    }

    /// An INTEGER whose constraint `lower..=upper` has an extension marker (X.691
    /// §13.1): a bit, then the number within the constraint, or any number as an
    /// unconstrained one. The bindings hold it in 64 bits without a sign.
    ///
    /// # Errors
    ///
    /// A number outside the constraint that is negative, or of more than 64 bits, is an
    /// error: the bindings have no type for it.
    pub fn unsigned_extensible(&mut self, lower: u64, upper: u64) -> Result<u64, Error> {
        if !self.bit()? {
            return self.unsigned(lower, upper);
        }
        let number = self.whole_number(true)?;
        u64::try_from(number).map_err(|_| Error::does_not_fit(64))
    }

    /// An INTEGER whose constraint `lower..=upper` has an extension marker, where the
    /// bounds may be negative. The bindings hold it in 64 bits with a sign.
    pub fn signed_extensible(&mut self, lower: i64, upper: i64) -> Result<i64, Error> {
        if !self.bit()? {
            return self.signed(lower, upper);
        }
        self.unconstrained()
    }

    /// An unconstrained whole number (X.691 §11.8), as an INTEGER without a
    /// PER-visible constraint is encoded (§13.2.4): the number of its octets, then
    /// its octets in two's complement.
    pub fn unconstrained(&mut self) -> Result<i64, Error> {
        let number = self.whole_number(true)?;
        i64::try_from(number).map_err(|_| Error::does_not_fit(64))
    }

    /// A semi-constrained whole number (X.691 §11.7), as an INTEGER with a lower
    /// bound and no upper one is encoded (§13.2.3): the number of the octets of what
    /// it has above `lower`, then these octets.
    pub fn semi_constrained(&mut self, lower: i64) -> Result<i64, Error> {
        let above = self.whole_number(false)?;
        let number = i128::from(lower) + above;
        i64::try_from(number).map_err(|_| Error::does_not_fit(64))
    }

    /// A semi-constrained whole number whose lower bound is not negative, in 64 bits
    /// without a sign.
    pub fn semi_constrained_unsigned(&mut self, lower: u64) -> Result<u64, Error> {
        let above = self.whole_number(false)?;
        let number = i128::from(lower) + above;
        u64::try_from(number).map_err(|_| Error::does_not_fit(64))
    }

    /// A whole number after the unconstrained length of its octets: in two's
    /// complement when it is `signed`.
    ///
    /// A writer gives a number the least octets that hold it (X.691 §11.3.6, §11.4.6).
    /// A reader takes octets that only extend the sign before them, and notes them, as
    /// long as the octets are no more than a number of 64 bits may have: eight, and
    /// nine in two's complement, where a number of 2^63 or more has a zero octet first.
    /// More octets are refused whatever they hold.
    fn whole_number(&mut self, signed: bool) -> Result<i128, Error> {
        let octets = self.open_type()?;
        if octets.is_empty() {
            return Err(Error::decode("a whole number has no octet"));
        }
        if octets.len() > if signed { 9 } else { 8 } {
            return Err(Error::does_not_fit(64));
        }
        // The least octets: the first eight bits are not all zero, and in two's
        // complement the first nine are neither all zero nor all ones.
        if let [first, second, ..] = &octets[..] {
            let extends = match signed {
                true => (*first == 0 && *second < 0x80) || (*first == 0xff && *second >= 0x80),
                false => *first == 0,
            };
            if extends {
                self.note(Fault::LongNumber, self.at - octets.len() * 8);
            }
        }
        let negative = signed && octets[0] & 0x80 != 0;
        let start: i128 = if negative { -1 } else { 0 };
        let number = octets.iter();
        Ok(number.fold(start, |number, octet| number << 8 | i128::from(*octet)))
    }

    /// A normally small non-negative whole number (X.691 §11.6): the index of an
    /// alternative or of a value that a later version adds.
    pub fn small(&mut self) -> Result<u64, Error> {
        if !self.bit()? {
            return self.bits(6);
        }
        let start = self.at - 1;
        let number = self.semi_constrained_unsigned(0)?;
        // 63 and less are six bits after a zero bit (§11.6.1).
        if number <= 63 {
            self.note(Fault::SmallNumberAsLarge, start);
        }
        Ok(number)
    }

    // ── Lengths ─────────────────────────────────────────────────────────────

    /// An unconstrained length determinant (X.691 §11.9.3.5 to §11.9.3.8): how many
    /// items follow it, and whether another length follows these items, which then
    /// are a fragment of one to four times 16K items.
    ///
    /// ```
    /// use oxirush_aper::Reader;
    ///
    /// // 5, then 130 in two octets, then a fragment of 16384
    /// let mut reader = Reader::new(&[0x05, 0x80, 0x82, 0xc1]);
    /// assert_eq!(reader.length()?, (5, false));
    /// assert_eq!(reader.length()?, (130, false));
    /// assert_eq!(reader.length()?, (16384, true));
    /// # Ok::<(), oxirush_aper::Error>(())
    /// ```
    #[inline]
    pub fn length(&mut self) -> Result<(usize, bool), Error> {
        let first = self.octet()?;
        match first {
            0..=0x7f => Ok((usize::from(first), false)),
            0x80..=0xbf => {
                let second = self.octet()?;
                let length = usize::from(first & 0x3f) << 8 | usize::from(second);
                // 127 and less are one octet (§11.9.3.6).
                if length < 0x80 {
                    self.note(Fault::LengthInTwoOctets, self.at - 16);
                }
                Ok((length, false))
            }
            0xc1..=0xc4 => Ok((usize::from(first & 0x3f) * SIXTEEN_K, true)),
            _ => Err(Error::decode(
                "a fragment has one to four times 16384 items",
            )),
        }
    }

    /// The length determinant that follows a fragment of `fragment` items, as
    /// [`length`](Self::length) reads it: how many items follow it, and whether they
    /// are a fragment too.
    ///
    /// A fragment is the largest that the items allow (X.691 §11.9.3.8.1), so the one
    /// before another has four times 16K items. A smaller one is read, and a reader
    /// that notes its faults notes it.
    ///
    /// ```
    /// use oxirush_aper::{Fault, Reader};
    ///
    /// // 32768 items in two fragments of 16384, where one fragment holds them
    /// let mut reader = Reader::noting(&[0xc1, 0xc1, 0x00]);
    /// let (first, _) = reader.length()?;
    /// let (second, _) = reader.length_after(first)?;
    /// assert_eq!(reader.length_after(second)?, (0, false));
    /// assert_eq!(reader.faults()[0].fault, Fault::SmallFragment);
    /// # Ok::<(), oxirush_aper::Error>(())
    /// ```
    #[inline]
    pub fn length_after(&mut self, fragment: usize) -> Result<(usize, bool), Error> {
        let (count, more) = self.length()?;
        if more && fragment < 4 * SIXTEEN_K {
            self.note(Fault::SmallFragment, self.at - 8);
        }
        Ok((count, more))
    }

    /// The size of a string or of a list whose size constraint is `lower..=upper`,
    /// with an upper bound below 64K: a constrained whole number (X.691 §11.9.3.3).
    /// For a size of one value there is no field.
    #[inline]
    pub fn size(&mut self, lower: usize, upper: usize) -> Result<usize, Error> {
        debug_assert!(lower <= upper && upper < 65536);
        let span = upper.wrapping_sub(lower) as u64;
        let offset = self.offset(span)?;
        if offset > span {
            return Err(Error::size_read_outside(
                lower.saturating_add(offset as usize),
                lower,
                Some(upper),
            ));
        }
        Ok(lower + offset as usize)
    }

    /// Whether `size`, which was read after an unconstrained length, is within the size
    /// constraint `lower..=upper`: the elements of a list in fragments are counted by
    /// its reader.
    pub fn check_size(size: usize, lower: usize, upper: usize) -> Result<(), Error> {
        check_size(size, lower, upper)
    }

    /// The bitmap of the extension additions (X.691 §19.7, §19.8) after its length, a
    /// normally small length (§11.9.3.4): 64 bits at most have their length, less one,
    /// in six bits, and more bits an unconstrained length.
    ///
    /// The clauses leave a sender one encoding, and a reader takes no other: the length
    /// is one at least (§11.9.3.1 a)), 64 bits at most are not counted in the form of
    /// more, and one addition at least is present, as the extension bit is set for
    /// nothing else (§19.1).
    fn addition_bitmap(&mut self) -> Result<BitString<'a>, Error> {
        const NONE: &str =
            "the extension bit of a SEQUENCE is set, and it has no extension addition";
        let bitmap = if self.bit()? {
            let bitmap = self.bit_string_in_fragments()?;
            match bitmap.len() {
                0 => return Err(Error::decode(NONE)),
                1..=64 => {
                    return Err(Error::decode(
                        "the bitmap of 64 extension additions at most has its length in six bits",
                    ));
                }
                _ => bitmap,
            }
        } else {
            let count = self.bits(6)? as usize + 1;
            self.bit_field(count)?
        };
        if (0..bitmap.len()).all(|addition| bitmap.get(addition) != Some(true)) {
            return Err(Error::decode(NONE));
        }
        Ok(bitmap)
    }

    // ── ENUMERATED and CHOICE ───────────────────────────────────────────────

    /// The index of a value of an ENUMERATED (X.691 §14) that has `root` values before
    /// its extension marker and `additions` after it, when it is `extensible`. The
    /// values are counted in the order of their numbers, those of the root first: the
    /// first addition has the index `root`.
    ///
    /// # Errors
    ///
    /// An index that no value has is an error: a field of two bits holds four indexes
    /// for three values, and a later version may add a value.
    #[inline]
    pub fn enumerated(
        &mut self,
        root: u64,
        additions: u64,
        extensible: bool,
    ) -> Result<u64, Error> {
        match self.index(root, extensible)? {
            (index, false) if index < root => Ok(index),
            (index, true) if index < additions => Ok(root + index),
            (index, addition) => Err(no_index(index, addition, "value of the ENUMERATED")),
        }
    }

    /// The index of the alternative of a CHOICE (X.691 §23) that has `root`
    /// alternatives before its extension marker and `additions` after it, when it is
    /// `extensible`. The first addition has the index `root`, and its value follows in
    /// an open type.
    ///
    /// # Errors
    ///
    /// An index that no alternative has is an error.
    #[inline]
    pub fn choice(&mut self, root: u64, additions: u64, extensible: bool) -> Result<u64, Error> {
        match self.index(root, extensible)? {
            (index, false) if index < root => Ok(index),
            (index, true) if index < additions => Ok(root + index),
            (index, addition) => Err(no_index(index, addition, "alternative of the CHOICE")),
        }
    }

    /// An index among `root` values or alternatives, or among the additions: which of
    /// the two it is.
    #[inline]
    fn index(&mut self, root: u64, extensible: bool) -> Result<(u64, bool), Error> {
        debug_assert!(root > 0);
        if extensible && self.bit()? {
            return Ok((self.small()?, true));
        }
        Ok((self.offset(root - 1)?, false))
    }

    // ── SEQUENCE ────────────────────────────────────────────────────────────

    /// Skip the extension additions of a SEQUENCE whose extension bit is set (X.691
    /// §19.7 to §19.9): the bitmap of their presence, then an open type for each one
    /// that is present. A later version of a specification adds them, and a reader of
    /// an earlier one does not know them.
    ///
    /// # Errors
    ///
    /// What the clauses give no encoding for is an error: a bitmap of no bit or of
    /// which no bit is set, as the extension bit is set when an addition is present
    /// (§19.1, §19.8); a bitmap of 64 bits at most whose length is in the form of more
    /// (§11.9.3.4); and an addition of no octet (§11.1.3.1).
    pub fn skip_additions(&mut self) -> Result<(), Error> {
        let additions = self.additions()?;
        additions.skip_rest(self)
    }

    /// The extension additions of a SEQUENCE whose extension bit is set: which of them
    /// are present. Each one that is present follows in an open type, in their order.
    /// The bitmap is refused as [`skip_additions`](Self::skip_additions) refuses it.
    ///
    /// ```
    /// use oxirush_aper::Reader;
    ///
    /// // Two additions, of which the second is present: an open type of one octet
    /// let mut reader = Reader::new(&[0b0000_0010, 0b1000_0000, 0x01, 0xaa]);
    /// let mut additions = reader.additions()?;
    /// assert!(!additions.present());
    /// assert!(additions.present());
    /// assert_eq!(reader.open_type()?, [0xaa]);
    /// additions.skip_rest(&mut reader)?;
    /// # Ok::<(), oxirush_aper::Error>(())
    /// ```
    pub fn additions(&mut self) -> Result<Additions<'a>, Error> {
        let bitmap = self.addition_bitmap()?;
        Ok(Additions { bitmap, next: 0 })
    }

    // ── OCTET STRING ────────────────────────────────────────────────────────

    /// An OCTET STRING of `N` octets (X.691 §17.6, §17.7): a bit-field for two octets
    /// at most, and octet-aligned otherwise.
    #[inline]
    pub fn octet_array<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let mut array = [0; N];
        if N > 2 {
            array.copy_from_slice(self.octets(N)?);
        } else {
            for octet in &mut array {
                *octet = self.bits(8)? as u8;
            }
        }
        Ok(array)
    }

    /// An OCTET STRING of the size constraint `lower..=upper` (X.691 §17), with
    /// [`UNBOUNDED`] for no upper bound. The octets are borrowed.
    ///
    /// A string of 16384 octets or more is in fragments when its size has no bound
    /// below 64K, and is then copied.
    #[inline]
    pub fn octet_string(&mut self, lower: usize, upper: usize) -> Result<Octets<'a>, Error> {
        if upper >= 65536 {
            let octets = self.octets_in_fragments()?;
            return check_size(octets.len(), lower, upper).map(|()| octets);
        }
        if lower == upper {
            // Two octets at most are a bit-field.
            return match lower {
                0 => Ok(Octets::EMPTY),
                1..=2 => {
                    let short = self.bits(lower * 8)?.to_be_bytes();
                    Ok(Octets::owned(short[8 - lower..].to_vec()))
                }
                _ => self.octets(lower).map(Octets::borrowed),
            };
        }
        match self.size(lower, upper)? {
            // Nothing is added for no octet, and no padding.
            0 => Ok(Octets::EMPTY),
            size => self.octets(size).map(Octets::borrowed),
        }
    }

    /// An OCTET STRING whose size constraint `lower..=upper` has an extension marker
    /// (X.691 §17.3): a bit, then the string within the constraint, or a string of any
    /// size.
    pub fn octet_string_extensible(
        &mut self,
        lower: usize,
        upper: usize,
    ) -> Result<Octets<'a>, Error> {
        match self.bit()? {
            false => self.octet_string(lower, upper),
            true => self.octets_in_fragments(),
        }
    }

    /// The octets of an open type (X.691 §11.2): the complete encoding of a value,
    /// after its length. The octets are borrowed, unless they are in fragments.
    #[inline]
    pub fn open_type(&mut self) -> Result<Octets<'a>, Error> {
        self.octets_in_fragments()
    }

    /// Octets after their unconstrained length: borrowed when they are in one piece.
    #[inline]
    fn octets_in_fragments(&mut self) -> Result<Octets<'a>, Error> {
        let (count, more) = self.length()?;
        let first = self.octets(count)?;
        if !more {
            return Ok(Octets::borrowed(first));
        }
        self.more_fragments(first).map(Octets::owned)
    }

    /// The octets of the fragments that follow `first`, after it.
    #[cold]
    fn more_fragments(&mut self, first: &[u8]) -> Result<Vec<u8>, Error> {
        let mut gathered = first.to_vec();
        let mut fragment = first.len();
        loop {
            let (count, more) = self.length_after(fragment)?;
            gathered.extend_from_slice(self.octets(count)?);
            if !more {
                return Ok(gathered);
            }
            fragment = count;
        }
    }

    /// Skip the open type of an extension addition: the complete encoding of a value
    /// (X.691 §19.9, §11.2.1), which is one octet at least (§11.1.3.1).
    fn skip_open_type(&mut self) -> Result<(), Error> {
        let mut octets = 0;
        let (mut count, mut more) = self.length()?;
        loop {
            self.octets(count)?;
            octets += count;
            if !more {
                break;
            }
            (count, more) = self.length_after(count)?;
        }
        if octets == 0 {
            return Err(Error::decode(
                "an extension addition has no octet: an open type holds a complete encoding",
            ));
        }
        Ok(())
    }

    // ── BIT STRING ──────────────────────────────────────────────────────────

    /// A BIT STRING of 64 bits at most, of the size constraint `lower..=upper` (X.691
    /// §16.9 to §16.11): a bit-field for one size of 16 bits at most, and octet-aligned
    /// otherwise, after its length when its size varies.
    #[inline]
    pub fn bits_sized(&mut self, lower: usize, upper: usize) -> Result<Bits, Error> {
        debug_assert!(lower <= upper && upper <= 64);
        let size = self.size(lower, upper)?;
        if size == 0 {
            return Ok(Bits::EMPTY);
        }
        if lower != upper || size > 16 {
            self.align();
        }
        Ok(Bits::low(self.bits(size)?, size))
    }

    /// A BIT STRING of the size constraint `lower..=upper` (X.691 §16), with
    /// [`UNBOUNDED`] for no upper bound. The octets are borrowed where the string is
    /// octet-aligned, and one of 64 bits at most is held without an allocation.
    ///
    /// A string of 16384 bits or more is in fragments when its size has no bound
    /// below 64K, and is then copied.
    #[inline]
    pub fn bit_string(&mut self, lower: usize, upper: usize) -> Result<BitString<'a>, Error> {
        if upper >= 65536 {
            let bits = self.bit_string_in_fragments()?;
            return check_size(bits.len(), lower, upper).map(|()| bits);
        }
        let size = self.size(lower, upper)?;
        if size == 0 {
            return Ok(BitString::default());
        }
        if lower == upper && size <= 16 {
            return self.bit_field(size);
        }
        let octets = self.aligned_bits(size)?;
        Ok(BitString::of(octets, size))
    }

    /// A BIT STRING whose size constraint `lower..=upper` has an extension marker
    /// (X.691 §16.6): a bit, then the string within the constraint, or a string of any
    /// size.
    pub fn bit_string_extensible(
        &mut self,
        lower: usize,
        upper: usize,
    ) -> Result<BitString<'a>, Error> {
        match self.bit()? {
            false => self.bit_string(lower, upper),
            true => self.bit_string_in_fragments(),
        }
    }

    /// `count` bits that are not aligned, as a bit string.
    fn bit_field(&mut self, count: usize) -> Result<BitString<'a>, Error> {
        if count <= 64 {
            return Ok(BitString::from(Bits::low(self.bits(count)?, count)));
        }
        let mut octets = Vec::with_capacity(count.div_ceil(8).min(self.remaining() / 8 + 1));
        let mut left = count;
        while left > 0 {
            let part = left.min(8);
            octets.push((self.bits(part)? << (8 - part)) as u8);
            left -= part;
        }
        Ok(BitString::of(Octets::owned(octets), count))
    }

    /// A bit string after its unconstrained length, which counts bits: borrowed when
    /// it is in one piece.
    fn bit_string_in_fragments(&mut self) -> Result<BitString<'a>, Error> {
        let (count, more) = self.length()?;
        if !more {
            let octets = self.aligned_bits(count)?;
            return Ok(BitString::of(octets, count));
        }
        // A fragment is a multiple of 16384 bits, and so whole octets.
        let mut gathered = self.octets(count / 8)?.to_vec();
        let mut bits = count;
        let mut fragment = count;
        loop {
            let (count, more) = self.length_after(fragment)?;
            bits += count;
            if more {
                gathered.extend_from_slice(self.octets(count / 8)?);
                fragment = count;
            } else {
                gathered.extend_from_slice(&self.aligned_bits(count)?);
                return Ok(BitString::of(Octets::owned(gathered), bits));
            }
        }
    }

    // ── Character strings ───────────────────────────────────────────────────

    /// A PrintableString of the size constraint `lower..=upper` (X.691 §30.5), with
    /// [`UNBOUNDED`] for no upper bound: a character is eight bits in the aligned
    /// variant.
    pub fn printable_string(&mut self, lower: usize, upper: usize) -> Result<Cow<'a, str>, Error> {
        self.known_multiplier(lower, upper, false, Alphabet::Printable)
    }

    /// A PrintableString whose size constraint has an extension marker (X.691 §30.4).
    pub fn printable_string_extensible(
        &mut self,
        lower: usize,
        upper: usize,
    ) -> Result<Cow<'a, str>, Error> {
        self.known_multiplier(lower, upper, true, Alphabet::Printable)
    }

    /// A VisibleString of the size constraint `lower..=upper`, with [`UNBOUNDED`] for
    /// no upper bound.
    pub fn visible_string(&mut self, lower: usize, upper: usize) -> Result<Cow<'a, str>, Error> {
        self.known_multiplier(lower, upper, false, Alphabet::Visible)
    }

    /// A VisibleString whose size constraint has an extension marker.
    pub fn visible_string_extensible(
        &mut self,
        lower: usize,
        upper: usize,
    ) -> Result<Cow<'a, str>, Error> {
        self.known_multiplier(lower, upper, true, Alphabet::Visible)
    }

    /// An IA5String of the size constraint `lower..=upper`, with [`UNBOUNDED`] for no
    /// upper bound.
    pub fn ia5_string(&mut self, lower: usize, upper: usize) -> Result<Cow<'a, str>, Error> {
        self.known_multiplier(lower, upper, false, Alphabet::Ia5)
    }

    /// An IA5String whose size constraint has an extension marker.
    pub fn ia5_string_extensible(
        &mut self,
        lower: usize,
        upper: usize,
    ) -> Result<Cow<'a, str>, Error> {
        self.known_multiplier(lower, upper, true, Alphabet::Ia5)
    }

    /// A UTF8String (X.691 §30.6): the length of its octets, then its octets. A size
    /// constraint on it is not PER-visible (§10.3.7), and is not looked at.
    pub fn utf8_string(&mut self) -> Result<Cow<'a, str>, Error> {
        let octets = self.octets_in_fragments()?;
        text(
            octets,
            |text| core::str::from_utf8(text).is_ok(),
            "UTF8String",
        )
    }

    /// A string of a known-multiplier type whose characters are eight bits.
    fn known_multiplier(
        &mut self,
        lower: usize,
        upper: usize,
        extensible: bool,
        alphabet: Alphabet,
    ) -> Result<Cow<'a, str>, Error> {
        let within = |octets: &[u8]| octets.iter().all(|octet| alphabet.has(*octet));
        // Outside the root, the string is encoded as one without a size constraint.
        let (lower, upper) = match extensible && self.bit()? {
            true => (0, UNBOUNDED),
            false => (lower, upper),
        };
        if upper >= 65536 {
            let octets = self.octets_in_fragments()?;
            check_size(octets.len(), lower, upper)?;
            return text(octets, within, alphabet.name());
        }
        let size = self.size(lower, upper)?;
        // One size of two characters at most, and a size that varies up to one
        // character, are bit-fields (X.691 §30.5.6, §30.5.7).
        let aligned = if lower == upper {
            upper * 8 > 16
        } else {
            upper * 8 >= 16
        };
        let octets = match (size, aligned) {
            (0, _) => Octets::EMPTY,
            (_, true) => Octets::borrowed(self.octets(size)?),
            (_, false) => {
                let short = self.bits(size * 8)?.to_be_bytes();
                Octets::owned(short[8 - size..].to_vec())
            }
        };
        text(octets, within, alphabet.name())
    }

    // ── OBJECT IDENTIFIER ───────────────────────────────────────────────────

    /// An OBJECT IDENTIFIER (X.691 §24): the length of its contents octets, then the
    /// contents octets that the Basic Encoding Rules give it.
    pub fn object_identifier(&mut self) -> Result<ObjectIdentifier, Error> {
        let octets = self.octets_in_fragments()?;
        ObjectIdentifier::from_contents(&octets)
    }

    // ── SEQUENCE OF ─────────────────────────────────────────────────────────

    /// An empty list that has room for `count` elements, as far as the octets that
    /// are left may hold them: a count that a peer claims reserves little.
    #[inline]
    pub fn list<T>(&self, count: usize) -> Vec<T> {
        let most = RESERVED / size_of::<T>().max(1);
        Vec::with_capacity(count.min(most.max(1)).min(self.remaining() + 1))
    }
}

/// The extension additions of a SEQUENCE that a reader was given: which of them are
/// present, in their order.
#[derive(Clone, Debug)]
pub struct Additions<'a> {
    bitmap: BitString<'a>,
    next: usize,
}

impl Additions<'_> {
    /// Whether the next addition is present: the first one at the first call. An
    /// addition that the encoding does not count is not present.
    pub fn present(&mut self) -> bool {
        let present = self.bitmap.get(self.next).unwrap_or(false);
        self.next += 1;
        present
    }

    /// Skip the additions that are present and were not asked for: those that a later
    /// version of the specification adds.
    pub fn skip_rest(self, reader: &mut Reader<'_>) -> Result<(), Error> {
        for position in self.next..self.bitmap.len() {
            if self.bitmap.get(position) == Some(true) {
                reader.skip_open_type()?;
            }
        }
        Ok(())
    }
}

/// The known-multiplier character string types whose characters are eight bits in the
/// aligned variant, each with the characters that it has.
#[derive(Clone, Copy)]
pub(crate) enum Alphabet {
    Printable,
    Visible,
    Ia5,
}

impl Alphabet {
    /// Whether the type has the character of a code (ITU-T X.680 (02/2021) §41.4).
    pub(crate) fn has(self, code: u8) -> bool {
        match self {
            Self::Printable => code.is_ascii_alphanumeric() || b" '()+,-./:=?".contains(&code),
            Self::Visible => (0x20..=0x7e).contains(&code),
            Self::Ia5 => code <= 0x7f,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Printable => "PrintableString",
            Self::Visible => "VisibleString",
            Self::Ia5 => "IA5String",
        }
    }
}

/// The text that octets are, when `valid` says that they are a string of the type
/// `name`.
fn text<'a>(
    octets: Octets<'a>,
    valid: impl Fn(&[u8]) -> bool,
    name: &str,
) -> Result<Cow<'a, str>, Error> {
    let not_text = || Error::decode(alloc::format!("a character string is no {name}"));
    if !valid(&octets) {
        return Err(not_text());
    }
    // What is valid is ASCII, or was checked as UTF-8.
    match octets.as_borrowed() {
        Some(borrowed) => match core::str::from_utf8(borrowed) {
            Ok(text) => Ok(Cow::Borrowed(text)),
            Err(_) => Err(not_text()),
        },
        None => match String::from_utf8(octets.into_vec()) {
            Ok(text) => Ok(Cow::Owned(text)),
            Err(_) => Err(not_text()),
        },
    }
}

/// The error of an index that nothing has: among the additions when it is one of an
/// `addition`.
#[cold]
fn no_index(index: u64, addition: bool, of: &str) -> Error {
    Error::decode(match addition {
        false => alloc::format!("{index} is no {of}"),
        true => alloc::format!("{index} is no {of} among those after its extension marker"),
    })
}

/// A size that was read after an unconstrained length is within `lower..=upper`.
fn check_size(size: usize, lower: usize, upper: usize) -> Result<(), Error> {
    if (lower..=upper).contains(&size) {
        return Ok(());
    }
    let upper = (upper != UNBOUNDED).then_some(upper);
    Err(Error::size_read_outside(size, lower, upper))
}

/// The error of a value that octets follow.
#[cold]
fn followed(octets: usize) -> Error {
    Error::decode(match octets {
        1 => String::from("the value is followed by 1 octet"),
        octets => alloc::format!("the value is followed by {octets} octets"),
    })
}
