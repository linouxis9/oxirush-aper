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

//! Write the bindings of TOY from the ASN.1 modules of `toy/`, as those of oxirush-ngap
//! and oxirush-s1ap are written: run it from the directory of the crate.

use generator::protocol::{Protocol, generate};

fn main() -> anyhow::Result<()> {
    generate(&Protocol {
        name: "TOY",
        modules: "toy",
        carried: &[("Container", "ContainerBlob")],
        foreign: &[],
        keys: &["SecurityKey"],
        plmn: "PLMNIdentity",
        correct: Ok,
    })
}
