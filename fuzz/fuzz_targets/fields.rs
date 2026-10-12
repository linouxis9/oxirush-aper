#![no_main]
//! See `targets::fields`.

libfuzzer_sys::fuzz_target!(|octets: &[u8]| targets::fields(octets));
