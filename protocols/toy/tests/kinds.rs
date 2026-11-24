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

//! The kind of a PDU: its message, and the initiating message that an outcome answers.

use oxirush_toy::{Direction, ToyPduKind as Kind};

// The initiating message of an outcome is a constant.
const SETUP: Option<Kind> = Kind::SetupResponse.initiating();

#[test]
fn an_outcome_answers_the_initiating_message_of_its_procedure() {
    assert_eq!(SETUP, Some(Kind::SetupRequest));
    assert_eq!(Kind::SetupResponse.initiating(), Some(Kind::SetupRequest));
    assert_eq!(Kind::SetupFailure.initiating(), Some(Kind::SetupRequest));
}

#[test]
fn an_initiating_message_answers_none() {
    // SETUP has both outcomes, and RELEASE, ERROR INDICATION, the private message and
    // NOTIFICATION have none.
    for kind in [
        Kind::SetupRequest,
        Kind::ReleaseRequest,
        Kind::ErrorIndication,
        Kind::PrivateMessage,
        Kind::Notification,
    ] {
        assert_eq!(kind.initiating(), None, "{kind:?}");
    }
}

#[test]
fn a_procedure_that_the_specification_does_not_have_answers_none() {
    // An outcome of RELEASE, which has none, and procedure 200, which TOY does not have.
    for direction in [
        Direction::Initiating,
        Direction::Successful,
        Direction::Unsuccessful,
    ] {
        for procedure_code in [2, 200] {
            let kind = Kind::Other {
                direction,
                procedure_code,
            };
            assert_eq!(kind.initiating(), None, "{kind:?}");
        }
    }
}
