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

//! What `check()` reports of a PDU against the object set of its message, and that it
//! refuses nothing: a message that deviates is built by the macros, written from a
//! tree, encoded and decoded as any other.
//!
//! TS 38.413 clause 9.4.1 and TS 36.413 clause 9.3.1 have what a sender constructs: the
//! IEs in the order of the object set, a mandatory IE once, and an optional or a
//! conditional IE at most once. Clause 10.3 of both has what a receiver does with a
//! message that is not so: with an IE that it does not comprehend (10.3.4), with a
//! missing one (10.3.5), and with IEs in a wrong order or with too many occurrences
//! (10.3.6). These tests are for a tool that sends such a message to see what its peer
//! does.

use oxirush_toy::aper::check::{Wrong, deviations};
use oxirush_toy::aper::{decode_strict, decode_with_faults};
use oxirush_toy::toy::*;
use oxirush_toy::{Deviation, Direction, Error, ErrorKind, Fault, Faults, Octets, Presence};
use oxirush_toy::{Toy, build_toy};

/// What `check()` reports of `pdu`, which is the same for the PDU that its octets
/// decode to. With the `inspect` feature, `tree` is the same message written as a
/// tree: it has the same octets.
fn checked(pdu: &TOYPDU<'_>, tree: &str) -> Vec<Deviation> {
    let deviations = pdu.check().unwrap();
    let octets = pdu.encode().unwrap();
    let received = TOYPDU::decode(&octets).unwrap();
    assert_eq!(received, *pdu);
    assert_eq!(received.check().unwrap(), deviations);
    #[cfg(feature = "inspect")]
    {
        let tree = serde_json::from_str(tree).unwrap();
        let written = TOYPDU::from_tree(&tree).unwrap();
        assert_eq!(written.encode().unwrap(), octets, "{tree}");
        assert_eq!(written.check().unwrap(), deviations);
    }
    let _ = tree;
    deviations
}

fn cause() -> Cause {
    Cause::radioNetwork(CauseRadioNetwork::user_inactivity)
}

#[test]
fn a_message_as_its_object_set_has_it_deviates_in_nothing() -> Result<(), Error> {
    // RELEASE REQUEST: its three mandatory IEs, in the order of the set.
    let pdu = build_toy!(IGNORE ReleaseRequest,
        REJECT UE_Core_ID(42u64),
        REJECT UE_RAN_ID(7u32),
        IGNORE Cause(cause()),
    )?;
    let tree = r#"{"message": "ReleaseRequest", "ies": [
        {"UE-Core-ID": 42},
        {"UE-RAN-ID": 7},
        {"Cause": {"radioNetwork": "user-inactivity"}}
    ]}"#;
    assert_eq!(checked(&pdu, tree), []);
    Ok(())
}

#[test]
fn a_mandatory_ie_that_is_missing_is_reported() -> Result<(), Error> {
    // Without its Cause, whose presence is mandatory.
    let pdu = build_toy!(IGNORE ReleaseRequest,
        REJECT UE_Core_ID(42u64),
        REJECT UE_RAN_ID(7u32),
    )?;
    let tree = r#"{"message": "ReleaseRequest", "ies": [
        {"UE-Core-ID": 42},
        {"UE-RAN-ID": 7}
    ]}"#;
    let missing = Deviation::MissingIe {
        id: 15,
        name: "Cause",
        criticality: Criticality::Ignore,
    };
    assert_eq!(checked(&pdu, tree), [missing]);
    Ok(())
}

/// A SETUP RESPONSE has a SecurityContext when the prose of its procedure says so: the
/// object set has it conditional, which is not mandatory.
#[test]
fn a_conditional_ie_is_not_reported_missing() -> Result<(), Error> {
    let pdu = build_toy!(SetupResponse, NodeName(NodeName("node-1".into())),)?;
    assert_eq!(pdu.check()?, []);
    Ok(())
}

