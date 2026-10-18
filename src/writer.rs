//! Writing aligned PER: the fields of ITU-T X.691 (02/2021), into octets.

use alloc::vec::Vec;

use crate::reader::Alphabet;
use crate::{BitString, Bits, Error, ObjectIdentifier, UNBOUNDED};

/// How many items a fragment of 16K items has (X.691 §11.9.3.8).
const SIXTEEN_K: usize = 16384;

/// A writer of aligned PER: it appends the fields of an encoding to the octets of its
/// caller.
///
/// Each method writes one field of ITU-T X.691. A field that is octet-aligned starts
/// the next octet, after padding bits that are zero. The bounds of a constraint are
/// arguments, which a caller writes as constants, and a value outside them is an
/// error.
///
/// ```
/// use oxirush_aper::Writer;
///
/// // INTEGER (0..7), then INTEGER (0..65535), which is two octets after padding
/// let mut octets = Vec::new();
/// let mut writer = Writer::new(&mut octets);
/// writer.unsigned(5, 0, 7)?;
/// writer.unsigned(0x1234, 0, 65535)?;
/// assert_eq!(octets, [0b1010_0000, 0x12, 0x34]);
/// # Ok::<(), oxirush_aper::Error>(())
/// ```
#[derive(Debug)]
pub struct Writer<'a> {
    octets: &'a mut Vec<u8>,
    /// How many bits of the last octet are not written yet: none at an octet boundary.
    free: usize,
    /// Whether a value outside the root of an extensible constraint is refused.
    strict: bool,
}

impl<'a> Writer<'a> {
    /// A writer that appends to `octets`, from an octet boundary.
    #[inline]
    pub fn new(octets: &'a mut Vec<u8>) -> Self {
        Self {
            octets,
            free: 0,
            strict: false,
        }
    }

    /// A writer that appends to `octets`, and refuses a value outside the root of an
    /// extensible constraint, where the writer of [`new`](Self::new) writes it as an
    /// extension: [`encode_strict`](crate::encode_strict) encodes with it.
    ///
    /// ```
    /// use oxirush_aper::{ErrorKind, Writer};
    ///
    /// // INTEGER (0..255, ...): 256 is a value of a later version of the type
    /// let (mut any, mut none) = (Vec::new(), Vec::new());
    /// assert!(Writer::new(&mut any).unsigned_extensible(256, 0, 255).is_ok());
    /// let error = Writer::strict(&mut none).unsigned_extensible(256, 0, 255).unwrap_err();
    /// assert_eq!(error.kind(), ErrorKind::Encode);
    /// assert_eq!(
    ///     error.to_string(),
    ///     "the value does not encode: 256 is outside the root of its constraint (0..255, ...)"
    /// );
    /// ```
    #[inline]
    pub fn strict(octets: &'a mut Vec<u8>) -> Self {
        Self {
            octets,
            free: 0,
            strict: true,
        }
    }

    /// How many octets the writer and its caller have written, with the one that is
    /// not full.
    #[inline]
    pub fn len(&self) -> usize {
        self.octets.len()
    }

