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

//! The protocol IEs of a PDU, whatever its message: read, and put back; those of a
//! transfer, which the macros read and build as those of a message; and the object sets
//! of the protocol, by its marker type.

use oxirush_toy::aper::BitString;
use oxirush_toy::aper::protocol::{Pdu, holds_key, protocol_ies, set_protocol_ies};
use oxirush_toy::toy::*;
use oxirush_toy::{Direction, Error, ErrorKind, Ie, Octets, Presence, Toy};
use oxirush_toy::{build_toy, extract_toy_ies, ie};

/// The RELEASE REQUEST of `__pdu!`: UE-Core-ID 42, UE-RAN-ID 7 and a Cause.
const RELEASE_REQUEST: &str = "00024014000003000a0002002a000b00020007000f400104";

fn cause() -> Cause {
    Cause::radioNetwork(CauseRadioNetwork::user_inactivity)
}

fn release_request() -> TOYPDU<'static> {
    TOYPDU::decode(&hex::decode(RELEASE_REQUEST).unwrap())
        .unwrap()
        .into_owned()
}

/// A PRIVATE MESSAGE of one private IE.
fn private() -> TOYPDU<'static> {
    let message = PrivateMessage {
        private_ies: PrivateMessagePrivateIEs(vec![AnonymousPrivateMessagePrivateIEs {
            id: PrivateIEID::local(1),
            criticality: Criticality::Ignore,
            value: Octets::from(vec![1, 2]),
        }]),
    };
    TOYPDU::initiatingMessage(InitiatingMessage {
        procedure_code: ProcedureCode(4),
        criticality: Criticality::Ignore,
        value: Octets::owned(oxirush_toy::encode(&message).unwrap()),
    })
}

/// An IE of an identifier and a criticality whose value is `octets`.
fn field(id: u16, criticality: Criticality, octets: &[u8]) -> ProtocolIEField<'static> {
    ProtocolIEField {
        id: ProtocolIEID(id),
        criticality,
        value: Octets::from(octets.to_vec()),
    }
}

#[test]
fn a_message_that_the_macros_built_has_the_ies_that_it_was_built_with() -> Result<(), Error> {
    let pdu = build_toy!(IGNORE ReleaseRequest,
        REJECT UE_Core_ID(42u64),
        REJECT UE_RAN_ID(7u32),
        IGNORE Cause(cause()),
    )?;
    let built = vec![
        ie::UE_Core_ID::field(Criticality::Reject, 42u64)?,
        ie::UE_RAN_ID::field(Criticality::Reject, 7u32)?,
        ie::Cause::field(Criticality::Ignore, cause())?,
    ];
    assert_eq!(pdu.protocol_ies()?.0, built);
    assert_eq!(
        (built[0].id.0, built[0].criticality),
        (10, Criticality::Reject)
    );
    assert_eq!(built[0].value, [0x00, 0x2a]);

    // They are those of the typed message, and of the octets of the release.
    let request: ReleaseRequest = pdu.decode_value()?;
    assert_eq!(pdu.protocol_ies()?, request.protocol_ies);
    assert_eq!(release_request().protocol_ies()?.0, built);
    assert_eq!(pdu.encode()?, hex::decode(RELEASE_REQUEST).unwrap());
    Ok(())
}

#[test]
fn an_unknown_ie_and_a_repeated_one_are_kept_with_their_criticality() -> Result<(), Error> {
    // An IE of the message, an IE that the protocol does not have, the first again with
    // another value and another criticality, and an IE of another message.
    let unknown = field(60000, Criticality::Notify, &[0xc0, 0xff]);
    let mut again = ie::UE_Core_ID::field(Criticality::Reject, 2u64)?;
    again.criticality = Criticality::Ignore;
    let name = ie::NodeName::field(Criticality::Notify, NodeName("node-1".into()))?;
    let ies = vec![
        ie::UE_Core_ID::field(Criticality::Reject, 1u64)?,
        unknown,
        again,
        name,
    ];

    // Put into a message that had three other IEs: the PDU is the same but for them.
    let mut pdu = release_request();
    pdu.set_protocol_ies(ProtocolIEContainer(ies.clone()))?;
    assert_eq!(pdu.kind(), oxirush_toy::ToyPduKind::ReleaseRequest);
    assert_eq!(pdu.criticality(), Criticality::Ignore);

    // They are read back as they were written, from the PDU and from its octets.
    assert_eq!(pdu.protocol_ies()?.0, ies);
    let received = TOYPDU::decode(&pdu.encode()?)?.into_owned();
    assert_eq!(received, pdu);
    let read = received.protocol_ies()?.0;
    assert_eq!(read, ies);
    let told: Vec<_> = read.iter().map(|ie| (ie.id.0, ie.criticality)).collect();
    assert_eq!(
        told,
        [
            (10, Criticality::Reject),
            (60000, Criticality::Notify),
            (10, Criticality::Ignore),
            (20, Criticality::Notify),
        ]
    );
    Ok(())
}