#[test]
fn an_ie_that_is_there_twice_is_reported() -> Result<(), Error> {
    let pdu = build_toy!(IGNORE ReleaseRequest,
        REJECT UE_Core_ID(42u64),
        REJECT UE_RAN_ID(7u32),
        IGNORE Cause(cause()),
        REJECT UE_Core_ID(43u64),
    )?;
    let tree = r#"{"message": "ReleaseRequest", "ies": [
        {"UE-Core-ID": 42},
        {"UE-RAN-ID": 7},
        {"Cause": {"radioNetwork": "user-inactivity"}},
        {"UE-Core-ID": 43}
    ]}"#;
    let repeated = Deviation::RepeatedIe {
        id: 10,
        name: "UE-Core-ID",
        position: 3,
        first: 0,
    };
    assert_eq!(checked(&pdu, tree), [repeated]);
    Ok(())
}

#[test]
fn an_ie_of_another_message_is_reported() -> Result<(), Error> {
    // NodeName is an IE of the setup procedure.
    let pdu = build_toy!(IGNORE ReleaseRequest,
        REJECT UE_Core_ID(42u64),
        REJECT UE_RAN_ID(7u32),
        IGNORE NodeName(NodeName("node-1".into())),
        IGNORE Cause(cause()),
    )?;
    let tree = r#"{"message": "ReleaseRequest", "ies": [
        {"UE-Core-ID": 42},
        {"UE-RAN-ID": 7},
        {"NodeName": "node-1", "criticality": "ignore"},
        {"Cause": {"radioNetwork": "user-inactivity"}}
    ]}"#;
    let unexpected = Deviation::UnexpectedIe {
        id: 20,
        name: Some("NodeName"),
        position: 2,
        criticality: Criticality::Ignore,
    };
    assert_eq!(checked(&pdu, tree), [unexpected]);
    Ok(())
}

#[test]
fn ies_that_are_out_of_the_order_of_the_object_set_are_reported() -> Result<(), Error> {
    // The two identifiers in the other order.
    let pdu = build_toy!(IGNORE ReleaseRequest,
        REJECT UE_RAN_ID(7u32),
        REJECT UE_Core_ID(42u64),
        IGNORE Cause(cause()),
    )?;
    let tree = r#"{"message": "ReleaseRequest", "ies": [
        {"UE-RAN-ID": 7},
        {"UE-Core-ID": 42},
        {"Cause": {"radioNetwork": "user-inactivity"}}
    ]}"#;
    let out_of_order = Deviation::OutOfOrder {
        id: 10,
        name: "UE-Core-ID",
        position: 1,
        after: 11,
        after_name: "UE-RAN-ID",
    };
    assert_eq!(checked(&pdu, tree), [out_of_order]);
    Ok(())
}

#[test]
fn a_criticality_that_is_not_the_assigned_one_is_reported() -> Result<(), Error> {
    // The Cause with `reject`, where the object set of the message has `ignore`.
    let pdu = build_toy!(IGNORE ReleaseRequest,
        REJECT UE_Core_ID(42u64),
        REJECT UE_RAN_ID(7u32),
        REJECT Cause(cause()),
    )?;
    let tree = r#"{"message": "ReleaseRequest", "ies": [
        {"UE-Core-ID": 42},
        {"UE-RAN-ID": 7},
        {"Cause": {"radioNetwork": "user-inactivity"}, "criticality": "reject"}
    ]}"#;
    let criticality = Deviation::IeCriticality {
        id: 15,
        name: "Cause",
        position: 2,
        criticality: Criticality::Reject,
        assigned: Criticality::Ignore,
    };
    assert_eq!(checked(&pdu, tree), [criticality]);

    // The PDU with `reject`, where the procedure has `ignore`.
    let pdu = build_toy!(REJECT ReleaseRequest,
        REJECT UE_Core_ID(42u64),
        REJECT UE_RAN_ID(7u32),
        IGNORE Cause(cause()),
    )?;
    let tree = r#"{"message": "ReleaseRequest", "criticality": "reject", "ies": [
        {"UE-Core-ID": 42},
        {"UE-RAN-ID": 7},
        {"Cause": {"radioNetwork": "user-inactivity"}}
    ]}"#;
    let of_the_pdu = Deviation::PduCriticality {
        criticality: Criticality::Reject,
        assigned: Criticality::Ignore,
    };
    assert_eq!(checked(&pdu, tree), [of_the_pdu]);
    Ok(())
}

