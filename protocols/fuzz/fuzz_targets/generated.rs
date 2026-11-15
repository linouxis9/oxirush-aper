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

#![no_main]

libfuzzer_sys::fuzz_target!(|octets: &[u8]| targets::generated(octets));

/// What AddressSanitizer takes when ASAN_OPTIONS does not say otherwise: the stack of
/// each allocation in 8 frames. It keeps each stack that it records for the life of the
/// process, and the functions that make, write and read a tree call themselves as deep
/// as the tree goes: with whole stacks, the memory of a run of NGAP grew to 2.7 GB in an
/// hour, and the stacks took 700 MB of the 1.2 GB that one pass over its corpus took.
#[unsafe(no_mangle)]
pub extern "C" fn __asan_default_options() -> *const core::ffi::c_char {
    c"malloc_context_size=8".as_ptr()
}
