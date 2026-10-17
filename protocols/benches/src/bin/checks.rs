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

//! `check` and `check_structure` of the PDUs of `compare/pdus.tsv`, each from the octets
//! of the PDU, as a receiver calls them:
//!
//! ```text
//! cargo run --release --bin checks
//! ```
//!
//! A row is the time of a call in the fastest of three runs of about 0.3 s, and its
//! allocations and the octets that they ask for.

use benches::Pdu;
use benches::counting::{Counting, allocations};
use std::hint::black_box;
use std::time::{Duration, Instant};

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// The time of a call: the fastest of three runs of about 0.3 s.
fn time(mut call: impl FnMut()) -> f64 {
    let mut run = |iterations: u32| {
        let started = Instant::now();
        for _ in 0..iterations {
            call();
        }
        started.elapsed()
    };
    let mut iterations = 1;
    while run(iterations) < Duration::from_millis(50) {
        iterations *= 2;
    }
    let iterations = iterations * 6;
    let fastest = (0..3).map(|_| run(iterations)).min().unwrap();
    fastest.as_secs_f64() * 1e9 / f64::from(iterations)
}

fn main() {
    let pdus = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/compare/pdus.tsv"));
    println!("| PDU (octets) | check ns | allocations | check_structure ns | allocations |");
    println!("|---|---|---|---|---|");
    for line in pdus
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
    {
        let (name, hex) = line.split_once('\t').unwrap();
        let octets = hex::decode(hex.trim()).unwrap();
        let check = || {
            black_box(
                Pdu::decode_borrowed(black_box(&octets))
                    .unwrap()
                    .check()
                    .unwrap(),
            );
        };
        let structure = || {
            let pdu = Pdu::decode_borrowed(black_box(&octets)).unwrap();
            black_box(pdu.check_structure().unwrap());
        };
        let (checked, structured) = (time(check), time(structure));
        let ([checks, _, _], [structures, _, _]) = (allocations(check), allocations(structure));
        println!(
            "| {name} ({}) | {checked:.0} | {checks} | {structured:.0} | {structures} |",
            octets.len()
        );
    }
}