#[test]
fn the_message_of_an_unknown_procedure_is_read_as_a_container_of_protocol_ies() -> Result<(), Error>
{
    // The IEs of the release request, under a procedure code that TOY does not have.
    let known = release_request();
    let TOYPDU::initiatingMessage(mut held) = known.clone() else {
        panic!("{known}");
    };
    held.procedure_code = ProcedureCode(200);
    let mut pdu = TOYPDU::initiatingMessage(held);
    assert!(matches!(pdu.kind(), oxirush_toy::ToyPduKind::Other { .. }));
    let mut ies = pdu.protocol_ies()?.into_owned();
    assert_eq!(ies, known.protocol_ies()?);
    ies.0.pop();
    pdu.set_protocol_ies(ies)?;
    assert_eq!(pdu.protocol_ies()?.0.len(), 2);
    assert_eq!(pdu.procedure_code(), 200);
    Ok(())
}

#[test]
fn a_message_without_protocol_ies_and_octets_that_are_none_are_errors() -> Result<(), Error> {
    // The private message has private IEs, and no container of protocol IEs.
    let mut private = private();
    let before = private.clone();
    let read = private.protocol_ies().unwrap_err();
    assert_eq!(read.kind(), ErrorKind::WrongMessage);
    assert_eq!(
        read.to_string(),
        "the message of this PDU is PrivateMessage, which has no container of protocol IEs"
    );
    let written = private.set_protocol_ies(ProtocolIEContainer(vec![]));
    assert_eq!(written, Err(read));
    assert_eq!(private, before);

    // Octets that are no message: the message is named.
    let request = release_request();
    let TOYPDU::initiatingMessage(mut held) = request.clone() else {
        panic!("{request}");
    };
    held.value = Octets::from(vec![0x00, 0x00, 0x03, 0x00]);
    let short = TOYPDU::initiatingMessage(held.clone());
    let error = short.protocol_ies().unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Decode);
    assert_eq!(
        error.to_string(),
        "the ReleaseRequest of this PDU does not decode: the octets end before the value \
         does, in its field `protocol_ies.id`"
    );
    held.value = Octets::from(vec![]);
    let error = TOYPDU::initiatingMessage(held.clone())
        .protocol_ies()
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "the ReleaseRequest of this PDU does not decode: there are no octets"
    );
    // The same octets under a procedure that the specification does not have.
    held.value = Octets::from(vec![0x00, 0x00, 0x03, 0x00]);
    held.procedure_code = ProcedureCode(200);
    let error = TOYPDU::initiatingMessage(held).protocol_ies().unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Decode);
    assert_eq!(
        error.to_string(),
        "the message of this PDU does not decode as a container of protocol IEs: the octets \
         end before the value does, in its field `protocol_ies.id`"
    );

    // More IEs than a container has: the PDU is as it was.
    let ie = ie::UE_RAN_ID::field(Criticality::Reject, 7u32)?;
    let mut pdu = request.clone();
    let error = pdu
        .set_protocol_ies(ProtocolIEContainer(vec![ie; 65536]))
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Encode);
    assert!(
        error
            .to_string()
            .starts_with("these 65536 protocol IEs do not encode as a message: "),
        "{error}"
    );
    assert_eq!(pdu, request);
    Ok(())
}

