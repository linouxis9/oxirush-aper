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

//! `Debug` prints no octet of a key, a `SecurityKey`: it shows the length, as does
//! every value that holds one. The octets of an IE and of a message whose type holds a
//! key, itself or in a value of its own, are shown by their length too. A tree is the
//! data of a message, and has the key.

use oxirush_toy::toy::*;
use oxirush_toy::{Error, build_toy, extract_toy_ies};

/// A key of which every octet is `5a`, and another of `a5`.
const KEY: [u8; 32] = [0x5a; 32];
const NEXT: [u8; 32] = [0xa5; 32];

/// What `Debug` printed has no four octets of the key in hexadecimal, in either case,
/// nor as numbers.
fn has_no_key(printed: &str) -> bool {
    let lower = printed.to_ascii_lowercase();
    !["5a5a5a5a", "a5a5a5a5", "90, 90, 90", "165, 165, 165"]
        .iter()
        .any(|key| lower.contains(key))
}

/// A SETUP REQUEST with the key.
fn setup() -> Result<TOYPDU<'static>, Error> {
    let area = AreaItem {
        tac: TAC::from([0, 0, 1]),
        broadcast_plmns: BroadcastPLMNs(vec![PLMNIdentity::from([0x02, 0xf8, 0x39])]),
        ie_extensions: None,
    };
    build_toy!(
        SetupRequest,
        Areas(Areas(vec![area])),
        SecurityKey(SecurityKey::from(KEY)),
    )
}

#[test]
fn debug_of_a_key_and_of_what_holds_one_shows_its_length() {
    let key = SecurityKey::from(KEY);
    assert_eq!(format!("{key:?}"), "SecurityKey(256 bits)");
    let context = SecurityContext {
        next_hop_chaining_count: 1,
        next_hop_nh: SecurityKey::from(NEXT),
        ie_extensions: None,
    };
    let printed = format!("{context:?}");
    assert!(has_no_key(&printed), "{printed}");
    assert!(
        printed.contains("next_hop_nh: SecurityKey(256 bits)"),
        "{printed}"
    );
    // The key is the value that it was, and compares as such.
    assert_eq!(key, SecurityKey::from(KEY));
    assert_ne!(key, SecurityKey::from(NEXT));
}

#[test]
fn debug_of_a_message_with_a_key_shows_the_length_of_its_octets() -> Result<(), Error> {
    let octets = setup()?.encode()?;
    assert!(hex::encode(&octets).contains(&"5a".repeat(32)));
    let pdu = TOYPDU::decode(&octets)?;
    let printed = format!("{pdu:?}");
    assert!(has_no_key(&printed), "{printed}");
    assert!(
        printed.contains(&format!("value: {} octets", octets.len() - 4)),
        "{printed}"
    );
    // Its IEs: the area is printed, and the key is 32 octets
    let ies = pdu.protocol_ies()?;
    let printed = format!("{ies:?}");
    assert!(has_no_key(&printed), "{printed}");
    assert!(printed.contains("02f839"), "{printed}");
    assert!(
        printed.contains("id: ProtocolIEID(21), criticality: Reject, value: 32 octets"),
        "{printed}"
    );
    // Its message, and the key that is read from it
    let message: SetupRequest = pdu.decode_value()?;
    let printed = format!("{message:?}");
    assert!(has_no_key(&printed), "{printed}");
    extract_toy_ies!(message, SetupRequest,
        req key: SecurityKey = SecurityKey(key) => key,
    );
    assert_eq!(format!("{key:?}"), "SecurityKey(256 bits)");
    assert_eq!(key, SecurityKey::from(KEY));
    Ok(())
}

#[test]
fn debug_of_a_message_with_a_value_that_holds_a_key_shows_no_key() -> Result<(), Error> {
    // A SETUP RESPONSE with the next key
    let context = SecurityContext {
        next_hop_chaining_count: 1,
        next_hop_nh: SecurityKey::from(NEXT),
        ie_extensions: None,
    };
    let pdu = build_toy!(SetupResponse, SecurityContext(context),)?;
    assert!(hex::encode(pdu.encode()?).contains(&"a5".repeat(32)));
    let printed = format!("{pdu:?} {:?}", pdu.protocol_ies()?);
    assert!(has_no_key(&printed), "{printed}");
    assert!(
        printed.contains("id: ProtocolIEID(33), criticality: Reject, value: 33 octets"),
        "{printed}"
    );
    Ok(())
}

#[test]
fn debug_of_a_message_without_a_key_shows_its_octets() -> Result<(), Error> {
    // A RELEASE REQUEST: its object set has no IE that holds a key
    let pdu = build_toy!(
        ReleaseRequest,
        UE_Core_ID(1u64),
        UE_RAN_ID(7u32),
        Cause(Cause::nas(CauseNas::normal_release)),
        NAS_PDU(vec![0x7e, 0x00, 0x5d]),
    )?;
    let printed = format!("{pdu:?}");
    assert!(printed.contains("7e005d"), "{printed}");
    let printed = format!("{:?}", pdu.protocol_ies()?);
    assert!(printed.contains("value: 037e005d"), "{printed}");
    Ok(())
}

#[cfg(feature = "inspect")]
#[test]
fn a_tree_has_the_key() -> Result<(), Error> {
    let written = setup()?.to_tree().to_string().to_ascii_lowercase();
    assert!(written.contains(&"5a".repeat(32)), "{written}");
    Ok(())
}

/// What is written in the place of a key is not shown in a refusal, which still says
/// why.
#[cfg(feature = "inspect")]
#[test]
fn a_refusal_of_a_key_shows_nothing_of_what_is_written() {
    use oxirush_toy::tree;

    let key = "5a".repeat(32);
    let wrong = |value: tree::Value| {
        let tree = tree!({"message": "SetupRequest", "ies": [{"SecurityKey": value}]});
        TOYPDU::from_tree(&tree).unwrap_err().to_string()
    };
    for (written, why) in [
        (
            tree!({"key": key.clone()}),
            "a bit string of 256 bits is expected, in hexadecimal, and a secret value is written",
        ),
        (
            tree!(format!("{key}5")),
            "a secret value is not hexadecimal, two digits for each octet",
        ),
        (
            tree!(key[..62].to_string()),
            "31 octets are written for a bit string of 256 bits, which has 32",
        ),
    ] {
        let error = wrong(written);
        assert!(has_no_key(&error), "{error}");
        assert!(error.contains(why), "{error}");
    }
}
