# Changelog

All notable changes to `oxirush-aper` are recorded here.

## Unreleased

### Fixed

- Each path that `paths` lists selects its value, where the fuzzing of
  oxirush-ngap 0.6.0 and oxirush-s1ap 0.4.0 found two kinds of tree in which it
  did not:
  - an IE that a value holds alone and that has no name, as one of an
    identifier that NGAP does not have in the `choice-Extensions` of
    `SONInformation`, is its identifier in digits in a tree, which `from_tree`
    reads: `select`, the edits and `check_path` take the digits after the
    member that holds it, where no segment is a position, as they took
    `@id=N`;
  - an entry of a list of IEs, or of a container in the place of the value of
    an IE, that holds the IE whose value the list or the container is, as an
    `E-RABSetupListCtxtSURes` in the list of that name, is listed by its
    position: after an IE its name stays on the IE, and `select` took the name
    of the entry so, for the list itself, and its criticality for the one
    beside the IE.

## 0.2.0 - 2026-11-08

### Changed

- The paths of `inspect` read the tree of a PDU, the family `Trees` on the engine of
  oxirush-utils: an IE goes by its name, its position or `@id=N` to its value, as in a
  NAS message, `criticality` after the value is the criticality beside it, the IEs of
  a container go by name in its place, and an IE that a value holds alone by its
  name after the member that holds it. The view is the codec of a tree, which the
  protocol crates no longer show: `tree_parts` writes a tree, and `tree_of_jer` reads
  the values of a PDU that another implementation wrote in the JSON Encoding Rules.
  A tree has a transfer as its value or as `{"octets": "…"}` alone, and no other form.
- oxirush-utils 0.2.1, whose `Next::entry_member` and `Next::beside` the trees take.
- `Criticality` is the type of the bindings too: for the `Criticality` of the
  ASN.1 of a protocol, the generator writes `pub use oxirush_aper::Criticality`,
  and this crate gives it its codec, its tree and its serde. `Display` and
  `name()` give `reject`, `ignore` and `notify`, as ASN.1 names them.
- `Direction` is the direction of a PDU: `Pdu::direction()`,
  `Message::DIRECTION`, the object sets, `Deviation::UnknownProcedure` and the
  errors of a wrong message have it. `Display` and `name()` give
  `InitiatingMessage` and the two others, which `named()` reads.
- `ObjectSet` is public: the direction, the procedure code, the name and the
  criticality of the procedure of a message, and each IE of its object set with
  its criticality and its `Presence`, which, as `Criticality`, the bindings take
  for theirs. The marker type of a protocol finds them: `object_sets()`,
  `object_set(direction, code)` and `object_set_named(name)`, beside
  `ie_names()` and `item_criticality(id)`.
- `build_ngap!` and `build_s1ap!` take the message alone, and the criticality
  of the PDU before it where it is not that of its procedure:
  `build_ngap!(IGNORE NGSetupRequest, ...)`. A variant of the kind of a PDU is
  the name of the type of its message, as `NGSetupFailure`.
- `set_ie::<T>(value)` of a message and of a transfer changes an IE, and
  `Ie::field(criticality, value)` and `Ie::field_of::<M>(value)` build one.
- A field of the macro that extracts IEs takes the one value that the type of an
  IE is a name for by `From`, which the bindings implement, as
  `From<AMFUENGAPID> for u64`.
- `Reader` and `Writer` have one method per form, with `extensible: bool`
  last: `unsigned(lower, upper, extensible)`, `signed`, `octet_string`,
  `bit_string`, `printable_string` and `visible_string`.
  `Reader::index(root, additions, extensible)` reads the index of an
  `ENUMERATED` and of a `CHOICE`, as `Writer::index` writes it, and its error
  says that the type has no such index.
- The methods of the PDU of a protocol and its `Display` are written once, in
  `__pdu!`, which the protocol crate calls with the clauses of its specification
  that their documentation cites, the octets of a UE CONTEXT RELEASE REQUEST for
  its examples, and the IEs of the identifiers of a UE that `Display` shows. The
  generator writes the types, their codecs and the registry alone.
