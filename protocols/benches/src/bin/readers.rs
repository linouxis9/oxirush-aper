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

//! The readers of the IEs of a message against `decode_value` and the macro that extracts
//! IEs (`extract_ies!`, `extract_s1ap_ies!`), on the four messages that `protocol.rs`
//! writes: for the first IE of each, and for every IE of each. And `check` against
//! `check_structure`. oxirush-ngap and oxirush-s1ap take this file from oxirush-aper; the
//! messages, with their IEs, are in `src/readers.rs` of the package of each.
//!
//! ```text
//! cargo run --release --bin readers --features readers [WORD...]
//! ```
//!
//! Each row is one operation on one message: the time of a call in the fastest of three
//! runs of about 0.2 s each, and its allocations and the octets that they ask for. A word
//! keeps the rows whose message or operation has it. Without the feature `readers`, the
//! program has the operations of a crate that has no reader of its own, as the crate had
//! before them: `decode_value` and the macro on the message, and `check`.

use benches::counting::{Counting, allocations};
use benches::{Pdu, codec, extract_ies, types};
use codec::Error;
use std::hint::black_box;
use std::time::{Duration, Instant};

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// The operations on one message, with the IEs that it has, the first first.
macro_rules! readers {
    ($module:ident $message:ident {
        $first:ident: $first_type:ident $(<$first_borrows:lifetime>)?,
        $($ie:ident: $type:ident $(<$borrows:lifetime>)?,)*
    }) => {
        #[allow(non_snake_case)]
        mod $module {
            use super::*;

            /// Every IE of a message, by the macro.
            fn all<'a>(message: &'a types::$message<'a>) -> Result<(
                types::$first_type $(<$first_borrows>)?, $(types::$type $(<$borrows>)?,)*
            ), Error> {
                extract_ies!(message, $message,
                    req $first: types::$first_type $(<$first_borrows>)? = $first(value) => value,
                    $(req $ie: types::$type $(<$borrows>)? = $ie(value) => value,)*
                );
                Ok(($first, $($ie,)*))
            }

            /// The first IE of a message, by the macro.
            fn one<'a>(
                message: &'a types::$message<'a>,
            ) -> Result<types::$first_type $(<$first_borrows>)?, Error> {
                extract_ies!(message, $message,
                    req $first: types::$first_type $(<$first_borrows>)? = $first(value) => value,
                );
                Ok($first)
            }

            /// Every IE of a message, by a loop over its IEs that decodes each where it finds
            /// it, as the macro was written before it walked the IEs.
            #[cfg(feature = "readers")]
            #[allow(unused_assignments)]
            fn loop_all(pdu: &Pdu) {
                let message: types::$message = pdu.decode_value().unwrap();
                read(&message);
            }

            #[cfg(feature = "readers")]
            #[allow(unused_assignments)]
            fn read<'a>(message: &'a types::$message<'a>) {
                let mut $first: Option<types::$first_type $(<$first_borrows>)?> = None;
                $(let mut $ie: Option<types::$type $(<$borrows>)?> = None;)*
                for ie in message.protocol_ies.0.iter() {
                    if ie.id.0 == <codec::ie::$first as codec::aper::protocol::Ie>::ID {
                        $first = Some(codec::aper::decode(&ie.value).unwrap());
                    }
                    $(if ie.id.0 == <codec::ie::$ie as codec::aper::protocol::Ie>::ID {
                        $ie = Some(codec::aper::decode(&ie.value).unwrap());
                    })*
                }
                black_box(($first.unwrap(), $($ie.unwrap(),)*));
            }

            fn decode_value_all(pdu: &Pdu) {
                let message: types::$message = pdu.decode_value().unwrap();
                black_box(all(&message).unwrap());
            }

            fn decode_value_one(pdu: &Pdu) {
                let message: types::$message = pdu.decode_value().unwrap();
                black_box(one(&message).unwrap());
            }

            /// Every IE of the message of a PDU, by the macro on the PDU.
            #[cfg(feature = "readers")]
            fn pdu_all(pdu: &Pdu) {
                #[allow(clippy::needless_lifetimes)]
                fn read<'a>(pdu: &'a Pdu) -> Result<(), Error> {
                    extract_ies!(pdu, $message,
                        req $first: types::$first_type $(<$first_borrows>)? = $first(value) => value,
                        $(req $ie: types::$type $(<$borrows>)? = $ie(value) => value,)*
                    );
                    black_box(($first, $($ie,)*));
                    Ok(())
                }
                read(pdu).unwrap()
            }

            /// The first IE of the message of a PDU, by the macro on the PDU.
            #[cfg(feature = "readers")]
            fn pdu_one(pdu: &Pdu) {
                #[allow(clippy::needless_lifetimes)]
                fn read<'a>(pdu: &'a Pdu) -> Result<(), Error> {
                    extract_ies!(pdu, $message,
                        req $first: types::$first_type $(<$first_borrows>)? = $first(value) => value,
                    );
                    black_box($first);
                    Ok(())
                }
                read(pdu).unwrap()
            }

            /// The first IE of the message of a PDU, by `ie::<T>()`.
            #[cfg(feature = "readers")]
            fn ie_one(pdu: &Pdu) {
                black_box(pdu.ie::<codec::ie::$first>().unwrap().unwrap());
            }

            /// The walk of every IE of the message of a PDU by the macro, which decodes none:
            /// an IE that no message of the four has.
            #[cfg(feature = "readers")]
            fn macro_walk(pdu: &Pdu) {
                #[allow(clippy::needless_lifetimes)]
                fn read<'a>(pdu: &'a Pdu) -> Result<Option<types::CriticalityDiagnostics>, Error> {
                    extract_ies!(pdu, $message,
                        lax absent: types::CriticalityDiagnostics = CriticalityDiagnostics(value) => value,
                    );
                    Ok(absent)
                }
                black_box(read(pdu).unwrap());
            }

            /// That the readers read what the macro on the message reads.
            #[cfg(feature = "readers")]
            pub fn same(pdu: &Pdu) {
                let message: types::$message = pdu.decode_value().unwrap();
                let read = all(&message).unwrap();
                assert_eq!(pdu.ie::<codec::ie::$first>().unwrap().unwrap(), read.0);
                #[allow(clippy::needless_lifetimes)]
                fn on_pdu<'a>(pdu: &'a Pdu) -> Result<(
                    types::$first_type $(<$first_borrows>)?, $(types::$type $(<$borrows>)?,)*
                ), Error> {
                    extract_ies!(pdu, $message,
                        req $first: types::$first_type $(<$first_borrows>)? = $first(value) => value,
                        $(req $ie: types::$type $(<$borrows>)? = $ie(value) => value,)*
                    );
                    Ok(($first, $($ie,)*))
                }
                assert_eq!(on_pdu(pdu).unwrap(), read);
            }

            #[cfg(not(feature = "readers"))]
            pub fn same(_: &Pdu) {}

            /// The operations, by name.
            pub const OPERATIONS: Operations = &[
                ("decode_value, then the macro: the first IE", decode_value_one),
                #[cfg(feature = "readers")]
                ("the macro on the PDU: the first IE", pdu_one),
                #[cfg(feature = "readers")]
                ("ie::<T>() of the PDU: the first IE", ie_one),
                ("decode_value, then the macro: every IE", decode_value_all),
                #[cfg(feature = "readers")]
                ("decode_value, then a loop: every IE", loop_all),
                #[cfg(feature = "readers")]
                ("the macro on the PDU: every IE", pdu_all),
                #[cfg(feature = "readers")]
                ("the macro on the PDU: walk every IE, decode none", macro_walk),
            ];
        }
    };
}

