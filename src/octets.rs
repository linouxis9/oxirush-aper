//! Octets that are borrowed from what was decoded, or owned.

use alloc::borrow::Cow;
use alloc::vec::Vec;
use core::fmt;
use core::ops::Deref;

/// The octets of an OCTET STRING or of an open type: borrowed from the octets that
/// they were decoded from, or owned.
///
/// A decoder borrows: the NAS-PDU of a message is a slice of the PDU, and reading it
/// allocates nothing. A value that is built owns its octets, or borrows them from the
/// caller. [`into_owned`](Self::into_owned) makes a value that borrows nothing, which
/// is kept after the octets that it was decoded from are gone.
///
/// ```
/// use oxirush_aper::Octets;
///
/// // Borrowed, and owned
/// let received = [0x7e, 0x00, 0x41];
/// let borrowed = Octets::from(&received[..]);
/// let owned = Octets::from(vec![0x7e, 0x00, 0x41]);
/// assert_eq!(borrowed, owned);
///
/// // It reads as a slice
/// assert_eq!(borrowed.len(), 3);
/// assert_eq!(borrowed[0], 0x7e);
/// assert_eq!(borrowed, [0x7e, 0x00, 0x41]);
///
/// // Kept after `received` is gone
/// let kept: Octets<'static> = borrowed.into_owned();
/// assert_eq!(format!("{kept:?}"), "7E0041");
/// ```
#[derive(Clone, Default)]
pub struct Octets<'a>(Cow<'a, [u8]>);

impl<'a> Octets<'a> {
    /// No octet.
    pub const EMPTY: Self = Self(Cow::Borrowed(&[]));

    /// Octets that are borrowed.
    pub const fn borrowed(octets: &'a [u8]) -> Self {
        Self(Cow::Borrowed(octets))
    }

    /// Octets that are owned.
    pub const fn owned(octets: Vec<u8>) -> Octets<'static> {
        Octets(Cow::Owned(octets))
    }

    /// The octets.
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// Whether the octets are borrowed.
    pub fn is_borrowed(&self) -> bool {
        matches!(self.0, Cow::Borrowed(_))
    }

    /// The octets for as long as what they are borrowed from, when they are borrowed:
    /// the slice outlives this value.
    pub fn as_borrowed(&self) -> Option<&'a [u8]> {
        match self.0 {
            Cow::Borrowed(octets) => Some(octets),
            Cow::Owned(_) => None,
        }
    }

    /// The same octets, owned and kept as long as they are needed: `Octets<'static>`
    /// is the type to write in a struct. They are copied when they were borrowed.
    #[doc(alias("to_owned", "owned", "keep"))]
    pub fn into_owned(self) -> Octets<'static> {
        Octets(Cow::Owned(self.0.into_owned()))
    }

    /// The octets as a vector: they are copied when they were borrowed.
    pub fn into_vec(self) -> Vec<u8> {
        self.0.into_owned()
    }

    /// The octets, to change them: they are copied first when they were borrowed.
    pub fn to_mut(&mut self) -> &mut Vec<u8> {
        self.0.to_mut()
    }
}

impl Deref for Octets<'_> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.0
    }
}

impl AsRef<[u8]> for Octets<'_> {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl<'a> From<&'a [u8]> for Octets<'a> {
    fn from(octets: &'a [u8]) -> Self {
        Self(Cow::Borrowed(octets))
    }
}

impl<'a, const N: usize> From<&'a [u8; N]> for Octets<'a> {
    fn from(octets: &'a [u8; N]) -> Self {
        Self(Cow::Borrowed(octets))
    }
}

impl<'a> From<&'a Vec<u8>> for Octets<'a> {
    fn from(octets: &'a Vec<u8>) -> Self {
        Self(Cow::Borrowed(octets))
    }
}

impl From<Vec<u8>> for Octets<'_> {
    fn from(octets: Vec<u8>) -> Self {
        Self(Cow::Owned(octets))
    }
}

impl<const N: usize> From<[u8; N]> for Octets<'_> {
    fn from(octets: [u8; N]) -> Self {
        Self(Cow::Owned(octets.to_vec()))
    }
}

impl<'a> From<Cow<'a, [u8]>> for Octets<'a> {
    fn from(octets: Cow<'a, [u8]>) -> Self {
        Self(octets)
    }
}