/// What the macros write under them: the IEs of a PDU read and set as they are, which
/// the codec of the message writes, and whether its message holds a key.
#[test]
fn the_protocol_ies_of_a_pdu_are_read_and_set_as_they_are() -> Result<(), Error> {
    let pdu = release_request();
    let ies = protocol_ies(&pdu)?;
    let ids: Vec<u16> = ies.iter().map(|ie| ie.id.0).collect();
    assert_eq!(ids, [10, 11, 15]);
    let mut changed = pdu.clone();
    let long = field(30, Criticality::Reject, &[0x61; 200]);
    let mine = vec![field(11, Criticality::Reject, &[0x00, 0x09]), long];
    set_protocol_ies(&mut changed, &mine)?;
    let expected = oxirush_toy::encode(&ReleaseRequest {
        protocol_ies: ProtocolIEContainer(mine.clone()),
    })?;
    assert_eq!(changed.value().as_slice(), &expected[..]);
    assert_eq!(
        protocol_ies(&private()).unwrap_err().kind(),
        ErrorKind::WrongMessage
    );
    // A SETUP REQUEST holds a key, its SecurityKey, and a SETUP RESPONSE one in a value
    // of its own, its SecurityContext; a RELEASE REQUEST holds none.
    assert!(holds_key::<Toy>(Direction::Initiating, 1));
    assert!(holds_key::<Toy>(Direction::Successful, 1));
    assert!(!holds_key::<Toy>(Direction::Initiating, 2));
    Ok(())
}

/// A field of the macro that extracts is the value of the IE when it has the type of
/// the IE, and the one value that the type is a name for when it has the type of that
/// value: a UE-Core-ID is a name for a number, and a Cause is a name for nothing.
#[test]
fn a_field_takes_the_value_of_an_ie_as_its_type_or_as_the_value_that_it_names() -> Result<(), Error>
{
    let pdu = release_request();
    let request: ReleaseRequest = pdu.decode_value()?;
    extract_toy_ies!(request, ReleaseRequest,
        req cause: Cause = Cause(cause),
        req id: UECoreID = UE_Core_ID(id),
        req number: u64 = UE_Core_ID(id),
        opt ran: UERANID = UE_RAN_ID(id),
        lax ran_number: u32 = UE_RAN_ID(id),
        opt twice: u64 = UE_Core_ID(id) => id.0 * 2,
    );
    assert_eq!(cause, self::cause());
    assert_eq!((id, number), (UECoreID(42), 42));
    assert_eq!((ran, ran_number), (Some(UERANID(7)), Some(7)));
    assert_eq!(twice, Some(84));
    Ok(())
}

/// A tunnel to an address of four octets.
fn tunnel() -> Tunnel<'static> {
    Tunnel::gTPTunnel(GTPTunnel {
        transport_layer_address: TransportLayerAddress(BitString::from([127, 0, 0, 22])),
        gtp_teid: GTPTEID([0, 0, 0x20, 0x01]),
        ie_extensions: None,
    })
}

/// The octets of a SETUP TRANSFER of these IEs.
fn transfer(ies: Vec<ProtocolIEField<'_>>) -> Result<Vec<u8>, Error> {
    oxirush_toy::encode(&SetupTransfer {
        protocol_ies: ProtocolIEContainer(ies),
    })
}

/// A transfer of protocol IEs, as the one that an item of a SETUP REQUEST holds as
/// octets, is read, changed and built by the macros as a message is.
#[test]
fn the_ies_of_a_transfer_are_read_changed_and_built_by_the_macros() -> Result<(), Error> {
    let kind = ie::TransferKind::field(Criticality::Reject, TransferKind::ipv4)?;
    let mut written = SetupTransfer {
        protocol_ies: ProtocolIEContainer(vec![kind]),
    };
    let octets = oxirush_toy::encode(&written)?;
    assert_eq!(hex::encode(&octets), "000001001f000100");

    // Read from its octets, by name
    let read: SetupTransfer = oxirush_toy::decode(&octets)?;
    let kind = || -> Result<TransferKind, Error> {
        extract_toy_ies!(read, SetupTransfer,
            req kind: TransferKind = TransferKind(kind) => kind,
        );
        Ok(kind)
    };
    assert_eq!(kind()?, TransferKind::ipv4);
    assert_eq!(read.ie::<ie::TransferKind>().unwrap()?, TransferKind::ipv4);

    // Changed in its place
    let changed = written.set_ie::<ie::TransferKind>(TransferKind::ipv6);
    assert_eq!(changed, Ok(true));
    assert_eq!(
        hex::encode(oxirush_toy::encode(&written)?),
        "000001001f000140"
    );

    // An IE that it does not have names the transfer, with the criticality that the
    // object set of the transfer assigns it
    let missing = || -> Result<(), Error> {
        extract_toy_ies!(read, SetupTransfer, req _tunnel: Tunnel = Tunnel(tunnel) => tunnel);
        Ok(())
    };
    let error = missing().unwrap_err();
    assert_eq!(error.kind(), ErrorKind::MissingIe);
    assert_eq!(
        error.to_string(),
        "this SetupTransfer has no Tunnel, which the field `_tunnel` requires"
    );
    let assigned = error.ie().map(|ie| (ie.id, ie.criticality));
    assert_eq!(assigned, Some((29, oxirush_toy::Criticality::Reject)));
    Ok(())
}