fn check(pdu: &Pdu) {
    black_box(pdu.check().unwrap());
}

/// The walk of every IE of the message of a PDU, which decodes none: an IE that no message
/// of the four has.
#[cfg(feature = "readers")]
fn walk(pdu: &Pdu) {
    black_box(pdu.ies::<codec::ie::CriticalityDiagnostics>().count());
}

#[cfg(feature = "readers")]
fn check_structure(pdu: &Pdu) {
    black_box(pdu.check_structure().unwrap());
}

/// Operations on a PDU, each by its name.
type Operations = &'static [(&'static str, fn(&Pdu))];

/// A message of `MESSAGES`: its name in `written()`, the operations on it, and the check
/// that the readers read what the macro reads.
type Message = (&'static str, Operations, fn(&Pdu));

/// The operations of every message.
const CHECKS: Operations = &[
    ("check", check),
    #[cfg(feature = "readers")]
    ("check_structure", check_structure),
    #[cfg(feature = "readers")]
    ("ies::<T>() of the PDU: walk every IE, decode none", walk),
];

/// An operation that checks nothing.
#[allow(dead_code)]
fn nothing(_: &Pdu) {}

// The messages of the protocol, each with the operations on it and the check that the
// readers read what the macro reads: `MESSAGES`.
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/readers.rs"));

/// The time of a call: the fastest of three runs of about 0.2 s.
fn time(call: fn(&Pdu), pdu: &Pdu) -> f64 {
    let run = |iterations: u32| {
        let started = Instant::now();
        for _ in 0..iterations {
            call(black_box(pdu));
        }
        started.elapsed()
    };
    let mut iterations = 100;
    while run(iterations) < Duration::from_millis(20) {
        iterations *= 2;
    }
    let iterations = iterations * 10;
    let fastest = (0..3).map(|_| run(iterations)).min().unwrap();
    fastest.as_secs_f64() * 1e9 / f64::from(iterations)
}

fn main() {
    let words: Vec<String> = std::env::args().skip(1).collect();
    // The core runs for half a second before the first row, so that its first row is
    // timed at the speed of the others.
    let (_, first) = benches::written().into_iter().next().unwrap();
    let started = Instant::now();
    while started.elapsed() < Duration::from_millis(500) {
        check(black_box(&first));
    }
    println!("| Message | Operation | ns | allocations | octets |");
    println!("|---|---|---|---|---|");
    for (name, pdu) in benches::written() {
        let mut operations = Vec::new();
        for (_, of_message, same) in MESSAGES.iter().filter(|(message, ..)| *message == name) {
            same(&pdu);
            operations.extend(of_message.iter());
        }
        assert!(!operations.is_empty(), "no operation on {name}");
        for (operation, call) in operations.into_iter().chain(CHECKS) {
            let row = format!("{name} {operation}");
            if !words.iter().all(|word| row.contains(word.as_str())) {
                continue;
            }
            let ns = time(*call, &pdu);
            let [count, octets, _] = allocations(|| call(&pdu));
            println!("| {name} | {operation} | {ns:.0} | {count} | {octets} |");
        }
    }
}
