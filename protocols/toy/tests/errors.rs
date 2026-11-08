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

//! The errors of a generated crate, each where a caller meets it: its kind, which is
//! what a caller matches on, and what its message says.

use oxirush_toy::toy::*;
use oxirush_toy::{AsnType, Bits, Direction, Error, ErrorKind, Ie, Message, Octets};
use oxirush_toy::{build_toy, extract_toy_ies, ie};

/// The RELEASE REQUEST of `__pdu!`: UE-Core-ID 42, UE-RAN-ID 7 and a Cause.
const RELEASE_REQUEST: &str = "00024014000003000a0002002a000b00020007000f400104";

fn release_request() -> TOYPDU<'static> {
    TOYPDU::decode(&hex::decode(RELEASE_REQUEST).unwrap())
        .unwrap()
        .into_owned()
}

/// What an error has for the one who handles it.
fn source(error: &Error) -> Option<String> {
    std::error::Error::source(error).map(|source| source.to_string())
}

#[test]
fn a_pdu_is_decoded_as_the_message_that_it_holds_and_as_no_other() {
    let pdu = release_request();
    let request: ReleaseRequest = pdu.decode_value().unwrap();
    assert_eq!(request.protocol_ies.0.len(), 3);

    // The octets of the message are those of a SETUP REQUEST too, which is a list of
    // IEs as well: nothing but the procedure of the PDU says which message it is.
    let TOYPDU::initiatingMessage(held) = &pdu else {
        panic!("an initiating message");
    };
    let mistaken: SetupRequest = oxirush_toy::decode(&held.value).unwrap();
    assert_eq!(mistaken.protocol_ies.0.len(), 3);

    let error = pdu.decode_value::<SetupRequest>().unwrap_err();
    assert_eq!(error.kind(), ErrorKind::WrongMessage);
    assert_eq!(
        error.to_string(),
        "the message of this PDU is ReleaseRequest, not SetupRequest: kind() says which \
         message a PDU holds"
    );
    assert_eq!(source(&error), None);
}

#[test]
fn a_message_of_the_same_procedure_in_another_direction_is_another_message() {
    // A SETUP FAILURE: the unsuccessful outcome of the procedure 1.
    let cause = Cause::misc(CauseMisc::unspecified);
    let failure = build_toy!(SetupFailure, Cause(cause),).unwrap();
    assert!(failure.decode_value::<SetupFailure>().is_ok());
    for kind in [
        failure.decode_value::<SetupRequest>().unwrap_err().kind(),
        failure.decode_value::<SetupResponse>().unwrap_err().kind(),
    ] {
        assert_eq!(kind, ErrorKind::WrongMessage);
    }
}

#[test]
fn a_pdu_of_a_procedure_that_the_specification_does_not_have_is_no_message() {
    let TOYPDU::initiatingMessage(mut held) = release_request() else {
        panic!("an initiating message");
    };
    held.procedure_code = ProcedureCode(200);
    let pdu = TOYPDU::initiatingMessage(held);
    let error = pdu.decode_value::<ReleaseRequest>().unwrap_err();
    assert_eq!(error.kind(), ErrorKind::WrongMessage);
    assert_eq!(
        error.to_string(),
        "the message of this PDU is not ReleaseRequest: the specification has no \
         InitiatingMessage of procedure 200"
    );
}

#[test]
fn the_type_of_a_message_says_which_message_it_is() {
    assert_eq!(ReleaseRequest::NAME, "ReleaseRequest");
    assert_eq!(ReleaseRequest::DIRECTION, Direction::Initiating);
    assert_eq!(ReleaseRequest::PROCEDURE_CODE, 2);
    // The name is the one that ASN.1 gives the message, which is not that of its
    // procedure.
    assert_eq!(SetupFailure::NAME, "SetupFailure");
    assert_eq!(SetupFailure::DIRECTION, Direction::Unsuccessful);
    assert_eq!(SetupFailure::PROCEDURE_CODE, 1);
    let pdu = release_request();
    let of = |direction, code| (pdu.direction(), pdu.procedure_code()) == (direction, code);
    assert!(of(
        ReleaseRequest::DIRECTION,
        ReleaseRequest::PROCEDURE_CODE
    ));
}

