//! Each field of ITU-T X.691 (02/2021) that the crate reads and writes, against the
//! octets that its clause gives: the notes of the clauses where they have an example,
//! and the encoding worked out from the clause otherwise.

use oxirush_aper::{BitString, Bits, ErrorKind, ObjectIdentifier, Reader, UNBOUNDED, Writer};

/// What a writer gives for the fields that `write` writes.
fn written(write: impl FnOnce(&mut Writer<'_>)) -> Vec<u8> {
    let mut octets = Vec::new();
    write(&mut Writer::new(&mut octets));
    octets
}

#[test]
fn a_constrained_whole_number_has_the_four_forms_of_11_5_7() {
    // The bit-field case: the table of §11.5.7.1, by the range.
    for (upper, bits) in [
        (1u64, 1),
        (2, 2),
        (3, 2),
        (4, 3),
        (7, 3),
        (8, 4),
        (15, 4),
        (16, 5),
        (31, 5),
        (32, 6),
        (63, 6),
        (64, 7),
        (127, 7),
        (128, 8),
        (254, 8),
    ] {
        let octets = written(|w| {
            w.unsigned(upper, 0, upper).unwrap();
            // A bit after it shows where the field ends.
            w.bit(true);
        });
        let mut reader = Reader::new(&octets);
        assert_eq!(reader.unsigned(0, upper).unwrap(), upper);
        assert_eq!(reader.position(), bits, "a range of {}", upper + 1);
        assert!(reader.bit().unwrap());
    }
    // One value: no bit (§11.5.4).
    assert!(written(|w| w.unsigned(5, 5, 5).unwrap()).is_empty());
    assert_eq!(Reader::new(&[]).unsigned(5, 5).unwrap(), 5);

    // The one-octet case (§11.5.7.2) and the two-octet case (§11.5.7.3) are
    // octet-aligned.
    let octets = written(|w| {
        w.bit(true);
        w.unsigned(256, 1, 256).unwrap();
        w.bit(true);
        w.unsigned(0x1234, 0, 65535).unwrap();
        w.bit(true);
        w.unsigned(257, 1, 257).unwrap();
    });
    assert_eq!(octets, [0x80, 0xff, 0x80, 0x12, 0x34, 0x80, 0x01, 0x00]);
    let mut reader = Reader::new(&octets);
    assert!(reader.bit().unwrap());
    assert_eq!(reader.unsigned(1, 256).unwrap(), 256);
    assert!(reader.bit().unwrap());
    assert_eq!(reader.unsigned(0, 65535).unwrap(), 0x1234);
    assert!(reader.bit().unwrap());
    assert_eq!(reader.unsigned(1, 257).unwrap(), 257);

    // The indefinite length case (§11.5.7.4), with the length of §13.2.6 a). The note
    // of that clause: `INTEGER (256..1234567) ::= 256` is 00xxxxxx 00000000.
    assert_eq!(
        written(|w| w.unsigned(256, 256, 1234567).unwrap()),
        [0x00, 0x00]
    );
    assert_eq!(
        Reader::new(&[0x00, 0x00]).unsigned(256, 1234567).unwrap(),
        256
    );
    // When the length starts with the two last bits of an octet: 00 00000000.
    let octets = written(|w| {
        w.bits(0b111111, 6);
        w.unsigned(256, 256, 1234567).unwrap();
    });
    assert_eq!(octets, [0b1111_1100, 0x00]);
    // The range of (256..1234567) takes three octets: the length is in two bits.
    assert_eq!(
        written(|w| w.unsigned(1234567, 256, 1234567).unwrap()),
        [0b1000_0000, 0x12, 0xd5, 0x87]
    );
    // Five octets for (0..1099511627775): the length is in three bits.
    assert_eq!(
        written(|w| w.unsigned(1099511627775, 0, 1099511627775).unwrap()),
        [0b1000_0000, 0xff, 0xff, 0xff, 0xff, 0xff]
    );
    assert_eq!(
        written(|w| w.unsigned(1, 0, 1099511627775).unwrap()),
        [0x00, 0x01]
    );
    // Eight octets for every number of 64 bits.
    assert_eq!(
        written(|w| w.unsigned(u64::MAX, 0, u64::MAX).unwrap()),
        [0b1110_0000, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
    );
    assert_eq!(
        Reader::new(&[0b1110_0000, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff])
            .unsigned(0, u64::MAX)
            .unwrap(),
        u64::MAX
    );

    // Bounds that are negative.
    assert_eq!(written(|w| w.signed(-127, -127, 127).unwrap()), [0x00]);
    assert_eq!(written(|w| w.signed(127, -127, 127).unwrap()), [0xfe]);
    assert_eq!(Reader::new(&[0xfe]).signed(-127, 127).unwrap(), 127);
    assert_eq!(
        Reader::new(&[0x00, 0x00])
            .signed(i64::MIN, i64::MAX)
            .unwrap(),
        i64::MIN
    );
    assert_eq!(
        written(|w| w.signed(i64::MAX, i64::MIN, i64::MAX).unwrap()),
        [0b1110_0000, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
    );

    // A number outside its constraint is not written, and a field that holds one is
    // not read: three bits hold 0 to 7 for (0..4).
    let mut octets = Vec::new();
    let error = Writer::new(&mut octets).unsigned(5, 0, 4).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Encode);
    assert_eq!(
        error.to_string(),
        "the value does not encode: 5 is outside its constraint (0..4)"
    );
    let error = Reader::new(&[0b1010_0000]).unsigned(0, 4).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Decode);
    assert_eq!(
        error.to_string(),
        "the octets do not decode: 5 is outside its constraint (0..4)"
    );
    assert!(Writer::new(&mut octets).signed(-128, -127, 127).is_err());
    assert!(octets.is_empty());
}

#[test]
fn an_integer_with_an_extension_marker_has_a_bit_before_it() {
    // §13.1: within the root, as without the marker.
    // The one octet of (0..255) is octet-aligned after the bit.
    assert_eq!(
        written(|w| w.unsigned_extensible(255, 0, 255)),
        [0x00, 0xff]
    );
    assert_eq!(written(|w| w.unsigned_extensible(5, 0, 7)), [0b0101_0000]);
    // Outside it, an unconstrained whole number: its length, then its octets in
    // two's complement (§11.8, §11.4).
    assert_eq!(
        written(|w| w.unsigned_extensible(256, 0, 255)),
        [0x80, 0x02, 0x01, 0x00]
    );
    assert_eq!(
        written(|w| w.signed_extensible(-1, 0, 255)),
        [0x80, 0x01, 0xff]
    );
    assert_eq!(
        written(|w| w.signed_extensible(128, 0, 100)),
        [0x80, 0x02, 0x00, 0x80]
    );
    for (value, octets) in [
        (255u64, vec![0x00, 0xff]),
        (256, vec![0x80, 0x02, 0x01, 0x00]),
        (
            u64::MAX,
            vec![
                0x80, 0x09, 0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            ],
        ),
    ] {
        assert_eq!(written(|w| w.unsigned_extensible(value, 0, 255)), octets);
        assert_eq!(
            Reader::new(&octets).unsigned_extensible(0, 255).unwrap(),
            value
        );
    }
    // What the bindings do not hold: a negative number where the root has none, and
    // more than 64 bits.
    let negative = Reader::new(&[0x80, 0x01, 0xff]).unsigned_extensible(0, 255);
    assert_eq!(
        negative.unwrap_err().to_string(),
        "the octets do not decode: the number does not fit in 64 bits"
    );
    let large = [0x80, 0x09, 0x01, 0, 0, 0, 0, 0, 0, 0, 0];
    assert!(Reader::new(&large).unsigned_extensible(0, 255).is_err());
    assert!(Reader::new(&large).signed_extensible(0, 255).is_err());
    // Octets that only extend the sign are not counted.
    let padded = [
        0x80, 0x0a, 0, 0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    ];
    assert_eq!(
        Reader::new(&padded).unsigned_extensible(0, 255).unwrap(),
        u64::MAX
    );
}

#[test]
fn a_whole_number_without_an_upper_bound_has_its_length_before_it() {
    // An unconstrained whole number (§11.8) is in two's complement, as few octets as
    // hold it (§11.4.6).
    for (value, octets) in [
        (0i64, vec![0x01, 0x00]),
        (127, vec![0x01, 0x7f]),
        (128, vec![0x02, 0x00, 0x80]),
        (-128, vec![0x01, 0x80]),
        (-129, vec![0x02, 0xff, 0x7f]),
        (4096, vec![0x02, 0x10, 0x00]),
        (
            i64::MAX,
            vec![0x08, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        ),
        (i64::MIN, vec![0x08, 0x80, 0, 0, 0, 0, 0, 0, 0]),
    ] {
        assert_eq!(written(|w| w.unconstrained(value)), octets, "{value}");
        assert_eq!(Reader::new(&octets).unconstrained().unwrap(), value);
    }
    // No octet is no number.
    assert!(Reader::new(&[0x00]).unconstrained().is_err());
    assert!(Reader::new(&[0x00]).semi_constrained(0).is_err());
    assert!(
        Reader::new(&[0x80, 0x00])
            .unsigned_extensible(0, 255)
            .is_err()
    );
    // A semi-constrained whole number (§11.7) is what it has above its lower bound.
    for (value, lower, octets) in [
        (0i64, 0i64, vec![0x01, 0x00]),
        (255, 0, vec![0x01, 0xff]),
        (256, 0, vec![0x02, 0x01, 0x00]),
        (4096, 4096, vec![0x01, 0x00]),
        (5, -5, vec![0x01, 0x0a]),
    ] {
        assert_eq!(
            written(|w| w.semi_constrained(value, lower).unwrap()),
            octets
        );
        assert_eq!(Reader::new(&octets).semi_constrained(lower).unwrap(), value);
    }
    assert_eq!(
        written(|w| w.semi_constrained_unsigned(u64::MAX, 0).unwrap()),
        [0x08, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
    );
    assert_eq!(
        Reader::new(&[0x08, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff])
            .semi_constrained_unsigned(0)
            .unwrap(),
        u64::MAX
    );
    let mut octets = Vec::new();
    assert!(Writer::new(&mut octets).semi_constrained(3, 4).is_err());

    // A normally small non-negative whole number (§11.6): six bits after a zero bit up
    // to 63, and a semi-constrained number after a one bit above.
    assert_eq!(written(|w| w.small(0)), [0b0000_0000]);
    assert_eq!(written(|w| w.small(63)), [0b0111_1110]);
    assert_eq!(written(|w| w.small(64)), [0b1000_0000, 0x01, 0x40]);
    for value in [0, 1, 63, 64, 255, 256, 70000] {
        let octets = written(|w| w.small(value));
        assert_eq!(Reader::new(&octets).small().unwrap(), value);
    }
}

#[test]
fn a_length_determinant_has_the_forms_of_11_9() {
    // The notes of §11.9.3.6 and §11.9.3.7: 4 is 0 0000100, and 130 is 10 000000
    // 10000010.
    assert_eq!(
        written(|w| {
            w.length(4);
        }),
        [0x04]
    );
    assert_eq!(
        written(|w| {
            w.length(130);
        }),
        [0x80, 0x82]
    );
    assert_eq!(
        written(|w| {
            w.length(127);
        }),
        [0x7f]
    );
    assert_eq!(
        written(|w| {
            w.length(128);
        }),
        [0x80, 0x80]
    );
    assert_eq!(
        written(|w| {
            w.length(16383);
        }),
        [0xbf, 0xff]
    );

    // Note 2 of §11.9.3.8.1: 144K + 1 items are fragments of 64K, 64K and 16K items,
    // then one item.
    let mut left = 144 * 1024 + 1;
    let mut fragments = Vec::new();
    let octets = written(|w| {
        loop {
            let (count, more) = w.length(left);
            fragments.push(count);
            left -= count;
            if !more {
                break;
            }
        }
    });
    assert_eq!(octets, [0xc4, 0xc4, 0xc1, 0x01]);
    assert_eq!(fragments, [65536, 65536, 16384, 1]);
    let mut reader = Reader::new(&octets);
    assert_eq!(reader.length().unwrap(), (65536, true));
    assert_eq!(reader.length().unwrap(), (65536, true));
    assert_eq!(reader.length().unwrap(), (16384, true));
    assert_eq!(reader.length().unwrap(), (1, false));

    // The note of §11.9.3.8.3: a multiple of 16K is followed by a length of zero.
    let mut fragments = Vec::new();
    let octets = written(|w| {
        let mut left = 32768;
        loop {
            let (count, more) = w.length(left);
            fragments.push((count, more));
            left -= count;
            if !more {
                break;
            }
        }
    });
    assert_eq!(octets, [0xc2, 0x00]);
    assert_eq!(fragments, [(32768, true), (0, false)]);

    // A fragment of no block, and of more than four, is none.
    for first in [0xc0, 0xc5, 0xff] {
        let error = Reader::new(&[first]).length().unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Decode);
    }

    // The note of §11.9.3.3: the length of `IA5String (SIZE (3..6))` is in a bit-field
    // of two bits, that of `(SIZE (40000..40254))` in one of eight bits, that of
    // `(SIZE (0..32000))` in two octets, and `(SIZE (64000))` has none.
    assert_eq!(written(|w| w.size(6, 3, 6).unwrap()), [0b1100_0000]);
    let octets = written(|w| {
        w.bit(true);
        w.size(40254, 40000, 40254).unwrap();
    });
    assert_eq!(octets, [0b1111_1111, 0b0000_0000]);
    let octets = written(|w| {
        w.bit(true);
        w.size(32000, 0, 32000).unwrap();
    });
    assert_eq!(octets, [0x80, 0x7d, 0x00]);
    assert!(written(|w| w.size(64000, 64000, 64000).unwrap()).is_empty());
    assert_eq!(Reader::new(&[0xc0]).size(3, 6).unwrap(), 6);

    // A size outside its constraint.
    let mut octets = Vec::new();
    let error = Writer::new(&mut octets).size(7, 3, 6).unwrap_err();
    assert_eq!(
        error.to_string(),
        "the value does not encode: a size of 7 is outside its constraint (3..6)"
    );
    // Three bits hold 0 to 7 for a size of (1..5).
    let error = Reader::new(&[0b1110_0000]).size(1, 5).unwrap_err();
    assert_eq!(
        error.to_string(),
        "the octets do not decode: a size of 8 is outside its constraint (1..5)"
    );
}

#[test]
fn an_enumerated_and_a_choice_are_an_index() {
    // §14.2: three values are a constrained whole number of (0..2).
    assert_eq!(written(|w| w.index(2, 3, false)), [0b1000_0000]);
    assert_eq!(
        Reader::new(&[0b1000_0000]).enumerated(3, 0, false).unwrap(),
        2
    );
    // Two bits hold an index that no value has.
    let error = Reader::new(&[0b1100_0000])
        .enumerated(3, 0, false)
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "the octets do not decode: 3 is no value of the ENUMERATED"
    );
    // §14.3: with an extension marker a bit comes first, and an addition is a
    // normally small number.
    assert_eq!(written(|w| w.index(2, 3, true)), [0b0100_0000]);
    assert_eq!(written(|w| w.index(3, 3, true)), [0b1000_0000]);
    assert_eq!(written(|w| w.index(5, 3, true)), [0b1000_0010]);
    assert_eq!(
        Reader::new(&[0b1000_0010]).enumerated(3, 4, true).unwrap(),
        5
    );
    // An addition that this version does not have.
    let error = Reader::new(&[0b1000_0010])
        .enumerated(3, 2, true)
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "the octets do not decode: 2 is no value of the ENUMERATED among those after its \
         extension marker"
    );
    // §23.4: a CHOICE of one alternative has no index.
    assert!(written(|w| w.index(0, 1, false)).is_empty());
    assert_eq!(Reader::new(&[]).choice(1, 0, false).unwrap(), 0);
    let error = Reader::new(&[0b1100_0000]).choice(3, 0, false).unwrap_err();
    assert_eq!(
        error.to_string(),
        "the octets do not decode: 3 is no alternative of the CHOICE"
    );
}

