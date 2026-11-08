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

//! The types of a protocol with the codec of another implementation of aligned PER,
//! `rasn`, for the benchmarks of the protocol crates, which time the two codecs on the
//! same PDUs: the generator of a crate writes these bindings from what rasn-compiler
//! resolves of the ASN.1, beside those of the crate, with the same types by the same
//! names.
//!
//! rasn 0.29 does not follow ITU-T X.691 everywhere, so a few codecs of these bindings
//! are written here in its place, each with its clause: `per` has the SEQUENCE and the
//! list, `sized` the strings whose length is octet-aligned, and the generator the
//! others. Without them rasn reads no PDU of the benchmarks as the crate does.

// The bindings of the protocol, which the generator of the crate writes in its
// `rasn/src/`.
#[allow(clippy::all, missing_docs)]
pub mod bindings {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bindings.rs"));
}
pub mod per;
pub mod sized;

pub use rasn;

/// The value of the type `T` that `octets` are the complete encoding of, in aligned PER:
/// one value, whole, and no octet after it.
pub fn decode<T: rasn::Decode>(octets: &[u8]) -> Result<T, String> {
    if octets.is_empty() {
        return Err("there are no octets".into());
    }
    match rasn::aper::decode_with_remainder::<T>(octets) {
        // A value of no bit has exactly one zero octet as its complete encoding.
        Ok((value, rest)) if rest.is_empty() || (octets == [0] && rest == octets) => Ok(value),
        Ok((_, rest)) => Err(format!("the value is followed by {} octets", rest.len())),
        Err(error) => Err(error.to_string()),
    }
}

/// The complete encoding of `value` in aligned PER, in which an empty encoding is one
/// zero octet (X.691 (02/2021) §11.1.4).
pub fn encode<T: rasn::Encode>(value: &T) -> Result<Vec<u8>, String> {
    let mut octets = rasn::aper::encode(value).map_err(|error| error.to_string())?;
    if octets.is_empty() {
        octets.push(0);
    }
    Ok(octets)
}