#[test]
fn an_ie_of_a_transfer_has_the_criticality_of_its_object_set() -> Result<(), Error> {
    // The object set of the transfer assigns reject to Tunnel, where that of SETUP
    // REQUEST assigns ignore.
    let ies = [
        ie::Tunnel::field_of::<SetupTransfer>(tunnel())?,
        ie::TransferKind::field_of::<SetupTransfer>(TransferKind::ipv4)?,
        ie::Tunnel::field_of::<SetupRequest>(tunnel())?,
    ];
    let criticalities: Vec<_> = ies.iter().map(|ie| (ie.id.0, ie.criticality)).collect();
    assert_eq!(
        criticalities,
        [
            (29, Criticality::Reject),
            (31, Criticality::Reject),
            (29, Criticality::Ignore),
        ]
    );
    // An IE that the object set of the transfer does not have is assigned none
    let error = ie::NodeName::field_of::<SetupTransfer>(NodeName("node-1".into())).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Argument);
    assert_eq!(
        error.to_string(),
        "NodeName is no IE of the object set of SetupTransfer, which assigns it no \
         criticality: write its criticality"
    );
    Ok(())
}

#[test]
fn a_transfer_is_checked_against_its_object_set() -> Result<(), Error> {
    // A transfer as the specification has a sender construct it deviates in nothing
    let octets = transfer(vec![
        ie::Tunnel::field_of::<SetupTransfer>(tunnel())?,
        ie::TransferKind::field_of::<SetupTransfer>(TransferKind::ipv4)?,
    ])?;
    assert_eq!(SetupTransfer::check(&octets)?, []);
    assert_eq!(SetupTransfer::check_structure(&octets)?, []);
    // One that has an IE of another criticality, one out of its order and one that its
    // set does not have
    let octets = transfer(vec![
        ie::TransferKind::field_of::<SetupTransfer>(TransferKind::ipv4)?,
        ie::Tunnel::field(Criticality::Ignore, tunnel())?,
        ie::NodeName::field(Criticality::Ignore, NodeName("node-1".into()))?,
    ])?;
    let found = SetupTransfer::check_structure(&octets)?;
    let found: Vec<_> = found.iter().map(ToString::to_string).collect();
    assert_eq!(
        found,
        [
            "Tunnel (IE 29), at position 1, comes after TransferKind (IE 31), which the object \
             set of the message has after it",
            "the criticality of Tunnel (IE 29), at position 1, is ignore, and the message \
             assigns it reject",
            "NodeName (IE 20), at position 2, is not an IE of the message",
        ]
    );
    // One that misses its Tunnel, which is mandatory
    let octets = transfer(vec![ie::TransferKind::field_of::<SetupTransfer>(
        TransferKind::ipv4,
    )?])?;
    let found = SetupTransfer::check_structure(&octets)?;
    let found: Vec<_> = found.iter().map(ToString::to_string).collect();
    assert_eq!(
        found,
        ["Tunnel (IE 29) is mandatory, and the message does not have it"]
    );
    assert_eq!(
        SetupTransfer::check(&octets)?,
        SetupTransfer::check_structure(&octets)?
    );
    // `check` decodes the values too: one that does not decode is reported
    let mut ies = vec![
        ie::Tunnel::field_of::<SetupTransfer>(tunnel())?,
        ie::TransferKind::field_of::<SetupTransfer>(TransferKind::ipv4)?,
    ];
    ies[1].value = Octets::owned(vec![]);
    let octets = transfer(ies)?;
    assert_eq!(SetupTransfer::check_structure(&octets)?, []);
    let found = SetupTransfer::check(&octets)?;
    let found: Vec<_> = found.iter().map(ToString::to_string).collect();
    assert_eq!(
        found,
        ["TransferKind (IE 31), at position 1, does not decode: there are no octets"]
    );
    // Octets that are no container of protocol IEs
    let error = SetupTransfer::check(&[0x00, 0x00]).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Decode);
    assert!(
        error
            .to_string()
            .starts_with("these octets do not decode as SetupTransfer"),
        "{error}"
    );
    Ok(())
}

