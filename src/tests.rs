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

//! The length determinants of the reader and of the writer, and the fragments of a value
//! that they loop on (X.691 §11.9). These run with the unit tests, before the tests of
//! `tests/`: a length that is read or written wrong makes a loop of fragments there that
//! never ends, which holds them, and these fail at once. A value in fragments is checked
//! after the lengths that it is written with, in the same test, for the same reason.

use crate::{BitString, Reader, UNBOUNDED, Writer};
use alloc::vec;
use alloc::vec::Vec;

/// A length determinant of each form is read as its form says: one octet up to 127, two up
/// to 16383, and a fragment of 16K items after which another length comes.
#[test]
fn the_reader_reads_each_form_of_a_length() {
    let mut reader = Reader::new(&[0x05, 0x80, 0x82, 0xc1, 0x05]);
    assert_eq!(reader.length().unwrap(), (5, false));
    assert_eq!(reader.length().unwrap(), (130, false));
    assert_eq!(reader.length().unwrap(), (16384, true));
    assert_eq!(reader.length_after(16384).unwrap(), (5, false));
    assert!(reader.finish().is_ok());
}

/// A length determinant of each form is written as its form says, and the value of more
/// than 16383 items in fragments, each after its length: an octet string, a bit string and
/// the bitmap of the additions of a sequence.
#[test]
fn the_writer_writes_each_form_of_a_length_and_the_fragments_of_a_value() {
    let mut octets = Vec::new();
    let mut writer = Writer::new(&mut octets);
    assert_eq!(writer.length(5), (5, false));
    assert_eq!(writer.length(130), (130, false));
    assert_eq!(writer.length(40000), (32768, true));
    assert_eq!(writer.length(7232), (7232, false));
    assert_eq!(octets, [0x05, 0x80, 0x82, 0xc2, 0x9c, 0x40]);

    // 20000 octets: a fragment of 16384, then 3616 after their length
    let value: Vec<u8> = (0..20000).map(|index| index as u8).collect();
    let mut octets = Vec::new();
    Writer::new(&mut octets)
        .octet_string(&value, 0, UNBOUNDED, false)
        .unwrap();
    let mut expected = vec![0xc1];
    expected.extend_from_slice(&value[..16384]);
    expected.extend_from_slice(&[0x8e, 0x20]);
    expected.extend_from_slice(&value[16384..]);
    assert!(octets == expected, "{} octets", octets.len());

    // 20000 bits, in the same fragments
    let value: Vec<u8> = (0..2500).map(|index| index as u8).collect();
    let bits = BitString::new(value.as_slice(), 20000).unwrap();
    let mut octets = Vec::new();
    Writer::new(&mut octets)
        .bit_string(&bits, 0, UNBOUNDED, false)
        .unwrap();
    let mut expected = vec![0xc1];
    expected.extend_from_slice(&value[..2048]);
    expected.extend_from_slice(&[0x8e, 0x20]);
    expected.extend_from_slice(&value[2048..]);
    assert!(octets == expected, "{} octets", octets.len());

    // The bitmap of 16385 additions: its bit, a fragment of 16384, then the last addition
    // after its length
    let mut octets = Vec::new();
    Writer::new(&mut octets).additions(&[true; 16385]).unwrap();
    let mut expected = vec![0x80, 0xc1];
    expected.extend_from_slice(&[0xff; 2048]);
    expected.extend_from_slice(&[0x01, 0x80]);
    assert!(octets == expected, "{} octets", octets.len());
}
