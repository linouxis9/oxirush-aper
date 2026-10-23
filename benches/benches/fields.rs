/*
   OxiRush
   Copyright 2025-2026 Valentin D'Emmanuele

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

   http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

//! The time of the fields that a message of NGAP or S1AP is made of.
//!
//! ```sh
//! cargo bench --bench fields              # every benchmark
//! cargo bench --bench fields -- read/     # those whose name has "read/"
//! ```
//!
//! A benchmark is `read/FIELDS` or `write/FIELDS`, and its time is that of one pass
//! over the fields: a thousand of them, or one string of the size that its name says.

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use oxirush_aper::{Bits, Reader, UNBOUNDED, Writer};
use std::hint::black_box;
use std::time::Duration;

/// How many fields a pass reads or writes.
const FIELDS: u64 = 1000;

/// The octets that `write` writes.
fn written(write: impl Fn(&mut Writer<'_>)) -> Vec<u8> {
    let mut octets = Vec::new();
    write(&mut Writer::new(&mut octets));
    octets
}

/// A pair of benchmarks: `write/NAME`, into a buffer that is kept, and `read/NAME`,
/// of what was written.
fn pair(
    criterion: &mut Criterion,
    name: &str,
    elements: u64,
    write: impl Fn(&mut Writer<'_>) + Copy,
    read: impl Fn(&mut Reader<'_>),
) {
    let octets = written(write);
    let mut group = criterion.benchmark_group("write");
    group.throughput(Throughput::Elements(elements));
    let mut buffer = Vec::with_capacity(octets.len());
    group.bench_function(name, |bencher| {
        bencher.iter(|| {
            buffer.clear();
            write(&mut Writer::new(black_box(&mut buffer)));
        })
    });
    group.finish();
    let mut group = criterion.benchmark_group("read");
    group.throughput(Throughput::Elements(elements));
    group.bench_function(name, |bencher| {
        bencher.iter(|| read(&mut Reader::new(black_box(&octets))))
    });
    group.finish();
}

fn fields(criterion: &mut Criterion) {
    // The identifiers of an IE: INTEGER (0..65535), a criticality, a length.
    pair(
        criterion,
        "ie-headers",
        FIELDS,
        |writer| {
            for id in 0..FIELDS {
                writer.unsigned(black_box(id), 0, 65535).unwrap();
                writer.index(black_box(id % 3), 3, false);
                let start = writer.begin_open_type();
                writer.unsigned(black_box(id), 0, 4_294_967_295).unwrap();
                writer.end_open_type(start);
            }
        },
        |reader| {
            for _ in 0..FIELDS {
                black_box(reader.unsigned(0, 65535).unwrap());
                black_box(reader.enumerated(3, 0, false).unwrap());
                black_box(reader.open_type().unwrap());
            }
        },
    );
    // Numbers that stay within an octet, and the bits of a preamble.
    pair(
        criterion,
        "small-numbers",
        FIELDS,
        |writer| {
            for number in 0..FIELDS {
                writer.bit(black_box(number & 1 == 0));
                writer.unsigned(black_box(number % 16), 0, 15).unwrap();
                writer.unsigned(black_box(number % 256), 0, 255).unwrap();
            }
        },
        |reader| {
            for _ in 0..FIELDS {
                black_box(reader.bit().unwrap());
                black_box(reader.unsigned(0, 15).unwrap());
                black_box(reader.unsigned(0, 255).unwrap());
            }
        },
    );
    // Numbers of several octets: an AMF UE NGAP ID and a bit rate.
    pair(
        criterion,
        "wide-numbers",
        FIELDS,
        |writer| {
            for number in 0..FIELDS {
                let id = black_box(number << 28);
                writer.unsigned(id, 0, 1_099_511_627_775).unwrap();
                writer
                    .unsigned_extensible(black_box(number * 1_000_000), 0, 4_000_000_000_000)
                    .unwrap();
            }
        },
        |reader| {
            for _ in 0..FIELDS {
                black_box(reader.unsigned(0, 1_099_511_627_775).unwrap());
                black_box(reader.unsigned_extensible(0, 4_000_000_000_000).unwrap());
            }
        },
    );
    // Short bit strings: an AMF Set ID, a gNB ID, an algorithm mask.
    pair(
        criterion,
        "bit-strings",
        FIELDS,
        |writer| {
            for number in 0..FIELDS {
                writer
                    .bits_sized(Bits::low(black_box(number), 10), 10, 10)
                    .unwrap();
                writer
                    .bits_sized(Bits::low(black_box(number), 24), 22, 32)
                    .unwrap();
                writer
                    .bits_sized(Bits::low(black_box(number), 16), 16, 16)
                    .unwrap();
            }
        },
        |reader| {
            for _ in 0..FIELDS {
                black_box(reader.bits_sized(10, 10).unwrap());
                black_box(reader.bits_sized(22, 32).unwrap());
                black_box(reader.bits_sized(16, 16).unwrap());
            }
        },
    );
    // Octet strings: a PLMN identity, a TAC, a GTP-TEID, and a name.
    pair(
        criterion,
        "octet-strings",
        FIELDS,
        |writer| {
            for number in 0..FIELDS {
                writer.octet_array(black_box(&[0x02, 0xf8, 0x39]));
                writer.octet_array(black_box(&[0, 0, number as u8]));
                writer.octet_array(black_box(&[1, 0, 0, number as u8]));
                writer
                    .printable_string_extensible(black_box("amf-1"), 1, 150)
                    .unwrap();
            }
        },
        |reader| {
            for _ in 0..FIELDS {
                black_box(reader.octet_array::<3>().unwrap());
                black_box(reader.octet_array::<3>().unwrap());
                black_box(reader.octet_array::<4>().unwrap());
                black_box(reader.printable_string_extensible(1, 150).unwrap());
            }
        },
    );
    // A NAS-PDU, and a container in fragments: octets that the reader borrows.
    for length in [1_500, 70_000] {
        let payload: Vec<u8> = (0..length).map(|at| (at * 7 + 3) as u8).collect();
        pair(
            criterion,
            &format!("octet-string-of-{length}"),
            1,
            |writer| {
                writer
                    .octet_string(black_box(&payload), 0, UNBOUNDED)
                    .unwrap()
            },
            |reader| {
                black_box(reader.octet_string(0, UNBOUNDED).unwrap());
            },
        );
    }
}

fn configuration() -> Criterion {
    Criterion::default()
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2))
        .sample_size(50)
}

criterion_group! {
    name = benches;
    config = configuration();
    targets = fields
}
criterion_main!(benches);