- A PDU decodes one way: `decode` borrows the octets of its message from what it
  is given, and `into_owned()` keeps it after them. `decode_value` is the one way
  to the message, which borrows from the PDU.
- `decode` and `encode` of a PDU are its codec, `set_protocol_ies` writes the
  message with the codec of the container, and `check` and `check_structure` read
  the container once, with a reader that notes its faults for `check`: the
  readers and the writers that did the same octet by octet go, and so does the
  walk that skipped to an IE of the usual layout. `field`, the walk of the
  readers and the macros, stays.
- An error has the kinds of what a program does with it: the eight of the paths and
  of the JSON forms are two, `Path` (a path that is not well formed, a name that the
  value at its place cannot have, an edit that selects nothing, a path past the
  limits) and `Value` (JSON that is no value of its type or no tree, a tree nested
  past the limits, an edit that is refused). Its public surface is `kind()`, `ie()`,
  `reason()`, `fields()` and `Display`: the functions that make one are hidden, for
  the bindings alone.
- An IE has one name in the macros and in the module `ie` of a protocol: the one
  after its identifier, as `AMFOverloadResponse`. The name of its type, which the
  one IE of a type had too, as `OverloadResponse`, is no longer one.
- The generator writes no `new` for a `SEQUENCE`: a struct literal builds one,
  with its members by their names. The `new` of a bit string of one size, which
  refuses a number that its bits do not hold, stays. The macro of the
  protocols builds the criticality diagnostics of a protocol with literals.
- The inspection tables of a protocol write the names of the members of each
  `SEQUENCE`, of the alternatives of each `CHOICE` and of the values of each
  `ENUMERATED` once, in its module `names`, which the trees of the bindings and
  the paths share: the bindings of NGAP are 2926 lines shorter, those of S1AP
  1259.
- oxirush-utils 0.2.0, whose trees read a member of a `SEQUENCE` by its name.
- The generator writes the codec of each member of a `SEQUENCE` in one line:
  the bindings of NGAP are 4794 lines shorter, those of S1AP 2106.

### Fixed

- `check_path` takes `*` in an IE that a value holds alone, as `select` does: the
  value of the IE and the criticality beside it.
- `family/` runs the block of the README that uses oxirush-ngap with the revision
  of oxirush-ngap that takes this crate, which no published version does yet:
  its copy of the block was that of 0.1.
- The generator writes the oracle of a protocol whose CHOICE of a fixed-size BIT
  STRING longer than 16 bits has an alternative of the type NULL: the fix-up of
  rasn's decoder of such a CHOICE refused that alternative as one that it did not
  recognize. TOY has one, in `NodeID`.

### Removed

- The oracle of the tests of the protocol crates, rasn, which the benchmarks
  alone keep: `protocols/oracle/` and its tests `codec.rs` and `compare.rs`, the
  comparison of a value with rasn (`read`, `same_octets`, `same_json`), the
  observer of the hooks of a protocol crate (`hooks::observe`) and
  `Protocol::jer`, which only the oracle read, and the list of its types that the
  generator wrote, `oracle/src/types.rs`; `tests/rasn.rs`, the dev-dependency on
  rasn, and the fuzz target `value`, which compared a value with rasn. The
  bindings for rasn of a protocol crate, `rasn/src/bindings.rs`, which the
  generator writes with the fix-ups of rasn's codec that they need, and
  `protocols/rasn/`, their codecs in the place of rasn's, are those of the
  benchmarks; the vectors of pycrate, the captures and the comparison with
  Wireshark stay.
- The family of the paths of a view and the functions of a view that only the
  protocol crates called: `view_parts`, `of_the_pdu` and `view_of_jer`.
