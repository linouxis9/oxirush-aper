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

//! What the receiver of a message that deviates answers it with: the cause and the
//! Criticality Diagnostics of clause 10 of TS 38.413 and of TS 36.413, from what
//! `check` reports of the message, and from the error of the macro that extracts its
//! IEs.
//!
//! Clause 10.2 has an IE whose value does not decode as a transfer syntax error, which
//! an error indication tells. Clause 10.3.4 has the receiver act on an IE that it does
//! not comprehend by the criticality that the IE was received with, and clause 10.3.5 on
//! a missing IE by the criticality that the specification assigns it. The Criticality
//! Diagnostics name each IE of the criticality `reject` or `notify` with that
//! criticality, its identifier and the type of the error.

use oxirush_toy::aper::check::criticality_diagnostics_for_response;
use oxirush_toy::aper::{self, BitString, ErrorIe};
use oxirush_toy::toy::*;
use oxirush_toy::{Deviation, Error, ErrorKind, Octets, Toy, build_toy, extract_toy_ies};

/// The identifiers of the IEs that the tests spoil or leave out: the Areas of a SETUP
/// REQUEST, mandatory with the criticality `reject`, and the UE-RAN-ID and the Cause
/// of a RELEASE REQUEST, mandatory with `reject` and with `ignore`.
const AREAS: u16 = 26;
const RAN: u16 = 11;
const CAUSE: u16 = 15;

fn areas() -> Areas {
    Areas(vec![AreaItem {
        tac: TAC([0, 0, 1]),
        broadcast_plmns: BroadcastPLMNs(vec![PLMNIdentity([0x02, 0xf8, 0x39])]),
        ie_extensions: None,
    }])
}

/// A SETUP REQUEST as a node sends it.
fn request() -> Result<TOYPDU<'static>, Error> {
    build_toy!(REJECT SetupRequest,
        REJECT Areas(areas()),
        IGNORE NodeName(NodeName("node-1".into())),
    )
}

/// The same request without its Areas.
fn without_areas() -> Result<TOYPDU<'static>, Error> {
    build_toy!(REJECT SetupRequest, IGNORE NodeName(NodeName("node-1".into())),)
}

/// A RELEASE REQUEST whose last IE is its Cause, without its UE-RAN-ID if `ran` is
/// none.
fn release(ran: Option<u32>) -> Result<Vec<u8>, Error> {
    let mut pdu = build_toy!(IGNORE ReleaseRequest,
        REJECT UE_Core_ID(1u64),
        REJECT UE_RAN_ID(ran.unwrap_or(7)),
        IGNORE Cause(Cause::nas(CauseNas::normal_release)),
    )?;
    if ran.is_none() {
        let mut ies = pdu.protocol_ies()?.into_owned();
        ies.0.remove(1);
        pdu.set_protocol_ies(ies)?;
    }
    pdu.encode()
}

/// The release with a Cause that is no value of its type: its one octet has the index 7
/// in the three bits of the alternative, of a CHOICE of five.
fn spoiled(ran: Option<u32>) -> Result<TOYPDU<'static>, Error> {
    let mut octets = release(ran)?;
    *octets.last_mut().unwrap() = 0xff;
    Ok(TOYPDU::decode(&octets)?.into_owned())
}

/// What a core reads of the setup request: the first IE that is missing or does not
/// decode is its error.
fn accept(request: &SetupRequest) -> Result<usize, Error> {
    extract_toy_ies!(request, SetupRequest,
        req areas: Areas = Areas(areas) => areas,
        opt name: NodeName = NodeName(name) => name,
    );
    let _ = name;
    Ok(areas.0.len())
}

/// What a core reads of the release request.
fn release_of(request: &ReleaseRequest) -> Result<Cause, Error> {
    extract_toy_ies!(request, ReleaseRequest,
        req _core: u64 = UE_Core_ID(id),
        req _ran: u32 = UE_RAN_ID(id),
        req cause: Cause = Cause(cause) => cause,
    );
    Ok(cause)
}

