# oxirush-aper

[![Crates.io](https://img.shields.io/crates/v/oxirush-aper.svg)](https://crates.io/crates/oxirush-aper)
[![Documentation](https://docs.rs/oxirush-aper/badge.svg)](https://docs.rs/oxirush-aper)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

Aligned PER (ITU-T X.691), the encoding of NGAP and S1AP, in Rust: a reader
and a writer for each kind of field, and the Rust types of the values. Most
programs get it through oxirush-ngap or oxirush-s1ap, which re-export the
version they are released with as `oxirush_ngap::aper`; use it directly to
write the codec of another protocol. It is `no_std` with `alloc`, and has no
`unsafe` code.

```rust
use oxirush_aper::{Reader, Writer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut octets = Vec::new();
    let mut writer = Writer::new(&mut octets);
    // unsigned(value, lower, upper, extensible)
    // INTEGER (0..7): three bits
    writer.unsigned(5, 0, 7, false)?;
    // INTEGER (0..255, ...): the extension bit, then one octet, aligned
    writer.unsigned(200, 0, 255, true)?;
    // index(index, values of the root, extensible): the fourth value of an
    // ENUMERATED with 6 root values and an extension marker
    writer.index(3, 6, true);
    // size(count, lower, upper): the count of a SEQUENCE (SIZE (1..256)) OF,
    // then its items
    writer.size(2, 1, 256)?;
    writer.unsigned(1, 0, 1, false)?;
    writer.unsigned(0, 0, 1, false)?;
    assert_eq!(octets, [0xa0, 0xc8, 0x30, 0x01, 0x80]);

    // A reader takes the same bounds, without the value. Its index takes the
    // values of the root, then the number of additions it knows
    let mut reader = Reader::new(&octets);
    assert_eq!(reader.unsigned(0, 7, false)?, 5);
    assert_eq!(reader.unsigned(0, 255, true)?, 200);
    assert_eq!(reader.index(6, 0, true)?, 3);
    assert_eq!(reader.size(1, 256)?, 2);
    assert_eq!((reader.bit()?, reader.bit()?), (true, false));

    let error = Writer::new(&mut Vec::new()).unsigned(8, 0, 7, false).unwrap_err();
    let refused = "the value does not encode: 8 is outside its constraint (0..7)";
    assert_eq!(error.to_string(), refused);
    Ok(())
}
```

## Installation

```toml
[dependencies]
oxirush-aper = "0.4"
```

| Feature | Adds | Needs |
|---------|------|-------|
| `inspect` | JSON trees and paths for generated bindings, as oxirush-ngap and oxirush-s1ap use them | `oxirush-utils` |
| `std` | With `inspect`, the register that reads one protocol's handover containers in the other's messages | The standard library |

The minimum supported Rust version is 1.88.

## Write the codec of a type

A codec writes and reads the fields of a type in order, each with the
bounds of its constraint, as the generated bindings of oxirush-ngap and
oxirush-s1ap do. Here, a SEQUENCE with an extension marker, an OPTIONAL
member, an extensible ENUMERATED, a character string and a SEQUENCE OF:

```rust
use oxirush_aper::{AsnType, Decode, Encode, Error, Reader, Writer};
use std::borrow::Cow;

// Cell ::= SEQUENCE {
//     id          INTEGER (0..4095),
//     kind        ENUMERATED { macro, small, ... },
//     name        PrintableString (SIZE (1..150)) OPTIONAL,
//     neighbours  SEQUENCE (SIZE (0..8)) OF INTEGER (0..4095),
//     ...
// }
#[derive(Debug, PartialEq)]
enum Kind {
    Macro,
    Small,
}

#[derive(Debug, PartialEq)]
struct Cell<'a> {
    id: u16,
    kind: Kind,
    name: Option<Cow<'a, str>>,
    neighbours: Vec<u16>,
}

impl AsnType for Cell<'_> {
    const NAME: &'static str = "Cell";
}

impl<'a> Decode<'a> for Cell<'a> {
    fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
        // The extension bit, then a bit for each member that may be absent
        let extended = reader.bit()?;
        let has_name = reader.bit()?;
        let id = reader.unsigned(0, 4095, false)? as u16;
        // An index among two values of the root, and none added that is known
        let kind = match reader.index(2, 0, true)? {
            0 => Kind::Macro,
            _ => Kind::Small,
        };
        let name = match has_name {
            true => Some(reader.printable_string(1, 150, false)?),
            false => None,
        };
        let count = reader.size(0, 8)?;
        let neighbours = (0..count)
            .map(|_| reader.unsigned(0, 4095, false).map(|id| id as u16))
            .collect::<Result<_, _>>()?;
        // What a later version adds is skipped
        if extended {
            reader.skip_additions()?;
        }
        Ok(Self { id, kind, name, neighbours })
    }
}

impl Encode for Cell<'_> {
    fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
        writer.bit(false);
        writer.bit(self.name.is_some());
        writer.unsigned(self.id.into(), 0, 4095, false)?;
        writer.index(match self.kind { Kind::Macro => 0, Kind::Small => 1 }, 2, true);
        if let Some(name) = &self.name {
            writer.printable_string(name, 1, 150, false)?;
        }
        writer.size(self.neighbours.len(), 0, 8)?;
        for neighbour in &self.neighbours {
            writer.unsigned((*neighbour).into(), 0, 4095, false)?;
        }
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cell = Cell {
        id: 0x123,
        kind: Kind::Small,
        name: Some("cell-1".into()),
        neighbours: vec![0x124, 0x125],
    };
    let octets = oxirush_aper::encode(&cell)?;
    assert_eq!(
        octets,
        [
            0x40, 0x01, 0x23, 0x41, 0x40, 0x63, 0x65, 0x6c, // "cell-1" starts at 0x63
            0x6c, 0x2d, 0x31, 0x20, 0x01, 0x24, 0x01, 0x25,
        ],
    );

    // The name of what is decoded is a slice of the octets
    let read: Cell = oxirush_aper::decode(&octets)?;
    assert_eq!(read, cell);
    assert!(matches!(read.name, Some(Cow::Borrowed("cell-1"))));

    // What its constraints do not hold is an error of the writer and of the reader
    let crowded = Cell { neighbours: vec![0; 9], ..read };
    assert_eq!(
        oxirush_aper::encode(&crowded).unwrap_err().to_string(),
        "a value of Cell does not encode: a size of 9 is outside its constraint (0..8)",
    );
    assert_eq!(
        oxirush_aper::decode::<Cell>(&octets[..5]).unwrap_err().to_string(),
        "these octets do not decode as Cell: the octets end before the value does",
    );
    Ok(())
}
```

## Map ASN.1 types to Rust

| ASN.1 | Rust |
| --- | --- |
| `BOOLEAN`, `NULL` | `bool`, `()` |
| `INTEGER` with a constraint | The native number that holds its range, as `u16` for `(0..65535)` |
| `INTEGER` with an extension marker, or without a constraint | `u64`, or `i64` when its root goes below zero |
| `OCTET STRING (SIZE (n))` | `[u8; n]` |
| Another `OCTET STRING`, an open type | `Octets<'a>` |
| `BIT STRING` of 64 bits at most, without an extension marker | `Bits` |
| Another `BIT STRING` | `BitString<'a>` |
| `PrintableString`, `VisibleString`, `UTF8String` | `Cow<'a, str>` |
| `OBJECT IDENTIFIER` | `ObjectIdentifier` |

`Octets` borrows the octets that were decoded, or owns its own.
`into_owned()` makes a value that outlives its input.

```rust
use oxirush_aper::Octets;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let received = vec![0x7e, 0x00, 0x41];
    let borrowed = Octets::from(&received[..]);
    assert!(borrowed.is_borrowed());

    let kept: Octets<'static> = borrowed.into_owned();
    drop(received);
    assert!(!kept.is_borrowed());
    assert_eq!(format!("{kept:?}"), "7e0041");
    Ok(())
}
```

## Hold a bit string

`Bits` is a number with a length, for the identifiers of 3GPP: it is `Copy`,
and compares as a value. `BitString` holds a string of any length.

```rust
use oxirush_aper::{BitString, Bits};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // AMF Set ID ::= BIT STRING (SIZE (10)): its octets start with its first bit
    let set = Bits::new(0x155, 10)?;
    assert_eq!((set.value(), set.len()), (0x155, 10));
    assert_eq!(set.to_vec(), [0x55, 0x40]);
    assert_eq!(Bits::from_octets(&[0x55, 0x40], 10)?, set);

    // A value that its length does not hold
    assert_eq!(Bits::new(0x400, 10).unwrap_err().to_string(), "1024 does not fit in 10 bits");

    // TransportLayerAddress ::= BIT STRING (SIZE (1..160, ...))
    let address = BitString::from([10, 0, 0, 1]);
    assert_eq!((address.as_octets(), address.len()), (&[10, 0, 0, 1][..], 32));
    Ok(())
}
```

## Read and write an open type

An open type is read as its octets, which borrow from the input, and is
decoded once its type is known. A writer writes it in place, without a
buffer of its own.

```rust
use oxirush_aper::{Reader, Writer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut octets = Vec::new();
    let mut writer = Writer::new(&mut octets);
    let start = writer.begin_open_type();
    writer.unsigned(0x1234, 0, 65535, false)?;
    writer.end_open_type(start);
    assert_eq!(octets, [0x02, 0x12, 0x34]);

    let mut reader = Reader::new(&octets);
    let held = reader.open_type()?;
    assert_eq!(held.as_ref(), [0x12, 0x34]);
    assert_eq!(Reader::new(&held).unsigned(0, 65535, false)?, 0x1234);
    Ok(())
}
```

`Octets::as_borrowed` gives the octets of an open type for as long as the
input lives, to decode a value that borrows from them.

## Read extension additions

```rust
use oxirush_aper::{Reader, Writer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // SEQUENCE { a BOOLEAN, ..., b INTEGER (0..255) OPTIONAL, c BOOLEAN OPTIONAL }
    let mut octets = Vec::new();
    let mut writer = Writer::new(&mut octets);
    writer.bit(true); // additions follow
    writer.bit(true); // a
    writer.additions(&[true, false])?; // b is there, c is not
    let start = writer.begin_open_type();
    writer.unsigned(7, 0, 255, false)?;
    writer.end_open_type(start);

    let mut reader = Reader::new(&octets);
    let extended = reader.bit()?;
    let a = reader.bit()?;
    let mut additions = reader.additions()?;
    let b = match additions.present() {
        true => Some(Reader::new(&reader.open_type()?).unsigned(0, 255, false)?),
        false => None,
    };
    // What a later version of the type added is skipped
    additions.skip_rest(&mut reader)?;
    assert_eq!((extended, a, b), (true, true, Some(7)));
    Ok(())
}
```

## Accept what other encoders write

A writer gives a value its one encoding. A reader also reads a complete
value in another form, as a peer's encoder may write it. A reader made with
`Reader::noting` records each such form as a `Fault`, with its clause and
the bit where it is.

| `Fault` | What is read | X.691 |
|---------|--------------|-------|
| `LengthInTwoOctets` | A length of 127 or less in two octets | 11.9.3.6 |
| `SmallFragment` | A fragment before another that is not the largest | 11.9.3.8.1 |
| `SmallNumberAsLarge` | A normally small number (≤ 63) in the long form | 11.6.1 |
| `LongNumber` | A whole number with octets that only extend it, up to the octets that a number of 64 bits may have | 11.3.6, 11.4.6 |
| `Padding` | Padding bits that are not zero | 11.1.4 |
| `LongArc` | An arc of an object identifier with a leading octet 0x80, which adds no bits | X.690 8.19.2 |

```rust
use oxirush_aper::{Octets, decode, decode_strict, decode_with_faults};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // An OCTET STRING of one octet, whose length is in two octets
    let octets = [0x80, 0x01, 0xaa];
    assert_eq!(decode::<Octets>(&octets)?.as_ref(), [0xaa]);

    // What a tool that tests the peer wants to know
    let (_, faults) = decode_with_faults::<Octets>(&octets)?;
    let fault = "at bit 0, a length of 127 or less is in two octets (X.691 §11.9.3.6)";
    assert_eq!(faults[0].to_string(), fault);

    // And a receiver that takes the one encoding alone
    let error = decode_strict::<Octets>(&octets).unwrap_err();
    let refused = format!(
        "these octets do not decode as OCTET STRING: {fault}: the value is read, \
         and a sender by the clause writes it otherwise"
    );
    assert_eq!(error.to_string(), refused);
    Ok(())
}
```

`decode` and a reader of `Reader::new` note nothing, and `Reader::faults`
has what a noting reader noted. Every reader refuses:

- an extension bit with no addition present;
- a number of no octet;
- a number, size or index outside its constraint;
- octets after a complete encoding;
- any other encoding that announces something that is not there.

## Limits

- It has no unaligned variant.
- It has no `REAL`, time types, `SET` or `SET OF`, `BMPString`,
  `UniversalString` or `NumericString`, and no permitted alphabet
  constraint.
- `DEFAULT` members and extension addition groups have no method of their
  own: the caller writes the bits of the preamble.
- An `INTEGER` beyond 64 bits is a decoding error.
- It does not parse ASN.1: the bindings of oxirush-ngap and oxirush-s1ap are
  generated ahead of time ([CONTRIBUTING.md](CONTRIBUTING.md)).

## Performance

Five values of NGAP, written by hand as the generated bindings of
oxirush-ngap write them, against the derive of
[rasn](https://crates.io/crates/rasn) 0.29 on the same type. Both read and
write the same octets.

| Value (octets) | Decoding | rasn 0.29 | Encoding | rasn 0.29 |
|---|---|---|---|---|
| AMF-UE-NGAP-ID (2) | 18 ns | 2.8× slower | 49 ns | 2.6× slower |
| NAS-PDU (61) | 22 ns | 4.3× slower | 145 ns | 1.4× slower |
| AMFName (18) | 74 ns | 4.9× slower | 116 ns | 3.4× slower |
| GUAMI (7) | 47 ns | 11× slower | 59 ns | 5.1× slower |
| UserLocationInformationNR (19) | 56 ns | 8.4× slower | 174 ns | 3.2× slower |

Medians of ten rounds, the two crates in turns, on one core of an Intel Xeon
E5-2680 v4 (a KVM guest, Linux 7.0, Rust 1.100.0), at 3ae0f3c on
2026-11-15. The rasn cells say how much slower rasn is than this crate; the
medians differ by more than 10 % and the ten times do not overlap for any
value. Decoding these values
allocates nothing. [`benches/README.md`](benches/README.md) has
the cycles, the instructions, the allocations, the method and the command.

## Tests and fuzzing

```sh
cargo test --all-features
```

Each example of this page checks what it gives with `assert_eq!`, and runs
as a documentation test. The worked examples of X.691 Annex A are reproduced
octet for octet, and each field is tested against its clause. Fuzz targets
read fields that the octets choose, and decode, edit and build the PDUs of
oxirush-ngap, oxirush-s1ap and TOY, the protocol of the tests.

[CONTRIBUTING.md](CONTRIBUTING.md) has the generator, the fuzz targets and
how to contribute. [SECURITY.md](SECURITY.md) says how to report a
vulnerability.

## The OxiRush family

- [oxirush-utils](https://github.com/linouxis9/oxirush-utils): the 3GPP identities, the JSON tree with its paths, and the fields and tables that the codecs are laid out on.
- [oxirush-aper](https://github.com/linouxis9/oxirush-aper): aligned PER, the runtime of the NGAP and S1AP codecs.
- [oxirush-nas](https://github.com/linouxis9/oxirush-nas): the NAS messages of 5GS and EPS.
- [oxirush-security](https://github.com/linouxis9/oxirush-security): NAS security, AKA and the derivation of keys.
- [oxirush-ngap](https://github.com/linouxis9/oxirush-ngap): NGAP, between a gNB and an AMF.
- [oxirush-s1ap](https://github.com/linouxis9/oxirush-s1ap): S1AP, between an eNB and an MME.
- [oxirush-gtp-u](https://github.com/linouxis9/oxirush-gtp-u): GTP-U tunnels, with a user-plane test peer.
- [oxirush-pfcp](https://github.com/linouxis9/oxirush-pfcp): PFCP, between the control plane and the user plane.
- [oxirush-sctp](https://github.com/linouxis9/oxirush-sctp): SCTP sockets, which carry NGAP and S1AP.

## License

Copyright 2025-2026 Valentin D'Emmanuele. Licensed under the Apache License,
Version 2.0: see [LICENSE](LICENSE).