- `inspect::view`, and what the view of a PDU kept for edits that only a view
  written by hand made, now that it is the codec of a tree alone: the marks
  `_edited`, `_original_id`, `_ie_name` and `_raw_message`, an IE written with
  its name as its `id`, and the octets received of a transfer or an IE beside an
  edited value, which a tree never has (`EDITED`, `written_ie`,
  `received_transfer`). `inspect` exports `named_ies` and `listed_ies` alone of
  what reads a view, which the tests of the tables take. The tests of a view
  written by hand, `tests/inspect.rs` and those of `tests/trees.rs` that TOY
  has, go: `tests/trees.rs` keeps the tests of a protocol written by hand for
  what TOY cannot have, types that hold one another 256 deep and the types of
  another protocol.
- `json::to_value(&x)`, which is `x.to_tree()`.
- `protocol::CriticalityOf`, the associated type `check::Diagnostics::Criticality`,
  the parameter of `protocol::ObjectSet` and the conversions between two
  criticalities.
- The `procedures` module of a registry, and the second decoding of each IE that
  a build with debug assertions made in `check()`.
- `check::deviations`, `check::structure_deviations` and `check::Wrong` are
  hidden: only the checks of a PDU call them.
- The macros `__with_ie_mut!` and `__build_ie!` (`with_ngap_ie_mut!` and
  `build_ngap_ie!` of the protocol crates), the module `messages` of a
  registry, `protocol::__Field`, and `Protocol::__procedure_criticality`.
- `ErrorKind::NoSuchMember`, `NothingSelected`, `NotOpened`, `Form`, `Edit` and
  `Limit`, which are `Path` or `Value`, and the functions of `Error` named after a
  kind, as `Error::argument`, which `Error::new(kind, what)` is (hidden).
- `procedure_name()` of a PDU: `kind()` names its message, and `Display` shows it.
- `encode_into` of a PDU, which `oxirush_aper::encode_into(&pdu, &mut octets)`
  is; `protocol::message_octets_as_its_type`, which `message_octets` is; and the
  hook `message_octets_both_ways`.
- `Message::__decode_owned`, and the fields of the `Protocol` of the generator
  that only the documentation of a PDU took: `aliases`, `ue_ids` (and `UeIds`),
  `release_request`, `release_display`, `other_message`, `no_message` and
  `examples`; `bindings()` takes no examples, and `__hooks!` no owned PDU.
- The `_extensible` twin of each form of `Reader` and `Writer`,
  `Reader::enumerated` and `Reader::choice`, and the forms that no binding
  takes: `unconstrained`, `semi_constrained`, `semi_constrained_unsigned` and
  `ia5_string`. The generator refuses an `INTEGER` without an upper bound and
  an `IA5String`, as it refuses the other types that it does not write.

### Migration from 0.1

