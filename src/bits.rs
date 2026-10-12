//! Bit strings: a short one is a number with a length, a long one is octets with a
//! length.

use alloc::format;
use alloc::vec::Vec;
use core::fmt;

use crate::octets::write_hex;
use crate::{Error, ErrorKind, Octets};

/// A BIT STRING of 64 bits at most: a number with a length.
///
/// The first bit of the string is the most significant bit of the number, as the
/// specifications read an identifier that is a bit string: an AMF Set ID of ten bits is
/// the number of the set.
///
/// ```
/// use oxirush_aper::Bits;
///
/// // An AMF Set ID: ten bits
/// let set = Bits::new(0x155, 10)?;
/// assert_eq!(set.value(), 0x155);
/// assert_eq!(set.len(), 10);
///
/// // A number that does not fit in its bits is refused
/// assert!(Bits::new(1024, 10).is_err());
///
/// // Of the width of a number, without a check
/// assert_eq!(Bits::from(0xE000u16), Bits::new(0xE000, 16)?);
///
/// // As octets, the last one padded with zero bits
/// assert_eq!(set.to_vec(), [0x55, 0x40]);
/// assert_eq!(Bits::from_octets(&[0x55, 0x40], 10)?, set);
/// # Ok::<(), oxirush_aper::Error>(())
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct Bits {
    value: u64,
    length: u8,
}

impl Bits {
    /// No bit.
    pub const EMPTY: Self = Self {
        value: 0,
        length: 0,
    };

    /// The bit string of `length` bits that is the number `value`.
    ///
    /// # Errors
    ///
    /// An error of the kind `Argument` for more than 64 bits, and for a number that
    /// does not fit in `length` bits.
    pub fn new(value: u64, length: usize) -> Result<Self, Error> {
        if length > 64 {
            return Err(Error::new(
                ErrorKind::Argument,
                format!("a bit string of {length} bits is no number of 64 bits: it is a BitString"),
            ));
        }
        if length < 64 && value >> length != 0 {
            return Err(Error::new(
                ErrorKind::Argument,
                format!("{value} does not fit in {length} bits"),
            ));
        }
        Ok(Self {
            value,
            length: length as u8,
        })
    }

    /// The bit string that is the `length` least significant bits of `value`: 64 bits
    /// for a longer one.
    pub const fn low(value: u64, length: usize) -> Self {
        match length {
            0 => Self::EMPTY,
            1..=63 => Self {
                value: value & ((1 << length) - 1),
                length: length as u8,
            },
            _ => Self { value, length: 64 },
        }
    }

    /// The number that the bits are, the first bit the most significant.
    pub const fn value(&self) -> u64 {
        self.value
    }

    /// How many bits the string has.
    pub const fn len(&self) -> usize {
        self.length as usize
    }

    /// Whether the string has no bit.
    pub const fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// The bit at a position, the first one at 0; `None` after the last.
    pub const fn get(&self, position: usize) -> Option<bool> {
        if position >= self.length as usize {
            return None;
        }
        Some(self.value >> (self.length as usize - 1 - position) & 1 == 1)
    }

    /// The bits from the first, in eight octets, with zero bits after the last.
    pub(crate) const fn leading(&self) -> [u8; 8] {
        match self.length {
            0 => [0; 8],
            length => (self.value << (64 - length)).to_be_bytes(),
        }
    }

    /// The octets of the string, the last one padded with zero bits.
    pub fn to_vec(&self) -> Vec<u8> {
        self.leading()[..self.len().div_ceil(8)].to_vec()
    }

    /// The bit string of `length` bits that `octets` hold from their first bit.
    ///
    /// # Errors
    ///
    /// An error of the kind `Argument` for more than 64 bits, for other octets than
    /// `length` bits take, and for bits after the last one that are not zero.
    pub fn from_octets(octets: &[u8], length: usize) -> Result<Self, Error> {
        check_octets(octets, length)?;
        if length > 64 {
            return Self::new(0, length);
        }
        let mut leading = [0; 8];
        leading[..octets.len()].copy_from_slice(octets);
        let value = match length {
            0 => 0,
            length => u64::from_be_bytes(leading) >> (64 - length),
        };
        Ok(Self {
            value,
            length: length as u8,
        })
    }
}

