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
| `protocols/` | The tools, the programs and the tests that oxirush-ngap and oxirush-s1ap share beside their code: the oracle and its tests, the fuzz targets and their seeds, the benchmarks and the comparison with other implementations, the test of the package alone and of a member of a workspace, the writer of their README tests (`protocols/tests/readme.py`) and of their pinned revisions (`protocols/build/pin.py`) | no |
| `benches/` | The comparison of the crate with rasn, and the benchmarks of the fields | no |
| `fuzz/` | The fuzz targets of the crate | no |
| `tests/` | The tests of the crate, among them `tests/readme.rs`, which checks each Rust block of the README with what it prints, and `tests/toy/`, a protocol written as the generator writes one | yes |

oxirush-ngap and oxirush-s1ap take `generator/` and `protocols/` from a
checkout of this repository beside theirs, at the revision that their
`build/revisions` records, and their CI calls the jobs of
`.github/workflows/protocol.yml` and `protocol-reproduce.yml` at that
revision. A change to either reaches them when they move to its revision:
`pin.py` writes it wherever they name it, and their CONTRIBUTING.md says
what else a move takes.

## The macros of the protocol crates

The macros of oxirush-ngap and oxirush-s1ap that build a message and read and
change its IEs by name, and their readers of one IE by its marker type, are
written once, in the module `protocol` of the crate. The generator in
`generator/` writes the registry of each protocol from its ASN.1: the list of
its procedures, its IEs and its transfers that the macros expand.
`tests/toy/mod.rs` is a protocol of two procedures written as the generator
writes one, which the tests and the README build and read.

## The README

The Rust blocks of the README are documentation tests, and
`tests/readme.rs` compares what each prints with the `text` block under it.
After a change of a block, write `tests/readme/blocks.rs` again and run the
test once more:

```sh
README_BLOCKS=write cargo test --all-features --test readme
cargo test --all-features --test readme
```

Until oxirush-utils 0.1.0 is on crates.io, these commands take the
`--config` options that the README gives.

[SECURITY.md](SECURITY.md) says how to report a vulnerability.
