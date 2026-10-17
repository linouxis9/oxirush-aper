# oxirush-aper

[![Crates.io](https://img.shields.io/crates/v/oxirush-aper.svg)](https://crates.io/crates/oxirush-aper)
[![Documentation](https://docs.rs/oxirush-aper/badge.svg)](https://docs.rs/oxirush-aper)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

Aligned PER (ITU-T X.691) for Rust bindings generated from 3GPP ASN.1: a reader
and a writer of the fields, and the types of the values. `no_std`, and without
a feature no dependency.

```rust
use oxirush_aper::{AsnType, Decode, Encode, Error, Octets, Reader, Writer};

// Sample ::= SEQUENCE { id INTEGER (0..65535), payload OCTET STRING OPTIONAL, ... }
#[derive(Debug, PartialEq)]
struct Sample<'a> {
    id: u16,
    payload: Option<Octets<'a>>,
}

impl AsnType for Sample<'_> {
    const NAME: &'static str = "Sample";
}

impl<'a> Decode<'a> for Sample<'a> {
    fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
        // The extension bit, then a bit for the member that may be absent
        let extended = reader.bit()?;
        let has_payload = reader.bit()?;
        let id = reader.unsigned(0, 65535)? as u16;
        let payload = match has_payload {
            true => Some(reader.octet_string(0, oxirush_aper::UNBOUNDED)?),
            false => None,
        };
        if extended {
            reader.skip_additions()?;
        }
        Ok(Self { id, payload })
    }
}

impl Encode for Sample<'_> {
    fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
        writer.bit(false);
        writer.bit(self.payload.is_some());
        writer.unsigned(self.id.into(), 0, 65535)?;
        if let Some(payload) = &self.payload {
            writer.octet_string(payload, 0, oxirush_aper::UNBOUNDED)?;
        }
        Ok(())
    }
}

fn main() -> Result<(), Error> {
    // The payload of what is decoded is a slice of the octets
    let octets = [0x40, 0x12, 0x34, 0x02, 0xca, 0xfe];
    let sample: Sample = oxirush_aper::decode(&octets)?;
    assert_eq!(sample.id, 0x1234);
    assert!(sample.payload.as_ref().is_some_and(|payload| payload.is_borrowed()));
    assert_eq!(oxirush_aper::encode(&sample)?, octets);

    // Octets that end early
    let short = oxirush_aper::decode::<Sample>(&octets[..5]).unwrap_err();
    println!("{short}");
    Ok(())
}
```

```text
these octets do not decode as Sample: the octets end before the value does
```

[`oxirush-ngap`](https://crates.io/crates/oxirush-ngap) and
[`oxirush-s1ap`](https://crates.io/crates/oxirush-s1ap) generate such
implementations for every type of their specifications. This page describes
0.1.0, which is not on crates.io yet.

## Installation

```toml
[dependencies]
oxirush-aper = "0.1"
```

The minimum supported Rust version is 1.88. The crate is `#![no_std]`, needs
`alloc`, and has no `unsafe`.

0.1.0 is not on crates.io yet, nor `oxirush-utils` 0.1.0, which the feature
`inspect` takes. Cargo resolves an optional dependency without its feature
too, so every build takes `oxirush-utils` from its repository, at the revision
that this revision of the crate is tested with, until it is published:

```toml
[dependencies]
oxirush-aper = { git = "https://github.com/linouxis9/oxirush-aper" }

[patch.crates-io]
oxirush-utils = { git = "https://github.com/linouxis9/oxirush-utils", rev = "6b492da612f8050924d7f19b590c3fa77d2e3646" }
```

In a clone of the repository, a cargo command takes it with `--config`, as CI
does:

```sh
cargo test --all-features \
  --config 'patch.crates-io.oxirush-utils.git="https://github.com/linouxis9/oxirush-utils"' \
  --config 'patch.crates-io.oxirush-utils.rev="6b492da612f8050924d7f19b590c3fa77d2e3646"'
```

A program that uses [oxirush-ngap](https://github.com/linouxis9/oxirush-ngap)
or [oxirush-s1ap](https://github.com/linouxis9/oxirush-s1ap) takes this crate
through them, as `oxirush_ngap::aper`: another version would have `Error` and
`Octets` types that do not convert into those of the bindings.

## Values

| ASN.1 | Rust |
| --- | --- |
| `BOOLEAN`, `NULL` | `bool`, `()` |
| `INTEGER` with a constraint | The native number that holds its range, as `u16` for `(0..65535)` |
| `INTEGER` with an extension marker, or without a constraint | `u64`, or `i64` when its root goes below zero |
| `OCTET STRING (SIZE (n))` | `[u8; n]` |
| Another `OCTET STRING`, an open type | `Octets<'a>` |
| `BIT STRING` of 64 bits at most, without an extension marker | `Bits` |
| Another `BIT STRING` | `BitString<'a>` |
| `PrintableString`, `VisibleString`, `IA5String`, `UTF8String` | `Cow<'a, str>` |
| `OBJECT IDENTIFIER` | `ObjectIdentifier` |

### Octets that borrow, and octets that are kept

`Octets` holds a slice of the octets that were decoded, or its own. A value
that outlives its input is made with `into_owned()`.

```rust
use oxirush_aper::Octets;

let received = vec![0x7e, 0x00, 0x41];
let borrowed = Octets::from(&received[..]);
assert!(borrowed.is_borrowed());

let kept: Octets<'static> = borrowed.into_owned();
drop(received);
assert_eq!(kept, [0x7e, 0x00, 0x41]);
```

### Bit strings

`Bits` is a number with a length, for the identifiers of 3GPP: it is `Copy`
and compares as a value. `BitString` holds any length.

```rust
use oxirush_aper::{BitString, Bits};

fn main() -> Result<(), oxirush_aper::Error> {
    // AMF Set ID ::= BIT STRING (SIZE (10))
    let set = Bits::new(0x155, 10)?;
    assert_eq!((set.value(), set.len()), (0x155, 10));
    // Its octets start with its first bit, and end with zero bits
    assert_eq!(set.to_vec(), [0x55, 0x40]);
    assert_eq!(Bits::from_octets(&[0x55, 0x40], 10)?, set);
    // A value that its length does not hold
    assert!(Bits::new(0x400, 10).is_err());

    // TransportLayerAddress ::= BIT STRING (SIZE (1..160, ...))
    let address = BitString::from([10, 0, 0, 1]);
    assert_eq!((address.as_octets(), address.len()), (&[10, 0, 0, 1][..], 32));
    Ok(())
}
```

## Fields

A `Reader` and a `Writer` have one method for each field of X.691 that NGAP
and S1AP use, and the bounds of a constraint are its arguments.

```rust
use oxirush_aper::{Reader, Writer};

fn main() -> Result<(), oxirush_aper::Error> {
    let mut octets = Vec::new();
    let mut writer = Writer::new(&mut octets);
    // INTEGER (0..7): three bits
    writer.unsigned(5, 0, 7)?;
    // INTEGER (0..255, ...): the extension bit, then one octet, aligned
    writer.unsigned_extensible(200, 0, 255)?;
    // ENUMERATED with 6 root values and an extension marker: the fourth
    writer.index(3, 6, true);
    // SIZE (1..256) of a SEQUENCE OF: its count, then its items
    writer.size(2, 1, 256)?;
    writer.unsigned(1, 0, 1)?;
    writer.unsigned(0, 0, 1)?;
    assert_eq!(octets, [0xa0, 0xc8, 0x30, 0x01, 0x80]);

    let mut reader = Reader::new(&octets);
    assert_eq!(reader.unsigned(0, 7)?, 5);
    assert_eq!(reader.unsigned_extensible(0, 255)?, 200);
    assert_eq!(reader.enumerated(6, 0, true)?, 3);
    assert_eq!(reader.size(1, 256)?, 2);
    assert_eq!((reader.bit()?, reader.bit()?), (true, false));
    Ok(())
}
```

A number outside its constraint is an error of the writer and of the reader:

```rust
use oxirush_aper::{ErrorKind, Writer};

let mut octets = Vec::new();
let error = Writer::new(&mut octets).unsigned(8, 0, 7).unwrap_err();
assert_eq!(error.kind(), ErrorKind::Encode);
assert_eq!(error.to_string(), "the value does not encode: 8 is outside its constraint (0..7)");
```

### An open type

An open type is read as its octets, which borrow from the input, and decoded
when its type is known. It is written in place, with no buffer of its own.

```rust
use oxirush_aper::{Reader, Writer};

fn main() -> Result<(), oxirush_aper::Error> {
    let mut octets = Vec::new();
    let mut writer = Writer::new(&mut octets);
    let start = writer.begin_open_type();
    writer.unsigned(0x1234, 0, 65535)?;
    writer.end_open_type(start);
    assert_eq!(octets, [0x02, 0x12, 0x34]);

    let mut reader = Reader::new(&octets);
    let held = reader.open_type()?;
    assert_eq!(held, [0x12, 0x34]);
    assert_eq!(Reader::new(&held).unsigned(0, 65535)?, 0x1234);
    Ok(())
}
```

A value that is decoded from an open type borrows from the input too, when it
is read from the octets that `as_borrowed()` gives: they live as long as the
input, where `&held` lives as long as the variable. An open type of 16384
octets or more comes in fragments, which are gathered: it owns its octets, and
what is decoded from it is copied out.

```rust
use oxirush_aper::{Error, Octets, Reader};

// The OCTET STRING (SIZE (0..255)) that an open type holds
fn held(input: &[u8]) -> Result<Octets<'_>, Error> {
    let open = Reader::new(input).open_type()?;
    match open.as_borrowed() {
        // In one piece: the string is a part of `input`
        Some(octets) => Reader::new(octets).octet_string(0, 255),
        // Gathered from fragments: the string is copied out of them
        None => Ok(Reader::new(&open).octet_string(0, 255)?.into_owned()),
    }
}

fn main() -> Result<(), Error> {
    let input = [0x03, 0x02, 0xab, 0xcd];
    let string = held(&input)?;
    assert_eq!(string, [0xab, 0xcd]);
    assert!(string.is_borrowed());
    Ok(())
}
```

### Extension additions

```rust
use oxirush_aper::{Reader, Writer};

fn main() -> Result<(), oxirush_aper::Error> {
    // SEQUENCE { a BOOLEAN, ..., b INTEGER (0..255) OPTIONAL, c BOOLEAN OPTIONAL }
    let mut octets = Vec::new();
    let mut writer = Writer::new(&mut octets);
    writer.bit(true); // additions follow
    writer.bit(true); // a
    writer.additions(&[true, false])?; // b is there, c is not
    let start = writer.begin_open_type();
    writer.unsigned(7, 0, 255)?;
    writer.end_open_type(start);

    let mut reader = Reader::new(&octets);
    let extended = reader.bit()?;
    let a = reader.bit()?;
    let mut additions = reader.additions()?;
    let b = match additions.present() {
        true => Some(Reader::new(&reader.open_type()?).unsigned(0, 255)?),
        false => None,
    };
    // What a later version of the type added is skipped
    additions.skip_rest(&mut reader)?;
    assert_eq!((extended, a, b), (true, true, Some(7)));
    Ok(())
}
```

## JSON

With the `inspect` feature, `oxirush_aper::json` has what generated bindings
write and read their trees with, the trees of `oxirush_utils::tree`: the form
of the JSON Encoding Rules (X.697), with octets and open types in hexadecimal,
and a readable form for a few well-known types of 3GPP, as `"208-93"` for a
PLMN identity. The views and the paths of the protocols are in the protocol
crates, and `oxirush_aper::check` has how a PDU deviates from the object set
of its message, without a feature.

```toml
[dependencies]
oxirush-aper = { version = "0.1", features = ["inspect"] }
```

## What the reader accepts

A writer gives a value its one encoding. A reader also takes a value that is
whole in another form, as the encoder of a peer may write it. One that is
asked to notes it: a `Fault`, with its clause and the bit at which it is.

| `Fault` | What is read | X.691 |
|---------|--------------|-------|
| `LengthInTwoOctets` | A length of 127 or less in two octets | 11.9.3.6 |
| `SmallFragment` | A fragment before another that is not the largest | 11.9.3.8.1 |
| `SmallNumberAsLarge` | A normally small number of 63 or less in the form of 64 and more | 11.6.1 |
| `LongNumber` | A whole number with octets that only extend it, up to the octets that a number of 64 bits may have | 11.3.6, 11.4.6 |
| `Padding` | Padding bits that are not zero | 11.1.4 |
| `LongArc` | An arc of an object identifier that starts with an octet that holds nothing | X.690 8.19.2 |

```rust
use oxirush_aper::{Fault, Octets, decode, decode_strict, decode_with_faults};

fn main() -> Result<(), oxirush_aper::Error> {
    // An OCTET STRING of one octet, whose length is in two octets
    let octets = [0x80, 0x01, 0xaa];
    assert_eq!(decode::<Octets>(&octets)?, [0xaa]);

    // What a tool that tests the peer wants to know
    let (_, faults) = decode_with_faults::<Octets>(&octets)?;
    assert_eq!(faults[0].fault, Fault::LengthInTwoOctets);
    assert_eq!(faults[0].to_string(), "at bit 0, a length of 127 or less is in two octets (X.691 §11.9.3.6)");

    // And a receiver that takes the one encoding alone
    assert!(decode_strict::<Octets>(&octets).is_err());
    Ok(())
}
```

`decode` and a reader of `Reader::new` note nothing. `Reader::noting` makes a
reader that notes, and `Reader::faults` has what it noted, the first four of
a reading in place and the count of all. A reader that does not note pays a
test before each octet-aligned field for the one that can: 4 to 13 ns on the
IEs of an NGAP message.

A reader refuses what announces something that is not there, and what has no
value: an extension bit without an addition that is present, an addition of
no octet, a number of no octet, a number of more octets than 64 bits may have
whatever they hold, a number, a size or an index outside its constraint, and
octets after a complete encoding.

## What it does not do

- The unaligned variant.
- `REAL`, the time types, `SET` and `SET OF`, `BMPString`, `UniversalString`,
  `NumericString`, and a permitted alphabet constraint.
- `DEFAULT` members and extension addition groups: a caller writes the bits of
  the preamble.
- An `INTEGER` beyond 64 bits, which is a decoding error.
- Parsing ASN.1: the bindings are written by a generator.

## Tests and fuzzing

The worked examples of X.691 Annex A are reproduced octet for octet, each
field is tested against its clause, and each is compared with
[`rasn`](https://crates.io/crates/rasn) on generated values: `tests/rasn.rs`
names the clause of each field where rasn 0.29 reads or writes otherwise.

```sh
cargo test --all-features
```

`fuzz/` has two [cargo-fuzz](https://crates.io/crates/cargo-fuzz) targets.
`fields` reads fields that the octets choose, with constraints that they
choose, and writes them back: they are read again as the same, from octets
that are written again as the same. `value` reads a value of one type with
the crate and with rasn: both read the same value and write the same octets,
or neither reads one.

```sh
cargo +nightly fuzz run fields -- -max_total_time=60 -max_len=512
cargo +nightly fuzz run value -- -max_total_time=60 -max_len=512
```

## Benchmarks

`benches/` is a package of its own, so that criterion is no dependency of the
crate. It times the fields that a message of NGAP or S1AP is made of.

```sh
cd benches
cargo bench --bench fields -- read/
```

## The generator

`generator/` is the library that the generators of `oxirush-ngap` and
`oxirush-s1ap` share: it writes the bindings of a protocol on this crate from
what [`rasn-compiler`](https://crates.io/crates/rasn-compiler) resolves of its
ASN.1, and the same types with the codec of rasn, which are the oracle of
their tests. It is not published.

## OxiRush crates

[oxirush-ngap](https://github.com/linouxis9/oxirush-ngap) and
[oxirush-s1ap](https://github.com/linouxis9/oxirush-s1ap) (the protocols
whose bindings use this crate) ·
[oxirush-utils](https://github.com/linouxis9/oxirush-utils) (identities and
trees) ·
[oxirush-nas](https://github.com/linouxis9/oxirush-nas) (NAS) ·
[oxirush-security](https://github.com/linouxis9/oxirush-security) (NAS keys
and algorithms) ·
[oxirush-gtp-u](https://github.com/linouxis9/oxirush-gtp-u) (user plane) ·
[oxirush-sctp](https://github.com/linouxis9/oxirush-sctp) (transport)

## Contributing

Open a pull request against `main` with a test for what it changes.
`cargo test --all-features` and `cargo clippy --all-targets --all-features`
pass on every commit. Sign off your commits (`git commit -s`), which agrees
to the [Developer Certificate of Origin](https://developercertificate.org/).
[SECURITY.md](SECURITY.md) says how to report a vulnerability.

## License

Copyright 2025-2026 Valentin D'Emmanuele

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE).