#[test]
fn octets_that_are_no_pdu_are_a_decode_error() {
    let octets = hex::decode(RELEASE_REQUEST).unwrap();

    // No octet, octets that end early, and an octet after the PDU.
    let none = TOYPDU::decode(&[]).unwrap_err();
    assert_eq!(none.kind(), ErrorKind::Decode);
    assert_eq!(
        none.to_string(),
        "these octets do not decode as TOY-PDU: there are no octets"
    );
    for cut in 1..octets.len() {
        let short = TOYPDU::decode(&octets[..cut]).unwrap_err();
        assert_eq!(short.kind(), ErrorKind::Decode, "{cut}");
        let text = short.to_string();
        assert!(
            text.starts_with("these octets do not decode as TOY-PDU: "),
            "{text}"
        );
        // The codec is that of the family: the error is whole, and has no source.
        assert_eq!(source(&short), None);
    }
    let long = TOYPDU::decode(&[&octets[..], &[0]].concat()).unwrap_err();
    assert_eq!(long.kind(), ErrorKind::Decode);
    assert_eq!(
        long.to_string(),
        "these octets do not decode as TOY-PDU: the value is followed by 1 octet"
    );
}

#[test]
fn an_open_type_that_is_not_the_value_asked_for_is_a_decode_error() {
    // A UE-Core-ID is an INTEGER (0..1099511627775): the length of the number in three
    // bits, then its octets.
    let id: UECoreID = oxirush_toy::decode(&Octets::from(vec![0x00, 0x2a])).unwrap();
    assert_eq!(id.0, 42);
    let short = oxirush_toy::decode::<UECoreID>(&Octets::from(vec![0x40, 0x2a])).unwrap_err();
    assert_eq!(short.kind(), ErrorKind::Decode);
    assert_eq!(
        short.to_string(),
        "these octets do not decode as UE-Core-ID: the octets end before the value does"
    );
    let long = oxirush_toy::decode::<UECoreID>(&Octets::from(vec![0x00, 0x2a, 0x00, 0x00]));
    assert_eq!(
        long.unwrap_err().to_string(),
        "these octets do not decode as UE-Core-ID: the value is followed by 2 octets"
    );
}

#[test]
fn a_value_outside_its_constraint_is_an_encode_error() {
    let error = oxirush_toy::encode(&UECoreID(u64::MAX)).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Encode);
    assert_eq!(
        error.to_string(),
        "a value of UE-Core-ID does not encode: 18446744073709551615 is outside its \
         constraint (0..1099511627775)"
    );
    assert_eq!(source(&error), None);
}

#[test]
fn a_member_that_does_not_encode_is_named_as_one_that_does_not_decode_is() {
    // The next hop chaining count is an INTEGER (0..7).
    let context = SecurityContext {
        next_hop_chaining_count: 8,
        next_hop_nh: SecurityKey::from([0; 32]),
        ie_extensions: None,
    };
    let error = oxirush_toy::encode(&context).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Encode);
    assert_eq!(
        error.to_string(),
        "a value of SecurityContext does not encode: 8 is outside its constraint (0..7), in \
         its field `next_hop_chaining_count`"
    );
    assert_eq!(
        error.fields().collect::<Vec<_>>(),
        ["next_hop_chaining_count"]
    );

    // In a value that holds it, the fields are the members of the SEQUENCEs on the way to
    // it, as they are for a value that does not decode: the alternative of a CHOICE is
    // not one of them.
    let pair = UEIDPair {
        ue_core_id: UECoreID(1 << 40),
        ue_ran_id: UERANID(7),
        ie_extensions: None,
    };
    let error = oxirush_toy::encode(&UEIDs::uE_ID_pair(pair)).unwrap_err();
    assert_eq!(
        error.to_string(),
        "a value of UE-IDs does not encode: 1099511627776 is outside its constraint \
         (0..1099511627775), in its field `ue_core_id`"
    );
    let error = oxirush_toy::decode::<UEIDs>(&[0x00]).unwrap_err();
    assert_eq!(
        error.to_string(),
        "these octets do not decode as UE-IDs: the octets end before the value does, in its \
         field `ue_core_id`"
    );

    // A list of no item where the type has one at least: an area is broadcast by one
    // network to six.
    let area = AreaItem {
        tac: TAC([0, 0, 1]),
        broadcast_plmns: BroadcastPLMNs(vec![]),
        ie_extensions: None,
    };
    let error = oxirush_toy::encode(&area).unwrap_err();
    assert_eq!(
        error.to_string(),
        "a value of AreaItem does not encode: a size of 0 is outside its constraint (1..6), in \
         its field `broadcast_plmns`"
    );
}

#[test]
fn an_error_is_the_error_of_a_function_of_the_caller() {
    // With `?`, in a function whose error is this one or holds any error.
    fn code(octets: &[u8]) -> Result<u8, Error> {
        Ok(TOYPDU::decode(octets)?.procedure_code())
    }
    fn boxed(octets: &[u8]) -> Result<u8, Box<dyn std::error::Error + Send + Sync>> {
        Ok(TOYPDU::decode(octets)?.procedure_code())
    }
    assert_eq!(code(&hex::decode(RELEASE_REQUEST).unwrap()), Ok(2));
    assert_eq!(code(&[]).unwrap_err().kind(), ErrorKind::Decode);
    let error = boxed(&[]).unwrap_err();
    let error = error
        .downcast_ref::<Error>()
        .expect("the error of the crate");
    assert_eq!(error.kind(), ErrorKind::Decode);
}