/// The Criticality Diagnostics that answer a request, with these items. In the SETUP
/// FAILURE, the response of the procedure, they have the IEs alone; in an ERROR
/// INDICATION also the procedure, the message and its criticality (TS 38.413 clause
/// 9.3.1.3). They have no list without an item, as the list has one at least.
fn diagnostics(
    indication: bool,
    items: Vec<CriticalityDiagnosticsIEItem>,
) -> CriticalityDiagnostics {
    let items = (!items.is_empty()).then_some(CriticalityDiagnosticsIEList(items));
    match indication {
        true => CriticalityDiagnostics {
            procedure_code: Some(ProcedureCode(2)),
            triggering_message: Some(TriggeringMessage::initiating_message),
            procedure_criticality: Some(Criticality::Ignore),
            ies_criticality_diagnostics: items,
            ie_extensions: None,
        },
        false => CriticalityDiagnostics {
            procedure_code: None,
            triggering_message: None,
            procedure_criticality: None,
            ies_criticality_diagnostics: items,
            ie_extensions: None,
        },
    }
}

/// The cause of the first deviation of a request that has one, and the items of the
/// Criticality Diagnostics of its IEs.
fn reported(pdu: &TOYPDU<'_>) -> Result<(Cause, Vec<CriticalityDiagnosticsIEItem>), Error> {
    let deviations = pdu.check()?;
    let cause = deviations.iter().find_map(Deviation::cause);
    let items = deviations.iter().filter_map(Deviation::diagnostics_item);
    let cause = Cause::protocol(cause.unwrap_or(CauseProtocol::unspecified));
    Ok((cause, items.collect()))
}

/// The SETUP FAILURE that rejects a request which deviates, as clauses 10.3.4.2,
/// 10.3.5 and 10.3.6 have the procedure rejected.
fn failure(pdu: &TOYPDU<'_>) -> Result<TOYPDU<'static>, Error> {
    let (cause, items) = reported(pdu)?;
    build_toy!(REJECT SetupFailure,
        IGNORE Cause(cause),
        IGNORE CriticalityDiagnostics(diagnostics(false, items)),
    )
}

/// The ERROR INDICATION that answers a request with a transfer syntax error, as clause
/// 10.2 has it.
fn indication(pdu: &TOYPDU<'_>) -> Result<TOYPDU<'static>, Error> {
    let (cause, items) = reported(pdu)?;
    build_toy!(IGNORE ErrorIndication,
        IGNORE Cause(cause),
        IGNORE CriticalityDiagnostics(diagnostics(true, items)),
    )
}

/// The cause and the Criticality Diagnostics of an answer, as its receiver reads them.
fn answered(answer: &TOYPDU<'_>) -> Result<(Cause, CriticalityDiagnostics), Error> {
    let octets = answer.encode()?;
    let received = TOYPDU::decode(&octets)?;
    let cause = received.ie::<oxirush_toy::ie::Cause>().unwrap()?;
    let diagnostics = received.ie::<oxirush_toy::ie::CriticalityDiagnostics>();
    Ok((cause, diagnostics.unwrap()?))
}

#[test]
fn a_request_as_the_specification_has_it_deviates_in_nothing() -> Result<(), Error> {
    let pdu = request()?;
    assert_eq!(pdu.check()?, []);
    assert_eq!(accept(&pdu.decode_value()?)?, 1);
    let octets = release(Some(7))?;
    let pdu = TOYPDU::decode(&octets)?;
    assert_eq!(pdu.check()?, []);
    let cause = release_of(&pdu.decode_value()?)?;
    assert_eq!(cause, Cause::nas(CauseNas::normal_release));
    Ok(())
}

#[test]
fn the_error_of_a_missing_ie_has_the_criticality_that_is_assigned() -> Result<(), Error> {
    let pdu = without_areas()?;
    let error = accept(&pdu.decode_value()?).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::MissingIe);
    let ie = ErrorIe::new(AREAS, aper::Criticality::Reject, None);
    assert_eq!(error.ie(), Some(ie));

    // The deviation of the error is the one that `check` reports.
    let missing = Deviation::MissingIe {
        id: AREAS,
        name: "Areas",
        criticality: Criticality::Reject,
    };
    assert_eq!(Deviation::of(&error), Some(missing.clone()));
    assert_eq!(pdu.check()?, std::slice::from_ref(&missing));

    // Clause 10.3.5: rejected, and reported as missing with the assigned criticality.
    assert_eq!(
        missing.cause(),
        Some(CauseProtocol::abstract_syntax_error_reject)
    );
    assert_eq!(missing.type_of_error(), Some(TypeOfError::missing));
    let item = CriticalityDiagnosticsIEItem {
        ie_criticality: Criticality::Reject,
        ie_id: ProtocolIEID(AREAS),
        type_of_error: TypeOfError::missing,
        ie_extensions: None,
    };
    assert_eq!(missing.diagnostics_item(), Some(item.clone()));
    let cause = Cause::protocol(CauseProtocol::abstract_syntax_error_reject);
    assert_eq!(
        answered(&failure(&pdu)?)?,
        (cause, diagnostics(false, vec![item]))
    );
    Ok(())
}

#[test]
fn the_error_of_an_ie_that_does_not_decode_has_the_criticality_that_was_received()
-> Result<(), Error> {
    let pdu = spoiled(Some(7))?;
    let error = release_of(&pdu.decode_value()?).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::UndecodableIe);
    assert_eq!(
        error.to_string(),
        "the Cause of this ReleaseRequest does not decode: the type has no index 7"
    );
    // The third IE of the message, which was sent with the criticality `ignore`.
    let ie = ErrorIe::new(CAUSE, aper::Criticality::Ignore, Some(2));
    assert_eq!(error.ie(), Some(ie));

    // The deviation of the error is the one that `check` reports.
    let undecodable = Deviation::of(&error).unwrap();
    assert!(matches!(
        undecodable,
        Deviation::UndecodableIe {
            id: CAUSE,
            name: "Cause",
            position: 2,
            criticality: Criticality::Ignore,
            ..
        }
    ));
    assert_eq!(pdu.check()?, std::slice::from_ref(&undecodable));
    assert_eq!(
        undecodable.to_string(),
        "Cause (IE 15), at position 2, does not decode: the type has no index 7"
    );

    // Clause 10.2: a transfer syntax error, which an error indication tells. The IE was
    // received as `ignore`, which no item of the diagnostics has.
    assert_eq!(
        undecodable.cause(),
        Some(CauseProtocol::transfer_syntax_error)
    );
    assert_eq!(
        undecodable.type_of_error(),
        Some(TypeOfError::not_understood)
    );
    assert_eq!(undecodable.diagnostics_item(), None);
    let cause = Cause::protocol(CauseProtocol::transfer_syntax_error);
    assert_eq!(
        answered(&indication(&pdu)?)?,
        (cause, diagnostics(true, vec![]))
    );
    Ok(())
}

