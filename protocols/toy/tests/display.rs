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

//! `Display` of a PDU: the name of its message, its procedure code and its direction,
//! then the identifiers of its UE, whether the message has them as two IEs or as one,
//! UE-IDs, which holds the pair or the core identifier alone.

use oxirush_toy::toy::*;
use oxirush_toy::{Error, build_toy};

/// The PDU that the octets of `pdu` decode to: what a peer receives.
fn received(pdu: &TOYPDU<'_>) -> Result<TOYPDU<'static>, Error> {
    Ok(TOYPDU::decode(&pdu.encode()?)?.into_owned())
}

#[test]
fn a_pdu_shows_the_identifiers_of_its_ue_however_its_message_carries_them() -> Result<(), Error> {
    // A NOTIFICATION, with the pair
    let pair = UEIDPair {
        ue_core_id: UECoreID(42),
        ue_ran_id: UERANID(7),
        ie_extensions: None,
    };
    let notification = build_toy!(Notification, UE_IDs(UEIDs::uE_ID_pair(pair)),)?;
    assert_eq!(
        received(&notification)?.to_string(),
        "Notification (procedure 5, initiating message), UE-Core-ID 42, UE-RAN-ID 7"
    );
    // and with the core identifier alone
    let notification = build_toy!(Notification, UE_IDs(UEIDs::uE_Core_ID(UECoreID(42))),)?;
    assert_eq!(
        received(&notification)?.to_string(),
        "Notification (procedure 5, initiating message), UE-Core-ID 42"
    );
    // A RELEASE REQUEST has them as two IEs, and a SETUP REQUEST none
    let release = build_toy!(
        ReleaseRequest,
        UE_Core_ID(42u64),
        UE_RAN_ID(7u32),
        Cause(Cause::nas(CauseNas::normal_release)),
    )?;
    assert_eq!(
        received(&release)?.to_string(),
        "ReleaseRequest (procedure 2, initiating message), UE-Core-ID 42, UE-RAN-ID 7"
    );
    let area = AreaItem {
        tac: TAC::from([0, 0, 1]),
        broadcast_plmns: BroadcastPLMNs(vec![PLMNIdentity::from([0x02, 0xf8, 0x39])]),
        ie_extensions: None,
    };
    let setup = build_toy!(SetupRequest, Areas(Areas(vec![area])),)?;
    assert_eq!(
        received(&setup)?.to_string(),
        "SetupRequest (procedure 1, initiating message)"
    );
    Ok(())
}
