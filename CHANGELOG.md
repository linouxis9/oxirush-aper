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
  values as the trees of `oxirush_utils::tree` (`json::Value`), whose `Tree`
  the bindings implement: the form of the JSON Encoding Rules, with octets in
  hexadecimal in lower case, the members in their order and names read
  however they are written, and the readers of the ASN.1 types that the trees
  do not know (`read_bits`, `read_bit_string`, `read_text`,
  `read_object_identifier`, `read_octets`). A well-known type of 3GPP has a
  readable form, `Form`, which its tree shows and in which it is read, as
  well as in the form of the rules: `"208-93"` for a PLMN identity, a number
  for an identifier of a fixed size, an IP address for a transport layer
  address, the digits of an IMSI. A string of decimal digits is never read
  as octets: `"310410"` for a PLMN identity, or `"123456"` for a TAC, is
  refused with the two values that it could be (310-410 and the octets
  0x310410, which are 134-001), and octets whose hexadecimal has no letter
  are written after `0x`, as `"0x134001"`.
  And `inspect`, hidden from the documentation, the plumbing that NGAP and
  S1AP share: the views and the trees of their PDUs and the paths into a
  view, on the trees and the paths of `oxirush-utils`. An edit is done in the
  place of the view, at each place that its path selects or at none; `open`
  opens every place of a segment in one walk, and takes what each place has
  as a selection does. An alternative of the type `NULL` is `{"pLMNWide":
  null}`, whose `null` a path lists and selects. A view marks an IE, a
  transfer or a message whose typed value encodes to other octets than those
  received, and a tree has those octets in its place; every PDU that decodes
  has a tree, a procedure that the protocol does not have by its code and
  its direction, and a message that has no list of IEs or does not decode by
  its octets. The message of a procedure that the protocol does not have is
  a container of IEs to the paths too, each IE of the type of its
  identifier, so that what `paths` lists there is what `select` takes.
- `protocol::Transfer`: a transfer of protocol IEs, as
  `PDUSessionResourceSetupRequestTransfer` of NGAP, with its object set,
  which the generator writes into the registry of a protocol. The macros give
  its IEs the criticality that the set assigns, the trees take it from the
  set, and `protocol::check_transfer` and `check_transfer_structure` compare
  its octets with the set as `check` does a message. `Debug` of `Octets` and
  of `BitString` prints hexadecimal in lower case.
- `check`: how a PDU of an application protocol deviates from the object set
  of its message, as a `Deviation` that a protocol tells in its own types
  through `Diagnostics`, and what its receiver answers for it: the protocol
  cause, and the item of the Criticality Diagnostics of an IE of the
  criticality `reject` or `notify`, as an item is never `ignore`. A procedure
  that the specification does not have is `UnknownProcedure`, which a
  receiver answers by the criticality of the PDU.
  `criticality_diagnostics` makes the Criticality Diagnostics of an ERROR
  INDICATION from the deviations, in one call, with the procedure code and
  the triggering message, and `criticality_diagnostics_for_response` that of
  a response of the procedure, without them (TS 38.413 clause 9.3.1.3, TS
  36.413 clause 9.2.1.21): `Diagnostics::criticality_diagnostics` takes the
  procedure code and the direction as an `Option`.
- `protocol`: what the application protocols of 3GPP in aligned PER have in
  common, written once for oxirush-ngap and oxirush-s1ap. The traits that
  their registries implement: `Protocol` for the marker type of a protocol,
  `Pdu`, `Field`, `Container` for a message or a transfer of protocol IEs,
  `Message`, `Ie` for the marker type of each IE, by the names that the macros
  take, and `List` for an IE whose value is a list of single containers. The
  readers that the protocol crates give as `ies::<T>()`, `ie::<T>()` and
  `items::<L>()` of a PDU and of a message: `Ies` reads every occurrence of an
  IE in order and decodes that IE alone, from the octets of the message of a
  PDU, which it walks without allocating, or from a message that is decoded,
  and `Items` walks the items of a list without building it. A repeated IE
  stays visible to `ies`. `check` and `check_structure` of a PDU, the second
  for the container alone without decoding a value (`check::structure_deviations`),
  `protocol_ies`, `set_protocol_ies`, `of_message`, and the macros of the
  protocol crates, hidden from the documentation, which their public macros
  call with their own path: `build_ngap!` is `__build!` of this crate. The
  registry of a protocol invokes the macros that write its procedures, its
  IEs with their marker types, its lists, the tables of `inspect`, the
  module `hooks` of its fuzz targets and its oracle, and the tests of its
  tables. The README shows the macros on the protocol of `tests/toy/`.
  `inspect::view_of_jer` and `Protocol::from_jer`, beside `Protocol::jer`: a view
  whose values another implementation wrote in the JSON Encoding Rules, as a view of
  the crate has them, where a string of decimal digits is octets; the tests of the
  protocol crates read the values of pycrate with them.
- `#![no_std]` with `alloc`, no dependency without a feature, no `unsafe`.
- The crate, oxirush-ngap and oxirush-s1ap are released in step: the two
  depend on one version of it exactly (`=0.1.0`), re-export it with its value
  types, and build on its module `inspect`, which is hidden from the
  documentation and changes with them, so that a release of the crate is one
  of both. `json::Form`, `json::Characters`, `check::ProtocolCause` and
  `check::Wrong` are `#[non_exhaustive]`, as `check::Deviation` and the
  errors are: a later version may add to them.
- In the repository and not in the crate: `generator/`, the library that
  writes the bindings of a protocol on this crate, with the type of ASN.1 of
  each binding as the first line of its documentation and none of the
  banners of the sections of the modules, and the driver that the generator
  of each protocol crate configures (`generator::protocol`); fuzz targets and
  benchmarks; and what oxirush-ngap and oxirush-s1ap share beside their code,
  which each takes from a checkout of the repository at the revision that it
  records: `protocols/` (the oracle and its tests, the fuzz targets, the
  benchmarks, the scripts of the fixtures, of the package, of a workspace and
  of the revisions, and the writer of the test of a README) and the jobs of
  `.github/workflows/protocol.yml` and `protocol-reproduce.yml`.
- The tests hold on 32 bits as on 64, which CI runs on
  `i686-unknown-linux-musl`.
