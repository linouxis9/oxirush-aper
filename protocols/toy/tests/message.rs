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

//! The message of a PDU as its type, and how long it lives: as long as the PDU, with
//! `decode_value`, and after it once `into_owned()` copies its octets.

use oxirush_toy::toy::*;
use oxirush_toy::{Error, ErrorKind, Ie, Message, build_toy, ie};

/// A RELEASE REQUEST of four IEs: UE-Core-ID 42, UE-RAN-ID 7, a Cause and a NAS-PDU of
/// three octets.
const RELEASE_REQUEST: &str = "0002401c000004000a0002002a000b00020007000f400104001e0004037e0041";

#[test]
fn a_message_borrows_its_octets_from_what_the_pdu_was_decoded_from() -> Result<(), Error> {
    let received = hex::decode(RELEASE_REQUEST).unwrap();
    let pdu = TOYPDU::decode(&received)?;
    let message: ReleaseRequest = pdu.decode_value()?;
    let within = received.as_ptr_range();
    for ie in &message.protocol_ies.0 {
        assert!(ie.value.is_borrowed());
        assert!(within.contains(&ie.value.as_slice().as_ptr()));
    }
    let nas = &message.protocol_ies.0[3].value;
    assert_eq!(nas.as_slice(), [0x03, 0x7e, 0x00, 0x41]);
    Ok(())
}

#[test]
fn a_message_that_is_kept_after_the_pdu_owns_its_octets() -> Result<(), Error> {
    let received = hex::decode(RELEASE_REQUEST).unwrap();
    let kept: ReleaseRequest<'static> = {
        let pdu = TOYPDU::decode(&received)?;
        pdu.decode_value::<ReleaseRequest>()?.into_owned()
    };
    drop(received);
    assert!(kept.protocol_ies.0.iter().all(|ie| !ie.value.is_borrowed()));
    assert_eq!(kept.protocol_ies.0[1].value.as_slice(), [0x00, 0x07]);
    Ok(())
}

#[test]
fn a_message_of_another_type_is_refused() -> Result<(), Error> {
    let received = hex::decode(RELEASE_REQUEST).unwrap();
    let pdu = TOYPDU::decode(&received)?;
    let refused = pdu.decode_value::<SetupRequest>().unwrap_err();
    assert_eq!(refused.kind(), ErrorKind::WrongMessage);
    Ok(())
}

/// A message of 16384 octets and more is in fragments (X.691 clause 11.9.3.8), which
/// are gathered: the PDU then owns the octets of its message, whatever it was decoded
/// from.
#[test]
fn a_message_in_fragments_is_gathered() -> Result<(), Error> {
    // A NAS-PDU of 20000 octets in a RELEASE REQUEST
    let nas = NASPDU(vec![0x7e; 20_000].into());
    let ie = ProtocolIEField {
        id: ProtocolIEID(30),
        criticality: Criticality::Reject,
        value: oxirush_toy::encode(&nas)?.into(),
    };
    let mut pdu = TOYPDU::decode(&hex::decode(RELEASE_REQUEST).unwrap())?.into_owned();
    pdu.set_protocol_ies(ProtocolIEContainer(vec![ie]))?;
    let received = pdu.encode()?;
    assert!(received.len() > 20_000);

    let again = TOYPDU::decode(&received)?;
    let message: ReleaseRequest = again.decode_value()?;
    assert_eq!(message.protocol_ies.0.len(), 1);
    assert_eq!(message, pdu.decode_value::<ReleaseRequest>()?);
    Ok(())
}

/// `Message::CRITICALITY` is the one of its procedure, and the macros give a PDU and an
/// IE whose criticality a call leaves out the one that the specification assigns.
#[test]
fn a_criticality_that_a_call_leaves_out_is_the_one_that_is_assigned() -> Result<(), Error> {
    let cause = || Cause::radioNetwork(CauseRadioNetwork::user_inactivity);
    assert_eq!(ReleaseRequest::CRITICALITY, Criticality::Ignore);
    assert_eq!(SetupRequest::CRITICALITY, Criticality::Reject);
    let written = build_toy!(IGNORE ReleaseRequest,
        REJECT UE_Core_ID(42u64),
        REJECT UE_RAN_ID(7u32),
        IGNORE Cause(cause()),
    )?;
    let assigned = build_toy!(
        ReleaseRequest,
        UE_Core_ID(42u64),
        UE_RAN_ID(7u32),
        Cause(cause()),
    )?;
    assert_eq!(assigned, written);
    assert!(assigned.check()?.is_empty());
    // Some written and some left out, in any order.
    let mixed = build_toy!(IGNORE ReleaseRequest,
        UE_Core_ID(42u64),
        REJECT UE_RAN_ID(7u32),
        Cause(cause()),
    )?;
    assert_eq!(mixed, written);
    let message: ReleaseRequest = assigned.decode_value()?;
    let again = TOYPDU::of_message(&message, ReleaseRequest::CRITICALITY)?;
    assert_eq!(again, written);
    // An IE that the object set of the message does not have is assigned none.
    let error = build_toy!(SetupRequest, UE_Core_ID(1u64),).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Argument);
    assert!(
        error
            .to_string()
            .contains("no IE of the object set of SetupRequest"),
        "{error}"
    );
    // One IE, of its message.
    let ie = ie::NodeName::field_of::<SetupRequest>(NodeName("node-1".into()))?;
    assert_eq!(ie.criticality, Criticality::Ignore);
    let error = ie::UE_Core_ID::field_of::<SetupRequest>(1u64).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Argument);
    // The procedure assigns reject to a SETUP REQUEST, and so does its object set to
    // its Areas, and ignore to its NodeName.
    let pdu = build_toy!(
        SetupRequest,
        Areas(areas()),
        NodeName(NodeName("node-1".into())),
    )?;
    let ies: Vec<_> = (pdu.protocol_ies()?.0.iter())
        .map(|ie| (ie.id.0, ie.criticality))
        .collect();
    assert_eq!(ies, [(26, Criticality::Reject), (20, Criticality::Ignore)]);
    assert!(pdu.check()?.is_empty());
    Ok(())
}

/// One tracking area of one network.
fn areas() -> Areas {
    Areas(vec![AreaItem {
        tac: TAC([0, 0, 1]),
        broadcast_plmns: BroadcastPLMNs(vec![PLMNIdentity([0x02, 0xf8, 0x39])]),
        ie_extensions: None,
    }])
}