/// TS 38.413 clause 9.3.1.3: the criticality of an item of the Criticality Diagnostics
/// is never `ignore` ("The value 'ignore' is not applicable"), as clauses 10.3.4.2 and
/// 10.3.5 have a receiver report the IEs of the criticalities `reject` and `notify`.
#[test]
fn an_ie_of_the_criticality_ignore_is_in_no_item_of_the_diagnostics() -> Result<(), Error> {
    // Received as `ignore`, and not decoded.
    let deviations = spoiled(Some(7))?.check()?;
    assert!(matches!(
        deviations[..],
        [Deviation::UndecodableIe {
            criticality: Criticality::Ignore,
            ..
        }]
    ));
    assert_eq!(deviations[0].diagnostics_item(), None);
    // Missing, where the object set assigns `ignore`.
    let missing = Deviation::MissingIe {
        id: CAUSE,
        name: "Cause",
        criticality: Criticality::Ignore,
    };
    assert_eq!(missing.diagnostics_item(), None);
    // The same IE missing as `reject` has its item.
    let reject = Deviation::MissingIe {
        id: CAUSE,
        name: "Cause",
        criticality: Criticality::Reject,
    };
    let item = CriticalityDiagnosticsIEItem {
        ie_criticality: Criticality::Reject,
        ie_id: ProtocolIEID(CAUSE),
        type_of_error: TypeOfError::missing,
        ie_extensions: None,
    };
    assert_eq!(reject.diagnostics_item(), Some(item));
    Ok(())
}

