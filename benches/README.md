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

| Value (octets) | This crate | Lowest to highest | Cycles | Instructions | Allocations | rasn 0.29 | Lowest to highest | Cycles | Instructions | Allocations | rasn, time | rasn, instructions |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| AMF-UE-NGAP-ID (2) | 18 ns | 18 to 20 | 53 | 192 | 0 | 52 ns | 52 to 55 | 151 | 450 | 0 | 2.8× slower | 2.3× more |
| NAS-PDU (61) | 22 ns | 20 to 28 | 57 | 147 | 0 | 93 ns | 90 to 98 | 265 | 522 | 1 (60 octets) | 4.3× slower | 3.5× more |
| AMFName (18) | 74 ns | 73 to 76 | 211 | 764 | 0 | 363 ns | 362 to 375 | 1,044 | 2,976 | 2 (32 octets) | 4.9× slower | 3.9× more |
| GUAMI (7) | 47 ns | 46 to 48 | 134 | 308 | 0 | 532 ns | 529 to 541 | 1,530 | 3,920 | 3 (24 octets) | 11.4× slower | 12.7× more |
| UserLocationInformationNR (19) | 56 ns | 56 to 57 | 163 | 385 | 0 | 476 ns | 471 to 546 | 1,353 | 3,555 | 1 (8 octets) | 8.4× slower | 9.2× more |

Encoding, per call:

| Value (octets) | This crate | Lowest to highest | Cycles | Instructions | Allocations | rasn 0.29 | Lowest to highest | Cycles | Instructions | Allocations | rasn, time | rasn, instructions |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| AMF-UE-NGAP-ID (2) | 49 ns | 49 to 50 | 143 | 286 | 1 (8 octets) | 128 ns | 127 to 132 | 371 | 992 | 1 (8 octets) | 2.6× slower | 3.5× more |
| NAS-PDU (61) | 145 ns | 144 to 146 | 419 | 1,111 | 2 (69 octets) | 196 ns | 195 to 212 | 567 | 1,482 | 2 (69 octets) | 1.4× slower | 1.3× more |
| AMFName (18) | 116 ns | 116 to 130 | 334 | 874 | 2 (26 octets) | 391 ns | 390 to 403 | 1,130 | 3,335 | 2 (26 octets) | 3.4× slower | 3.8× more |
| GUAMI (7) | 59 ns | 59 to 60 | 170 | 460 | 1 (8 octets) | 298 ns | 296 to 304 | 853 | 2,365 | 1 (8 octets) | 5.1× slower | 5.1× more |
| UserLocationInformationNR (19) | 174 ns | 173 to 186 | 501 | 1,242 | 3 (56 octets) | 557 ns | 553 to 574 | 1,599 | 4,027 | 3 (56 octets) | 3.2× slower | 3.2× more |

## The method of the comparison

Each operation of the two crates ran once a round, for about 0.2 s, in turns,
ten rounds: a time is the median of the ten, beside the lowest and the
highest, and a ratio of times is given where the ten times of the two do
not overlap. The cycles and the instructions, the median of the rounds, are
those that `perf stat` counts for the calls alone: the difference between a
run of twice the calls and one of the calls. A counting allocator counts the
allocations. The figures were taken at 3ae0f3c on core 3 of an Intel Xeon
E5-2680 v4 (Broadwell, 2.4 GHz; a KVM guest bound to NUMA node 0; Linux 7.0,
Rust 1.100.0), on 2026-11-15.
