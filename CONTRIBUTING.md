# Contributing to oxirush-aper

Each commit is signed off (`git commit -s`): a contribution is under the
[Developer Certificate of Origin](https://developercertificate.org/) and the
licence of the crate. A pull request comes with a test for what it changes.
This page is for those who change the repository beyond the crate; the
README is for those who use the crate.

## What the repository has beside the crate

| Directory | Contents | Published |
|-----------|----------|-----------|
| `src/` | The crate: the reader and the writer of the fields, the types of the values, `check`, the protocol traits and the bodies of the macros, `json` with `inspect` | yes |
| `generator/` | The library that the generators of oxirush-ngap and oxirush-s1ap share, each of which adds the names of its protocol: it reads the ASN.1 with rasn-compiler and writes the bindings, the registry and what `inspect` needs | no |
| `protocols/` | The tools, the programs and the tests that oxirush-ngap and oxirush-s1ap share beside their code: the bindings for rasn of their benchmarks (`protocols/rasn/`), the fuzz targets and their seeds, the benchmarks and the comparison with other implementations, the test of the package alone and of a member of a workspace, the comparison of their PDUs with Wireshark (`protocols/tests/wireshark/`), the writer of their pinned revisions (`protocols/build/pin.py`), and TOY (`protocols/toy/`), the protocol that the generator writes from its own ASN.1, whose tests are those of what the generated crates share | no |
| `benches/` | The comparison of the crate with rasn, and the benchmarks of the fields | no |
| `fuzz/` | The fuzz targets of the crate | no |
| `tests/` | The tests of the crate | yes |

oxirush-ngap and oxirush-s1ap take `generator/` and `protocols/` from a
checkout of this repository beside theirs, at the revision that their
`build/revisions` records, and their CI calls the jobs of
`.github/workflows/protocol.yml`, `protocol-reproduce.yml`,
`protocol-mutants.yml`, `protocol-fuzz.yml` and `protocol-wireshark.yml` at
that revision. A change to either reaches them when they move to its revision:
`pin.py` writes it wherever they name it, and their CONTRIBUTING.md says
what else a move takes.

## The macros of the protocol crates

The macros of oxirush-ngap and oxirush-s1ap that build a message and read and
change its IEs by name, and their readers of one IE by its marker type, are
written once, in the module `protocol` of the crate. The generator in
`generator/` writes the registry of each protocol from its ASN.1: the list of
its procedures, its IEs and its transfers that the macros expand.
`protocols/toy/` is TOY, a protocol of five procedures whose ASN.1 modules,
in `protocols/toy/toy/`, have one of each form that the generator writes: a
transfer of protocol IEs, which an IE contains too, and one of a value, octets
that carry a type, a list of single containers, an IE of an extension that has the type that the type
it extends gives it, the well-known types and the identifiers of a UE. The
generator writes its bindings as it writes those of oxirush-ngap and
oxirush-s1ap, and its tests are the tests of what the generated crates
share, which the protocol crates do not hold again. It is a member of the
workspace of the crate: `cargo test -p oxirush-toy` runs them, cargo-mutants
runs them on each mutant of the crate (`test_workspace` in
`.cargo/mutants.toml`), and `.github/mutants-generator.py` compares its
bindings as it compares those of the protocol crates. To write them again,
from `protocols/toy/`:

```sh
CARGO="$(command -v cargo)" cargo run --locked --manifest-path build/Cargo.toml
rustfmt --edition 2024 src/toy.rs src/registry.rs src/inspect_registry.rs
```

`build/asn1.sha256` records the digest of each module, which the generator
checks: write it again after a change of a module, with
`sha256sum toy/*.asn > build/asn1.sha256` in the order of the list. The
generator writes the bindings for rasn of the benchmarks too, in `rasn/`,
which TOY, which has no benchmarks, does not keep.

## The README

The Rust blocks of the README and of the documentation of `src/lib.rs` are
documentation tests: each shows what it gives with `assert_eq!`, not with
`println!`, so that `cargo test --all-features --doc` checks it.

[SECURITY.md](SECURITY.md) says how to report a vulnerability.