#[test]
fn every_deviation_of_a_message_is_reported_in_the_order_of_the_message() -> Result<(), Error> {
    // Each sentence is what `Display` says.
    let pdu = build_toy!(REJECT ReleaseRequest,
        REJECT UE_RAN_ID(7u32),
        IGNORE UE_Core_ID(42u64),
        IGNORE NodeName(NodeName("node-1".into())),
        REJECT UE_RAN_ID(8u32),
    )?;
    let tree = r#"{"message": "ReleaseRequest", "criticality": "reject", "ies": [
        {"UE-RAN-ID": 7},
        {"UE-Core-ID": 42, "criticality": "ignore"},
        {"NodeName": "node-1", "criticality": "ignore"},
        {"UE-RAN-ID": 8}
    ]}"#;
    let said: Vec<_> = checked(&pdu, tree)
        .iter()
        .map(|deviation| deviation.to_string())
        .collect();
    assert_eq!(
        said,
        [
            "the criticality of the PDU is reject, and that of its procedure is ignore",
            "UE-Core-ID (IE 10), at position 1, comes after UE-RAN-ID (IE 11), which the \
             object set of the message has after it",
            "the criticality of UE-Core-ID (IE 10), at position 1, is ignore, and the message \
             assigns it reject",
            "NodeName (IE 20), at position 2, is not an IE of the message",
            "UE-RAN-ID (IE 11), at position 3, is in the message already, at position 0",
            "Cause (IE 15) is mandatory, and the message does not have it",
        ]
    );

    // An IE that the protocol does not have is told by its identifier.
    let mut unknown = pdu.clone();
    let mut ies = unknown.protocol_ies()?.into_owned();
    ies.0.truncate(1);
    ies.0.push(ProtocolIEField {
        id: ProtocolIEID(60000),
        criticality: Criticality::Ignore,
        value: Octets::from(vec![0xc0, 0xff]),
    });
    unknown.set_protocol_ies(ies)?;
    let said = unknown.check()?[1].to_string();
    assert_eq!(
        said,
        "IE 60000, at position 1, is not an IE of the message, nor of the protocol"
    );
    Ok(())
}

