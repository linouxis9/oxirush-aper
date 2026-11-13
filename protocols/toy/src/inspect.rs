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

//! The tree of a PDU of TOY and its paths, as oxirush-ngap and oxirush-s1ap have them:
//! their documentation says what they are.

// The tables, the paths and the functions of the module, which oxirush-aper writes for
// both protocols.
oxirush_aper::__inspect! {
    bindings toy, pdu TOYPDU, any crate::toy::ErrorIndication<'a>
}

use crate::toy::TOYPDU;
use oxirush_aper::Octets;
use oxirush_aper::protocol::Pdu;

/// The tree of a PDU, with the `inspect` feature: the short form in which a message is
/// shown and written, which the paths of [`inspect`](crate::inspect) read and edit.
impl TOYPDU<'_> {
    /// The tree of this PDU: the name of its message and its IEs by name.
    pub fn to_tree(&self) -> Value {
        let (direction, code) = (self.direction(), self.procedure_code());
        let criticality = self.criticality().to_tree();
        shared::tree(&TABLES, direction, code, criticality, self.value())
    }

    /// The PDU of a tree, as [`to_tree`](Self::to_tree) shows one.
    pub fn from_tree(tree: &Value) -> Result<TOYPDU<'static>, Error> {
        let parts = shared::tree_parts(&TABLES, tree)?;
        let criticality = json::from_value(&parts.criticality)?;
        let value = Octets::owned(parts.message);
        Ok(TOYPDU::__of(
            parts.direction,
            parts.code,
            criticality,
            value,
        ))
    }
}