    /// Whether nothing is written.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.octets.is_empty()
    }

    // ── Bits and octets ─────────────────────────────────────────────────────

    /// One bit: a BOOLEAN (X.691 §12), and the bit of an extension marker or of a
    /// member that may be absent.
    #[inline]
    pub fn bit(&mut self, bit: bool) {
        if self.free == 0 {
            self.octets.push(u8::from(bit) << 7);
            self.free = 7;
        } else {
            self.free -= 1;
            if let Some(last) = self.octets.last_mut() {
                *last |= u8::from(bit) << self.free;
            }
        }
    }

    /// The `count` least significant bits of `value`, the most significant first: 64
    /// bits at most. They are not aligned.
    #[inline]
    pub fn bits(&mut self, value: u64, count: usize) {
        debug_assert!(count <= 64);
        let mut left = count;
        if self.free > 0 && left > 0 {
            let part = left.min(self.free);
            left -= part;
            self.free -= part;
            let bits = (value >> left) as u8 & (0xff >> (8 - part));
            if let Some(last) = self.octets.last_mut() {
                *last |= bits << self.free;
            }
        }
        while left >= 8 {
            left -= 8;
            self.octets.push((value >> left) as u8);
        }
        if left > 0 {
            self.octets.push((value as u8) << (8 - left));
            self.free = 8 - left;
        }
    }

    /// Go to the next octet boundary: the padding of an octet-aligned field is zero
    /// bits (X.691 §11.1.4).
    #[inline]
    pub fn align(&mut self) {
        self.free = 0;
    }

    /// One octet, octet-aligned.
    #[inline]
    pub fn octet(&mut self, octet: u8) {
        self.free = 0;
        self.octets.push(octet);
    }

    /// Octets, octet-aligned.
    #[inline]
    pub fn octets(&mut self, octets: &[u8]) {
        self.free = 0;
        self.octets.extend_from_slice(octets);
    }

    /// The first `count` bits of `octets`, from an octet boundary. What follows them
    /// is not aligned.
    fn aligned_bits(&mut self, octets: &[u8], count: usize) {
        self.free = 0;
        self.octets.extend_from_slice(octets);
        self.free = octets.len() * 8 - count;
    }

    // ── Whole numbers ───────────────────────────────────────────────────────

    /// A constrained whole number (X.691 §11.5) of `lower..=upper`, as an INTEGER of
    /// that constraint is encoded (§13.2.2).
    ///
    /// # Errors
    ///
    /// A number outside the constraint is an error.
    #[inline]
    pub fn unsigned(&mut self, value: u64, lower: u64, upper: u64) -> Result<(), Error> {
        debug_assert!(lower <= upper);
        if value < lower || value > upper {
            return Err(Error::number_written_outside(
                value.into(),
                lower.into(),
                upper.into(),
            ));
        }
        self.offset(value - lower, upper - lower);
        Ok(())
    }

    /// A constrained whole number of `lower..=upper`, where the bounds may be negative.
    #[inline]
    pub fn signed(&mut self, value: i64, lower: i64, upper: i64) -> Result<(), Error> {
        debug_assert!(lower <= upper);
        if value < lower || value > upper {
            return Err(Error::number_written_outside(
                value.into(),
                lower.into(),
                upper.into(),
            ));
        }
        let offset = (value as u64).wrapping_sub(lower as u64);
        self.offset(offset, (upper as u64).wrapping_sub(lower as u64));
        Ok(())
    }

    /// How far a constrained whole number is from its lower bound, for a constraint
    /// whose upper bound is `span` above the lower one.
    #[inline]
    fn offset(&mut self, offset: u64, span: u64) {
        match span {
            0 => {}
            1..=254 => self.bits(offset, 64 - span.leading_zeros() as usize),
            255 => self.octet(offset as u8),
            256..=65535 => self.octets(&(offset as u16).to_be_bytes()),
            _ => self.offset_with_length(offset, span),
        }
    }

    /// The indefinite length case (X.691 §11.5.7.4, §13.2.6 a): the number of octets
    /// that the number takes, then these octets.
    fn offset_with_length(&mut self, offset: u64, span: u64) {
        let most = (64 - span.leading_zeros() as usize).div_ceil(8);
        let length_bits = usize::BITS as usize - (most - 1).leading_zeros() as usize;
        let length = (64 - offset.leading_zeros() as usize).div_ceil(8).max(1);
        self.bits(length as u64 - 1, length_bits);
        self.octets(&offset.to_be_bytes()[8 - length..]);
    }

    /// An INTEGER whose constraint `lower..=upper` has an extension marker (X.691
    /// §13.1): a bit, then the number within the constraint, or a number outside it as
    /// an unconstrained one.
    ///
    /// # Errors
    ///
    /// A number outside the constraint, for a writer that is
    /// [`strict`](Self::strict): no other writer refuses one.
    pub fn unsigned_extensible(&mut self, value: u64, lower: u64, upper: u64) -> Result<(), Error> {
        let outside = value < lower || value > upper;
        if outside && self.strict {
            return Err(Error::number_written_outside_root(
                value.into(),
                lower.into(),
                upper.into(),
            ));
        }
        self.bit(outside);
        match outside {
            false => self.offset(value - lower, upper - lower),
            true => self.whole_number(value.into(), true),
        }
        Ok(())
    }

    /// An INTEGER whose constraint `lower..=upper` has an extension marker, where the
    /// bounds may be negative.
    ///
    /// # Errors
    ///
    /// A number outside the constraint, for a writer that is
    /// [`strict`](Self::strict).
    pub fn signed_extensible(&mut self, value: i64, lower: i64, upper: i64) -> Result<(), Error> {
        let outside = value < lower || value > upper;
        if outside && self.strict {
            return Err(Error::number_written_outside_root(
                value.into(),
                lower.into(),
                upper.into(),
            ));
        }
        self.bit(outside);
        match outside {
            false => {
                let offset = (value as u64).wrapping_sub(lower as u64);
                self.offset(offset, (upper as u64).wrapping_sub(lower as u64));
            }
            true => self.whole_number(value.into(), true),
        }
        Ok(())
    }

    /// An unconstrained whole number (X.691 §11.8): the number of its octets, then its
    /// octets in two's complement, as few as hold it.
    pub fn unconstrained(&mut self, value: i64) {
        self.whole_number(value.into(), true);
    }

    /// A semi-constrained whole number (X.691 §11.7): the number of the octets of what
    /// it has above `lower`, then these octets.
    ///
    /// # Errors
    ///
    /// A number below `lower` is an error.
    pub fn semi_constrained(&mut self, value: i64, lower: i64) -> Result<(), Error> {
        if value < lower {
            return Err(Error::encode(alloc::format!(
                "{value} is outside its constraint ({lower}..MAX)"
            )));
        }
        self.whole_number(i128::from(value) - i128::from(lower), false);
        Ok(())
    }

    /// A semi-constrained whole number whose lower bound is not negative.
    pub fn semi_constrained_unsigned(&mut self, value: u64, lower: u64) -> Result<(), Error> {
        if value < lower {
            return Err(Error::encode(alloc::format!(
                "{value} is outside its constraint ({lower}..MAX)"
            )));
        }
        self.whole_number(i128::from(value) - i128::from(lower), false);
        Ok(())
    }

    /// A whole number after the unconstrained length of its octets, which are as few
    /// as hold it: in two's complement when it is `signed`.
    fn whole_number(&mut self, value: i128, signed: bool) {
        let octets = value.to_be_bytes();
        let fill = if value < 0 { 0xff } else { 0x00 };
        let mut first = octets.iter().position(|octet| *octet != fill).unwrap_or(15);
        // The first bit of a number in two's complement is its sign.
        if signed && first > 0 && (octets[first] & 0x80 != 0) != (value < 0) {
            first -= 1;
        }
        self.octet((16 - first) as u8);
        self.octets(&octets[first..]);
    }

    /// A normally small non-negative whole number (X.691 §11.6).
    pub fn small(&mut self, value: u64) {
        if value < 64 {
            self.bit(false);
            self.bits(value, 6);
        } else {
            self.bit(true);
            self.whole_number(value.into(), false);
        }
    }

    // ── Lengths ─────────────────────────────────────────────────────────────

    /// An unconstrained length determinant (X.691 §11.9.3.5 to §11.9.3.8) for `left`
    /// items that are still to be written: how many of them follow it. When these are
    /// fewer than `left`, or `left` is a multiple of 16384, they are a fragment, and
    /// another length follows them: the caller writes them, then calls again with what
    /// is left, down to a length of less than 16384.
    ///
    /// ```
    /// use oxirush_aper::Writer;
    ///
    /// // 40000 items: fragments of 32768 and of 7232
    /// let mut octets = Vec::new();
    /// let mut writer = Writer::new(&mut octets);
    /// assert_eq!(writer.length(40000), (32768, true));
    /// assert_eq!(writer.length(40000 - 32768), (7232, false));
    /// assert_eq!(octets, [0xc2, 0x9c, 0x40]);
    /// ```
    #[inline]
    pub fn length(&mut self, left: usize) -> (usize, bool) {
        match left {
            0..=0x7f => {
                self.octet(left as u8);
                (left, false)
            }
            0x80..=0x3fff => {
                self.octets(&(left as u16 | 0x8000).to_be_bytes());
                (left, false)
            }
            _ => {
                let blocks = (left / SIXTEEN_K).min(4);
                self.octet(0xc0 | blocks as u8);
                (blocks * SIXTEEN_K, true)
            }
        }
    }

    /// The size of a string or of a list whose size constraint is `lower..=upper`,
    /// with an upper bound below 64K: a constrained whole number (X.691 §11.9.3.3).
    ///
    /// ```
    /// use oxirush_aper::{Reader, Writer};
    ///
    /// // SEQUENCE (SIZE (1..16)) OF: a count of three, in four bits
    /// let mut octets = Vec::new();
    /// Writer::new(&mut octets).size(3, 1, 16)?;
    /// assert_eq!(octets, [0x20]);
    /// assert_eq!(Reader::new(&octets).size(1, 16)?, 3);
    ///
    /// // A count outside the constraint
    /// assert!(Writer::new(&mut Vec::new()).size(17, 1, 16).is_err());
    /// # Ok::<(), oxirush_aper::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// A size outside the constraint is an error.
    #[inline]
    pub fn size(&mut self, size: usize, lower: usize, upper: usize) -> Result<(), Error> {
        debug_assert!(lower <= upper && upper < 65536);
        if size < lower || size > upper {
            return Err(Error::size_written_outside(size, lower, Some(upper)));
        }
        self.offset((size - lower) as u64, (upper - lower) as u64);
        Ok(())
    }

    /// Whether `size` is within the size constraint `lower..=upper`, for a list that is
    /// written after an unconstrained length, in fragments.
    pub fn check_size(size: usize, lower: usize, upper: usize) -> Result<(), Error> {
        check_size(size, lower, upper)
    }

    // ── ENUMERATED and CHOICE ───────────────────────────────────────────────

    /// The index of a value of an ENUMERATED (X.691 §14) or of an alternative of a
    /// CHOICE (§23) that has `root` of them before its extension marker, when it is
    /// `extensible`. The first addition has the index `root`, and the alternative of a
    /// CHOICE that is an addition follows in an open type.
    ///
    /// ```
    /// use oxirush_aper::{Reader, Writer};
    ///
    /// // ENUMERATED { a, b, c, ... }: the extension bit, then c in two bits
    /// let mut octets = Vec::new();
    /// Writer::new(&mut octets).index(2, 3, true);
    /// assert_eq!(octets, [0x40]);
    /// assert_eq!(Reader::new(&octets).enumerated(3, 0, true)?, 2);
    /// # Ok::<(), oxirush_aper::Error>(())
    /// ```
    #[inline]
    pub fn index(&mut self, index: u64, root: u64, extensible: bool) {
        debug_assert!(root > 0 && (extensible || index < root));
        let addition = index >= root;
        if extensible {
            self.bit(addition);
        }
        match addition {
            false => self.offset(index, root - 1),
            true => self.small(index - root),
        }
    }

    // ── SEQUENCE ────────────────────────────────────────────────────────────

    /// The bitmap of the extension additions of a SEQUENCE (X.691 §19.7, §19.8),
    /// whose extension bit is set: whether each addition is present, in their order.
    /// Each one that is present follows in an open type.
    ///
    /// ```
    /// use oxirush_aper::Writer;
    ///
    /// // Two additions, of which the second is present, in an open type of one octet
    /// let mut octets = Vec::new();
    /// let mut writer = Writer::new(&mut octets);
    /// writer.additions(&[false, true])?;
    /// writer.open_type(&[0xaa]);
    /// assert_eq!(octets, [0b0000_0010, 0b1000_0000, 0x01, 0xaa]);
    ///
    /// // With none present, the extension bit is not set and no bitmap is written
    /// assert!(Writer::new(&mut Vec::new()).additions(&[false, false]).is_err());
    /// # Ok::<(), oxirush_aper::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// No addition, and none that is present, is an error: the extension bit is then
    /// not set (§19.1), and nothing follows the root.
    pub fn additions(&mut self, present: &[bool]) -> Result<(), Error> {
        let count = match present.contains(&true) {
            true => present.len(),
            false => 0,
        };
        match count {
            0 => return Err(Error::encode("a SEQUENCE has no addition to write")),
            1..=64 => {
                self.bit(false);
                self.bits(count as u64 - 1, 6);
                present.iter().for_each(|bit| self.bit(*bit));
            }
            _ => {
                self.bit(true);
                let mut left = present;
                loop {
                    let (count, more) = self.length(left.len());
                    self.align();
                    left[..count].iter().for_each(|bit| self.bit(*bit));
                    left = &left[count..];
                    if !more {
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    // ── OCTET STRING ────────────────────────────────────────────────────────

    /// An OCTET STRING of `N` octets (X.691 §17.6, §17.7): a bit-field for two octets
    /// at most, and octet-aligned otherwise.
    #[inline]
    pub fn octet_array<const N: usize>(&mut self, octets: &[u8; N]) {
        if N > 2 {
            self.octets(octets);
        } else {
            octets
                .iter()
                .for_each(|octet| self.bits((*octet).into(), 8));
        }
    }

    /// An OCTET STRING of the size constraint `lower..=upper` (X.691 §17), with
    /// [`UNBOUNDED`] for no upper bound.
    ///
    /// # Errors
    ///
    /// A size outside the constraint is an error.
    #[inline]
    pub fn octet_string(&mut self, octets: &[u8], lower: usize, upper: usize) -> Result<(), Error> {
        if upper >= 65536 {
            check_size(octets.len(), lower, upper)?;
            self.octets_in_fragments(octets);
            return Ok(());
        }
        self.size(octets.len(), lower, upper)?;
        if lower == upper && upper <= 2 {
            octets
                .iter()
                .for_each(|octet| self.bits((*octet).into(), 8));
        } else if !octets.is_empty() {
            // Nothing is added for no octet, and no padding.
            self.octets(octets);
        }
        Ok(())
    }

    /// An OCTET STRING whose size constraint `lower..=upper` has an extension marker
    /// (X.691 §17.3): a bit, then the string within the constraint, or a string of
    /// another size after its length.
    ///
    /// # Errors
    ///
    /// A size outside the constraint, for a writer that is [`strict`](Self::strict).
    pub fn octet_string_extensible(
        &mut self,
        octets: &[u8],
        lower: usize,
        upper: usize,
    ) -> Result<(), Error> {
        let outside = octets.len() < lower || octets.len() > upper;
        if outside && self.strict {
            return Err(Error::size_written_outside_root(octets.len(), lower, upper));
        }
        self.bit(outside);
        if outside || self.octet_string(octets, lower, upper).is_err() {
            self.octets_in_fragments(octets);
        }
        Ok(())
    }

    /// An open type (X.691 §11.2): the octets of the complete encoding of a value,
    /// after their length. The octets are written as they are given: those that
    /// [`encode`](crate::encode) gives are one at least (§11.1.4), and no octet, which
    /// is no complete encoding, is written as a length of zero.
    #[inline]
    pub fn open_type(&mut self, octets: &[u8]) {
        self.octets_in_fragments(octets);
    }

    /// Start an open type whose value is written next, by this writer: the place of
    /// its length is kept, and [`end_open_type`](Self::end_open_type) fills it once
    /// the value is written. This writes a value into the octets that hold it, without
    /// octets of its own.
    ///
    /// ```
    /// use oxirush_aper::Writer;
    ///
    /// // An open type that holds an INTEGER (0..65535)
    /// let mut octets = Vec::new();
    /// let mut writer = Writer::new(&mut octets);
    /// let start = writer.begin_open_type();
    /// writer.unsigned(7, 0, 65535)?;
    /// writer.end_open_type(start);
    /// assert_eq!(octets, [0x02, 0x00, 0x07]);
    /// # Ok::<(), oxirush_aper::Error>(())
    /// ```
    #[inline]
    pub fn begin_open_type(&mut self) -> usize {
        self.octet(0);
        self.octets.len()
    }

    /// End the open type that started at `start`: its value is whole, padded to an
    /// octet, and its length is written before it.
    ///
    /// ```
    /// use oxirush_aper::Writer;
    ///
    /// // An open type of 130 octets: from 128, its length takes two octets
    /// let mut octets = Vec::new();
    /// let mut writer = Writer::new(&mut octets);
    /// let start = writer.begin_open_type();
    /// for value in 0..130 {
    ///     writer.unsigned(value, 0, 255)?;
    /// }
    /// writer.end_open_type(start);
    /// assert_eq!(octets[..4], [0x80, 0x82, 0x00, 0x01]);
    /// assert_eq!(octets.len(), 132);
    /// # Ok::<(), oxirush_aper::Error>(())
    /// ```
    #[inline]
    pub fn end_open_type(&mut self, start: usize) {
        self.free = 0;
        // The encoding of no bit is one zero octet (X.691 §11.1.4).
        if self.octets.len() == start {
            self.octets.push(0);
        }
        let length = self.octets.len() - start;
        match length {
            0..=0x7f => self.octets[start - 1] = length as u8,
            _ => self.long_open_type(start, length),
        }
    }

    /// The length of an open type of 128 octets or more, whose value starts at
    /// `start` after one octet that was kept for its length.
    #[cold]
    fn long_open_type(&mut self, start: usize, length: usize) {
        if length < SIXTEEN_K {
            self.octets[start - 1] = 0x80 | (length >> 8) as u8;
            self.octets.insert(start, length as u8);
        } else {
            let value = self.octets.split_off(start);
            self.octets.truncate(start - 1);
            self.octets_in_fragments(&value);
        }
    }

    /// Octets after their unconstrained length, in fragments from 16384 octets.
    #[inline]
    fn octets_in_fragments(&mut self, octets: &[u8]) {
        let mut left = octets;
        loop {
            let (count, more) = self.length(left.len());
            self.octets(&left[..count]);
            left = &left[count..];
            if !more {
                break;
            }
        }
    }

    // ── BIT STRING ──────────────────────────────────────────────────────────

    /// A BIT STRING of 64 bits at most, of the size constraint `lower..=upper` (X.691
    /// §16.9 to §16.11): a bit-field for one size of 16 bits at most, and octet-aligned
    /// otherwise, after its length when its size varies.
    ///
    /// # Errors
    ///
    /// A size outside the constraint is an error.
    #[inline]
    pub fn bits_sized(&mut self, bits: Bits, lower: usize, upper: usize) -> Result<(), Error> {
        debug_assert!(lower <= upper && upper <= 64);
        let size = bits.len();
        self.size(size, lower, upper)?;
        if size == 0 {
            return Ok(());
        }
        if lower != upper || size > 16 {
            self.align();
        }
        self.bits(bits.value(), size);
        Ok(())
    }

    /// A BIT STRING of the size constraint `lower..=upper` (X.691 §16), with
    /// [`UNBOUNDED`] for no upper bound.
    ///
    /// # Errors
    ///
    /// A size outside the constraint is an error.
    #[inline]
    pub fn bit_string(
        &mut self,
        bits: &BitString<'_>,
        lower: usize,
        upper: usize,
    ) -> Result<(), Error> {
        let size = bits.len();
        if upper >= 65536 {
            check_size(size, lower, upper)?;
            self.bit_string_in_fragments(bits);
            return Ok(());
        }
        self.size(size, lower, upper)?;
        if size == 0 {
            return Ok(());
        }
        if lower == upper && size <= 16 {
            self.bit_field(bits.as_octets(), size);
        } else {
            self.aligned_bits(bits.as_octets(), size);
        }
        Ok(())
    }

    /// A BIT STRING whose size constraint `lower..=upper` has an extension marker
    /// (X.691 §16.6): a bit, then the string within the constraint, or a string of
    /// another size after its length.
    ///
    /// # Errors
    ///
    /// A size outside the constraint, for a writer that is [`strict`](Self::strict).
    pub fn bit_string_extensible(
        &mut self,
        bits: &BitString<'_>,
        lower: usize,
        upper: usize,
    ) -> Result<(), Error> {
        let outside = bits.len() < lower || bits.len() > upper;
        if outside && self.strict {
            return Err(Error::size_written_outside_root(bits.len(), lower, upper));
        }
        self.bit(outside);
        if outside || self.bit_string(bits, lower, upper).is_err() {
            self.bit_string_in_fragments(bits);
        }
        Ok(())
    }

    /// The first `count` bits of `octets`, not aligned.
    fn bit_field(&mut self, octets: &[u8], count: usize) {
        let mut left = count;
        for octet in octets {
            let part = left.min(8);
            self.bits(u64::from(*octet) >> (8 - part), part);
            left -= part;
        }
    }

    /// A bit string after its unconstrained length, which counts bits, in fragments
    /// from 16384 bits.
    fn bit_string_in_fragments(&mut self, bits: &BitString<'_>) {
        let mut octets = bits.as_octets();
        let mut left = bits.len();
        loop {
            let (count, more) = self.length(left);
            let whole = count.div_ceil(8);
            self.aligned_bits(&octets[..whole], count);
            octets = &octets[whole..];
            left -= count;
            if !more {
                break;
            }
        }
    }

    // ── Character strings ───────────────────────────────────────────────────

    /// A PrintableString of the size constraint `lower..=upper` (X.691 §30.5), with
    /// [`UNBOUNDED`] for no upper bound.
    ///
    /// # Errors
    ///
    /// A character that the type does not have, and a size outside the constraint,
    /// are errors.
    pub fn printable_string(
        &mut self,
        text: &str,
        lower: usize,
        upper: usize,
    ) -> Result<(), Error> {
        self.known_multiplier(text, lower, upper, false, Alphabet::Printable)
    }

    /// A PrintableString whose size constraint has an extension marker (X.691 §30.4).
    pub fn printable_string_extensible(
        &mut self,
        text: &str,
        lower: usize,
        upper: usize,
    ) -> Result<(), Error> {
        self.known_multiplier(text, lower, upper, true, Alphabet::Printable)
    }

    /// A VisibleString of the size constraint `lower..=upper`, with [`UNBOUNDED`] for
    /// no upper bound.
    pub fn visible_string(&mut self, text: &str, lower: usize, upper: usize) -> Result<(), Error> {
        self.known_multiplier(text, lower, upper, false, Alphabet::Visible)
    }

    /// A VisibleString whose size constraint has an extension marker.
    pub fn visible_string_extensible(
        &mut self,
        text: &str,
        lower: usize,
        upper: usize,
    ) -> Result<(), Error> {
        self.known_multiplier(text, lower, upper, true, Alphabet::Visible)
    }

    /// An IA5String of the size constraint `lower..=upper`, with [`UNBOUNDED`] for no
    /// upper bound.
    pub fn ia5_string(&mut self, text: &str, lower: usize, upper: usize) -> Result<(), Error> {
        self.known_multiplier(text, lower, upper, false, Alphabet::Ia5)
    }

    /// An IA5String whose size constraint has an extension marker.
    pub fn ia5_string_extensible(
        &mut self,
        text: &str,
        lower: usize,
        upper: usize,
    ) -> Result<(), Error> {
        self.known_multiplier(text, lower, upper, true, Alphabet::Ia5)
    }

    /// A UTF8String (X.691 §30.6): the length of its octets, then its octets. A size
    /// constraint on it is not PER-visible (§10.3.7).
    pub fn utf8_string(&mut self, text: &str) {
        self.octets_in_fragments(text.as_bytes());
    }

    /// A string of a known-multiplier type whose characters are eight bits.
    fn known_multiplier(
        &mut self,
        text: &str,
        lower: usize,
        upper: usize,
        extensible: bool,
        alphabet: Alphabet,
    ) -> Result<(), Error> {
        let octets = text.as_bytes();
        if let Some(other) = text
            .chars()
            .find(|character| u8::try_from(*character).map_or(true, |code| !alphabet.has(code)))
        {
            return Err(Error::encode(alloc::format!(
                "{other:?} is no character of {}",
                alphabet.name()
            )));
        }
        let size = octets.len();
        // Outside the root, the string is encoded as one without a size constraint.
        let (lower, upper) = match extensible {
            true => {
                let outside = size < lower || size > upper;
                if outside && self.strict {
                    return Err(Error::size_written_outside_root(size, lower, upper));
                }
                self.bit(outside);
                if outside {
                    (0, UNBOUNDED)
                } else {
                    (lower, upper)
                }
            }
            false => (lower, upper),
        };
        if upper >= 65536 {
            check_size(size, lower, upper)?;
            self.octets_in_fragments(octets);
            return Ok(());
        }
        self.size(size, lower, upper)?;
        // One size of two characters at most, and a size that varies up to one
        // character, are bit-fields (X.691 §30.5.6, §30.5.7).
        let aligned = if lower == upper {
            upper * 8 > 16
        } else {
            upper * 8 >= 16
        };
        match (size, aligned) {
            (0, _) => {}
            (_, true) => self.octets(octets),
            (_, false) => octets
                .iter()
                .for_each(|octet| self.bits((*octet).into(), 8)),
        }
        Ok(())
    }

    // ── OBJECT IDENTIFIER ───────────────────────────────────────────────────

    /// An OBJECT IDENTIFIER (X.691 §24): the length of its contents octets, then the
    /// contents octets that the Basic Encoding Rules give it.
    pub fn object_identifier(&mut self, identifier: &ObjectIdentifier) {
        let contents = identifier.to_contents();
        self.octets_in_fragments(&contents);
    }
}

/// A size that is written after an unconstrained length is within `lower..=upper`.
fn check_size(size: usize, lower: usize, upper: usize) -> Result<(), Error> {
    if (lower..=upper).contains(&size) {
        return Ok(());
    }
    let upper = (upper != UNBOUNDED).then_some(upper);
    Err(Error::size_written_outside(size, lower, upper))
}