#[test]
fn an_octet_string_is_aligned_unless_it_has_two_octets_at_most() {
    // §17.6: one size of two octets at most is a bit-field.
    let octets = written(|w| {
        w.bit(true);
        w.octet_array(&[0xab, 0xcd]);
    });
    assert_eq!(octets, [0b1101_0101, 0b1110_0110, 0b1000_0000]);
    let mut reader = Reader::new(&octets);
    assert!(reader.bit().unwrap());
    assert_eq!(reader.octet_array::<2>().unwrap(), [0xab, 0xcd]);
    // §17.7: one size of more is octet-aligned, without a length.
    let octets = written(|w| {
        w.bit(true);
        w.octet_array(&[1, 2, 3]);
    });
    assert_eq!(octets, [0x80, 1, 2, 3]);
    let mut reader = Reader::new(&octets);
    assert!(reader.bit().unwrap());
    assert_eq!(reader.octet_array::<3>().unwrap(), [1, 2, 3]);
    // The same through the strings whose size is an argument.
    let octets = written(|w| {
        w.bit(true);
        w.octet_string(&[0xab, 0xcd], 2, 2).unwrap();
        w.octet_string(&[1, 2, 3], 3, 3).unwrap();
        w.octet_string(&[], 0, 0).unwrap();
    });
    assert_eq!(octets, [0b1101_0101, 0b1110_0110, 0b1000_0000, 1, 2, 3]);
    let mut reader = Reader::new(&octets);
    assert!(reader.bit().unwrap());
    assert_eq!(reader.octet_string(2, 2).unwrap(), [0xab, 0xcd]);
    assert_eq!(reader.octet_string(3, 3).unwrap(), [1, 2, 3]);
    assert_eq!(reader.octet_string(0, 0).unwrap(), []);

    // §17.8: a size that varies is a length, then the octets, octet-aligned. No octet
    // is nothing, and no padding (§11.9.3.3, note 2).
    let octets = written(|w| {
        w.bit(true);
        w.octet_string(&[0xaa], 0, 3).unwrap();
        w.bit(true);
        w.octet_string(&[], 0, 3).unwrap();
        w.bit(true);
    });
    assert_eq!(octets, [0b1010_0000, 0xaa, 0b1001_0000]);
    let mut reader = Reader::new(&octets);
    assert!(reader.bit().unwrap());
    assert_eq!(reader.octet_string(0, 3).unwrap(), [0xaa]);
    assert!(reader.bit().unwrap());
    assert_eq!(reader.octet_string(0, 3).unwrap(), []);
    assert!(reader.bit().unwrap());

    // A range of 256 sizes or more: the length is octet-aligned, in one octet or two
    // (§11.5.7.2, §11.5.7.3).
    let octets = written(|w| {
        w.bit(true);
        w.octet_string(&[0x11, 0x22, 0x33], 1, 1000).unwrap();
    });
    assert_eq!(octets, [0x80, 0x00, 0x02, 0x11, 0x22, 0x33]);
    let octets = written(|w| {
        w.bit(true);
        w.octet_string(&[0x11], 1, 256).unwrap();
    });
    assert_eq!(octets, [0x80, 0x00, 0x11]);

    // No upper bound: an unconstrained length, in fragments from 16K octets.
    assert_eq!(
        written(|w| w.octet_string(&[7, 8], 0, UNBOUNDED).unwrap()),
        [2, 7, 8]
    );
    let long = vec![0x5a; 40000];
    let octets = written(|w| w.octet_string(&long, 0, UNBOUNDED).unwrap());
    assert_eq!(octets.len(), 1 + 32768 + 2 + 7232);
    assert_eq!(
        (octets[0], octets[32769], octets[32770]),
        (0xc2, 0x9c, 0x40)
    );
    let read = Reader::new(&octets).octet_string(0, UNBOUNDED).unwrap();
    assert_eq!(read, long);
    assert!(!read.is_borrowed());
    // In one piece, the octets are borrowed.
    assert!(
        Reader::new(&[2, 7, 8])
            .octet_string(0, UNBOUNDED)
            .unwrap()
            .is_borrowed()
    );

    // §17.3: with an extension marker a bit comes first, and a string outside the
    // root has an unconstrained length.
    assert_eq!(
        written(|w| w.octet_string_extensible(&[1, 2, 3], 3, 3)),
        [0x00, 1, 2, 3]
    );
    assert_eq!(
        written(|w| w.octet_string_extensible(&[1, 2], 3, 3)),
        [0x80, 2, 1, 2]
    );
    assert_eq!(
        Reader::new(&[0x80, 2, 1, 2])
            .octet_string_extensible(3, 3)
            .unwrap(),
        [1, 2]
    );

    // The octets of an open type are written as they are given. A complete encoding
    // is one octet at least (§11.1.4, §11.2): a value that is written in place has it.
    assert_eq!(written(|w| w.open_type(&[9])), [1, 9]);
    assert_eq!(written(|w| w.open_type(&[])), [0]);
    let in_place = written(|w| {
        let start = w.begin_open_type();
        w.end_open_type(start);
    });
    assert_eq!(in_place, [1, 0]);
}