/// A number of a width over the 64 bits of a number, or over the width that it is
/// given, is refused, and does not panic.
#[test]
fn an_argument_that_a_constructor_does_not_take_is_an_error_and_no_panic() {
    for error in [
        Bits::new(1, 65).unwrap_err(),
        Bits::new(0x100, 8).unwrap_err(),
    ] {
        assert_eq!(error.kind(), ErrorKind::Argument, "{error}");
        assert_eq!(source(&error), None);
    }
    assert!(Bits::new(0xff, 8).is_ok());
}

/// What the macro reads in a request: an identifier that it requires, and one that
/// the request may be without.
fn ids(request: &ReleaseRequest) -> Result<(u64, Option<u32>), Error> {
    extract_toy_ies!(request, ReleaseRequest,
        req core_id: u64 = UE_Core_ID(id),
        opt ran_id: u32 = UE_RAN_ID(id),
    );
    Ok((core_id, ran_id))
}

fn request(ies: Vec<ProtocolIEField>) -> ReleaseRequest {
    ReleaseRequest {
        protocol_ies: ProtocolIEContainer(ies),
    }
}

/// The entry of a UE-Core-ID and that of a UE-RAN-ID.
fn core_and_ran() -> (ProtocolIEField<'static>, ProtocolIEField<'static>) {
    let core = ie::UE_Core_ID::field(Criticality::Reject, 42u64);
    let ran = ie::UE_RAN_ID::field(Criticality::Reject, 7u32);
    (core.unwrap(), ran.unwrap())
}

/// An entry with the identifier of an IE, whose octets are not the value of that IE.
fn undecodable(id: u16) -> ProtocolIEField<'static> {
    ProtocolIEField {
        id: ProtocolIEID(id),
        criticality: Criticality::Reject,
        value: Octets::from(vec![0xff]),
    }
}

#[test]
fn a_required_ie_that_is_absent_is_missing_and_an_optional_one_is_none() {
    let (core, ran) = core_and_ran();
    assert_eq!(
        ids(&request(vec![core.clone(), ran.clone()])),
        Ok((42, Some(7)))
    );
    assert_eq!(ids(&request(vec![core])), Ok((42, None)));

    let missing = ids(&request(vec![ran])).unwrap_err();
    assert_eq!(missing.kind(), ErrorKind::MissingIe);
    assert_eq!(
        missing.to_string(),
        "this ReleaseRequest has no UE-Core-ID, which the field `core_id` requires"
    );
    assert_eq!(source(&missing), None);
}

#[test]
fn an_ie_that_does_not_decode_is_an_error_for_a_required_and_for_an_optional_field() {
    let (core, ran) = core_and_ran();

    // The one that is required.
    let required = ids(&request(vec![undecodable(10), ran])).unwrap_err();
    assert_eq!(required.kind(), ErrorKind::UndecodableIe);
    assert_eq!(
        required.to_string(),
        "the UE-Core-ID of this ReleaseRequest does not decode: the octets end before the \
         value does"
    );
    assert_eq!(source(&required), None);

    // The one that may be absent is there: it is not `None`.
    let optional = ids(&request(vec![core, undecodable(11)])).unwrap_err();
    assert_eq!(optional.kind(), ErrorKind::UndecodableIe);
    let text = optional.to_string();
    assert!(
        text.starts_with("the UE-RAN-ID of this ReleaseRequest does not decode: "),
        "{text}"
    );
}

/// The RELEASE REQUEST with the one octet of its Cause spoiled: `e4` for `04`, which
/// is the alternative 7 of a CHOICE of five. The IE has the criticality "ignore", so a
/// receiver that does not comprehend it goes on as if it had not been received (TS
/// 38.413 clause 10.3.4.2).
const SPOILED_CAUSE: &str = "00024014000003000a0002002a000b00020007000f4001e4";

fn spoiled_request() -> ReleaseRequest<'static> {
    let pdu = TOYPDU::decode(&hex::decode(SPOILED_CAUSE).unwrap())
        .unwrap()
        .into_owned();
    let request: ReleaseRequest = pdu.decode_value().unwrap();
    request.into_owned()
}

/// What a receiver reads in a request when it takes the cause if it can.
fn id_and_cause<'a>(request: &'a ReleaseRequest<'a>) -> Result<(u64, Option<Cause>), Error> {
    extract_toy_ies!(request, ReleaseRequest,
        req id: u64 = UE_Core_ID(id),
        lax cause: Cause = Cause(cause) => cause,
    );
    Ok((id, cause))
}