/// Whether `octets` are the octets of a bit string of `length` bits: as many as the
/// bits take, with zero bits after the last one.
fn check_octets(octets: &[u8], length: usize) -> Result<(), Error> {
    if octets.len() != length.div_ceil(8) {
        return Err(Error::new(
            ErrorKind::Argument,
            format!(
                "a bit string of {length} bits has {} octets, not {}",
                length.div_ceil(8),
                octets.len()
            ),
        ));
    }
    let unused = octets.len() * 8 - length;
    if octets
        .last()
        .is_some_and(|last| last.trailing_zeros() < unused as u32)
    {
        return Err(Error::new(
            ErrorKind::Argument,
            format!(
                "the last {unused} bits of the octets are after the {length} bits of the bit \
                 string, and are not zero"
            ),
        ));
    }
    Ok(())
}

impl From<u8> for Bits {
    fn from(value: u8) -> Self {
        Self {
            value: value.into(),
            length: 8,
        }
    }
}

impl From<u16> for Bits {
    fn from(value: u16) -> Self {
        Self {
            value: value.into(),
            length: 16,
        }
    }
}

impl From<u32> for Bits {
    fn from(value: u32) -> Self {
        Self {
            value: value.into(),
            length: 32,
        }
    }
}

impl From<u64> for Bits {
    fn from(value: u64) -> Self {
        Self { value, length: 64 }
    }
}

/// The number in hexadecimal, and the length: `0x155/10`.
impl fmt::Debug for Bits {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#x}/{}", self.value, self.length)
    }
}

/// A BIT STRING of any length: its octets, the last one padded with zero bits, and how
/// many bits it has.
///
/// The octets are borrowed from what was decoded, or owned, as those of [`Octets`]. A
/// string of 64 bits at most is held in the value itself, and never allocates.
///
/// ```
/// use oxirush_aper::BitString;
///
/// // A key of 256 bits: every bit of its octets
/// let key = BitString::from([0x5a; 32]);
/// assert_eq!(key.len(), 256);
/// assert_eq!(key.as_octets().len(), 32);
///
/// // A transport layer address of 20 bits has three octets
/// let bits = BitString::new(vec![0xab, 0xcd, 0xe0], 20)?;
/// assert_eq!(bits.get(0), Some(true));
/// assert_eq!(bits.get(20), None);
///
/// // The bits after the last one are zero
/// assert!(BitString::new(vec![0xab, 0xcd, 0xef], 20).is_err());
/// # Ok::<(), oxirush_aper::Error>(())
/// ```
#[derive(Clone)]
pub struct BitString<'a>(Held<'a>);

#[derive(Clone)]
enum Held<'a> {
    /// 64 bits at most, from the first bit of the octets.
    Short {
        octets: [u8; 8],
        length: u8,
    },
    Long {
        octets: Octets<'a>,
        length: usize,
    },
}

impl<'a> BitString<'a> {
    /// No bit.
    pub const EMPTY: Self = Self(Held::Short {
        octets: [0; 8],
        length: 0,
    });

    /// The bit string of `length` bits that `octets` hold from their first bit.
    ///
    /// # Errors
    ///
    /// An error of the kind `Argument` for other octets than `length` bits take, and
    /// for bits after the last one that are not zero.
    pub fn new(octets: impl Into<Octets<'a>>, length: usize) -> Result<Self, Error> {
        let octets = octets.into();
        check_octets(&octets, length)?;
        Ok(Self::of(octets, length))
    }

    /// The bit string that is every bit of `octets`.
    pub fn from_octets(octets: impl Into<Octets<'a>>) -> Self {
        let octets = octets.into();
        let length = octets.len() * 8;
        Self::of(octets, length)
    }

