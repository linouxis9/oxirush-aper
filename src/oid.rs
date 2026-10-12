//! OBJECT IDENTIFIER values.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use crate::{Error, ErrorKind};

/// An OBJECT IDENTIFIER: its arcs.
///
/// ```
/// use oxirush_aper::ObjectIdentifier;
///
/// let identifier = ObjectIdentifier::new(vec![0, 4, 0, 1989, 0])?;
/// assert_eq!(identifier.arcs(), [0, 4, 0, 1989, 0]);
/// assert_eq!(identifier.to_string(), "0.4.0.1989.0");
///
/// // The first arc is 0, 1 or 2, and the second is below 40 under 0 and 1
/// assert!(ObjectIdentifier::new(vec![3, 1]).is_err());
/// assert!(ObjectIdentifier::new(vec![1, 40]).is_err());
/// # Ok::<(), oxirush_aper::Error>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjectIdentifier(Vec<u32>);

impl ObjectIdentifier {
    /// The object identifier of these arcs.
    ///
    /// # Errors
    ///
    /// An error of the kind `Argument` for fewer than two arcs, a first arc above 2,
    /// and a second arc that the first one does not have (ITU-T X.660 (07/2011)
    /// §A.2): 40 or more under 0 and 1.
    pub fn new(arcs: Vec<u32>) -> Result<Self, Error> {
        let argument = |what: String| Error::new(ErrorKind::Argument, what);
        match arcs[..] {
            [] | [_] => Err(argument(format!(
                "an object identifier has two arcs at least, not {}",
                arcs.len()
            ))),
            [first, ..] if first > 2 => Err(argument(format!(
                "the first arc of an object identifier is 0, 1 or 2, not {first}"
            ))),
            [first, second, ..] if first < 2 && second >= 40 => Err(argument(format!(
                "the second arc of an object identifier is below 40 under {first}, not {second}"
            ))),
            [2, second, ..] if second > u32::MAX - 80 => Err(argument(format!(
                "the second arc of an object identifier, {second}, is more than this crate holds"
            ))),
            _ => Ok(Self(arcs)),
        }
    }

    /// The arcs.
    pub fn arcs(&self) -> &[u32] {
        &self.0
    }

    /// The contents octets that the Basic Encoding Rules give the identifier (ITU-T
    /// X.690 (02/2021) §8.19): the first two arcs in one number, then each arc, seven
    /// bits to an octet, the most significant first, with a bit that says that another
    /// octet follows.
    pub(crate) fn to_contents(&self) -> Vec<u8> {
        let mut contents = Vec::with_capacity(self.0.len() + 2);
        let first = u64::from(self.0[0]) * 40 + u64::from(self.0[1]);
        let rest = self.0[2..].iter().map(|arc| u64::from(*arc));
        for number in core::iter::once(first).chain(rest) {
            let septets = (64 - number.leading_zeros() as usize).div_ceil(7).max(1);
            for septet in (0..septets).rev() {
                let more = if septet > 0 { 0x80 } else { 0x00 };
                contents.push((number >> (septet * 7)) as u8 & 0x7f | more);
            }
        }
        contents
    }

    /// The identifier of contents octets.
    pub(crate) fn from_contents(contents: &[u8]) -> Result<Self, Error> {
        let mut arcs = Vec::new();
        let mut number: u64 = 0;
        let mut started = false;
        for octet in contents {
            // A writer gives a number as few octets as hold it (X.690 §8.19.2). One that
            // starts with octets that hold nothing is read as the number that it is.
            started = true;
            number = number << 7 | u64::from(octet & 0x7f);
            if number > u64::from(u32::MAX) {
                return Err(Error::decode(
                    "an arc of an object identifier is more than 32 bits",
                ));
            }
            if octet & 0x80 == 0 {
                match arcs.is_empty() {
                    true => {
                        let first = (number / 40).min(2);
                        arcs.push(first as u32);
                        arcs.push((number - first * 40) as u32);
                    }
                    false => arcs.push(number as u32),
                }
                (number, started) = (0, false);
            }
        }
        if started || arcs.is_empty() {
            return Err(Error::decode(
                "the octets of an object identifier end before its last arc does",
            ));
        }
        Ok(Self(arcs))
    }
}

/// The arcs in decimal, with a full stop between them: `0.4.0.1989.0`.
impl fmt::Display for ObjectIdentifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (at, arc) in self.0.iter().enumerate() {
            if at > 0 {
                formatter.write_str(".")?;
            }
            write!(formatter, "{arc}")?;
        }
        Ok(())
    }
}

/// The identifier that `text` writes: its arcs in decimal, with a full stop between
/// them.
impl core::str::FromStr for ObjectIdentifier {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Error> {
        let arcs: Result<Vec<u32>, _> = text.split('.').map(str::parse).collect();
        match arcs {
            Ok(arcs) => Self::new(arcs),
            Err(_) => Err(Error::new(
                ErrorKind::Argument,
                format!(
                    "{text:?} is no object identifier: its arcs are written in decimal, with a \
                     full stop between them"
                ),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use alloc::vec;

    #[test]
    fn an_object_identifier_is_its_arcs_and_the_contents_octets_of_ber() {
        // ITU-T X.690 (02/2021) §8.19.5: {joint-iso-itu-t 100 3} is 0x813403.
        let example = ObjectIdentifier::new(vec![2, 100, 3]).unwrap();
        assert_eq!(example.to_contents(), [0x81, 0x34, 0x03]);
        assert_eq!(
            ObjectIdentifier::from_contents(&[0x81, 0x34, 0x03]).unwrap(),
            example
        );
        assert_eq!(example.to_string(), "2.100.3");
        assert_eq!("2.100.3".parse::<ObjectIdentifier>().unwrap(), example);

        for arcs in [
            vec![0, 0],
            vec![0, 39],
            vec![1, 39, 0],
            vec![2, 0],
            vec![2, 999, u32::MAX, 0, 127, 128, 16383, 16384],
            vec![0, 4, 0, 1989, 0],
        ] {
            let identifier = ObjectIdentifier::new(arcs.clone()).unwrap();
            let contents = identifier.to_contents();
            let read = ObjectIdentifier::from_contents(&contents).unwrap();
            assert_eq!(read.arcs(), arcs);
        }

        for arcs in [vec![], vec![1], vec![3, 0], vec![0, 40], vec![1, 40]] {
            let error = ObjectIdentifier::new(arcs).unwrap_err();
            assert_eq!(error.kind(), ErrorKind::Argument);
        }
        assert!("1.2.x".parse::<ObjectIdentifier>().is_err());
        assert!("".parse::<ObjectIdentifier>().is_err());

        // An arc that starts with an octet that holds nothing is the number that it is.
        let padded = ObjectIdentifier::from_contents(&[0x2a, 0x80, 0x01]).unwrap();
        assert_eq!(padded.arcs(), [1, 2, 1]);
        // No arc, an arc that does not end, and one of more than 32 bits.
        for contents in [&[][..], &[0x81], &[0x2a, 0x90, 0x80, 0x80, 0x80, 0x00]] {
            let error = ObjectIdentifier::from_contents(contents).unwrap_err();
            assert_eq!(error.kind(), ErrorKind::Decode, "{contents:02x?}");
        }
    }
}
