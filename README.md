# oxirush-aper

[![Crates.io](https://img.shields.io/crates/v/oxirush-aper.svg)](https://crates.io/crates/oxirush-aper)
[![Documentation](https://docs.rs/oxirush-aper/badge.svg)](https://docs.rs/oxirush-aper)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

Aligned PER (ITU-T X.691) for Rust bindings generated from 3GPP ASN.1: a reader
and a writer of the fields, and the types of the values. `no_std`, without a
dependency.

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
# Ok::<(), oxirush_aper::Error>(())
```

## Fields

A `Reader` and a `Writer` have one method for each field of X.691 that NGAP
and S1AP use, and the bounds of a constraint are its arguments.

```rust
use oxirush_aper::{Reader, Writer};

let mut octets = Vec::new();
let mut writer = Writer::new(&mut octets);
// INTEGER (0..7): three bits
writer.unsigned(5, 0, 7)?;
// INTEGER (0..255, ...): the extension bit, then one octet, aligned
writer.unsigned_extensible(200, 0, 255);
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
# Ok::<(), oxirush_aper::Error>(())
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
# Ok::<(), oxirush_aper::Error>(())
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

let input = [0x03, 0x02, 0xab, 0xcd];
let string = held(&input)?;
assert_eq!(string, [0xab, 0xcd]);
assert!(string.is_borrowed());
# Ok::<(), oxirush_aper::Error>(())
```

### Extension additions

```rust
use oxirush_aper::{Reader, Writer};

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
# Ok::<(), oxirush_aper::Error>(())
```

## JSON

With the `inspect` feature, `oxirush_aper::json` has what generated bindings
write and read their JSON forms with: `serde_json` values in the form of the
JSON Encoding Rules (X.697), with octets and open types in hexadecimal.

```toml
[dependencies]
oxirush-aper = { version = "0.1", features = ["inspect"] }
```

## What the reader accepts

A writer gives one encoding to a value. A reader takes what has one meaning:
padding bits that are not zero, a length in a longer form than it needs, an
arc of an object identifier that starts with an octet that holds nothing, and
a number with octets that only extend its sign, up to the octets that a
number of 64 bits may have. It refuses a number of no octet, a number of more
octets than that whatever they hold, a number, a size or an index outside its
constraint, and octets after a complete encoding.

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