#[test]
fn check_reports_every_ie_that_is_missing_or_does_not_decode() -> Result<(), Error> {
    // Without its UE-RAN-ID, and with a Cause that does not decode: the macro stops at
    // the first, and `check` has both.
    let pdu = spoiled(None)?;
    // The first of its fields that has no value: the UE-RAN-ID, which is missing.
    let first = release_of(&pdu.decode_value()?).unwrap_err();
    assert_eq!(first.kind(), ErrorKind::MissingIe);

    let deviations = pdu.check()?;
    let ies: Vec<_> = deviations.iter().map(Deviation::ie).collect();
    assert_eq!(
        ies,
        [
            Some((CAUSE, Criticality::Ignore)),
            Some((RAN, Criticality::Reject)),
        ]
    );
    let types: Vec<_> = deviations.iter().map(Deviation::type_of_error).collect();
    assert_eq!(
        types,
        [
            Some(TypeOfError::not_understood),
            Some(TypeOfError::missing)
        ]
    );
    // The items: the missing IE, which is `reject`, and not the one that does not
    // decode, which was received as `ignore`.
    let items: Vec<_> = (deviations.iter())
        .filter_map(Deviation::diagnostics_item)
        .collect();
    let missing = CriticalityDiagnosticsIEItem {
        ie_criticality: Criticality::Reject,
        ie_id: ProtocolIEID(RAN),
        type_of_error: TypeOfError::missing,
        ie_extensions: None,
    };
    assert_eq!(items, [missing]);
    Ok(())
}

/// Clause 10.3.4.2: an IE that the receiver does not comprehend is acted on by the
/// criticality that it was received with, and clause 10.3.6 rejects a message with an
/// IE that is there twice or in a wrong order.
#[test]
fn each_deviation_has_the_cause_of_its_clause() -> Result<(), Error> {
    // A RELEASE REQUEST with its identifiers in the other order, an IE of no message,
    // received as `notify`, and another as `ignore`.
    let ie = |id: u16, criticality| ProtocolIEField {
        id: ProtocolIEID(id),
        criticality,
        value: vec![0].into(),
    };
    let mut pdu = build_toy!(IGNORE ReleaseRequest,
        REJECT UE_RAN_ID(7u32),
        REJECT UE_Core_ID(1u64),
        IGNORE Cause(Cause::nas(CauseNas::normal_release)),
    )?;
    let mut ies = pdu.protocol_ies()?.into_owned();
    ies.0.push(ie(60_000, Criticality::Notify));
    ies.0.push(ie(60_001, Criticality::Ignore));
    pdu.set_protocol_ies(ies)?;

    let deviations = pdu.check()?;
    let causes: Vec<_> = deviations.iter().map(Deviation::cause).collect();
    assert_eq!(
        causes,
        [
            Some(CauseProtocol::abstract_syntax_error_falsely_constructed_message),
            Some(CauseProtocol::abstract_syntax_error_ignore_and_notify),
            None,
        ],
        "{deviations:?}"
    );
    let items: Vec<_> = (deviations.iter())
        .filter_map(Deviation::diagnostics_item)
        .collect();
    let item = |id: u16, criticality| CriticalityDiagnosticsIEItem {
        ie_criticality: criticality,
        ie_id: ProtocolIEID(id),
        type_of_error: TypeOfError::not_understood,
        ie_extensions: None,
    };
    // The IE received as `ignore` has none.
    assert_eq!(items, [item(60_000, Criticality::Notify)]);
    // The items of the Criticality Diagnostics of a response, for the deviations that
    // have one, and none without them.
    let response = criticality_diagnostics_for_response::<Toy>(Criticality::Reject, &deviations);
    let none = criticality_diagnostics_for_response::<Toy>(Criticality::Reject, &[]);
    assert_eq!(
        response.ies_criticality_diagnostics,
        Some(CriticalityDiagnosticsIEList(vec![item(
            60_000,
            Criticality::Notify
        )]))
    );
    assert_eq!(none.ies_criticality_diagnostics, None);
    Ok(())
}