/// A call with `lax` fields alone has no error: its function returns no `Result`.
fn cause_if_read<'a>(request: &'a ReleaseRequest<'a>) -> Option<Cause> {
    extract_toy_ies!(request, ReleaseRequest,
        lax cause: Cause = Cause(cause) => cause,
    );
    cause
}

fn optional_cause<'a>(request: &'a ReleaseRequest<'a>) -> Result<Option<Cause>, Error> {
    extract_toy_ies!(request, ReleaseRequest,
        opt cause: Cause = Cause(cause) => cause,
    );
    Ok(cause)
}

fn required_cause<'a>(request: &'a ReleaseRequest<'a>) -> Result<Cause, Error> {
    extract_toy_ies!(request, ReleaseRequest,
        req cause: Cause = Cause(cause) => cause,
    );
    Ok(cause)
}

#[test]
fn a_lax_field_is_the_ie_if_it_decodes_and_none_otherwise() {
    let inactivity = Cause::radioNetwork(CauseRadioNetwork::user_inactivity);

    // The IE is there and decodes.
    let pdu = release_request();
    let received: ReleaseRequest = pdu.decode_value().unwrap();
    assert_eq!(id_and_cause(&received), Ok((42, Some(inactivity.clone()))));
    assert_eq!(cause_if_read(&received), Some(inactivity));

    // It is absent.
    let (core, _) = core_and_ran();
    let without = request(vec![core]);
    assert_eq!(id_and_cause(&without), Ok((42, None)));
    assert_eq!(cause_if_read(&without), None);

    // It is there and does not decode: the message that was received, with one octet
    // of the value of the IE spoiled.
    let spoiled = spoiled_request();
    assert_eq!(spoiled.protocol_ies.0[2].criticality, Criticality::Ignore);
    assert_eq!(id_and_cause(&spoiled), Ok((42, None)));
    assert_eq!(cause_if_read(&spoiled), None);

    // The same octets are an error for an `opt` and for a `req` field.
    let optional = optional_cause(&spoiled).unwrap_err();
    let required = required_cause(&spoiled).unwrap_err();
    assert_eq!(optional, required);
    assert_eq!(optional.kind(), ErrorKind::UndecodableIe);
    assert_eq!(
        optional.to_string(),
        "the Cause of this ReleaseRequest does not decode: the type has no index 7"
    );
}

#[test]
fn the_first_of_an_ie_that_is_there_twice_decides_each_field() {
    let inactivity = Cause::radioNetwork(CauseRadioNetwork::user_inactivity);
    let pdu = release_request();
    let received: ReleaseRequest = pdu.decode_value().unwrap();
    let read = received.protocol_ies.0[2].clone();
    let unread = spoiled_request().protocol_ies.0[2].clone();

    // The first one decodes: each field has it, and the second one is not decoded.
    let first_read = request(vec![read.clone(), unread.clone()]);
    assert_eq!(cause_if_read(&first_read), Some(inactivity.clone()));
    assert_eq!(
        optional_cause(&first_read).unwrap(),
        Some(inactivity.clone())
    );
    assert_eq!(required_cause(&first_read).unwrap(), inactivity);

    // The first one does not decode: a `lax` field is `None`, and it is the error of an
    // `opt` and of a `req` field, whatever comes after it.
    let first_unread = request(vec![unread, read]);
    assert_eq!(cause_if_read(&first_unread), None);
    let optional = optional_cause(&first_unread).unwrap_err();
    assert_eq!(optional.kind(), ErrorKind::UndecodableIe);
    let required = required_cause(&first_unread).unwrap_err();
    assert_eq!(required.kind(), ErrorKind::UndecodableIe);

    // `ies` gives both.
    let both: Vec<_> = first_read.ies::<ie::Cause>().collect();
    assert_eq!(both.len(), 2);
    assert_eq!(both[0], Ok(inactivity));
    assert_eq!(
        both[1].as_ref().unwrap_err().kind(),
        ErrorKind::UndecodableIe
    );
}

#[test]
fn a_value_outside_its_constraint_is_an_error_of_the_macros_that_build() {
    // A UE-Core-ID is an INTEGER (0..1099511627775), and the macro takes a `u64`.
    let pdu = build_toy!(IGNORE ReleaseRequest,
        REJECT UE_Core_ID(u64::MAX),
        REJECT UE_RAN_ID(7u32),
    );
    let error = pdu.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Encode);
    assert_eq!(
        error.to_string(),
        "a value of UE-Core-ID does not encode: 18446744073709551615 is outside its \
         constraint (0..1099511627775)"
    );
    let ie = ie::UE_Core_ID::field(Criticality::Reject, u64::MAX);
    assert_eq!(ie.unwrap_err(), error);

    // The last value of the constraint is built.
    let last = build_toy!(IGNORE ReleaseRequest,
        REJECT UE_Core_ID((1u64 << 40) - 1),
    );
    assert!(last.is_ok());
}

