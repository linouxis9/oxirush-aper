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
    println!("{sample:?}, borrowed: {}", sample.payload.as_ref().unwrap().is_borrowed());
    println!("{:02x?}", oxirush_aper::encode(&sample)?);

    // Octets that end early
    println!("{}", oxirush_aper::decode::<Sample>(&octets[..5]).unwrap_err());
    Ok(())
}
```

```text
Sample { id: 4660, payload: Some(CAFE) }, borrowed: true
[40, 12, 34, 02, ca, fe]
these octets do not decode as Sample: the octets end before the value does
```

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
oxirush-utils = { git = "https://github.com/linouxis9/oxirush-utils", rev = "eb1c453cd08dc3d0b712f53d4273595793cb9106" }
```

In a clone of the repository, a cargo command takes it with `--config`, as CI
does:

```sh
cargo test --all-features \
  --config 'patch.crates-io.oxirush-utils.git="https://github.com/linouxis9/oxirush-utils"' \
  --config 'patch.crates-io.oxirush-utils.rev="eb1c453cd08dc3d0b712f53d4273595793cb9106"'
```

The minimum supported Rust version is 1.88. The crate is `#![no_std]`, needs
`alloc`, and has no `unsafe`. oxirush-ngap and oxirush-s1ap of a release
depend on this crate at one version exactly, and are released with it: a
program takes it through them, as `oxirush_ngap::aper`.

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

## Read and write the fields

A `Reader` and a `Writer` have one method for each field of X.691 that NGAP
and S1AP use, and the bounds of a constraint are its arguments. A number
outside its constraint is an error of the writer and of the reader.

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
    let string = held(&[0x03, 0x02, 0xab, 0xcd])?;
    println!("{string:?} borrowed: {}", string.is_borrowed());
    Ok(())
}
```

```text
ABCD borrowed: true
```

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
`with_ngap_ie_mut!`, and the same with `s1ap`), and readers of one IE of a
PDU or a message by its marker type. Their code is here, in the module
`protocol`: the traits that the registry of a protocol implements, the
readers, and the bodies of the macros, hidden from the documentation, which
the macros of each crate forward to with its paths. The generator of the
repository writes the registry of each protocol from its ASN.1. Here they are
on the protocol of `tests/toy/mod.rs`, which is written as the generator
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
- Parsing ASN.1: bindings are written by a generator, `generator/` of the
  repository, which oxirush-ngap and oxirush-s1ap share and which is not
  published, as `protocols/` is: the tools, the programs and the tests that
  the two crates share beside their code, which each takes from a checkout of
  this repository, with the jobs of `.github/workflows/protocol*.yml`.

## Performance

The crate against the derive of [rasn](https://crates.io/crates/rasn) 0.29,
on five values of NGAP (TS 38.413 clause 9.4), each with the same type in
both: written by hand as the generated bindings of oxirush-ngap write it, and
derived for rasn. Both read and write the same octets.

Decoding, per call:

| Value (octets) | this crate | rasn 0.29 | rasn / this crate |
|---|---|---|---|
| AMF-UE-NGAP-ID (2) | 11 ns, 47 cycles, 190 instructions, 0 allocations | 40 ns, 176 cycles, 587 instructions, 0 allocations | 3.7x the time, 3.1x the instructions |
| NAS-PDU (61) | 12 ns, 50 cycles, 155 instructions, 0 allocations | 60 ns, 262 cycles, 579 instructions, 1 allocation (60 octets) | 5.1x the time, 3.7x the instructions |
| AMFName (18) | 32 ns, 136 cycles, 663 instructions, 0 allocations | 193 ns, 828 cycles, 3,285 instructions, 2 allocations (32 octets) | 6.1x the time, 5.0x the instructions |
| GUAMI (7) | 34 ns, 136 cycles, 317 instructions, 0 allocations | 347 ns, 1,495 cycles, 4,631 instructions, 3 allocations (24 octets) | 10.1x the time, 14.6x the instructions |
| UserLocationInformationNR (19) | 38 ns, 171 cycles, 393 instructions, 0 allocations | 320 ns, 1,391 cycles, 4,034 instructions, 1 allocation (8 octets) | 8.3x the time, 10.3x the instructions |

Encoding, per call:

| Value (octets) | this crate | rasn 0.29 | rasn / this crate |
|---|---|---|---|
| AMF-UE-NGAP-ID (2) | 22 ns, 103 cycles, 304 instructions, 1 allocation (8 octets) | 65 ns, 280 cycles, 1,041 instructions, 1 allocation (8 octets) | 2.9x the time, 3.4x the instructions |
| NAS-PDU (61) | 62 ns, 252 cycles, 1,125 instructions, 2 allocations (69 octets) | 86 ns, 371 cycles, 1,496 instructions, 2 allocations (69 octets) | 1.4x the time, 1.3x the instructions |
| AMFName (18) | 45 ns, 200 cycles, 835 instructions, 2 allocations (26 octets) | 200 ns, 832 cycles, 3,480 instructions, 2 allocations (26 octets) | 4.5x the time, 4.2x the instructions |
| GUAMI (7) | 31 ns, 218 cycles, 483 instructions, 1 allocation (8 octets) | 162 ns, 691 cycles, 2,641 instructions, 1 allocation (8 octets) | 5.2x the time, 5.5x the instructions |
| UserLocationInformationNR (19) | 70 ns, 307 cycles, 1,293 instructions, 3 allocations (56 octets) | 306 ns, 1,334 cycles, 4,454 instructions, 3 allocations (56 octets) | 4.4x the time, 3.4x the instructions |


The time is the fastest of three runs of a call repeated for about 0.2 s, the
cycles and the instructions those of `perf stat` (the difference between a
run of twice the calls and one of the calls, so that starting the program
counts for nothing), and the allocations those of a counting allocator. The
figures were taken on an AMD Ryzen 7 5700G, the program on one core, while
other work kept the load average of the machine between 5.3 and 5.9 (of 16
threads). `benches/` is a package of its own, which fetches rasn: from the
root of the repository,

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
[SECURITY.md](SECURITY.md) says how to report a vulnerability.

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
