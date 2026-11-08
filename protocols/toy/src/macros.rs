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

//! The macros of TOY, as those of oxirush-ngap and oxirush-s1ap: those of oxirush-aper
//! with the bindings of the protocol.

/// Read the IEs of a message of TOY, as `extract_ngap_ies!` reads those of NGAP.
#[macro_export]
macro_rules! extract_toy_ies {
    ($($arguments:tt)*) => {
        $crate::aper::__extract_ies!($crate, toy, $($arguments)*)
    };
}

/// Build a PDU of TOY, as `build_ngap!` builds one of NGAP.
#[macro_export]
macro_rules! build_toy {
    ($($arguments:tt)*) => {
        $crate::aper::__build!($crate, toy, "build_toy!", $($arguments)*)
    };
}
