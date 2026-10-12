#![no_main]
//! See `targets::value`.

libfuzzer_sys::fuzz_target!(|octets: &[u8]| targets::value(octets));
