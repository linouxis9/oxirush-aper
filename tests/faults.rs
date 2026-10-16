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

//! The encodings that ITU-T X.691 (02/2021) gives one form for, in another form: a
//! value that is whole, which a sender by the clauses writes otherwise. A reader takes
//! each, as the encoder of a peer may write it; one that is asked to notes it as a
//! fault with its clause and the bit at which it is, and a strict decoding refuses it. Each test has the
//! encoding of the clause, which has no fault, and the other form of the same value.

use oxirush_aper::{
    ErrorKind, Fault, Faults, Reader, UNBOUNDED, decode, decode_strict, decode_with_faults,
};

/// What `read` reads from `octets`, to their end, and the faults that a reader which
/// notes them noted. A reader that does not note reads the same value.
fn read<T: PartialEq + std::fmt::Debug>(
    octets: &[u8],
    read: impl Fn(&mut Reader<'_>) -> T,
) -> (T, Faults) {
    let mut reader = Reader::noting(octets);
    let value = read(&mut reader);
    reader.finish().unwrap();
    let mut silent = Reader::new(octets);
    assert_eq!(read(&mut silent), value);
    assert!(silent.faults().is_empty());
    (value, *reader.faults())
}

/// The faults as their kinds and their bits.
fn noted(faults: &Faults) -> Vec<(Fault, usize)> {
    faults
        .iter()
        .map(|noted| (noted.fault, noted.bit))
        .collect()
}

#[test]
fn a_length_of_127_or_less_is_one_octet() {
    // §11.9.3.6: "If "n" is less than or equal to 127, then "n" shall be encoded [...]
    // into bits 7 (most significant) to 1 (least significant) of a single octet and bit
    // 8 shall be set to zero." An OCTET STRING of five octets, after its length.
    let string = |reader: &mut Reader<'_>| reader.octet_string(0, UNBOUNDED).unwrap().to_vec();
    let (value, faults) = read(&[5, 1, 2, 3, 4, 5], string);
    assert_eq!((value, noted(&faults)), (vec![1, 2, 3, 4, 5], vec![]));
    // The same length in the two octets of §11.9.3.7, which are for 128 to 16383.
    let (value, faults) = read(&[0x80, 5, 1, 2, 3, 4, 5], string);
    assert_eq!(value, [1, 2, 3, 4, 5]);
    assert_eq!(noted(&faults), [(Fault::LengthInTwoOctets, 0)]);
    assert_eq!(Fault::LengthInTwoOctets.clause(), "11.9.3.6");
    // 128 is in two octets, and 127 is not.
    let long = [&[0x80, 128][..], &[7; 128]].concat();
    assert!(read(&long, string).1.is_empty());
    let short = [&[0x80, 127][..], &[7; 127]].concat();
    assert_eq!(read(&short, string).1.len(), 1);
}

#[test]
fn a_fragment_before_another_is_the_largest() {
    // §11.9.3.8.1: "The choice of the integer in bits 6 to 1 shall be the maximum allowed
    // value such that the associated field or list of fields contains more than or
    // exactly "m" octets, bits, components or characters". 32768 octets are one fragment
    // of twice 16K, then a length of zero.
    let string = |reader: &mut Reader<'_>| reader.octet_string(0, UNBOUNDED).unwrap().len();
    let one = [&[0xc2][..], &[9; 32768], &[0]].concat();
    let (length, faults) = read(&one, string);
    assert_eq!((length, noted(&faults)), (32768, vec![]));
    // The same octets in two fragments of 16K: the first is not the largest.
    let two = [&[0xc1][..], &[9; 16384], &[0xc1], &[9; 16384], &[0]].concat();
    let (length, faults) = read(&two, string);
    assert_eq!(length, 32768);
    assert_eq!(noted(&faults), [(Fault::SmallFragment, 8 + 16384 * 8)]);
    assert_eq!(Fault::SmallFragment.clause(), "11.9.3.8.1");
    // After a fragment of four times 16K, any other follows: 80K octets.
    let most = [&[0xc4][..], &[9; 65536], &[0xc1], &[9; 16384], &[0]].concat();
    assert!(read(&most, string).1.is_empty());
    // The elements of a list and the bits of a string are counted the same way.
    let bits = |reader: &mut Reader<'_>| reader.bit_string(0, UNBOUNDED).unwrap().len();
    let two = [&[0xc1][..], &[0; 2048], &[0xc1], &[0; 2048], &[0]].concat();
    let (length, faults) = read(&two, bits);
    assert_eq!((length, faults.len()), (32768, 1));
}

#[test]
fn a_normally_small_number_of_63_or_less_is_six_bits() {
    // §11.6.1: "If the non-negative whole number, "n", is less than or equal to 63, then
    // a single-bit bit-field shall be appended to the field-list with the bit set to 0,
    // and "n" shall be encoded as a non-negative-binary-integer in a 6-bit bit-field."
    let small = |reader: &mut Reader<'_>| reader.small().unwrap();
    let (number, faults) = read(&[0b0000_1010], small);
    assert_eq!((number, noted(&faults)), (5, vec![]));
    // The same number as §11.6.2 has 64 and more: a one bit, then a semi-constrained
    // whole number, which is the number of its octets and its octets.
    let (number, faults) = read(&[0x80, 1, 5], small);
    assert_eq!(number, 5);
    assert_eq!(noted(&faults), [(Fault::SmallNumberAsLarge, 0)]);
    assert_eq!(Fault::SmallNumberAsLarge.clause(), "11.6.1");
    // 64 is in that form, and 63 is not.
    assert!(read(&[0x80, 1, 64], small).1.is_empty());
    assert_eq!(read(&[0x80, 1, 63], small).1.len(), 1);
}

#[test]
fn a_whole_number_has_the_least_octets_that_hold_it() {
    // §11.3.6: a non-negative-binary-integer in the least octets: "the leading eight
    // bits of the field shall not all be zero unless the field is precisely eight bits
    // long". A semi-constrained whole number (§11.7.4) of 5.
    let above = |reader: &mut Reader<'_>| reader.semi_constrained(0).unwrap();
    assert_eq!(read(&[1, 5], above), (5, Faults::default()));
    let (number, faults) = read(&[2, 0, 5], above);
    assert_eq!(number, 5);
    assert_eq!(noted(&faults), [(Fault::LongNumber, 8)]);
    assert_eq!(Fault::LongNumber.clause(), "11.3.6, 11.4.6");
    // Zero is one octet of zero.
    assert!(read(&[1, 0], above).1.is_empty());
    // §11.4.6: a 2's-complement-binary-integer in the least octets: "the leading nine
    // bits of the field shall not all be zero and shall not all be ones". An
    // unconstrained whole number (§11.8.3) of 5, of -1, of 128 and of -129.
    let any = |reader: &mut Reader<'_>| reader.unconstrained().unwrap();
    for (least, more, number) in [
        (&[1, 0x05][..], &[2, 0x00, 0x05][..], 5),
        (&[1, 0xff], &[2, 0xff, 0xff], -1),
        (&[2, 0x00, 0x80], &[3, 0x00, 0x00, 0x80], 128),
        (&[2, 0xff, 0x7f], &[3, 0xff, 0xff, 0x7f], -129),
    ] {
        assert_eq!(read(least, any), (number, Faults::default()), "{number}");
        let (read, faults) = read(more, any);
        assert_eq!(read, number);
        assert_eq!(noted(&faults), [(Fault::LongNumber, 8)], "{number}");
    }
    // §11.5.7.4, the constrained whole number of a range above 64K: the number of its
    // octets, less one, in the bits that the range needs, then the octets, which are
    // the least (§11.3.6). 5 of 0..4294967295: two bits, then an octet.
    let wide = |reader: &mut Reader<'_>| reader.unsigned(0, 4_294_967_295).unwrap();
    assert_eq!(read(&[0x00, 5], wide), (5, Faults::default()));
    let (number, faults) = read(&[0x40, 0, 5], wide);
    assert_eq!(number, 5);
    assert_eq!(noted(&faults), [(Fault::LongNumber, 8)]);
}

#[test]
fn padding_bits_are_zero() {
    // §11.1.4: "any octet-aligned bit-fields shall be concatenated after (zero to seven)
    // zero bits have been concatenated to make the length of the encoding produced so far
    // a multiple of eight bits". INTEGER (0..7), then INTEGER (0..65535) in two octets.
    let two = |reader: &mut Reader<'_>| {
        let first = reader.unsigned(0, 7).unwrap();
        (first, reader.unsigned(0, 65535).unwrap())
    };
    assert_eq!(
        read(&[0b1010_0000, 0x12, 0x34], two),
        ((5, 0x1234), Faults::default())
    );
    let (numbers, faults) = read(&[0b1010_0100, 0x12, 0x34], two);
    assert_eq!(numbers, (5, 0x1234));
    assert_eq!(noted(&faults), [(Fault::Padding, 3)]);
    assert_eq!(Fault::Padding.clause(), "11.1.4");
    assert_eq!(Fault::Padding.standard(), "X.691");
    // Before an octet, before octets and before the bits of a string that is aligned.
    let octet = |reader: &mut Reader<'_>| {
        let first = reader.bit().unwrap();
        (first, reader.octet().unwrap())
    };
    assert_eq!(noted(&read(&[0x81, 7], octet).1), [(Fault::Padding, 1)]);
    let string = |reader: &mut Reader<'_>| {
        let first = reader.bit().unwrap();
        (first, reader.octet_string(3, 3).unwrap().to_vec())
    };
    assert_eq!(
        noted(&read(&[0xc0, 1, 2, 3], string).1),
        [(Fault::Padding, 1)]
    );
    let bits = |reader: &mut Reader<'_>| {
        let first = reader.bit().unwrap();
        (first, reader.bit_string(24, 24).unwrap().len())
    };
    assert_eq!(
        noted(&read(&[0xa0, 1, 2, 3], bits).1),
        [(Fault::Padding, 1)]
    );
    // "If it is a non-empty bit string and it is not a multiple of eight bits, (zero to
    // seven) zero bits shall be appended to it": the end of a complete encoding.
    assert_eq!(
        decode_with_faults::<bool>(&[0x80]),
        Ok((true, Faults::default()))
    );
    let (value, faults) = decode_with_faults::<bool>(&[0x81]).unwrap();
    assert!(value);
    assert_eq!(noted(&faults), [(Fault::Padding, 1)]);
}