#[test]
fn changing_an_ie_says_whether_it_did_or_why_it_did_not() {
    let (core, _) = core_and_ran();
    let mut message = request(vec![core]);

    // The IE is changed, and another one is not there.
    assert_eq!(message.set_ie::<ie::UE_Core_ID>(43u64), Ok(true));
    assert_eq!(message.set_ie::<ie::UE_RAN_ID>(8u32), Ok(false));
    assert_eq!(ids(&message), Ok((43, None)));

    // A value that does not encode: the message is as it was.
    let before = message.clone();
    let outside = message.set_ie::<ie::UE_Core_ID>(u64::MAX);
    assert_eq!(outside.unwrap_err().kind(), ErrorKind::Encode);
    assert_eq!(message, before);

    // An IE that does not decode is replaced: its value is not read.
    let mut message = request(vec![undecodable(10)]);
    assert_eq!(message.set_ie::<ie::UE_Core_ID>(1u64), Ok(true));
    assert_eq!(ids(&message), Ok((1, None)));
}

/// The errors of the `inspect` feature: a test for each kind, as a caller matches on it.
#[cfg(feature = "inspect")]
mod inspect_errors {
    use super::release_request;
    use oxirush_toy::inspect;
    use oxirush_toy::toy::*;
    use oxirush_toy::{Error, ErrorKind, Octets};
    use oxirush_toy::{tree, tree::Value};

    /// A misspelled name is refused with the nearest names of the protocol.
    #[test]
    fn a_misspelled_name_is_refused_with_the_nearest_names() {
        let ie = "\"UE-Core-IDD\" is not an IE of TOY: the nearest name is UE-Core-ID";
        let tree = tree!({"message": "ReleaseRequest", "ies": [{"UE-Core-IDD": 1}]});
        assert_eq!(TOYPDU::from_tree(&tree).unwrap_err().to_string(), ie);
        let tree = release_request().to_tree();
        let error = inspect::select(&tree, "/toy/UE-Core-IDD").unwrap_err();
        assert_eq!(error.to_string(), format!("{ie}, at /toy/UE-Core-IDD"));
        let tree = tree!({"message": "ReleaseRequst", "ies": []});
        assert_eq!(
            TOYPDU::from_tree(&tree).unwrap_err().to_string(),
            "\"ReleaseRequst\" is not a message of TOY: the nearest name is ReleaseRequest"
        );
        // A name that is near none says none.
        let tree = tree!({"message": "ReleaseRequest", "ies": [{"TimeToTravel": 1}]});
        let error = TOYPDU::from_tree(&tree).unwrap_err().to_string();
        assert_eq!(error, "\"TimeToTravel\" is not an IE of TOY");
    }

    /// A member of a tree is read however it is written, once, as in a value: written
    /// twice, in two spellings, it is refused.
    #[test]
    fn a_member_of_a_tree_written_twice_in_two_spellings_is_refused() {
        let tree = tree!({
            "message": "ReleaseRequest",
            "ies": [{"UE-Core-ID": 1}],
            "IES": [{"UE-RAN-ID": 2}],
        });
        let error = TOYPDU::from_tree(&tree).unwrap_err();
        assert_eq!(
            error.to_string(),
            "the member \"ies\" of a tree is written twice, as \"ies\" and as \"IES\""
        );
        let tree = tree!({"Message": "ReleaseRequest", "message": "x", "ies": []});
        assert!(TOYPDU::from_tree(&tree).is_err());
    }

    /// The tree of the RELEASE REQUEST.
    fn tree() -> Value {
        release_request().to_tree()
    }

    /// The kind of the error that a call returned.
    fn kind<T: std::fmt::Debug>(result: Result<T, Error>) -> ErrorKind {
        result.unwrap_err().kind()
    }