#[test]
fn a_bit_string_is_aligned_unless_it_has_one_size_of_sixteen_bits_at_most() {
    // §16.9: one size of sixteen bits at most is a bit-field.
    let ten = Bits::new(0b10_0000_0001, 10).unwrap();
    let octets = written(|w| {
        w.bit(true);
        w.bits_sized(ten, 10, 10).unwrap();
        w.bit(true);
    });
    assert_eq!(octets, [0b1100_0000, 0b0011_0000]);
    let mut reader = Reader::new(&octets);
    assert!(reader.bit().unwrap());
    assert_eq!(reader.bits_sized(10, 10).unwrap(), ten);
    assert!(reader.bit().unwrap());

    // §16.10: one size of more is octet-aligned, and what follows it is not.
    let twenty = Bits::new(0xabcde, 20).unwrap();
    let octets = written(|w| {
        w.bit(true);
        w.bits_sized(twenty, 20, 20).unwrap();
        w.bit(true);
    });
    assert_eq!(octets, [0x80, 0xab, 0xcd, 0xe8]);
    let mut reader = Reader::new(&octets);
    assert!(reader.bit().unwrap());
    assert_eq!(reader.bits_sized(20, 20).unwrap(), twenty);
    assert!(reader.bit().unwrap());
    // The same as a long string: the bit after it is not one of its bits.
    let mut reader = Reader::new(&octets);
    assert!(reader.bit().unwrap());
    let read = reader.bit_string(20, 20).unwrap();
    assert_eq!(read, BitString::new(vec![0xab, 0xcd, 0xe0], 20).unwrap());
    assert!(reader.bit().unwrap());

    // §16.11: a size that varies is a length, then the bits, octet-aligned.
    let octets = written(|w| {
        w.bit(true);
        w.bits_sized(Bits::new(0x3f_ffff, 22).unwrap(), 22, 32)
            .unwrap();
    });
    assert_eq!(octets, [0b1000_0000, 0xff, 0xff, 0xfc]);
    let mut reader = Reader::new(&octets);
    assert!(reader.bit().unwrap());
    assert_eq!(
        reader.bits_sized(22, 32).unwrap(),
        Bits::new(0x3f_ffff, 22).unwrap()
    );

    // A key of 256 bits is its 32 octets, which are borrowed.
    let key = BitString::from([0x5a; 32]);
    let octets = written(|w| {
        w.bit(true);
        w.bit_string(&key, 256, 256).unwrap();
    });
    assert_eq!(octets.len(), 33);
    let mut reader = Reader::new(&octets);
    assert!(reader.bit().unwrap());
    assert_eq!(reader.bit_string(256, 256).unwrap(), key);

    // §16.6: with an extension marker a bit comes first.
    let sixteen = BitString::from([0xe0, 0x00]);
    assert_eq!(
        written(|w| w.bit_string_extensible(&sixteen, 16, 16)),
        [0b0111_0000, 0b0000_0000, 0b0000_0000]
    );
    let read = Reader::new(&[0b0111_0000, 0, 0])
        .bit_string_extensible(16, 16)
        .unwrap();
    assert_eq!(read, sixteen);
    let eight = BitString::from([0xe0]);
    assert_eq!(
        written(|w| w.bit_string_extensible(&eight, 16, 16)),
        [0x80, 0x08, 0xe0]
    );
    assert_eq!(
        Reader::new(&[0x80, 0x08, 0xe0])
            .bit_string_extensible(16, 16)
            .unwrap(),
        eight
    );

    // No upper bound: an unconstrained length, which counts bits, in fragments from
    // 16K bits.
    let bits = BitString::new(vec![0xff; 2501], 20001).unwrap_err();
    assert_eq!(bits.kind(), ErrorKind::Argument);
    let mut long = vec![0xff; 2501];
    long[2500] = 0x80;
    let bits = BitString::new(long, 20001).unwrap();
    let octets = written(|w| w.bit_string(&bits, 0, UNBOUNDED).unwrap());
    assert_eq!(octets.len(), 1 + 2048 + 2 + 453);
    assert_eq!(octets[0], 0xc1);
    assert_eq!(Reader::new(&octets).bit_string(0, UNBOUNDED).unwrap(), bits);
}

