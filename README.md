# oxirush-aper

[![Crates.io](https://img.shields.io/crates/v/oxirush-aper.svg)](https://crates.io/crates/oxirush-aper)
[![Documentation](https://docs.rs/oxirush-aper/badge.svg)](https://docs.rs/oxirush-aper)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

Aligned PER (ITU-T X.691) for Rust bindings generated from 3GPP ASN.1: a reader
and a writer of the fields, and the types of the values. `no_std`, and without
a feature no dependency.

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
    println!("{octets:02x?}");

    let mut reader = Reader::new(&octets);
    println!("{}", reader.unsigned(0, 7)?);
    println!("{}", reader.unsigned_extensible(0, 255)?);
    println!("{}", reader.enumerated(6, 0, true)?);
    println!("{} items: {} {}", reader.size(1, 256)?, reader.bit()?, reader.bit()?);

    println!("{}", Writer::new(&mut Vec::new()).unsigned(8, 0, 7).unwrap_err());
    Ok(())
}
```

```text
[a0, c8, 30, 01, 80]
5
200
3
2 items: true false
the value does not encode: 8 is outside its constraint (0..7)
```

A `Reader` and a `Writer` have one method for each field of X.691 that NGAP
and S1AP use, and the bounds of a constraint are their arguments. The text
under a block is what the block prints.

This page describes 0.1.0, which is not on crates.io yet.

## Installation

```toml
[dependencies]
oxirush-aper = "0.1"
# With what generated bindings write and read their JSON trees with:
# oxirush-aper = { version = "0.1", features = ["inspect"] }
```

| Feature | Adds |
|---------|------|
| `inspect` | `json`: the trees of `oxirush_utils::tree` of the values, in the form of the JSON Encoding Rules (X.697) with a readable form for a few well-known types of 3GPP, as `"208-93"` for a PLMN identity |

`check`, how a PDU deviates from the object set of its message, needs no
feature. Until 0.1.0 is on crates.io, nor `oxirush-utils` 0.1.0, which the
feature `inspect` takes, every build takes `oxirush-utils` from its
repository, at the revision that this revision of the crate is tested with:
cargo resolves an optional dependency without its feature too.

```toml
[dependencies]
oxirush-aper = { git = "https://github.com/linouxis9/oxirush-aper" }

[patch.crates-io]
oxirush-utils = { git = "https://github.com/linouxis9/oxirush-utils", rev = "2f99edcc40baa5cf7397f94ecfc131dc1143bac5" }
```

In a clone of the repository, a cargo command takes it with `--config`, as CI
does:

```sh
cargo test --all-features \
  --config 'patch.crates-io.oxirush-utils.git="https://github.com/linouxis9/oxirush-utils"' \
  --config 'patch.crates-io.oxirush-utils.rev="2f99edcc40baa5cf7397f94ecfc131dc1143bac5"'
```

The minimum supported Rust version is 1.88. The crate is `#![no_std]`, needs
`alloc`, and has no `unsafe`. oxirush-ngap and oxirush-s1ap of a release
depend on this crate at one version exactly, and are released with it: a
program takes it through them, as `oxirush_ngap::aper`.

## A protocol of its own

The codec of a type is its fields in order, each with the bounds of its
constraint, as the generated bindings of oxirush-ngap and oxirush-s1ap
write theirs. Here a SEQUENCE with an extension marker, an OPTIONAL member,
an extensible ENUMERATED, a character string and a SEQUENCE OF:

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
        let id = reader.unsigned(0, 4095)? as u16;
        // An index among two values of the root, and none added that is known
        let kind = match reader.enumerated(2, 0, true)? {
            0 => Kind::Macro,
            _ => Kind::Small,
        };
        let name = match has_name {
            true => Some(reader.printable_string(1, 150)?),
            false => None,
        };
        let count = reader.size(0, 8)?;
        let neighbours = (0..count)
            .map(|_| reader.unsigned(0, 4095).map(|id| id as u16))
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
        writer.unsigned(self.id.into(), 0, 4095)?;
        writer.index(match self.kind { Kind::Macro => 0, Kind::Small => 1 }, 2, true);
        if let Some(name) = &self.name {
            writer.printable_string(name, 1, 150)?;
        }
        writer.size(self.neighbours.len(), 0, 8)?;
        for neighbour in &self.neighbours {
            writer.unsigned((*neighbour).into(), 0, 4095)?;
        }
        Ok(())
    }
}

fn main() -> Result<(), Error> {
    let cell = Cell {
        id: 0x123,
        kind: Kind::Small,
        name: Some("cell-1".into()),
        neighbours: vec![0x124, 0x125],
    };
    let octets = oxirush_aper::encode(&cell)?;
    println!("{octets:02x?}");

    // The name of what is decoded is a slice of the octets
    let read: Cell = oxirush_aper::decode(&octets)?;
    println!("{read:?}");
    println!("borrowed: {}", matches!(read.name, Some(Cow::Borrowed(_))));

    // What its constraints do not hold is an error of the writer and of the reader
    let crowded = Cell { neighbours: vec![0; 9], ..read };
    println!("{}", oxirush_aper::encode(&crowded).unwrap_err());
    println!("{}", oxirush_aper::decode::<Cell>(&octets[..5]).unwrap_err());
    Ok(())
}
```

```text
[40, 01, 23, 41, 40, 63, 65, 6c, 6c, 2d, 31, 20, 01, 24, 01, 25]
Cell { id: 291, kind: Small, name: Some("cell-1"), neighbours: [292, 293] }
borrowed: true
a value of Cell does not encode: a size of 9 is outside its constraint (0..8)
these octets do not decode as Cell: the octets end before the value does
```

## The types of the values

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

`Octets` holds a slice of the octets that were decoded, or its own. A value
that outlives its input is made with `into_owned()`.

```rust
use oxirush_aper::Octets;

let received = vec![0x7e, 0x00, 0x41];
let borrowed = Octets::from(&received[..]);
println!("{borrowed:?} borrowed: {}", borrowed.is_borrowed());

let kept: Octets<'static> = borrowed.into_owned();
drop(received);
println!("{kept:?} borrowed: {}", kept.is_borrowed());
```

```text
7E0041 borrowed: true
7E0041 borrowed: false
```

## Bit strings

`Bits` is a number with a length, for the identifiers of 3GPP: it is `Copy`
and compares as a value. `BitString` holds any length.

```rust
use oxirush_aper::{BitString, Bits};

fn main() -> Result<(), oxirush_aper::Error> {
    // AMF Set ID ::= BIT STRING (SIZE (10)): its octets start with its first bit
    let set = Bits::new(0x155, 10)?;
    println!("{:#x} of {} bits: {:02x?}", set.value(), set.len(), set.to_vec());
    println!("{}", Bits::from_octets(&[0x55, 0x40], 10)? == set);

    // A value that its length does not hold
    println!("{}", Bits::new(0x400, 10).unwrap_err());

    // TransportLayerAddress ::= BIT STRING (SIZE (1..160, ...))
    let address = BitString::from([10, 0, 0, 1]);
    println!("{:?} of {} bits", address.as_octets(), address.len());
    Ok(())
}
```

```text
0x155 of 10 bits: [55, 40]
true
1024 does not fit in 10 bits
[10, 0, 0, 1] of 32 bits
```

## An open type

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
    println!("{octets:02x?}");

    let mut reader = Reader::new(&octets);
    let held = reader.open_type()?;
    println!("{held:?}: {:#x}", Reader::new(&held).unsigned(0, 65535)?);
    Ok(())
}
```

```text
[02, 12, 34]
1234: 0x1234
```

`Octets::as_borrowed` gives the octets of an open type for as long as the
input, to decode a value that borrows from it.

## Extension additions

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
    println!("{extended} {a} {b:?}");
    Ok(())
}
```

```text
true true Some(7)
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
use oxirush_aper::{Octets, decode, decode_strict, decode_with_faults};

