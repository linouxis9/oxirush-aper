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

//! TOY, a protocol of five procedures in the form of the application protocols of 3GPP:
//! the bindings that the generator of oxirush-aper writes from the ASN.1 of `toy/`, with
//! the macros of oxirush-aper over them, as oxirush-ngap and oxirush-s1ap have them. The
//! tests of this crate are those of what the generated crates share.

#![forbid(unsafe_code)]

extern crate alloc;

oxirush_aper::__hooks! { cfg oxirush_toy_hooks, bindings toy, pdu TOYPDU }

#[cfg(feature = "inspect")]
pub mod inspect;
#[cfg(feature = "inspect")]
mod inspect_registry;
pub mod macros;
#[doc(hidden)]
pub mod registry;
#[rustfmt::skip]
pub mod toy;

oxirush_aper::__pdu! {
    pdu TOYPDU, protocol Toy, name oxirush_toy, bindings toy,
    specification "the ASN.1 of TOY", construction "TOY-PDU-Descriptions", diagnostics "CriticalityDiagnostics",
    release "00024014000003000a0002002a000b00020007000f400104",
    display "ReleaseRequest (procedure 2, initiating message), UE-Core-ID 42, UE-RAN-ID 7",
    ue UE_Core_ID UE_RAN_ID,
    pair UE_IDs UEIDs uE_ID_pair(ue_core_id, ue_ran_id) uE_Core_ID
}

pub use oxirush_aper as aper;
pub use oxirush_utils::identity;
#[cfg(feature = "inspect")]
pub use oxirush_utils::tree;

pub use oxirush_aper::protocol::{Ie, Message};
pub use oxirush_aper::{
    AsnType, BitString, Bits, Criticality, Decode, Direction, Encode, Error, ErrorKind, Fault,
    Faults, ObjectSet, Octets, Presence, decode, encode,
};
pub use registry::Deviation;
pub use registry::Toy;
pub use registry::ie;
pub use toy::ToyPduKind;