#[test]
fn a_character_string_is_its_characters_in_eight_bits() {
    // §30.5: a PrintableString of (SIZE (1..150, ...)), as the name of a node.
    let octets = written(|w| w.printable_string_extensible("amf1", 1, 150).unwrap());
    assert_eq!(octets, [0b0000_0001, 0b1000_0000, b'a', b'm', b'f', b'1']);
    let read = Reader::new(&octets)
        .printable_string_extensible(1, 150)
        .unwrap();
    assert_eq!(read, "amf1");
    assert!(matches!(read, std::borrow::Cow::Borrowed(_)));
    // A character that the type has not.
    let mut unwritten = Vec::new();
    let error = Writer::new(&mut unwritten)
        .printable_string_extensible("amf_1", 1, 150)
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "the value does not encode: '_' is no character of PrintableString"
    );
    let error = Reader::new(&[0x00, 0x00, b'_'])
        .printable_string_extensible(1, 150)
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "the octets do not decode: a character string is no PrintableString"
    );
    // A VisibleString has it.
    let octets = written(|w| w.visible_string("amf_1", 0, UNBOUNDED).unwrap());
    assert_eq!(octets, [5, b'a', b'm', b'f', b'_', b'1']);
    assert_eq!(
        Reader::new(&octets).visible_string(0, UNBOUNDED).unwrap(),
        "amf_1"
    );

    // §30.5.6, §30.5.7: one size of two characters is a bit-field, and a size that
    // varies up to two characters is octet-aligned.
    let octets = written(|w| {
        w.bit(true);
        w.ia5_string("ab", 2, 2).unwrap();
    });
    assert_eq!(octets, [0b1011_0000, 0b1011_0001, 0b0000_0000]);
    let mut reader = Reader::new(&octets);
    assert!(reader.bit().unwrap());
    assert_eq!(reader.ia5_string(2, 2).unwrap(), "ab");
    let octets = written(|w| {
        w.bit(true);
        w.ia5_string("ab", 1, 2).unwrap();
    });
    assert_eq!(octets, [0b1100_0000, b'a', b'b']);
    let mut reader = Reader::new(&octets);
    assert!(reader.bit().unwrap());
    assert_eq!(reader.ia5_string(1, 2).unwrap(), "ab");

    // §30.6: a UTF8String is its octets after their length, whatever its size
    // constraint.
    let octets = written(|w| w.utf8_string("né"));
    assert_eq!(octets, [3, b'n', 0xc3, 0xa9]);
    assert_eq!(Reader::new(&octets).utf8_string().unwrap(), "né");
    assert!(Reader::new(&[1, 0xc3]).utf8_string().is_err());
}

