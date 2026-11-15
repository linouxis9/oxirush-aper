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

The time of one call, the median of ten rounds:

| Value (octets) | Decoding | rasn 0.29 | rasn | Encoding | rasn 0.29 | rasn |
|---|---|---|---|---|---|---|
| AMF-UE-NGAP-ID (2) | 18 ns | 52 ns | 2.8× slower | 49 ns | 128 ns | 2.6× slower |
| NAS-PDU (61) | 22 ns | 93 ns | 4.3× slower | 145 ns | 196 ns | 1.4× slower |
| AMFName (18) | 74 ns | 363 ns | 4.9× slower | 116 ns | 391 ns | 3.4× slower |
| GUAMI (7) | 47 ns | 532 ns | 11× slower | 59 ns | 298 ns | 5.1× slower |
| UserLocationInformationNR (19) | 56 ns | 476 ns | 8.4× slower | 174 ns | 557 ns | 3.2× slower |

What `perf stat` and the allocator count for one call, decoding:

| Value (octets) | Cycles | Instructions | Allocations | rasn: cycles | rasn: instructions | rasn: allocations | rasn, instructions |
|---|---|---|---|---|---|---|---|
| AMF-UE-NGAP-ID (2) | 53 | 192 | 0 | 151 | 450 | 0 | 2.3× more |
| NAS-PDU (61) | 57 | 147 | 0 | 265 | 522 | 1 (60 octets) | 3.5× more |
| AMFName (18) | 211 | 764 | 0 | 1,044 | 2,976 | 2 (32 octets) | 3.9× more |
| GUAMI (7) | 134 | 308 | 0 | 1,530 | 3,920 | 3 (24 octets) | 13× more |
| UserLocationInformationNR (19) | 163 | 385 | 0 | 1,353 | 3,555 | 1 (8 octets) | 9.2× more |

Encoding:

| Value (octets) | Cycles | Instructions | Allocations | rasn: cycles | rasn: instructions | rasn: allocations | rasn, instructions |
|---|---|---|---|---|---|---|---|
| AMF-UE-NGAP-ID (2) | 143 | 286 | 1 (8 octets) | 371 | 992 | 1 (8 octets) | 3.5× more |
| NAS-PDU (61) | 419 | 1,111 | 2 (69 octets) | 567 | 1,482 | 2 (69 octets) | 1.3× more |
| AMFName (18) | 334 | 874 | 2 (26 octets) | 1,130 | 3,335 | 2 (26 octets) | 3.8× more |
| GUAMI (7) | 170 | 460 | 1 (8 octets) | 853 | 2,365 | 1 (8 octets) | 5.1× more |
| UserLocationInformationNR (19) | 501 | 1,242 | 3 (56 octets) | 1,599 | 4,027 | 3 (56 octets) | 3.2× more |

## The method of the comparison

Each operation of the two crates ran once a round, for about 0.2 s, in turns,
ten rounds: a time is the median of the ten. The ten times of a value spread by
up to 27 % above their median (decoding NAS-PDU with this crate, 20 to 28 ns),
and two times are about the same when their medians are within 10 % or
their ten times overlap; none is here. The cycles and the instructions, the median of the rounds, are
those that `perf stat` counts for the calls alone: the difference between a
run of twice the calls and one of the calls. A counting allocator counts the
allocations. The figures were taken at 3ae0f3c on core 3 of an Intel Xeon
E5-2680 v4 (Broadwell, 2.4 GHz; a KVM guest bound to NUMA node 0; Linux 7.0,
Rust 1.100.0), on 2026-11-15.