#[test]
fn an_encoding_that_is_not_the_canonical_one_is_read_and_reported() -> Result<(), Error> {
    // RELEASE REQUEST, as its sender is to write it.
    let canonical = build_toy!(IGNORE ReleaseRequest,
        REJECT UE_Core_ID(42u64),
        REJECT UE_RAN_ID(7u32),
        IGNORE Cause(cause()),
    )?;
    assert_eq!(canonical.check()?, []);
    let octets = canonical.encode()?;
    let whole = decode_with_faults::<TOYPDU>(&octets)?;
    assert_eq!(whole, (canonical.clone(), Faults::default()));

    // The same message from an encoder that is not canonical. The value of its first
    // IE, a number of 0..1099511627775, has the number of its octets in three bits,
    // padding, then its octets: here two octets of which the first is zero (X.691
    // §11.3.6), after padding of which a bit is one (§11.1.4).
    let mut ies = canonical.protocol_ies()?.into_owned();
    ies.0[0].value = Octets::owned(vec![0x21, 0x00, 0x2a]);
    let mut other = canonical.clone();
    other.set_protocol_ies(ies)?;
    // And the length of that value in two octets (§11.9.3.6): the message starts after
    // the four octets of the PDU, and the length is its seventh octet.
    let mut octets = other.encode()?;
    octets.insert(10, 0x80);
    octets[3] += 1;
    let other = TOYPDU::decode(&octets)?;

    // It is read as the same message.
    let ies = other.protocol_ies()?;
    let read: UECoreID = oxirush_toy::decode(&ies.0[0].value)?;
    assert_eq!(read.0, 42);
    assert_eq!(ies.0.len(), 3);
    // And each of the three is reported, with its clause and the bit at which it is:
    // that of the message first, then those of the value.
    let ie = Some((10, "UE-Core-ID", 0));
    let deviations = other.check()?;
    let not_canonical = |ie, fault, bit| Deviation::NotCanonical { ie, fault, bit };
    assert_eq!(
        deviations,
        [
            not_canonical(None, Fault::LengthInTwoOctets, 48),
            not_canonical(ie, Fault::Padding, 3),
            not_canonical(ie, Fault::LongNumber, 8),
        ]
    );
    assert_eq!(
        deviations[0].to_string(),
        "the message is not written as its sender is to write it: at bit 48, a length of 127 \
         or less is in two octets (X.691 §11.9.3.6)"
    );
    assert_eq!(
        deviations[2].to_string(),
        "the value of UE-Core-ID (IE 10), at position 0, is not written as its sender is to \
         write it: at bit 8, a whole number is in more octets than it needs (X.691 §11.3.6, \
         11.4.6)"
    );
    // A receiver answers nothing for them.
    for deviation in &deviations {
        assert_eq!((deviation.ie(), deviation.cause()), (None, None));
        assert!(deviation.diagnostics_item().is_none() && deviation.type_of_error().is_none());
    }

    // The octets before the message are the PDU, which was read when it was decoded: a
    // decoding of the PDU has their faults, and a strict one refuses them. Here the
    // length of the message is in two octets.
    let mut octets = canonical.encode()?;
    octets.insert(3, 0x80);
    assert_eq!(TOYPDU::decode(&octets)?, canonical);
    let (pdu, faults) = decode_with_faults::<TOYPDU>(&octets)?;
    assert_eq!(pdu, canonical);
    assert_eq!(
        (faults.len(), faults[0].fault, faults[0].bit),
        (1, Fault::LengthInTwoOctets, 24)
    );
    let error = decode_strict::<TOYPDU>(&octets).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Decode);
    assert!(
        error
            .to_string()
            .contains("at bit 24, a length of 127 or less is in two octets")
    );
    assert_eq!(decode_strict::<TOYPDU>(&canonical.encode()?)?, canonical);
    Ok(())
}