    /// The bit string of `length` bits whose octets are known to be those of such a
    /// string.
    pub(crate) fn of(octets: Octets<'a>, length: usize) -> Self {
        debug_assert!(check_octets(&octets, length).is_ok());
        if length > 64 {
            return Self(Held::Long { octets, length });
        }
        let mut short = [0; 8];
        short[..octets.len()].copy_from_slice(&octets);
        Self(Held::Short {
            octets: short,
            length: length as u8,
        })
    }

    /// How many bits the string has.
    pub fn len(&self) -> usize {
        match &self.0 {
            Held::Short { length, .. } => usize::from(*length),
            Held::Long { length, .. } => *length,
        }
    }

    /// Whether the string has no bit.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The octets of the string, the last one padded with zero bits.
    pub fn as_octets(&self) -> &[u8] {
        match &self.0 {
            Held::Short { octets, length } => &octets[..usize::from(*length).div_ceil(8)],
            Held::Long { octets, .. } => octets,
        }
    }

    /// The bit at a position, the first one at 0; `None` after the last.
    pub fn get(&self, position: usize) -> Option<bool> {
        if position >= self.len() {
            return None;
        }
        Some(self.as_octets()[position / 8] & (0x80 >> (position % 8)) != 0)
    }

    /// The string as a number, when it has 64 bits at most.
    pub fn to_bits(&self) -> Option<Bits> {
        match &self.0 {
            Held::Short { octets, length } => Some(Bits::low(
                u64::from_be_bytes(*octets)
                    .checked_shr(64 - u32::from(*length))
                    .unwrap_or(0),
                usize::from(*length),
            )),
            Held::Long { .. } => None,
        }
    }

    /// The same string, with octets that it owns: they are copied when they were
    /// borrowed.
    pub fn into_owned(self) -> BitString<'static> {
        BitString(match self.0 {
            Held::Short { octets, length } => Held::Short { octets, length },
            Held::Long { octets, length } => Held::Long {
                octets: octets.into_owned(),
                length,
            },
        })
    }
}

impl Default for BitString<'_> {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl PartialEq for BitString<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.as_octets() == other.as_octets()
    }
}

impl Eq for BitString<'_> {}

impl core::hash::Hash for BitString<'_> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.len().hash(state);
        self.as_octets().hash(state);
    }
}

impl PartialOrd for BitString<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// By the bits, a string before those that it starts.
impl Ord for BitString<'_> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        let octets = self.as_octets().cmp(other.as_octets());
        octets.then(self.len().cmp(&other.len()))
    }
}

impl<'a> From<&'a [u8]> for BitString<'a> {
    fn from(octets: &'a [u8]) -> Self {
        Self::from_octets(octets)
    }
}

impl<'a, const N: usize> From<&'a [u8; N]> for BitString<'a> {
    fn from(octets: &'a [u8; N]) -> Self {
        Self::from_octets(octets)
    }
}

impl From<Vec<u8>> for BitString<'_> {
    fn from(octets: Vec<u8>) -> Self {
        Self::from_octets(octets)
    }
}

impl<const N: usize> From<[u8; N]> for BitString<'_> {
    fn from(octets: [u8; N]) -> Self {
        Self::from_octets(octets)
    }
}

impl From<Bits> for BitString<'_> {
    fn from(bits: Bits) -> Self {
        Self(Held::Short {
            octets: bits.leading(),
            length: bits.len() as u8,
        })
    }
}

