# Changelog

All notable changes to `oxirush-aper` are recorded here.

## Unreleased (0.1.0)

The first version: the aligned variant of the Packed Encoding Rules, ITU-T
X.691 (02/2021), for the constructs that the ASN.1 of NGAP (3GPP TS 38.413)
and S1AP (TS 36.413) uses.

- `Reader` and `Writer`: a method for each field, with the bounds of its
  constraint as arguments. Whole numbers with and without a constraint or an
  extension marker, normally small numbers, length determinants with their
  fragments, the indexes of `ENUMERATED` and `CHOICE`, the preamble and the
  extension additions of a `SEQUENCE`, `OCTET STRING`, `BIT STRING`,
  `PrintableString`, `VisibleString`, `IA5String`, `UTF8String`,
  `OBJECT IDENTIFIER` and open types, which a writer encodes in place.
- A value outside the root of an extensible constraint is written as an
  extension, which a receiver of a later version of the type reads.
  `Writer::strict` and `encode_strict` refuse it: a sender of this version
  sends none.
- A value that is whole in another form than the one that its clause gives
  it is read, and noted as a `Fault` with its clause and its bit: a length
  of 127 or less in two octets, a fragment that is not the largest, a
  normally small number in the form of a larger one, a whole number in more
  octets than it needs, padding bits that are not zero, and an arc of an
  object identifier that starts with an octet that holds nothing (X.690
  §8.19.2).
  A reader of `Reader::noting` has them as its `faults`, `decode_with_faults`
  gives them, and `decode_strict` refuses them. An extension bit without an addition that is present, and an
  addition of no octet, are refused by every reading.
- The values: `Octets`, which borrows from the octets that are decoded or
  owns its own, `Bits` for a bit string of 64 bits at most, `BitString` for
  the others, and `ObjectIdentifier`.
- `AsnType`, `Decode` and `Encode`, which generated bindings implement, and
  `decode`, `encode` and `encode_into` for a complete encoding.
- `Error`, with an `ErrorKind`, the reason, and the names of the fields on the
  way to where a decoding stopped. An error that the bindings of a protocol
  make about an IE of a message has it: `ie()` gives an `ErrorIe`, the
  identifier of the IE, its `Criticality` and its position. The errors that
  bindings make of a PDU, of its message and of its IEs have one wording, in
  constructors of `Error`: `wrong_message`, `missing_ie`, `undecodable_ie`
  and the others.
- The `inspect` feature: `json`, with which bindings write and read their
  values as `serde_json` values in the form of the JSON Encoding Rules.
  And `inspect`, the views and the trees of the PDUs of an application
  protocol and the paths into a view, which NGAP and S1AP share, on the
  trees and the paths of `oxirush-utils`: a protocol gives its tables and
  the functions of its types as a `Protocol`. An edit is done in the place
  of the view, at each place that its path selects or at none.
- `check`: how a PDU of an application protocol deviates from the object set
  of its message, as a `Deviation` that a protocol tells in its own types
  through `Diagnostics`, and what its receiver answers for it.
- `#![no_std]` with `alloc`, no dependency without a feature, no `unsafe`.
- The crate, oxirush-ngap and oxirush-s1ap are released in step: the two
  re-export it with its value types, and a breaking release of the crate is
  one of both. The README says what a program with two versions gets.
- In the repository and not in the crate: `generator/`, the library that
  writes the bindings of a protocol on this crate, fuzz targets and
  benchmarks.