#[test]
fn an_object_identifier_is_the_contents_octets_of_ber_after_their_length() {
    // §24, and ITU-T X.690 (02/2021) §8.19.5 for {2 100 3}.
    let identifier = ObjectIdentifier::new(vec![2, 100, 3]).unwrap();
    let octets = written(|w| w.object_identifier(&identifier));
    assert_eq!(octets, [3, 0x81, 0x34, 0x03]);
    assert_eq!(
        Reader::new(&octets).object_identifier().unwrap(),
        identifier
    );
}

#[test]
fn the_additions_of_a_sequence_are_a_bitmap_and_open_types() {
    // §19.7 to §19.9: three additions, of which the first and the last are present.
    let octets = written(|w| {
        w.additions(&[true, false, true]).unwrap();
        w.open_type(&[0xaa]);
        w.open_type(&[0xbb, 0xcc]);
        w.bit(true);
    });
    assert_eq!(
        octets,
        [0b0000_0101, 0b0100_0000, 1, 0xaa, 2, 0xbb, 0xcc, 0x80]
    );
    // A reader that knows the first one.
    let mut reader = Reader::new(&octets);
    let mut additions = reader.additions().unwrap();
    assert!(additions.present());
    assert_eq!(reader.open_type().unwrap(), [0xaa]);
    additions.skip_rest(&mut reader).unwrap();
    assert!(reader.bit().unwrap());
    // One that knows none.
    let mut reader = Reader::new(&octets);
    reader.skip_additions().unwrap();
    assert!(reader.bit().unwrap());
    // One that knows more than the encoding has.
    let mut reader = Reader::new(&octets);
    let mut additions = reader.additions().unwrap();
    let present: Vec<bool> = (0..5).map(|_| additions.present()).collect();
    assert_eq!(present, [true, false, true, false, false]);

    // More than 64 additions: an unconstrained length after a one bit (§11.9.3.4).
    let many: Vec<bool> = (0..70).map(|at| at % 7 == 0).collect();
    let octets = written(|w| w.additions(&many).unwrap());
    assert_eq!(octets[..2], [0x80, 70]);
    let mut reader = Reader::new(&octets);
    let mut additions = reader.additions().unwrap();
    let read: Vec<bool> = (0..70).map(|_| additions.present()).collect();
    assert_eq!(read, many);
    let mut none = Vec::new();
    assert!(Writer::new(&mut none).additions(&[]).is_err());
}