#[test]
fn what_has_no_object_set_is_not_checked_against_one() -> Result<(), Error> {
    // The private message has private IEs: the criticality of its PDU is all that the
    // specification assigns it.
    let message = PrivateMessage {
        private_ies: PrivateMessagePrivateIEs(vec![AnonymousPrivateMessagePrivateIEs {
            id: PrivateIEID::local(1),
            criticality: Criticality::Ignore,
            value: Octets::from(vec![1, 2]),
        }]),
    };
    let mut held = InitiatingMessage {
        procedure_code: ProcedureCode(4),
        criticality: Criticality::Ignore,
        value: Octets::owned(oxirush_toy::encode(&message)?),
    };
    assert_eq!(TOYPDU::initiatingMessage(held.clone()).check()?, []);
    held.criticality = Criticality::Reject;
    let of_the_pdu = Deviation::PduCriticality {
        criticality: Criticality::Reject,
        assigned: Criticality::Ignore,
    };
    let private = TOYPDU::initiatingMessage(held.clone());
    assert_eq!(private.check()?, [of_the_pdu]);

    // A procedure that the specification does not have in that direction has no object
    // set, and is the one deviation, which a receiver answers by the criticality of the
    // PDU (TS 38.413 clause 10.3.4.1): it rejects the procedure, or ignores it and
    // notifies the sender, with an Error Indication that has the procedure code, the
    // triggering message and the procedure criticality, or ignores it.
    held.procedure_code = ProcedureCode(200);
    for (criticality, cause) in [
        (
            Criticality::Reject,
            Some(CauseProtocol::abstract_syntax_error_reject),
        ),
        (
            Criticality::Notify,
            Some(CauseProtocol::abstract_syntax_error_ignore_and_notify),
        ),
        (Criticality::Ignore, None),
    ] {
        held.criticality = criticality;
        let pdu = TOYPDU::initiatingMessage(held.clone());
        let deviations = pdu.check()?;
        let unknown = Deviation::UnknownProcedure {
            procedure_code: 200,
            direction: Direction::Initiating,
            criticality,
        };
        assert_eq!(deviations, [unknown]);
        assert_eq!(deviations[0].cause(), cause);
        assert_eq!(deviations[0].ie(), None);
        assert_eq!(deviations[0].diagnostics_item(), None);
        assert_eq!(
            pdu.criticality_diagnostics(&deviations),
            CriticalityDiagnostics {
                procedure_code: Some(ProcedureCode(200)),
                triggering_message: Some(TriggeringMessage::initiating_message),
                procedure_criticality: Some(criticality),
                ies_criticality_diagnostics: None,
                ie_extensions: None
            }
        );
    }
    assert_eq!(
        TOYPDU::initiatingMessage(held).check()?[0].to_string(),
        "the InitiatingMessage of procedure 200 is no message of the specification, and \
         the criticality of the PDU is ignore"
    );

    // A message that does not decode is an error, and no deviation.
    let request = build_toy!(IGNORE ReleaseRequest, REJECT UE_Core_ID(42u64),)?;
    let TOYPDU::initiatingMessage(mut held) = request else {
        panic!("an initiating message");
    };
    held.value = Octets::from(vec![0x00, 0x00, 0x03, 0x00]);
    let error = TOYPDU::initiatingMessage(held).check().unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Decode);
    Ok(())
}

/// A SETUP FAILURE is a message of the procedure in its direction, as a SETUP RESPONSE
/// is, and is checked against its own object set.
#[test]
fn an_outcome_is_checked_against_the_object_set_of_its_direction() -> Result<(), Error> {
    let failure = build_toy!(SetupFailure, Cause(cause()),)?;
    assert_eq!(failure.check()?, []);
    let failure = build_toy!(SetupFailure, IGNORE NodeName(NodeName("node-1".into())),)?;
    let said: Vec<_> = failure.check()?.iter().map(ToString::to_string).collect();
    assert_eq!(
        said,
        [
            "NodeName (IE 20), at position 0, is not an IE of the message",
            "Cause (IE 15) is mandatory, and the message does not have it",
        ]
    );
    Ok(())
}

/// What `check` reports of the IEs of a message, against an object set of four IEs: the
/// first and the third are mandatory, and the identifiers are not in their own order.
mod against_a_set {
    use super::*;

    fn set() -> Vec<(u16, Criticality, Presence)> {
        vec![
            (3, Criticality::Reject, Presence::Mandatory),
            (1, Criticality::Ignore, Presence::Optional),
            (2, Criticality::Reject, Presence::Mandatory),
            (4, Criticality::Notify, Presence::Conditional),
        ]
    }

    /// What a message of these IEs deviates in, when the value of each decodes.
    fn found(ies: &[(u16, Criticality)]) -> Vec<Deviation> {
        let mut found = Vec::new();
        deviations::<Toy>(&set(), ies, &|_| (None, Faults::default()), &mut found);
        found
    }

    const REJECT: Criticality = Criticality::Reject;
    const IGNORE: Criticality = Criticality::Ignore;
    const NOTIFY: Criticality = Criticality::Notify;

    #[test]
    fn a_message_as_its_object_set_has_it_deviates_in_nothing() {
        assert_eq!(found(&[(3, REJECT), (2, REJECT)]), []);
        assert_eq!(
            found(&[(3, REJECT), (1, IGNORE), (2, REJECT), (4, NOTIFY)]),
            []
        );
    }