fn main() -> Result<(), oxirush_aper::Error> {
    // An OCTET STRING of one octet, whose length is in two octets
    let octets = [0x80, 0x01, 0xaa];
    println!("{:?}", decode::<Octets>(&octets)?);

    // What a tool that tests the peer wants to know
    let (_, faults) = decode_with_faults::<Octets>(&octets)?;
    println!("{}", faults[0]);

    // And a receiver that takes the one encoding alone
    println!("{}", decode_strict::<Octets>(&octets).unwrap_err());
    Ok(())
}
```

```text
AA
at bit 0, a length of 127 or less is in two octets (X.691 §11.9.3.6)
these octets do not decode as OCTET STRING: at bit 0, a length of 127 or less is in two octets (X.691 §11.9.3.6): the value is read, and a sender by the clause writes it otherwise
```

`decode` and a reader of `Reader::new` note nothing; `Reader::noting` makes a
reader that notes, and `Reader::faults` has what it noted. A reader refuses
what announces something that is not there, and what has no value: an
extension bit without an addition that is present, an addition of no octet, a
number of no octet or of more octets than 64 bits may have, a number, a size
or an index outside its constraint, and octets after a complete encoding.

## The macros of a protocol crate

oxirush-ngap and oxirush-s1ap give macros that build a message and read and
change its IEs by name (`build_ngap!`, `extract_ngap_ies!`,
`with_ngap_ie_mut!`, and the same with `s1ap`), and readers of one IE by its
marker type. Their code is here, in the module `protocol`, and the generator
of the repository writes the registry of each protocol from its ASN.1. Here
they are on the protocol of `tests/toy/mod.rs`, written as the generator
writes one:

```rust
// The toy protocol: a SETUP REQUEST of an Identity, a Name and a list of Items
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/toy/mod.rs"));
use toy::*;