#[test]
fn an_open_type_is_written_into_the_octets_that_hold_it() {
    // A value of 127 octets has a length of one octet, one of 128 a length of two.
    for length in [0usize, 1, 127, 128, 16383, 16384, 40000] {
        let value = vec![0x5a; length];
        let octets = written(|w| {
            w.bit(true);
            let start = w.begin_open_type();
            w.octets(&value);
            w.end_open_type(start);
            w.bit(true);
        });
        // No octet is no complete encoding: in place, it is one zero octet.
        let complete = if value.is_empty() {
            vec![0]
        } else {
            value.clone()
        };
        let separately = written(|w| {
            w.bit(true);
            w.open_type(&complete);
            w.bit(true);
        });
        assert_eq!(octets, separately, "{length} octets");
        let mut reader = Reader::new(&octets);
        assert!(reader.bit().unwrap());
        let read = reader.open_type().unwrap();
        assert_eq!(read, complete);
        assert!(reader.bit().unwrap());
    }
    // A value that ends within an octet is padded to it.
    let octets = written(|w| {
        let start = w.begin_open_type();
        w.bits(0b101, 3);
        w.end_open_type(start);
        w.bit(true);
    });
    assert_eq!(octets, [1, 0b1010_0000, 0x80]);
}

#[test]
fn a_reader_stops_at_the_end_of_the_octets() {
    // Each field, from octets that end before it does.
    let ended = "the octets do not decode: the octets end before the value does";
    assert_eq!(Reader::new(&[]).bit().unwrap_err().to_string(), ended);
    assert_eq!(Reader::new(&[0xff]).bits(9).unwrap_err().to_string(), ended);
    assert_eq!(Reader::new(&[]).octet().unwrap_err().to_string(), ended);
    assert_eq!(
        Reader::new(&[1, 2]).octets(3).unwrap_err().to_string(),
        ended
    );
    assert_eq!(
        Reader::new(&[0x12])
            .unsigned(0, 65535)
            .unwrap_err()
            .to_string(),
        ended
    );
    assert_eq!(
        Reader::new(&[0x80]).length().unwrap_err().to_string(),
        ended
    );
    assert_eq!(
        Reader::new(&[3, 1, 2]).open_type().unwrap_err().to_string(),
        ended
    );
    assert_eq!(
        Reader::new(&[0xc1, 0]).open_type().unwrap_err().to_string(),
        ended
    );
    assert!(Reader::new(&[1, 2]).octet_array::<3>().is_err());
    assert!(Reader::new(&[0xff]).bits_sized(10, 10).is_err());
    assert!(Reader::new(&[0xff; 31]).bit_string(256, 256).is_err());
    assert!(
        Reader::new(&[0x80, 0x10])
            .bit_string_extensible(16, 16)
            .is_err()
    );
    assert!(
        Reader::new(&[0x00, 0x80])
            .printable_string_extensible(1, 150)
            .is_err()
    );
    assert!(Reader::new(&[5, 1]).utf8_string().is_err());
    assert!(Reader::new(&[0b0000_0011]).skip_additions().is_err());

    // Bits near the end, and across it: every count from every position of a few
    // octets is what a reader of one bit at a time gives.
    let octets = [
        0xa5, 0x3c, 0xf0, 0x0f, 0x81, 0x7e, 0x55, 0xaa, 0x12, 0x34, 0x56,
    ];
    for start in 0..octets.len() * 8 {
        for count in 0..=64 {
            let mut reader = Reader::new(&octets);
            (0..start).for_each(|_| assert!(reader.bit().is_ok()));
            let mut bit_by_bit = reader.clone();
            let slowly: Option<u64> = (0..count).try_fold(0u64, |number, _| {
                Some(number << 1 | u64::from(bit_by_bit.bit().ok()?))
            });
            assert_eq!(reader.bits(count).ok(), slowly, "{count} bits from {start}");
        }
    }

    // A complete encoding ends with its value.
    let mut reader = Reader::new(&[0x80, 0x00]);
    assert!(reader.bit().unwrap());
    assert_eq!(
        reader.finish().unwrap_err().to_string(),
        "the octets do not decode: the value is followed by 1 octet"
    );
    // No bit is one zero octet.
    Reader::new(&[0]).finish().unwrap();
    assert!(Reader::new(&[1]).finish().is_err());
    assert!(Reader::new(&[0, 0]).finish().is_err());
}

#[test]
fn a_writer_writes_bits_as_a_reader_reads_them() {
    // Fields of every width at every position, read back.
    let mut state = 0x9e37_79b9_7f4a_7c15u64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for _ in 0..200 {
        let fields: Vec<(u64, usize)> = (0..40)
            .map(|_| {
                let count = (next() % 65) as usize;
                let value = match count {
                    0 => 0,
                    64 => next(),
                    count => next() & ((1 << count) - 1),
                };
                (value, count)
            })
            .collect();
        let octets = written(|w| {
            for (value, count) in &fields {
                w.bits(*value, *count);
            }
        });
        let total: usize = fields.iter().map(|(_, count)| count).sum();
        assert_eq!(octets.len(), total.div_ceil(8));
        let mut reader = Reader::new(&octets);
        for (value, count) in &fields {
            assert_eq!(reader.bits(*count).unwrap(), *value);
        }
        assert_eq!(reader.position(), total);
    }
}