#[test]
fn an_arc_of_an_object_identifier_is_the_fewest_octets_that_hold_it() {
    // X.690 §8.19.2, which §24 of X.691 takes for the contents octets: "The subidentifier
    // shall be encoded in the fewest possible octets, that is, the leading octet of the
    // subidentifier shall not have the value 80 (hexadecimal)." {1 2 840} after the
    // length of its contents: 42, then 840 in two octets.
    let identifier = |reader: &mut Reader<'_>| reader.object_identifier().unwrap().arcs().to_vec();
    let (arcs, faults) = read(&[3, 0x2a, 0x86, 0x48], identifier);
    assert_eq!((arcs, noted(&faults)), (vec![1, 2, 840], vec![]));
    // 840 after an octet that holds nothing.
    let (arcs, faults) = read(&[4, 0x2a, 0x80, 0x86, 0x48], identifier);
    assert_eq!(arcs, [1, 2, 840]);
    assert_eq!(noted(&faults), [(Fault::LongArc, 16)]);
    assert_eq!(
        (Fault::LongArc.standard(), Fault::LongArc.clause()),
        ("X.690", "8.19.2")
    );
    assert_eq!(
        faults[0].to_string(),
        "at bit 16, an arc of an object identifier starts with an octet that holds nothing (X.690 \
         §8.19.2)"
    );
    // 0x80 in an arc after its first octet is a part of the number: 16384 is 81 80 00.
    let (arcs, faults) = read(&[4, 0x2a, 0x81, 0x80, 0x00], identifier);
    assert_eq!((arcs, faults.len()), (vec![1, 2, 16384], 0));
}