fn main() -> Result<(), oxirush_aper::Error> {
    // A PDU by the names of its IEs, each with the criticality that the object
    // set of the message assigns it
    let pdu = build_toy!(InitiatingMessage, Setup, SetupRequest,
        Identity(7u32),
        Name("gNB-1"),
    )?;
    let octets = pdu.encode()?;
    println!("{}", octets.iter().map(|octet| format!("{octet:02x}")).collect::<String>());

    // Its IEs, read in one pass over the octets of its message, without
    // `decode_value`: each that a field asks for is decoded, and no other
    let pdu = TOYPDU::decode(&octets)?;
    extract_toy_ies!(pdu, SetupRequest,
        req identity: u32 = Identity(id),
        opt name: Vec<u8> = Name(name) => name.0.to_vec(),
    );
    println!("{identity} {:?}", name.as_deref().map(String::from_utf8_lossy));

    // One IE by its marker type, which has its identifier
    println!("{:?}, IE {}", pdu.ie::<ie::Identity>(), <ie::Identity as oxirush_aper::protocol::Ie>::ID);
    Ok(())
}
```

```text
000100130000020001000200070002400605674e422d31
7 Some("gNB-1")
Some(Ok(Identity(7))), IE 1
```

```rust
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/toy/mod.rs"));
use toy::*;

