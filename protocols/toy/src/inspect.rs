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
use oxirush_aper::json::{self, Tree};
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

#[cfg(test)]
mod kinds {
    use super::*;

    /// What a type is to the paths, as `inspect::kinds` makes it from the table of the
    /// types: the shape of a member names its type, its list, its container or the type
    /// that the octets of a transfer contain.
    #[test]
    fn the_paths_read_what_a_type_is_from_the_table_of_the_types() {
        let members = |name: &str| match kind_of(name) {
            Some(Kind::Sequence(members) | Kind::Choice(members)) => members.to_vec(),
            other => panic!("{name}: {other:?}"),
        };
        let shape = |name: &str| match kind_of(name) {
            Some(Kind::Is(shape)) => *shape,
            other => panic!("{name}: {other:?}"),
        };
        // A bit string whose size varies has its length; one of one size has none.
        assert_eq!(shape("Algorithms"), Shape::Bits);
        let context = members("SecurityContext");
        assert_eq!(context[1], ("nextHopNH", Shape::Plain, false));
        assert_eq!(context[2], ("iE-Extensions", Shape::Extensions(""), true));
        // A list, of a type or of the field of an IE.
        assert_eq!(shape("Areas"), Shape::List(&Shape::Named("AreaItem")));
        assert_eq!(shape("Items"), Shape::List(&Shape::Ie(None)));
        // A transfer is the type that its octets contain; NULL is an alternative.
        let transfer = members("TransferItem");
        assert_eq!(
            transfer[1],
            ("setupTransfer", Shape::Transfer("SetupTransfer"), false)
        );
        assert_eq!(transfer[2].1, Shape::Transfer("FailureTransfer"));
        assert_eq!(members("NodeID")[2], ("unassigned", Shape::Null, false));
        assert_eq!(
            members("SetupTransfer"),
            [("protocolIEs", Shape::Ies(None), false)]
        );
        // An ENUMERATED has its names, in the order of their indexes, and no members.
        assert!(kind_of("CauseNas").is_none());
        let names = TABLES.enumeration("CauseNas").expect("an ENUMERATED");
        assert!(names.len() > 1 && names.iter().all(|name| !name.is_empty()));
    }
}