#[test]
fn a_decoding_takes_them_and_a_strict_one_refuses_them() {
    // The bit of a BOOLEAN, with padding that is not zero.
    assert_eq!(decode::<bool>(&[0x81]), Ok(true));
    assert_eq!(decode_strict::<bool>(&[0x80]), Ok(true));
    let error = decode_strict::<bool>(&[0x81]).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Decode);
    assert_eq!(
        error.to_string(),
        "these octets do not decode as BOOLEAN: at bit 1, padding bits are not zero (X.691 \
         §11.1.4): the value is read, and a sender by the clause writes it otherwise"
    );
    // What does not decode is the same error in both.
    assert_eq!(decode_strict::<bool>(&[]), decode::<bool>(&[]));
    assert_eq!(
        decode_strict::<bool>(&[0x80, 0]),
        decode::<bool>(&[0x80, 0])
    );
}

#[test]
fn the_faults_are_counted_and_the_first_are_kept() {
    // Six lengths in two octets, each before one octet: the list holds the first four,
    // in their order, and knows how many there were.
    let octets = [0x80, 1, 9].repeat(6);
    let mut reader = Reader::noting(&octets);
    for _ in 0..6 {
        assert_eq!(reader.octet_string(0, UNBOUNDED).unwrap(), [9]);
    }
    let faults = reader.faults();
    assert_eq!(faults.count(), 6);
    assert_eq!(faults.len(), 4);
    let bits: Vec<usize> = faults.iter().map(|noted| noted.bit).collect();
    assert_eq!(bits, [0, 24, 48, 72]);
    assert_eq!(
        faults[0].to_string(),
        "at bit 0, a length of 127 or less is in two octets (X.691 §11.9.3.6)"
    );
    // A reader of octets by the clauses has none.
    let reader = Reader::noting(&[]);
    assert!(reader.faults().is_empty());
    assert_eq!(reader.faults().count(), 0);
}