/// The octets in hexadecimal, and the length: `ABCDE0/20`.
impl fmt::Debug for BitString<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(self.as_octets(), formatter)?;
        write!(formatter, "/{}", self.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use alloc::vec;

    #[test]
    fn a_short_bit_string_is_a_number_with_a_length() {
        let set = Bits::new(0x155, 10).unwrap();
        assert_eq!((set.value(), set.len()), (0x155, 10));
        assert_eq!(set.to_vec(), [0x55, 0x40]);
        assert_eq!(Bits::from_octets(&[0x55, 0x40], 10).unwrap(), set);
        assert_eq!(set.get(0), Some(false));
        assert_eq!(set.get(1), Some(true));
        assert_eq!(set.get(9), Some(true));
        assert_eq!(set.get(10), None);
        assert_eq!(format!("{set:?}"), "0x155/10");

        // Every length up to 64, and no other.
        assert_eq!(Bits::new(0, 0).unwrap(), Bits::EMPTY);
        assert!(Bits::EMPTY.is_empty());
        assert!(Bits::EMPTY.to_vec().is_empty());
        assert_eq!(Bits::new(u64::MAX, 64).unwrap(), Bits::from(u64::MAX));
        assert_eq!(Bits::from(u64::MAX).to_vec(), [0xff; 8]);
        assert_eq!(Bits::new(1, 65).unwrap_err().kind(), ErrorKind::Argument);
        assert_eq!(Bits::new(1024, 10).unwrap_err().kind(), ErrorKind::Argument);
        assert_eq!(
            Bits::new(1024, 10).unwrap_err().to_string(),
            "1024 does not fit in 10 bits"
        );

        // The least significant bits of a number.
        assert_eq!(Bits::low(0xffff, 10), Bits::new(0x3ff, 10).unwrap());
        assert_eq!(Bits::low(0xffff, 0), Bits::EMPTY);
        assert_eq!(Bits::low(u64::MAX, 64), Bits::from(u64::MAX));
        assert_eq!(Bits::low(u64::MAX, 99).len(), 64);

        // Of the width of a number.
        assert_eq!(Bits::from(0x80u8), Bits::new(0x80, 8).unwrap());
        assert_eq!(Bits::from(0xe000u16).to_vec(), [0xe0, 0x00]);
        assert_eq!(Bits::from(7u32).len(), 32);

        // Octets that are not those of the string.
        assert!(Bits::from_octets(&[0x55], 10).is_err());
        assert!(Bits::from_octets(&[0x55, 0x41], 10).is_err());
        assert!(Bits::from_octets(&[0; 9], 72).is_err());
        assert_eq!(Bits::from_octets(&[], 0).unwrap(), Bits::EMPTY);
    }

    #[test]
    fn a_long_bit_string_is_its_octets_with_a_length() {
        let key = BitString::from([0x5a; 32]);
        assert_eq!(key.len(), 256);
        assert_eq!(key.to_bits(), None);

        let bits = BitString::new(vec![0xab, 0xcd, 0xe0], 20).unwrap();
        assert_eq!(bits.as_octets(), [0xab, 0xcd, 0xe0]);
        assert_eq!(bits.get(0), Some(true));
        assert_eq!(bits.get(1), Some(false));
        assert_eq!(bits.get(19), Some(false));
        assert_eq!(bits.get(20), None);
        assert_eq!(format!("{bits:?}"), "ABCDE0/20");
        assert_eq!(bits.to_bits(), Some(Bits::new(0xabcde, 20).unwrap()));
        assert_eq!(BitString::from(Bits::new(0xabcde, 20).unwrap()), bits);

        assert!(BitString::new(vec![0xab, 0xcd, 0xef], 20).is_err());
        assert!(BitString::new(vec![0xab, 0xcd], 20).is_err());
        assert!(BitString::default().is_empty());
        assert_eq!(BitString::default(), BitString::EMPTY);

        // A short string is the same whatever holds it.
        let short = BitString::new(vec![0xff, 0x80], 9).unwrap();
        assert_eq!(short, BitString::from(Bits::new(0x1ff, 9).unwrap()));
        assert_eq!(short.to_bits(), Some(Bits::new(0x1ff, 9).unwrap()));
        assert_eq!(BitString::EMPTY.to_bits(), Some(Bits::EMPTY));
        assert_eq!(
            BitString::from([0xffu8; 8]).to_bits(),
            Some(Bits::from(u64::MAX))
        );
        assert!(BitString::new(vec![0xa0], 4).unwrap() < BitString::new(vec![0xa0], 8).unwrap());

        let received = [0xffu8, 0x80];
        let borrowed = BitString::new(&received[..], 9).unwrap();
        let kept: BitString<'static> = borrowed.clone().into_owned();
        assert_eq!(kept, borrowed);
    }
}