    /// The PDU of a tree: a release request with these IEs.
    fn written(ies: Value) -> Result<TOYPDU<'static>, Error> {
        TOYPDU::from_tree(&tree!({"message": "ReleaseRequest", "ies": ies}))
    }

    /// A release request whose procedure code is one that the specification does not
    /// have.
    fn of_no_procedure() -> TOYPDU<'static> {
        let TOYPDU::initiatingMessage(mut held) = release_request() else {
            panic!("an initiating message");
        };
        held.procedure_code = ProcedureCode(200);
        TOYPDU::initiatingMessage(held)
    }

    #[test]
    fn a_message_that_the_protocol_does_not_have_is_unknown() {
        // A name that is no message, for a path and in a tree.
        let checked = inspect::check_path("NoSuchMessage", "/toy/Cause");
        assert_eq!(kind(checked), ErrorKind::UnknownMessage);
        let names = inspect::enumerated_at("NoSuchMessage", "/toy/Cause");
        assert_eq!(kind(names), ErrorKind::UnknownMessage);
        let tree = tree!({"message": "NoSuchMessage", "ies": []});
        let error = TOYPDU::from_tree(&tree).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::UnknownMessage);
        assert_eq!(
            error.to_string(),
            "\"NoSuchMessage\" is not a message of TOY"
        );

        // A procedure that the specification does not have has a tree by its procedure
        // code and its direction, of the IEs of its message.
        let tree = of_no_procedure().to_tree();
        assert_eq!(
            (&tree["procedure_code"], &tree["direction"]),
            (&tree!(200), &tree!("InitiatingMessage"))
        );
        assert_eq!(
            tree["ies"][2]["Cause"],
            tree!({"radioNetwork": "user-inactivity"})
        );
        // Its code is no name of a message, and a message of the protocol is written by
        // its name alone.
        let by_code = tree!({"procedure_code": 2, "direction": "InitiatingMessage", "ies": []});
        let error = TOYPDU::from_tree(&by_code).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Value);
        assert_eq!(
            error.to_string(),
            "the InitiatingMessage of procedure 2 is ReleaseRequest: it is written as \
             {\"message\": \"ReleaseRequest\"}"
        );
    }

    #[test]
    fn an_ie_that_the_protocol_does_not_have_is_unknown() {
        // A name that is no IE: after the root of a path, and in a tree.
        let tree = tree();
        let error = inspect::select(&tree, "/toy/NoSuchIe").unwrap_err();
        assert_eq!(error.kind(), ErrorKind::UnknownIe);
        assert_eq!(
            error.to_string(),
            "\"NoSuchIe\" is not an IE of TOY, at /toy/NoSuchIe"
        );
        assert_eq!(
            kind(written(tree!([{"NoSuchIe": 1}]))),
            ErrorKind::UnknownIe
        );

        // An identifier that has no type is written by its octets: its value has no
        // type to be encoded as.
        let error = written(tree!([{"60000": 1, "criticality": "ignore"}])).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::UnknownIe);
        assert_eq!(
            error.to_string(),
            "TOY IE 60000 is unknown: write its value as its octets, {\"octets\": \"…\"}"
        );
        let octets = tree!([{"60000": {"octets": "c0ffee"}, "criticality": "ignore"}]);
        assert!(written(octets).is_ok());
    }

    #[test]
    fn a_path_that_is_not_well_formed_is_a_path_error() {
        let tree = tree();
        // No `/` before the first segment, `~` before what is no 0 and no 1, and
        // `@id=` before what is no identifier of an IE.
        for path in ["toy/Cause", "/toy/Cause/~2", "/toy/@id=x", "/toy/@id=70000"] {
            assert_eq!(
                kind(inspect::select(&tree, path)),
                ErrorKind::Path,
                "{path}"
            );
            let checked = inspect::check_path("ReleaseRequest", path);
            assert_eq!(kind(checked), ErrorKind::Path, "{path}");
        }
        let error = inspect::select(&tree, "toy/Cause").unwrap_err();
        assert_eq!(
            error.to_string(),
            "\"toy/Cause\" is no path: a path starts with \"/\", at toy/Cause"
        );
        // The empty path is the tree itself, as that of a JSON pointer is the document.
        assert_eq!(inspect::select(&tree, ""), Ok(vec![&tree]));
    }

    #[test]
    fn a_name_that_the_type_at_its_place_cannot_have_is_no_member() {
        let mut tree = tree();
        // A member that a CHOICE does not have, a member of a value that has none, the
        // identifier of an IE where there is no list, and a name where a list goes by
        // position.
        let error = inspect::select(&tree, "/toy/Cause/radioNetwrok").unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Path);
        let text = error.to_string();
        assert!(
            text.starts_with("\"radioNetwrok\" is not a member of Cause, which has radioNetwork, "),
            "{text}"
        );
        for path in [
            "/toy/Cause/radioNetwork/deeper",
            "/toy/Cause/@id=1",
            "/toy/UE-Core-ID/value",
        ] {
            assert_eq!(
                kind(inspect::select(&tree, path)),
                ErrorKind::Path,
                "{path}"
            );
        }
        // An edit at such a path, and the tree is as it was.
        let before = tree.clone();
        let edit = inspect::set(&mut tree, "/toy/Cause/radioNetwrok", tree!(1));
        assert_eq!(kind(edit), ErrorKind::Path);
        assert_eq!(tree, before);
        // An IE that the object set of the message does not have, without a tree.
        let checked = inspect::check_path("ReleaseRequest", "/toy/Areas");
        assert_eq!(kind(checked), ErrorKind::Path);
        // A member that the type of a written value does not have.
        let error = TOYPDU::from_tree(&tree!({"message": "SetupRequest", "ies": [
            {"Areas": [{"tAC": 1, "broadcastPLMNs": ["208-93"], "other": 2}]},
        ]}))
        .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Path);
        assert_eq!(
            error.to_string(),
            "this is not a value of Areas: \"other\" is not a member that its type has: it has \
             tAC, broadcastPLMNs, iE-Extensions, at 0"
        );
    }

    #[test]
    fn an_edit_whose_path_selects_nothing_is_an_error() {
        let mut tree = tree();
        let before = tree.clone();
        // An IE that the message could have and does not have, and a position after
        // its last IE: a selection has nothing there, and an edit says so.
        for path in ["/toy/NAS-PDU", "/toy/7"] {
            assert_eq!(inspect::select(&tree, path), Ok(vec![]), "{path}");
            let edit = inspect::set(&mut tree, path, tree!(1));
            assert_eq!(kind(edit), ErrorKind::Path, "{path}");
        }
        let removed = inspect::remove(&mut tree, "/toy/NAS-PDU");
        assert_eq!(kind(removed), ErrorKind::Path);
        assert_eq!(tree, before);
    }

    #[test]
    fn octets_that_carry_a_type_are_not_opened_until_they_are() {
        let setup = tree!({"message": "SetupRequest", "ies": [
            {"Container": {"ContainerBlob": {"tAC": 1, "nodeName": "node-1"}}},
        ]});
        let pdu = TOYPDU::from_tree(&setup).unwrap();
        let mut tree = pdu.to_tree();
        let name = "/toy/Container/ContainerBlob/nodeName";

        let error = inspect::select(&tree, name).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Path);
        let edit = inspect::set(&mut tree, name, tree!("node-2"));
        assert_eq!(kind(edit), ErrorKind::Path);

        // The kind says what to do about it.
        inspect::open(&mut tree, name).unwrap();
        assert_eq!(inspect::select(&tree, name), Ok(vec![&tree!("node-1")]));
    }

    #[test]
    fn a_path_into_what_did_not_decode_says_so() {
        // An IE whose identifier has no type keeps its octets in a tree.
        let unknown = tree!([{"65000": {"octets": "c0ffee"}, "criticality": "ignore"}]);
        let tree = written(unknown).unwrap().to_tree();
        assert_eq!(
            inspect::select(&tree, "/toy/@id=65000/octets"),
            Ok(vec![&tree!("c0ffee")])
        );
        let error = inspect::select(&tree, "/toy/@id=65000/member").unwrap_err();
        assert_eq!(error.kind(), ErrorKind::UndecodableIe);
        assert_eq!(
            error.to_string(),
            "\"member\" is in no value here: the value is its octets, as it did not decode or \
             is written as it is sent, at /toy/@id=65000/member"
        );
    }

    #[test]
    fn a_value_that_is_not_of_its_type_is_a_value_error() {
        // A string where the type has a number, a name that the ENUMERATED does not
        // have, a number outside the constraint of its type, and octets that are no
        // hexadecimal.
        let string = written(tree!([{"UE-Core-ID": "forty-two"}])).unwrap_err();
        let name = written(tree!([{"Cause": {"protocol": "no-such-cause"}}])).unwrap_err();
        let outside = written(tree!([{"UE-Core-ID": 1u64 << 40}])).unwrap_err();
        let octets = written(tree!([{"UE-Core-ID": {"octets": "zz"}}])).unwrap_err();
        for error in [&string, &name, &outside, &octets] {
            assert_eq!(error.kind(), ErrorKind::Value, "{error}");
            assert!(!error.to_string().contains("Codec"), "{error}");
        }
        assert_eq!(
            string.to_string(),
            "this is not a value of UE-Core-ID: a number from 0 to 1099511627775 is expected, \
             and \"forty-two\" is written"
        );
        // The names that it has, and where it is written after them.
        assert_eq!(
            name.to_string(),
            "this is not a value of Cause: \"no-such-cause\" is none of its names: \
             transfer-syntax-error, abstract-syntax-error-reject, \
             abstract-syntax-error-ignore-and-notify, message-not-compatible-with-receiver-state, \
             semantic-error, abstract-syntax-error-falsely-constructed-message, unspecified, at \
             protocol"
        );
        assert_eq!(
            outside.to_string(),
            "this is not a value of UE-Core-ID: a number from 0 to 1099511627775 is expected, \
             and 1099511627776 is written"
        );
        assert_eq!(
            octets.to_string(),
            "this is not a value of ReleaseRequest: octets in hexadecimal, two digits for \
             each, are expected, and \"zz\" is written, at protocolIEs/0/value"
        );

        // A value of a member, by the names that ASN.1 gives the members on the way:
        // a PLMN identity that is none, and a number where the type has a CHOICE.
        let setup = |plmn: Value| {
            TOYPDU::from_tree(&tree!({"message": "SetupRequest", "ies": [{"Areas": [
                {"tAC": 1, "broadcastPLMNs": [plmn]},
            ]}]}))
        };
        assert!(setup(tree!("208-93")).is_ok());
        let plmn = setup(tree!("20-93")).unwrap_err();
        assert_eq!(plmn.kind(), ErrorKind::Value);
        // The type of the value and the place of the fault are said, as for every value.
        assert_eq!(
            plmn.to_string(),
            "this is not a value of Areas: 20-93: a PLMN identity is MCC-MNC, as 208-93, at \
             0/broadcastPLMNs/0"
        );
        // Without a hyphen it is its three octets: what is neither says both forms.
        for (neither, why) in [
            ("20893", "a PLMN identity is MCC-MNC: write \"208-93\""),
            (
                "2089",
                "a PLMN identity is MCC-MNC, as 208-93, or its three octets in hexadecimal",
            ),
            (
                "208 93",
                "a PLMN identity is MCC-MNC, as 208-93, or its three octets in hexadecimal",
            ),
            (
                "20893g",
                "a PLMN identity is MCC-MNC, as 208-93, or its three octets in hexadecimal",
            ),
        ] {
            let plmn = setup(tree!(neither)).unwrap_err();
            assert_eq!(plmn.kind(), ErrorKind::Value);
            assert_eq!(
                plmn.to_string(),
                format!("this is not a value of Areas: {neither}: {why}, at 0/broadcastPLMNs/0")
            );
        }
        assert!(setup(tree!("02f839")).is_ok());
        let member = written(tree!([{"Cause": 5}])).unwrap_err();
        assert_eq!(member.kind(), ErrorKind::Value);
        assert_eq!(
            member.to_string(),
            "this is not a value of Cause: an object with one member, the name of an \
             alternative with its value, is expected, and 5 is written"
        );
        assert!(std::error::Error::source(&member).is_none());
    }

    #[test]
    fn json_that_is_no_tree_is_a_form_error() {
        // A tree is an object with the name of a message and its IEs, each one member:
        // an IE that the message does not have says its criticality.
        let message = "ReleaseRequest";
        for tree in [
            tree!([]),
            tree!({"ies": []}),
            tree!({"message": message}),
            tree!({"message": message, "ies": [], "other": 1}),
            tree!({"message": message, "ies": [{"a": 1, "b": 2}]}),
            tree!({"message": message, "ies": [{"NodeName": "node-1"}]}),
            tree!({"message": message, "ies": [{"id": 15, "criticality": "ignore", "value": 1}]}),
        ] {
            let error = TOYPDU::from_tree(&tree).unwrap_err();
            assert_eq!(error.kind(), ErrorKind::Value, "{tree}: {error}");
        }
    }

    #[test]
    fn an_edit_that_is_refused_is_a_value_error() {
        let mut tree = tree();
        // A member that its type always has and the alternative of a CHOICE are not taken
        // out, and nothing is inserted where there is no list.
        let removed = inspect::remove(&mut tree, "/toy/Cause/radioNetwork");
        assert_eq!(kind(removed), ErrorKind::Value);
        let inserted = inspect::insert(&mut tree, "/toy/Cause/radioNetwork", tree!("unspecified"));
        assert_eq!(kind(inserted), ErrorKind::Value);
    }

    #[test]
    fn what_is_over_the_limits_of_the_inspection_is_refused() {
        let tree = tree();
        // A path of more than 64 segments, and one of more than 4096 octets.
        let long = "/0".repeat(65);
        assert_eq!(kind(inspect::select(&tree, &long)), ErrorKind::Path);
        let wide = format!("/{}", "a".repeat(4096));
        assert_eq!(kind(inspect::select(&tree, &wide)), ErrorKind::Path);

        // A tree that is nested deeper than a tree is read.
        let mut deep = tree!(1);
        for _ in 0..300 {
            deep = tree!([deep]);
        }
        assert_eq!(
            kind(written(tree!([{"UE-Core-ID": deep}]))),
            ErrorKind::Value
        );
    }

    #[test]
    fn a_message_that_does_not_decode_is_a_decode_error() {
        // A PDU whose octets are not the message of its procedure.
        let TOYPDU::initiatingMessage(mut held) = release_request() else {
            panic!("an initiating message");
        };
        held.value = Octets::from(vec![0xff]);
        let pdu = TOYPDU::initiatingMessage(held);
        let error = pdu.decode_value::<ReleaseRequest<'_>>().unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Decode);
        // Its tree has the octets of its message, which it sends back, and no IEs to
        // select.
        let tree = pdu.to_tree();
        assert_eq!(tree, tree!({"message": "ReleaseRequest", "octets": "ff"}));
        assert_eq!(inspect::select(&tree, "/toy/Cause"), Ok(vec![]));
    }
}
