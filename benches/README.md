# Benchmarks of oxirush-aper

A package of its own, which the published crate does not have. It fetches
[rasn](https://crates.io/crates/rasn) 0.29 and compares the two crates on
five values of NGAP (TS 38.413 clause 9.4), each with the same type in both:
written by hand as the generated bindings of oxirush-ngap write it, and
derived for rasn. Both read and write the same octets. The
[README](../README.md#performance) sums the comparison up, and `fields`
times with criterion the fields that a message of NGAP or S1AP is made of.
From the root of the repository:

```sh
python3 benches/compare.py --core 3 --perf "sudo -n perf"
cd benches && cargo bench --bench fields
```

## The comparison with rasn

Decoding, per call:

| Value (octets) | this crate | rasn 0.29 | rasn / this crate |
|---|---|---|---|
| AMF-UE-NGAP-ID (2) | 18 ns (18 to 20), 53 cycles, 192 instructions, 0 allocations | 52 ns (52 to 55), 151 cycles, 450 instructions, 0 allocations | 2.8x the time, 2.3x the instructions |
| NAS-PDU (61) | 22 ns (20 to 28), 57 cycles, 147 instructions, 0 allocations | 93 ns (90 to 98), 265 cycles, 522 instructions, 1 allocation (60 octets) | 4.3x the time, 3.5x the instructions |
| AMFName (18) | 74 ns (73 to 76), 211 cycles, 764 instructions, 0 allocations | 363 ns (362 to 375), 1,044 cycles, 2,976 instructions, 2 allocations (32 octets) | 4.9x the time, 3.9x the instructions |
| GUAMI (7) | 47 ns (46 to 48), 134 cycles, 308 instructions, 0 allocations | 532 ns (529 to 541), 1,530 cycles, 3,920 instructions, 3 allocations (24 octets) | 11.4x the time, 12.7x the instructions |
| UserLocationInformationNR (19) | 56 ns (56 to 57), 163 cycles, 385 instructions, 0 allocations | 476 ns (471 to 546), 1,353 cycles, 3,555 instructions, 1 allocation (8 octets) | 8.4x the time, 9.2x the instructions |

Encoding, per call:

| Value (octets) | this crate | rasn 0.29 | rasn / this crate |
|---|---|---|---|
| AMF-UE-NGAP-ID (2) | 49 ns (49 to 50), 143 cycles, 286 instructions, 1 allocation (8 octets) | 128 ns (127 to 132), 371 cycles, 992 instructions, 1 allocation (8 octets) | 2.6x the time, 3.5x the instructions |
| NAS-PDU (61) | 145 ns (144 to 146), 419 cycles, 1,111 instructions, 2 allocations (69 octets) | 196 ns (195 to 212), 567 cycles, 1,482 instructions, 2 allocations (69 octets) | 1.4x the time, 1.3x the instructions |
| AMFName (18) | 116 ns (116 to 130), 334 cycles, 874 instructions, 2 allocations (26 octets) | 391 ns (390 to 403), 1,130 cycles, 3,335 instructions, 2 allocations (26 octets) | 3.4x the time, 3.8x the instructions |
| GUAMI (7) | 59 ns (59 to 60), 170 cycles, 460 instructions, 1 allocation (8 octets) | 298 ns (296 to 304), 853 cycles, 2,365 instructions, 1 allocation (8 octets) | 5.1x the time, 5.1x the instructions |
| UserLocationInformationNR (19) | 174 ns (173 to 186), 501 cycles, 1,242 instructions, 3 allocations (56 octets) | 557 ns (553 to 574), 1,599 cycles, 4,027 instructions, 3 allocations (56 octets) | 3.2x the time, 3.2x the instructions |

## The method of the comparison

Each operation of the two crates ran once a round, for about 0.2 s, in turns,
ten rounds: a time is the median of the ten, with the lowest and the highest
in brackets, and a ratio of times is given where the ten times of the two do
not overlap. The cycles and the instructions, the median of the rounds, are
those that `perf stat` counts for the calls alone: the difference between a
run of twice the calls and one of the calls. A counting allocator counts the
allocations. The figures were taken at 3ae0f3c on core 3 of an Intel Xeon
E5-2680 v4 (Broadwell, 2.4 GHz; a KVM guest bound to NUMA node 0; Linux 7.0,
Rust 1.100.0), on 2026-11-15.