/// The object sets of TOY, by its marker type: each message with the criticality of its
/// procedure and its IEs, found by its direction and its procedure code, or by its name
/// however it is written.
#[test]
fn the_marker_of_a_protocol_finds_the_object_set_of_each_message() {
    let sets = Toy::object_sets();
    assert_eq!(sets.len(), 7);
    let request = Toy::object_set(Direction::Initiating, 1).unwrap();
    assert_eq!(request.name, "SetupRequest");
    assert_eq!(request.criticality, Criticality::Reject);
    let ies = request.ies.unwrap();
    assert_eq!(ies[0], (26, Criticality::Reject, Presence::Mandatory));
    assert_eq!(ies[1], (20, Criticality::Ignore, Presence::Optional));
    assert_eq!(Toy::object_set_named("setup-request"), Some(request));
    assert_eq!(Toy::object_set_named("SETUP REQUEST"), Some(request));
    assert_eq!(Toy::object_set_named("setup"), None);
    let response = Toy::object_set(Direction::Successful, 1).unwrap();
    assert_eq!(
        response.ies.unwrap()[1],
        (33, Criticality::Reject, Presence::Conditional)
    );
    let release = Toy::object_set(Direction::Initiating, 2).unwrap();
    assert_eq!(release.criticality, Criticality::Ignore);
    assert_eq!(Toy::object_set(Direction::Successful, 2), None);
    assert_eq!(Toy::object_set_named("PrivateMessage").unwrap().ies, None);
    assert_eq!(Toy::ie_names()[0], (10, "UE-Core-ID"));
    // An IE has the criticality that the set of each message gives it.
    let error = Toy::object_set_named("ErrorIndication")
        .unwrap()
        .ies
        .unwrap();
    assert_eq!(error[0], (10, Criticality::Ignore, Presence::Optional));
    // Every message is found by its direction and its procedure code, and has the IEs of
    // its set, each of them known.
    for set in Toy::object_sets() {
        assert_eq!(
            Toy::object_set(set.direction, set.procedure_code),
            Some(set)
        );
        let ies = set.ies.unwrap_or_default();
        assert!(
            set.name == "PrivateMessage" || !ies.is_empty(),
            "{}",
            set.name
        );
        for (ie, ..) in ies {
            let known = Toy::ie_names().iter().any(|(known, _)| known == ie);
            assert!(known, "{} {ie}", set.name);
        }
    }
}

/// An IE that a value holds alone, in a single container, has the criticality of its
/// object set: the Item of the list of Items, and the ExtendedCause that extends the
/// CHOICE of a cause.
#[test]
fn an_ie_that_a_value_holds_alone_has_the_criticality_of_its_object_sets() {
    assert_eq!(Toy::item_criticality(23), Some(Criticality::Reject));
    assert_eq!(Toy::item_criticality(41), Some(Criticality::Ignore));
    // The IEs of a message and those of a transfer are in no single container.
    assert_eq!(Toy::item_criticality(10), None);
    assert_eq!(Toy::item_criticality(31), None);
    assert_eq!(Toy::item_criticality(u16::MAX), None);
}

#[test]
fn a_kind_that_the_specification_does_not_have_says_the_direction_of_its_pdu() {
    let pdu = TOYPDU::unsuccessfulOutcome(UnsuccessfulOutcome {
        procedure_code: ProcedureCode(2),
        criticality: Criticality::Ignore,
        value: Octets::from(vec![0]),
    });
    let oxirush_toy::ToyPduKind::Other {
        direction,
        procedure_code,
    } = pdu.kind()
    else {
        panic!("{:?}", pdu.kind());
    };
    assert_eq!((direction, procedure_code), (pdu.direction(), 2));
}