fn main() -> Result<(), oxirush_aper::Error> {
    // A list of single containers: each item is an IE of its own
    let items = vec![build_toy_ie!(REJECT Item(1u8))?, build_toy_ie!(REJECT Item(2u8))?];
    let pdu = build_toy!(InitiatingMessage, Setup, SetupRequest,
        Identity(7u32),
        Items(items),
    )?;
    // Each item, where it is in the octets of the PDU
    for item in pdu.items::<ie::Items>() {
        println!("item {}", item?.0);
    }

    // An IE of a message, changed in place
    let mut request: SetupRequest = pdu.decode_value()?;
    with_toy_ie_mut!(request, SetupRequest, Identity(id) = 9)?;
    println!("{:?}", request.ie::<ie::Identity>());

    // A SETUP REQUEST of two Identities: `ies::<T>()` gives each, the macros
    // take the first, and its container is checked against the object set of
    // its message without decoding a value
    let twice = build_toy!(InitiatingMessage, Setup, SetupRequest,
        Identity(7u32),
        Identity(8u32),
    )?;
    let every: Vec<_> = twice.ies::<ie::Identity>().collect::<Result<_, _>>()?;
    println!("{every:?}");
    for deviation in twice.check_structure()? {
        println!("{deviation}");
    }
    Ok(())
}
```

```text
item 1
item 2
Some(Ok(Identity(9)))
[Identity(7), Identity(8)]
Identity (IE 1), at position 1, is in the message already, at position 0
```

## Limits

- The unaligned variant.
- `REAL`, the time types, `SET` and `SET OF`, `BMPString`, `UniversalString`,
  `NumericString`, and a permitted alphabet constraint.
- `DEFAULT` members and extension addition groups: a caller writes the bits of
  the preamble.
- An `INTEGER` beyond 64 bits, which is a decoding error.
- Parsing ASN.1: the bindings of oxirush-ngap and oxirush-s1ap are written
  ahead of time by the generator in `generator/`, which is in this repository
  but not published.

## Performance

The crate against the derive of [rasn](https://crates.io/crates/rasn) 0.29,
on five values of NGAP (TS 38.413 clause 9.4), each with the same type in
both: written by hand as the generated bindings of oxirush-ngap write it, and
derived for rasn. Both read and write the same octets.

Decoding, per call:

| Value (octets) | this crate | rasn 0.29 | rasn / this crate |
|---|---|---|---|
| AMF-UE-NGAP-ID (2) | 11 ns, 48 cycles, 193 instructions, 0 allocations | 31 ns, 142 cycles, 494 instructions, 0 allocations | 2.9x the time, 2.6x the instructions |
| NAS-PDU (61) | 11 ns, 51 cycles, 148 instructions, 0 allocations | 50 ns, 249 cycles, 517 instructions, 1 allocation (60 octets) | 4.6x the time, 3.5x the instructions |
| AMFName (18) | 34 ns, 156 cycles, 649 instructions, 0 allocations | 147 ns, 672 cycles, 2,995 instructions, 2 allocations (32 octets) | 4.3x the time, 4.6x the instructions |
| GUAMI (7) | 30 ns, 131 cycles, 306 instructions, 0 allocations | 262 ns, 1,197 cycles, 3,977 instructions, 3 allocations (24 octets) | 8.8x the time, 13.0x the instructions |
| UserLocationInformationNR (19) | 35 ns, 163 cycles, 383 instructions, 0 allocations | 286 ns, 1,322 cycles, 3,675 instructions, 1 allocation (8 octets) | 8.1x the time, 9.6x the instructions |

Encoding, per call:

| Value (octets) | this crate | rasn 0.29 | rasn / this crate |
|---|---|---|---|
| AMF-UE-NGAP-ID (2) | 21 ns, 95 cycles, 278 instructions, 1 allocation (8 octets) | 58 ns, 266 cycles, 994 instructions, 1 allocation (8 octets) | 2.7x the time, 3.6x the instructions |
| NAS-PDU (61) | 60 ns, 276 cycles, 1,110 instructions, 2 allocations (69 octets) | 80 ns, 357 cycles, 1,478 instructions, 2 allocations (69 octets) | 1.3x the time, 1.3x the instructions |
| AMFName (18) | 50 ns, 229 cycles, 784 instructions, 2 allocations (26 octets) | 174 ns, 790 cycles, 3,333 instructions, 2 allocations (26 octets) | 3.5x the time, 4.2x the instructions |
| GUAMI (7) | 30 ns, 125 cycles, 461 instructions, 1 allocation (8 octets) | 137 ns, 641 cycles, 2,362 instructions, 1 allocation (8 octets) | 4.7x the time, 5.1x the instructions |
| UserLocationInformationNR (19) | 71 ns, 314 cycles, 1,243 instructions, 3 allocations (56 octets) | 258 ns, 1,190 cycles, 4,023 instructions, 3 allocations (56 octets) | 3.6x the time, 3.2x the instructions |


The time is the fastest of three runs of a call repeated for about 0.2 s, the
cycles and the instructions those of `perf stat` (the difference between a
run of twice the calls and one of the calls, so that starting the program
counts for nothing), and the allocations those of a counting allocator. The
figures were taken on an AMD Ryzen 7 5700G (16 threads, Linux 7.0, Rust
1.98.1), the program on one core, measured on a quiet machine on 2026-10-18:
each is the median of three runs of the script. `benches/` is a package of
its own, which fetches rasn: from the root of the repository,

```sh
python3 benches/compare.py --core 3 --perf "sudo -n perf"
cd benches && cargo bench --bench fields
```

`fields` times with criterion the fields that a message of NGAP or S1AP is
made of.

## Tests and fuzzing

The worked examples of X.691 Annex A are reproduced octet for octet, each
field is tested against its clause, and each is compared with
[`rasn`](https://crates.io/crates/rasn) on generated values: `tests/rasn.rs`
names the clause of each field where rasn 0.29 reads or writes otherwise.
`fuzz/` has two [cargo-fuzz](https://crates.io/crates/cargo-fuzz) targets:
`fields` reads fields that the octets choose, with constraints that they
choose, and writes them back, and `value` reads a value of one type with the
crate and with rasn, which read the same value and write the same octets, or
neither reads one.

```sh
cargo test --all-features
cargo +nightly fuzz run fields -- -max_total_time=60 -max_len=512
cargo +nightly fuzz run value -- -max_total_time=60 -max_len=512
```

A pull request comes with a test for what it changes; the Rust blocks of
this page are checked with what they print ([`tests/readme.rs`](tests/readme.rs)
says how). Sign off your commits (`git commit -s`): by contributing you agree
to the [Developer Certificate of Origin](https://developercertificate.org/).
[CONTRIBUTING.md](CONTRIBUTING.md) has what the repository holds beside the
crate, and [SECURITY.md](SECURITY.md) how to report a vulnerability.

## The OxiRush family

- [oxirush-utils](https://github.com/linouxis9/oxirush-utils): the 3GPP identities, and the JSON tree with its paths that the others share.
- [oxirush-aper](https://github.com/linouxis9/oxirush-aper): aligned PER, the runtime of the NGAP and S1AP codecs.
- [oxirush-nas](https://github.com/linouxis9/oxirush-nas): the NAS messages of 5GS and EPS.
- [oxirush-security](https://github.com/linouxis9/oxirush-security): NAS security, AKA and the derivation of keys.
- [oxirush-ngap](https://github.com/linouxis9/oxirush-ngap): NGAP, between a gNB and an AMF.
- [oxirush-s1ap](https://github.com/linouxis9/oxirush-s1ap): S1AP, between an eNB and an MME.
- [oxirush-gtp-u](https://github.com/linouxis9/oxirush-gtp-u): GTP-U tunnels, with a test UPF.
- [oxirush-sctp](https://github.com/linouxis9/oxirush-sctp): SCTP sockets, which carry NGAP and S1AP.

## License

Copyright 2025-2026 Valentin D'Emmanuele. Licensed under the Apache License,
Version 2.0: see [LICENSE](LICENSE).