    #[test]
    fn each_deviation_is_told_with_its_ie_and_its_position() {
        // A mandatory IE that is missing, and an optional one is not.
        let missing = found(&[(3, REJECT)]);
        assert!(matches!(missing[..], [Deviation::MissingIe { id: 2, .. }]));
        // An IE that the set does not have: one of the protocol or not.
        let other = found(&[(3, REJECT), (65000, IGNORE), (2, REJECT)]);
        let unexpected = Deviation::UnexpectedIe {
            id: 65000,
            name: None,
            position: 1,
            criticality: IGNORE,
        };
        assert_eq!(other, [unexpected]);
        // An IE that is there twice, whatever is between the two.
        let twice = found(&[(3, REJECT), (2, REJECT), (3, REJECT)]);
        assert!(matches!(
            twice[..],
            [Deviation::RepeatedIe {
                id: 3,
                position: 2,
                first: 0,
                ..
            }]
        ));
        // Two IEs in the other order: the second is after one that the set has after it.
        let swapped = found(&[(2, REJECT), (3, REJECT)]);
        assert!(matches!(
            swapped[..],
            [Deviation::OutOfOrder {
                id: 3,
                position: 1,
                after: 2,
                ..
            }]
        ));
        // A criticality that is not the one of the set.
        let criticality = found(&[(3, REJECT), (2, IGNORE)]);
        assert!(matches!(
            criticality[..],
            [Deviation::IeCriticality {
                id: 2,
                position: 1,
                criticality: IGNORE,
                assigned: REJECT,
                ..
            }]
        ));
    }

    #[test]
    fn the_order_is_that_of_the_set_and_leaves_out_what_the_set_does_not_have() {
        // The last IE of the set first: each of the others comes after it.
        let late = found(&[(4, NOTIFY), (3, REJECT), (1, IGNORE), (2, REJECT)]);
        let after: Vec<_> = late
            .iter()
            .map(|deviation| match deviation {
                Deviation::OutOfOrder {
                    id,
                    position,
                    after,
                    ..
                } => (*id, *position, *after),
                other => panic!("{other}"),
            })
            .collect();
        assert_eq!(after, [(3, 1, 4), (1, 2, 4), (2, 3, 4)]);
        // An IE that the set does not have is between two that are in order.
        let between = found(&[(3, REJECT), (65000, REJECT), (2, REJECT)]);
        assert!(matches!(between[..], [Deviation::UnexpectedIe { .. }]));
        // A repeated IE is told once, as repeated, and the missing ones come last.
        let all = found(&[(1, REJECT), (1, IGNORE), (3, REJECT)]);
        assert!(matches!(
            all[..],
            [
                Deviation::IeCriticality {
                    id: 1,
                    position: 0,
                    ..
                },
                Deviation::RepeatedIe {
                    id: 1,
                    position: 1,
                    first: 0,
                    ..
                },
                Deviation::OutOfOrder {
                    id: 3,
                    position: 2,
                    after: 1,
                    ..
                },
                Deviation::MissingIe { id: 2, .. },
            ]
        ));
    }

    #[test]
    fn a_value_that_does_not_decode_is_told_at_its_position() {
        let mut found = Vec::new();
        let error = Error::new(ErrorKind::UndecodableIe, "spoiled");
        let wrong = std::cell::RefCell::new(Some(error));
        let value = |position: usize| match position {
            1 => (
                wrong.borrow_mut().take().map(Wrong::Undecodable),
                Faults::default(),
            ),
            _ => (None, Faults::default()),
        };
        deviations::<Toy>(&set(), &[(3, REJECT), (2, REJECT)], &value, &mut found);
        assert!(matches!(
            found[..],
            [Deviation::UndecodableIe {
                id: 2,
                position: 1,
                ..
            }]
        ));
    }
}