/// A node name is a PrintableString (SIZE (1..150, ...)), and the algorithms a BIT
/// STRING (SIZE (16, ...)). A name of no character and algorithms of eight bits encode,
/// as values of a later version of their types, which a sender of this version does not
/// send. Clause 10.3.1 has a receiver handle a value that its version does not define as
/// an IE that it does not comprehend.
#[test]
fn a_value_outside_the_root_of_an_extensible_constraint_is_reported() -> Result<(), Error> {
    // It encodes, and an encoding that is strict refuses it.
    let none = NodeName("".into());
    assert_eq!(aper::encode(&none)?, [0x80, 0x00]);
    let refused = aper::encode_strict(&none).unwrap_err();
    assert_eq!(refused.kind(), ErrorKind::Encode);
    assert_eq!(
        refused.to_string(),
        "a value of NodeName does not encode: a size of 0 is outside the root of its \
         constraint (1..150, ...)"
    );

    // A SETUP REQUEST with them is built as any other, and `check` says it.
    let setup = |name: &'static str, algorithms: &[u8]| {
        let algorithms = Algorithms(BitString::from(algorithms.to_vec()));
        build_toy!(
            SetupRequest,
            Areas(areas()),
            NodeName(NodeName(name.into())),
            Algorithms(algorithms),
        )
    };
    assert_eq!(setup("node-1", &[0xe0, 0x00])?.check()?, []);
    let deviations = setup("", &[0xe0])?.check()?;
    assert!(matches!(
        deviations[..],
        [
            Deviation::OutsideRoot {
                id: 20,
                name: "NodeName",
                position: 1,
                criticality: Criticality::Ignore,
                ..
            },
            Deviation::OutsideRoot {
                id: 27,
                name: "Algorithms",
                position: 2,
                criticality: Criticality::Reject,
                ..
            }
        ]
    ));
    assert_eq!(
        deviations[0].to_string(),
        "NodeName (IE 20), at position 1, has a value of a later version of its type: a size \
         of 0 is outside the root of its constraint (1..150, ...)"
    );
    // The name is ignored, and the algorithms reject the procedure.
    assert_eq!(deviations[0].ie(), Some((20, Criticality::Ignore)));
    assert_eq!(deviations[0].cause(), None);
    assert_eq!(deviations[1].ie(), Some((27, Criticality::Reject)));
    assert_eq!(
        deviations[1].type_of_error(),
        Some(TypeOfError::not_understood)
    );
    assert_eq!(
        deviations[1].cause(),
        Some(CauseProtocol::abstract_syntax_error_reject)
    );
    Ok(())
}

/// A deviation of an IE that the message does not have says the IE, the type of the
/// error, the item of the Criticality Diagnostics and the cause, and so does the
/// deviation that an error of the macros tells of.
#[test]
fn a_deviation_says_its_ie_its_item_and_its_cause() -> Result<(), Error> {
    let unexpected = Deviation::UnexpectedIe {
        id: 9,
        name: None,
        position: 1,
        criticality: Criticality::Reject,
    };
    assert_eq!(unexpected.ie(), Some((9, Criticality::Reject)));
    assert_eq!(
        unexpected.type_of_error(),
        Some(TypeOfError::not_understood)
    );
    assert_eq!(
        unexpected.diagnostics_item(),
        Some(CriticalityDiagnosticsIEItem {
            ie_criticality: Criticality::Reject,
            ie_id: ProtocolIEID(9),
            type_of_error: TypeOfError::not_understood,
            ie_extensions: None,
        })
    );
    assert_eq!(
        unexpected.cause(),
        Some(CauseProtocol::abstract_syntax_error_reject)
    );
    // An error of no IE tells of no deviation.
    let error = TOYPDU::decode(&[]).unwrap_err();
    assert_eq!(Deviation::of(&error), None);
    // Nor does a spoiled IE of an extraction that names no message.
    let spoiled = ProtocolIEField {
        id: ProtocolIEID(RAN),
        criticality: Criticality::Reject,
        value: Octets::from(vec![0xc0]),
    };
    let error = oxirush_toy::decode::<UERANID>(&spoiled.value).unwrap_err();
    assert_eq!(Deviation::of(&error), None);
    Ok(())
}

