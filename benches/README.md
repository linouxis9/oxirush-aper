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
| AMF-UE-NGAP-ID (2) | 11 ns, 48 cycles, 193 instructions, 0 allocations | 31 ns, 142 cycles, 494 instructions, 0 allocations | 2.9x the time, 2.6x the instructions |
| NAS-PDU (61) | 11 ns, 51 cycles, 148 instructions, 0 allocations | 50 ns, 249 cycles, 517 instructions, 1 allocation (60 octets) | 4.6x the time, 3.5x the instructions |
| AMFName (18) | 34 ns, 156 cycles, 649 instructions, 0 allocations | 147 ns, 672 cycles, 2,995 instructions, 2 allocations (32 octets) | 4.3x the time, 4.6x the instructions |
| GUAMI (7) | 30 ns, 131 cycles, 306 instructions, 0 allocations | 262 ns, 1,197 cycles, 3,977 instructions, 3 allocations (24 octets) | 8.8x the time, 13x the instructions |
| UserLocationInformationNR (19) | 35 ns, 163 cycles, 383 instructions, 0 allocations | 286 ns, 1,322 cycles, 3,675 instructions, 1 allocation (8 octets) | 8.1x the time, 9.6x the instructions |

Encoding, per call:

| Value (octets) | this crate | rasn 0.29 | rasn / this crate |
|---|---|---|---|
| AMF-UE-NGAP-ID (2) | 21 ns, 95 cycles, 278 instructions, 1 allocation (8 octets) | 58 ns, 266 cycles, 994 instructions, 1 allocation (8 octets) | 2.7x the time, 3.6x the instructions |
| NAS-PDU (61) | 60 ns, 276 cycles, 1,110 instructions, 2 allocations (69 octets) | 80 ns, 357 cycles, 1,478 instructions, 2 allocations (69 octets) | 1.3x the time, 1.3x the instructions |
| AMFName (18) | 50 ns, 229 cycles, 784 instructions, 2 allocations (26 octets) | 174 ns, 790 cycles, 3,333 instructions, 2 allocations (26 octets) | 3.5x the time, 4.2x the instructions |
| GUAMI (7) | 30 ns, 125 cycles, 461 instructions, 1 allocation (8 octets) | 137 ns, 641 cycles, 2,362 instructions, 1 allocation (8 octets) | 4.7x the time, 5.1x the instructions |
| UserLocationInformationNR (19) | 71 ns, 314 cycles, 1,243 instructions, 3 allocations (56 octets) | 258 ns, 1,190 cycles, 4,023 instructions, 3 allocations (56 octets) | 3.6x the time, 3.2x the instructions |

## The method of the comparison

A time is the fastest of three runs of a call repeated for about 0.2 s. The
cycles and the instructions are those that `perf stat` counts for the calls
alone: the difference between a run of twice the calls and one of the calls.
A counting allocator counts the allocations. The figures were taken at
65d0e09 on an AMD Ryzen 7 5700G (16 threads, Linux 7.0, Rust 1.98.1), the
program on one core of a quiet machine, on 2026-10-18: each is the median of
three runs of the script.