| 0.1 | 0.2 |
|-----|-----|
| `json::to_value(&x)` | `x.to_tree()` |
| `Criticality::reject` (of the bindings) | `Criticality::Reject` |
| `pdu.direction() == "InitiatingMessage"` | `pdu.direction() == Direction::Initiating` |
| `Error::wrong_message(expected, held, "SuccessfulOutcome", code)` | `Error::wrong_message(expected, held, Direction::Successful, code)` |
| `impl Diagnostics { type Criticality = ...; }` | no such type: the criticality is `Criticality` |
| `build_ngap!(InitiatingMessage, NGSetup, NGSetupRequest, ...)` | `build_ngap!(NGSetupRequest, ...)` |
| `build_ngap!(SuccessfulOutcome, NGSetup, IGNORE, NGSetupResponse, ...)` | `build_ngap!(IGNORE NGSetupResponse, ...)` |
| `NgapPduKind::Unsuccessful_NGSetup` | `NgapPduKind::NGSetupFailure` |
| `with_ngap_ie_mut!(m, M, AMF_UE_NGAP_ID(id) = 9u64)` | `m.set_ie::<ie::AMF_UE_NGAP_ID>(9u64)` |
| `build_ngap_ie!(REJECT Item(v))` | `ie::Item::field(Criticality::Reject, v)` (trait `Ie` in scope) |
| `build_ngap_ie!(Msg, Cause(v))` | `ie::Cause::field_of::<Msg>(v)` |
| `inspect::message_ies(name)`, `message_ie_criticalities(name)` | `Ngap::object_set_named(name)?.ies` |
| `inspect::message_criticality(name)` | `Ngap::object_set_named(name)?.criticality` |
| `inspect::message_named(name)` | `Ngap::object_set_named(name)`, its `direction` and `procedure_code` |
| `inspect::message_names()` | `Ngap::object_sets()` |
| `inspect::message_name(&pdu)` | `Ngap::object_set(pdu.direction(), pdu.procedure_code())?.name` |
| `inspect::ie_names()`, `inspect::item_criticality(id)` | `Ngap::ie_names()`, `Ngap::item_criticality(id)` (a `Criticality`) |
| `reader.unsigned(0, 255)`, `writer.unsigned(v, 0, 255)` | `reader.unsigned(0, 255, false)`, `writer.unsigned(v, 0, 255, false)` |
| `reader.unsigned_extensible(0, 255)` (and of each form) | `reader.unsigned(0, 255, true)` |
| `reader.enumerated(root, additions, ext)`, `reader.choice(...)` | `reader.index(root, additions, ext)` |
| `pdu.procedure_name()` | `pdu.kind()`, or `pdu.to_string()` |
| `ErrorKind::NoSuchMember`, `NothingSelected`, `NotOpened` | `ErrorKind::Path` |
| `ErrorKind::Form`, `Edit`, `Limit` (of a tree) | `ErrorKind::Value` (`Path` for the limits of a path) |
| `ie::OverloadResponse` (the name of the type of an IE) | `ie::AMFOverloadResponse` (its name after `id-`) |
| `GUAMI::new(plmn, region, set, pointer, None)` (of the bindings) | `GUAMI { plmn_identity, amf_region_id, amf_set_id, amf_pointer, ie_extensions: None }` |

### Added

- The octets of an IE of one protocol may carry a type of another, as the
  transparent containers of a handover between 5GS and EPS do: S1AP's in
  NGAP, and NGAP's in S1AP (TS 38.413 clauses 9.3.1.20 and 9.3.1.21, TS
  36.413 clauses 9.2.1.56 and 9.2.1.57). The tables of a protocol name them,
  `foreign`, with the protocol of each, and a program registers the tables of
  that protocol with `register()`, which each protocol crate has in its
  module `inspect`. The trees, the views and the paths of the first protocol
  then write, open, list and edit such a container by the name of its type,
  with the tables of the other: its members, its IEs by their names and its
  enumerations. Without registration, the name is refused with the call that
  registers it. The generator writes the table from what the build of the
  protocol says, `Protocol::foreign`.
- The feature `std` holds the register, a lock of the standard library:
  without it, no protocol is registered. The build of the CI for
  `thumbv7em-none-eabihf` takes `inspect` and not every feature.
- TOY (`protocols/toy/`), a protocol of five procedures that the generator
  writes from its own ASN.1 modules as it writes NGAP and S1AP, with one of each
  form that the generator writes, an alternative of the type NULL among them. Its tests are those of what the generated
  crates share, which oxirush-ngap and oxirush-s1ap held twice: the codec and the
  `Display` of a PDU, its keys, the memory that a claimed count reserves, the
  macros and the readers, `check` and the Criticality Diagnostics, the errors,
  the trees and their paths. It is a member of the workspace of the crate and
  is not published: `cargo test` at the root tests the crate alone, `cargo test
  -p oxirush-toy` TOY, and cargo-mutants runs both on each mutant of the crate.
  The mutants of the generator are judged on its bindings too. The tests of
  `oxirush_aper::protocol` on the protocol written by hand in `tests/toy/` are
  those of TOY now; the README keeps that protocol for its examples, which run
  where TOY is not, in the package.

## 0.1.0 - 2026-10-24

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
- The views of `inspect` read the value of an IE and a transfer as the trees do: in
  another form than the one of X.691, as with padding bits that are not zero, they
  are their octets with the `_decode_error` of `decode_strict`, which a path list
  lists, as it lists `_lossy`.
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