impl<'a> From<Octets<'a>> for Vec<u8> {
    fn from(octets: Octets<'a>) -> Self {
        octets.into_vec()
    }
}

impl PartialEq for Octets<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl Eq for Octets<'_> {}

impl PartialOrd for Octets<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Octets<'_> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.as_slice().cmp(other.as_slice())
    }
}

impl core::hash::Hash for Octets<'_> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state);
    }
}

impl PartialEq<[u8]> for Octets<'_> {
    fn eq(&self, other: &[u8]) -> bool {
        self.as_slice() == other
    }
}

impl PartialEq<&[u8]> for Octets<'_> {
    fn eq(&self, other: &&[u8]) -> bool {
        self.as_slice() == *other
    }
}

impl PartialEq<Vec<u8>> for Octets<'_> {
    fn eq(&self, other: &Vec<u8>) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<const N: usize> PartialEq<[u8; N]> for Octets<'_> {
    fn eq(&self, other: &[u8; N]) -> bool {
        self.as_slice() == other
    }
}

impl PartialEq<Octets<'_>> for [u8] {
    fn eq(&self, other: &Octets<'_>) -> bool {
        self == other.as_slice()
    }
}

impl PartialEq<Octets<'_>> for Vec<u8> {
    fn eq(&self, other: &Octets<'_>) -> bool {
        self.as_slice() == other.as_slice()
    }
}

/// As a type of its own, octets are an OCTET STRING without a size constraint.
impl crate::AsnType for Octets<'_> {
    const NAME: &'static str = "OCTET STRING";
}

impl<'a> crate::Decode<'a> for Octets<'a> {
    fn decode(reader: &mut crate::Reader<'a>) -> Result<Self, crate::Error> {
        reader.octet_string(0, crate::UNBOUNDED)
    }
}

impl crate::Encode for Octets<'_> {
    fn encode(&self, writer: &mut crate::Writer<'_>) -> Result<(), crate::Error> {
        writer.octet_string(self, 0, crate::UNBOUNDED)
    }
}

#[cfg(feature = "inspect")]
impl crate::json::Tree for Octets<'_> {
    const NAME: &'static str = "OCTET STRING";

    fn to_tree(&self) -> crate::json::Value {
        crate::json::octets(self)
    }

    fn from_tree(
        value: &crate::json::Value,
        reading: &mut crate::json::Reading,
    ) -> Result<Self, crate::json::Refused> {
        crate::json::read_octets(reading, value)
    }
}

/// The octets in hexadecimal, in upper case.
impl fmt::Debug for Octets<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(self.as_slice(), formatter)
    }
}

/// Octets in hexadecimal, in upper case.
pub(crate) fn write_hex(octets: &[u8], formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    for octet in octets {
        write!(formatter, "{octet:02X}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;
    use alloc::vec;

    #[test]
    fn octets_are_borrowed_or_owned_and_compare_by_what_they_hold() {
        let received = vec![1u8, 2, 3];
        let borrowed = Octets::from(&received);
        assert!(borrowed.is_borrowed());
        let owned = Octets::from(vec![1u8, 2, 3]);
        assert!(!owned.is_borrowed());
        assert_eq!(borrowed, owned);
        assert_eq!(borrowed, [1, 2, 3]);
        assert_eq!(borrowed, vec![1, 2, 3]);
        assert_eq!(borrowed, &[1u8, 2, 3][..]);
        assert_eq!(received, borrowed);
        let (less, more) = (Octets::from([1u8, 2]), Octets::from([1u8, 3]));
        assert!(less < more);
        assert_eq!(Octets::EMPTY.len(), 0);
        assert_eq!(Octets::default(), Octets::EMPTY);

        let kept: Octets<'static> = borrowed.clone().into_owned();
        assert!(!kept.is_borrowed());
        assert_eq!(kept.clone().into_vec(), [1, 2, 3]);
        assert_eq!(Vec::from(kept), [1, 2, 3]);

        let mut changed = borrowed;
        changed.to_mut().push(4);
        assert_eq!(changed, [1, 2, 3, 4]);
        assert_eq!(format!("{changed:?}"), "01020304");
    }
}