/// Clause 10.3: the receiver of a message rejects it, and executes none of it, for an IE
/// of the criticality `reject` that it does not comprehend (clause 10.3.4.2), whose value
/// does not decode (clause 10.3.2) or that is missing (clause 10.3.5), for an IE twice or
/// in a wrong order whatever its criticality (clause 10.3.6), and for a procedure of the
/// criticality `reject` that it does not comprehend (clause 10.3.4.1). It ignores an IE of
/// the criticality `ignore`, and goes on for one of `notify`, which it reports. Its cause is
/// that of the deviation that rejects the message, even after one of `notify` (clause
/// 10.5), or of the first that it reports.
#[test]
fn the_receiver_rejects_a_message_or_goes_on_by_the_criticality_of_each_deviation()
-> Result<(), Error> {
    use oxirush_toy::aper::Direction;
    use oxirush_toy::aper::check::answer;
    let rejects = |deviations: &[Deviation]| -> Vec<bool> {
        deviations.iter().map(Deviation::rejects).collect()
    };

    // A value that does not decode, received as `ignore`: ignored.
    let ignored = spoiled(Some(7))?.check()?;
    assert_eq!(rejects(&ignored), [false]);
    assert_eq!(answer(&ignored), None);
    // The same IE received as `reject`: a transfer syntax error that rejects it.
    let Deviation::UndecodableIe {
        id,
        name,
        position,
        error,
        ..
    } = ignored[0].clone()
    else {
        panic!("{ignored:?}");
    };
    let undecodable = Deviation::UndecodableIe {
        id,
        name,
        position,
        criticality: Criticality::Reject,
        error,
    };
    assert!(undecodable.rejects());
    assert_eq!(
        answer(std::slice::from_ref(&undecodable)),
        Some((CauseProtocol::transfer_syntax_error, true))
    );

    // A missing IE of the criticality `reject`.
    let missing = without_areas()?.check()?;
    assert_eq!(rejects(&missing), [true]);
    assert_eq!(
        answer(&missing),
        Some((CauseProtocol::abstract_syntax_error_reject, true))
    );

    // An IE of no message received as `notify`, alone and before one that rejects.
    let notify = Deviation::UnexpectedIe {
        id: 60_000,
        name: None,
        position: 3,
        criticality: Criticality::Notify,
    };
    assert!(!notify.rejects());
    assert_eq!(
        answer(std::slice::from_ref(&notify)),
        Some((
            CauseProtocol::abstract_syntax_error_ignore_and_notify,
            false
        ))
    );
    let reject = Deviation::MissingIe {
        id: RAN,
        name: "UE-RAN-ID",
        criticality: Criticality::Reject,
    };
    assert_eq!(
        answer(&[notify.clone(), reject]),
        Some((CauseProtocol::abstract_syntax_error_reject, true))
    );

    // An IE in a wrong order rejects the message whatever its criticality.
    let wrong_order = Deviation::OutOfOrder {
        id: RAN,
        name: "UE-RAN-ID",
        position: 0,
        after: 10,
        after_name: "UE-Core-ID",
    };
    let repeated = Deviation::RepeatedIe {
        id: RAN,
        name: "UE-RAN-ID",
        position: 2,
        first: 1,
    };
    assert_eq!(rejects(&[wrong_order.clone(), repeated]), [true, true]);
    assert_eq!(
        answer(&[notify, wrong_order]),
        Some((
            CauseProtocol::abstract_syntax_error_falsely_constructed_message,
            true
        ))
    );

    // A procedure that the receiver does not comprehend, by the criticality of the PDU.
    let unknown = |criticality| Deviation::UnknownProcedure {
        procedure_code: 200,
        direction: Direction::Initiating,
        criticality,
    };
    assert_eq!(
        answer(&[unknown(Criticality::Reject)]),
        Some((CauseProtocol::abstract_syntax_error_reject, true))
    );
    assert_eq!(
        answer(&[unknown(Criticality::Notify)]),
        Some((
            CauseProtocol::abstract_syntax_error_ignore_and_notify,
            false
        ))
    );
    assert_eq!(answer(&[unknown(Criticality::Ignore)]), None);

    // What a receiver answers nothing for: a criticality other than the assigned one.
    let criticality = [
        Deviation::PduCriticality {
            criticality: Criticality::Notify,
            assigned: Criticality::Reject,
        },
        Deviation::IeCriticality {
            id: RAN,
            name: "UE-RAN-ID",
            position: 1,
            criticality: Criticality::Ignore,
            assigned: Criticality::Reject,
        },
    ];
    assert_eq!(rejects(&criticality), [false, false]);
    assert_eq!(answer(&criticality), None);
    assert_eq!(answer::<Toy>(&[]), None);
    Ok(())
}
