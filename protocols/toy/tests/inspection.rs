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

//! The tree of a PDU and its paths: what `to_tree` shows, what `from_tree` writes, and
//! what `select`, `set`, `remove`, `insert`, `open`, `check_path` and `enumerated_at`
//! do with it.

#![cfg(feature = "inspect")]

use oxirush_toy::inspect;
use oxirush_toy::toy::*;
use oxirush_toy::tree;
use oxirush_toy::tree::Value;
use oxirush_toy::{Criticality, Direction, Error, ErrorKind, Octets, Toy, build_toy};

/// The name that ASN.1 gives the message of a PDU.
fn name_of(pdu: &TOYPDU<'_>) -> Option<&'static str> {
    Toy::object_set(pdu.direction(), pdu.procedure_code()).map(|set| set.name)
}

/// The tree of the PDU that `tree` writes, which is written and decoded again.
fn sent(tree: &Value) -> Result<Value, Error> {
    Ok(TOYPDU::from_tree(tree)?.to_tree())
}

/// The tree of a RELEASE REQUEST of the IEs `ies`, written and decoded again.
fn released(ies: Value) -> Result<Value, Error> {
    sent(&tree!({"message": "ReleaseRequest", "ies": ies}))
}

/// The octets of each IE of the message of a PDU, in their order.
fn octets_of(pdu: &TOYPDU<'_>) -> Vec<Vec<u8>> {
    let ies = pdu.protocol_ies().unwrap();
    ies.0.iter().map(|ie| ie.value.to_vec()).collect()
}

/// The values at `path` in `tree`.
fn select(tree: &Value, path: &str) -> Vec<Value> {
    let selected = inspect::select(tree, path).unwrap();
    selected.into_iter().cloned().collect()
}

/// A RELEASE REQUEST: the two identifiers of a UE and a cause.
fn release_request() -> TOYPDU<'static> {
    build_toy!(IGNORE ReleaseRequest,
        REJECT UE_Core_ID(1u64),
        REJECT UE_RAN_ID(7u32),
        IGNORE Cause(Cause::radioNetwork(CauseRadioNetwork::user_inactivity)),
    )
    .unwrap()
}

/// The Areas of a SETUP REQUEST: one area of one network.
fn areas() -> Value {
    tree!([{"tAC": 1, "broadcastPLMNs": ["208-93"]}])
}

/// A tunnel to 10.0.0.1, of the endpoint 9.
fn tunnel() -> Value {
    tree!({"gTPTunnel": {"transportLayerAddress": "10.0.0.1", "gTP-TEID": 9}})
}

/// A SETUP REQUEST of its Areas and one transfer: a transfer of protocol IEs and one of a
/// value, which its item holds as octets.
fn setup_request() -> Value {
    tree!({"message": "SetupRequest", "ies": [
        {"Areas": areas()},
        {"TransferList": [{
            "transfer-ID": 1,
            "setupTransfer": {"protocolIEs": [
                {"Tunnel": tunnel()},
                {"TransferKind": "ipv4"},
            ]},
            "failureTransfer": {"cause": {"protocol": "semantic-error"}},
        }]},
    ]})
}

/// The path of the transfer of protocol IEs of `setup_request`.
const TRANSFER: &str = "/toy/TransferList/0/setupTransfer";

#[test]
fn repeated_ies_are_edited_one_by_one_in_their_order() {
    let pdu = build_toy!(IGNORE ReleaseRequest,
        REJECT UE_Core_ID(1u64),
        REJECT UE_RAN_ID(7u32),
        REJECT UE_Core_ID(2u64),
    )
    .unwrap();
    let mut tree = pdu.to_tree();
    // Each goes by its position, which is how `paths` names it.
    let paths = inspect::paths(&tree);
    assert_eq!(
        paths,
        [
            ("/toy/0".to_string(), tree!(1)),
            ("/toy/UE-RAN-ID".to_string(), tree!(7)),
            ("/toy/2".to_string(), tree!(2)),
        ]
    );
    // The name selects each of them, in the order of the message.
    assert_eq!(select(&tree, "/toy/UE-Core-ID"), [tree!(1), tree!(2)]);
    inspect::set(&mut tree, "/toy/2", tree!(3)).unwrap();
    let edited = TOYPDU::from_tree(&tree).unwrap();
    let tree = edited.to_tree();
    assert_eq!(select(&tree, "/toy/UE-Core-ID"), [tree!(1), tree!(3)]);
    assert_eq!(select(&tree, "/toy/1"), [tree!(7)]);
    // The other IEs keep their octets.
    let (before, after) = (octets_of(&pdu), octets_of(&edited));
    assert_eq!((&before[..2], before[2] != after[2]), (&after[..2], true));
}

#[test]
fn an_edit_changes_the_ies_that_it_changes_and_no_other() {
    // The RELEASE REQUEST of `__pdu!`
    let wire = hex::decode("00024014000003000a0002002a000b00020007000f400104").unwrap();
    let pdu = TOYPDU::decode(&wire).unwrap();
    let mut tree = pdu.to_tree();
    inspect::set(&mut tree, "/toy/Cause/radioNetwork", tree!("unspecified")).unwrap();
    inspect::remove(&mut tree, "/toy/UE-Core-ID").unwrap();
    let edited = TOYPDU::from_tree(&tree).unwrap();
    assert_eq!(
        hex::encode(edited.encode().unwrap()),
        "0002400e000002000b00020007000f400100"
    );
    assert_eq!(
        edited.to_tree(),
        tree!({"message": "ReleaseRequest", "ies": [
            {"UE-RAN-ID": 7},
            {"Cause": {"radioNetwork": "unspecified"}},
        ]})
    );
    // A value set at a path changes the octets of its IE alone.
    let mut tree = release_request().to_tree();
    inspect::set(&mut tree, "/toy/UE-RAN-ID", tree!(9)).unwrap();
    let cause = tree!("Release Due To RAN Generated Reason");
    inspect::set(&mut tree, "/toy/Cause/radioNetwork", cause).unwrap();
    let edited = TOYPDU::from_tree(&tree).unwrap();
    let (before, after) = (octets_of(&release_request()), octets_of(&edited));
    assert_eq!(after[0], before[0]);
    assert_eq!(after[1], [0, 9]);
    assert_ne!(after[2], before[2]);
    let cause = select(&edited.to_tree(), "/toy/Cause/radioNetwork");
    assert_eq!(cause, [tree!("release-due-to-ran-generated-reason")]);
}

#[test]
fn a_transfer_is_its_decoded_value_which_an_edit_goes_into() {
    let pdu = TOYPDU::from_tree(&setup_request()).unwrap();
    let mut tree = pdu.to_tree();
    let path = "/toy/TransferList/0/failureTransfer/cause/protocol";
    assert_eq!(select(&tree, path), [tree!("semantic-error")]);
    inspect::set(&mut tree, path, tree!("abstract-syntax-error-reject")).unwrap();
    let edited = TOYPDU::from_tree(&tree).unwrap();
    let checked = edited.to_tree();
    assert_eq!(
        select(&checked, path),
        [tree!("abstract-syntax-error-reject")]
    );
    assert_eq!(select(&checked, "/toy/Areas/0/tAC"), [tree!(1)]);
    let (before, after) = (octets_of(&pdu), octets_of(&edited));
    assert_eq!(before[0], after[0]);
    assert_ne!(before[1], after[1]);
}

#[test]
fn an_ie_whose_value_has_no_place_for_its_octets_is_its_octets() {
    // The Areas of one item that has an addition of a later version of its type, after
    // its extension bit: the bitmap of one addition, which is present, and its open type.
    let raw = hex::decode("080000010002f839010100").unwrap();
    let areas: Areas = oxirush_toy::decode(&raw).unwrap();
    assert_eq!(areas.0[0].tac, TAC([0, 0, 1]));
    let pdu = build_toy!(SetupRequest, Areas(areas),).unwrap();
    let mut ies = pdu.protocol_ies().unwrap().into_owned();
    ies.0[0].value = raw.clone().into();
    let mut pdu = pdu.clone();
    pdu.set_protocol_ies(ies).unwrap();
    let wire = pdu.encode().unwrap();
    let mut tree = pdu.to_tree();
    let octets = tree!({"octets": hex::encode(&raw)});
    assert_eq!(tree["ies"][0], tree!({"Areas": octets}));
    assert_eq!(TOYPDU::from_tree(&tree).unwrap().encode().unwrap(), wire);
    // A typed edit cannot drop the addition: there is no value to edit.
    let error = inspect::set(&mut tree, "/toy/Areas/0/tAC", tree!(3)).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::UndecodableIe);
    assert_eq!(
        select(&tree, "/toy/Areas/octets"),
        [tree!(hex::encode(&raw))]
    );
}

#[test]
fn an_ie_is_written_by_its_name_with_its_value_or_its_octets() {
    // UE-RAN-ID 9 as its octets, and an IE that the crate does not know.
    let tree = released(tree!([
        {"UE-Core-ID": 1},
        {"UE-RAN-ID": {"octets": "0009"}},
        {"60000": {"octets": "C0FFEE"}, "criticality": "ignore"},
    ]))
    .unwrap();
    assert_eq!(tree["ies"][1], tree!({"UE-RAN-ID": 9}));
    assert_eq!(
        tree["ies"][2],
        tree!({"60000": {"octets": "c0ffee"}, "criticality": "ignore"})
    );
    // A typed value of the type of its identifier, by its name or its identifier.
    let with_cause = release_request().to_tree();
    for cause in ["Cause", "15", "cause"] {
        let written = released(tree!([
            {"UE-Core-ID": 1},
            {"UE-RAN-ID": 7},
            {(cause): {"radioNetwork": "user-inactivity"}},
        ]));
        assert_eq!(written.unwrap(), with_cause, "{cause}");
    }
    // A value that the type does not have, and an identifier without a type.
    let cause = tree!({"radioNetwork": "no-such-cause"});
    assert!(released(tree!([{"Cause": cause}])).is_err());
    let error = released(tree!([{"60000": 5, "criticality": "ignore"}])).unwrap_err();
    assert!(error.to_string().contains("as its octets"), "{error}");
    // An ENUMERATED by its name and an OCTET STRING by its octets, not the octets of
    // their open types, in either case.
    let typed = build_toy!(IGNORE ReleaseRequest,
        REJECT UE_Core_ID(1u64),
        IGNORE TransferKind(TransferKind::ipv6),
        REJECT NAS_PDU(vec![0x7e, 0x00]),
    )
    .unwrap();
    for octets in ["7e00", "7E00"] {
        let written = TOYPDU::from_tree(&tree!({"message": "ReleaseRequest", "ies": [
            {"UE-Core-ID": 1},
            {"TransferKind": "ipv6", "criticality": "ignore"},
            {"NAS-PDU": octets},
        ]}));
        assert_eq!(written.unwrap(), typed, "{octets}");
    }
}

#[test]
fn an_ie_that_does_not_decode_is_its_octets() {
    let request = ReleaseRequest {
        protocol_ies: ProtocolIEContainer(vec![ProtocolIEField {
            id: ProtocolIEID(60000),
            criticality: Criticality::Ignore,
            value: vec![0xC0, 0xFF, 0xEE].into(),
        }]),
    };
    let pdu = TOYPDU::initiatingMessage(InitiatingMessage {
        procedure_code: ProcedureCode(2),
        criticality: Criticality::Ignore,
        value: oxirush_toy::encode(&request).unwrap().into(),
    });
    let mut tree = pdu.to_tree();
    let ie = tree!({"60000": {"octets": "c0ffee"}, "criticality": "ignore"});
    assert_eq!(tree["ies"][0], ie);
    assert_eq!(select(&tree, "/toy/@id=60000/octets"), [tree!("c0ffee")]);
    assert_eq!(TOYPDU::from_tree(&tree).unwrap(), pdu);
    // Its octets are set, and its type is unknown, so a typed value has no encoding.
    inspect::set(&mut tree, "/toy/0", tree!({"octets": "DECAFBAD"})).unwrap();
    let edited = TOYPDU::from_tree(&tree).unwrap().to_tree();
    assert_eq!(select(&edited, "/toy/0/octets"), [tree!("decafbad")]);
    inspect::set(&mut tree, "/toy/0", tree!(5)).unwrap();
    assert!(TOYPDU::from_tree(&tree).is_err());
}

/// The FailureTransfer of a SETUP FAILURE is an OCTET STRING (CONTAINING
/// FailureTransfer).
#[test]
fn an_ie_that_contains_a_type_is_its_decoded_value() {
    let failure = tree!({"message": "SetupFailure", "ies": [
        {"Cause": {"misc": "unspecified"}},
        {"FailureTransfer": {"cause": {"radioNetwork": "unspecified"}}},
        {"CriticalityDiagnostics": {"procedureCode": 1}},
    ]});
    let pdu = TOYPDU::from_tree(&failure).unwrap();
    let mut tree = pdu.to_tree();
    assert_eq!(tree, failure);
    let path = "/toy/FailureTransfer/cause/radioNetwork";
    assert_eq!(select(&tree, path), [tree!("unspecified")]);
    inspect::set(&mut tree, path, tree!("user-inactivity")).unwrap();
    let edited = TOYPDU::from_tree(&tree).unwrap();
    assert_eq!(select(&edited.to_tree(), path), [tree!("user-inactivity")]);
    // The other IEs keep their octets.
    let (before, after) = (octets_of(&pdu), octets_of(&edited));
    assert_eq!((&before[0], &before[2]), (&after[0], &after[2]));
    assert_ne!(before[1], after[1]);
    // Its octets are an OCTET STRING whose octets are the transfer.
    let transfer = FailureTransfer {
        cause: Cause::radioNetwork(CauseRadioNetwork::user_inactivity),
        ie_extensions: None,
    };
    let carried = oxirush_toy::encode(&transfer).unwrap();
    let held: Octets = oxirush_toy::decode(&after[1]).unwrap();
    assert_eq!(held.as_slice(), carried);
}

#[test]
fn a_transfer_is_written_as_its_decoded_value_or_its_octets() {
    let transfer = tree!({"protocolIEs": [{"Tunnel": tunnel()}]});
    let written = |transfer: Value| {
        sent(&tree!({"message": "SetupRequest", "ies": [
            {"Areas": areas()},
            {"TransferList": [{"transfer-ID": 5, "setupTransfer": transfer}]},
        ]}))
    };
    let sent = written(transfer.clone()).unwrap();
    assert_eq!(select(&sent, TRANSFER), std::slice::from_ref(&transfer));
    // Its octets, alone, are the same transfer.
    let pdu = TOYPDU::from_tree(&sent).unwrap();
    let ies = pdu.protocol_ies().unwrap();
    let list: TransferList = oxirush_toy::decode(&ies.0[1].value).unwrap();
    let octets = hex::encode(&list.0[0].setup_transfer);
    assert_eq!(written(tree!({"octets": &octets})).unwrap(), sent);
    // The form of another JSON, a member beside the octets, and a member that the type
    // does not have are refused.
    for refused in [
        tree!({"decoded": &transfer}),
        tree!({"octets": &octets, "other": 1}),
        tree!({"no-such-member": 1}),
    ] {
        assert!(written(refused.clone()).is_err(), "{refused}");
    }
}

#[test]
fn a_message_is_written_by_its_name_and_its_ies_by_theirs() {
    // The name gives the procedure and the criticalities, in any case; an IE that the
    // message does not have says its own, and octets are sent as they are.
    let request = tree!({
        "message": "release request",
        "ies": [
            {"ue-core-id": 1},
            {"UE-RAN-ID": 2},
            {"Cause": {"radioNetwork": "user-inactivity"}},
            {"65000": {"octets": "00"}, "criticality": "reject"},
        ],
    });
    let pdu = TOYPDU::from_tree(&request).unwrap();
    assert_eq!(name_of(&pdu), Some("ReleaseRequest"));
    assert_eq!(pdu.criticality(), Criticality::Ignore);
    let ies = pdu.protocol_ies().unwrap();
    let criticalities: Vec<_> = ies.0.iter().map(|ie| ie.criticality.name()).collect();
    assert_eq!(criticalities, ["reject", "reject", "ignore", "reject"]);
    let shown = pdu.to_tree();
    assert_eq!(shown["message"], tree!("ReleaseRequest"));
    assert_eq!(shown["ies"][0], tree!({"UE-Core-ID": 1}));
    assert_eq!(
        shown["ies"][3],
        tree!({"65000": {"octets": "00"}, "criticality": "reject"})
    );
    // What is not a message, not an IE, or an IE of another message without its
    // criticality, is refused with what is wrong.
    let refused = |tree: Value| TOYPDU::from_tree(&tree).unwrap_err().to_string();
    let named = |message: &str, ie: &str| tree!({"message": message, "ies": [{ie: 1}]});
    assert!(refused(named("ReleaseRequst", "UE-Core-ID")).contains("is not a message"));
    assert!(refused(named("ReleaseRequest", "UE-ID")).contains("is not an IE"));
    assert!(refused(named("ReleaseRequest", "NodeName")).contains("says its \"criticality\""));
    assert!(refused(tree!({"message": "Notification", "ies": [{}]})).contains("one IE"));
    assert!(refused(tree!({"message": "Notification", "ie": []})).contains("is no member"));
}

/// The members of `value`, a tree or a part of one, that only repeat what its form says:
/// those of an IE or of a transfer of another form, and a `criticality` that the tables
/// could give. `holder` is the name of the member that has `value`, or the list it is in.
fn redundant(value: &Value, holder: &str, found: &mut Vec<String>) {
    match value {
        Value::Object(members) => {
            for (name, member) in members {
                let wraps = match name.as_str() {
                    "decoded" | "id" | "extensionValue" => true,
                    // A bit string whose size varies has a `value` and a `length`.
                    "value" => !members.contains_key("length"),
                    // The tables have the criticality that an object set assigns to the
                    // IEs of a message and of a transfer, and to those of a single
                    // container.
                    "criticality" => true,
                    name => name.starts_with('_'),
                };
                if wraps {
                    found.push(format!("{name} in {holder}"));
                }
                redundant(member, name, found);
            }
        }
        Value::Array(entries) => (entries.iter()).for_each(|entry| redundant(entry, holder, found)),
        _ => {}
    }
}

#[test]
fn a_message_is_shown_and_written_by_name_to_any_depth() {
    let written = |tree: &Value| TOYPDU::from_tree(tree).map(|pdu| pdu.encode().unwrap());
    // A transfer is its decoded value, and the IEs of the message that it contains go
    // by name, with the criticality that the object set of the transfer assigns them.
    let request = setup_request();
    let wire = written(&request).unwrap();
    let shown = TOYPDU::decode(&wire).unwrap().to_tree();
    assert_eq!(shown, request);
    // It has no member that only wraps a value, nor a criticality that the tables have.
    let mut found = Vec::new();
    redundant(&shown["ies"], "SetupRequest", &mut found);
    assert_eq!(found, Vec::<String>::new());
    // An IE of another form than a tree's is refused.
    let mut explicit = request.clone();
    let ies = tree!({"protocolIEs": [{"id": 31, "criticality": "reject", "value": "ipv4"}]});
    inspect::set(&mut explicit, TRANSFER, ies).unwrap();
    assert!(written(&explicit).is_err());
    // A criticality that is written in the place of the one of the object set of the
    // transfer is sent, and shown.
    let mut unassigned = request.clone();
    let kind = format!("{TRANSFER}/TransferKind/criticality");
    inspect::set(&mut unassigned, &kind, tree!("ignore")).unwrap();
    let pdu = TOYPDU::from_tree(&unassigned).unwrap();
    assert_eq!(pdu.to_tree(), unassigned);
    assert_ne!(pdu.encode().unwrap(), wire);
    assert_eq!(select(&pdu.to_tree(), &kind), [tree!("ignore")]);
    // An IE that the object set of the transfer does not have says its criticality, and
    // one of an extension container too: `to_tree` shows both.
    let mut unexpected = request.clone();
    let cause = tree!({"Cause": {"misc": "unspecified"}});
    inspect::insert(&mut unexpected, &format!("{TRANSFER}/-"), cause).unwrap();
    let error = written(&unexpected).unwrap_err().to_string();
    assert_eq!(
        error,
        "SetupTransfer has no IE Cause: one that is sent in it says its \"criticality\""
    );
    let mut extended = request.clone();
    let index = tree!({"ExtensionIndex": 2, "criticality": "ignore"});
    let extensions = "/toy/TransferList/0/iE-Extensions";
    inspect::set(&mut extended, extensions, tree!([index])).unwrap();
    let pdu = TOYPDU::from_tree(&extended).unwrap();
    let tree = pdu.to_tree();
    let value = select(&tree, &format!("{extensions}/ExtensionIndex"));
    assert_eq!(value, [tree!(2)]);
    assert_eq!(tree, extended);
    let index = tree!({"ExtensionIndex": 2});
    inspect::set(&mut extended, extensions, tree!([index])).unwrap();
    let error = written(&extended).unwrap_err().to_string();
    assert!(error.contains("Index says its \"criticality\""), "{error}");
    // An IE that a value holds alone has the criticality of its object set, and goes by
    // its name after the member that holds it.
    let failure = |cause: Value| tree!({"message": "SetupFailure", "ies": [{"Cause": {"choice-Extensions": cause}}]});
    let extended = failure(tree!({"ExtendedCause": "congestion"}));
    let pdu = TOYPDU::from_tree(&extended).unwrap();
    let tree = pdu.to_tree();
    let held = "/toy/Cause/choice-Extensions";
    assert!(select(&tree, &format!("{held}/criticality")).is_empty());
    let cause = select(&tree, &format!("{held}/ExtendedCause"));
    assert_eq!(cause, [tree!("congestion")]);
    assert_eq!(tree, extended);
    // One that its set does not have says its own, which is shown.
    let name = tree!({"NodeName": "node-1"});
    let error = written(&failure(name)).unwrap_err().to_string();
    assert!(
        error.contains("NodeName says its \"criticality\""),
        "{error}"
    );
    let name = tree!({"NodeName": "node-1", "criticality": "ignore"});
    let node = failure(name);
    let pdu = TOYPDU::from_tree(&node).unwrap();
    assert_eq!(pdu.to_tree(), node);
    assert_eq!(
        select(&node, &format!("{held}/criticality")),
        [tree!("ignore")]
    );
    // A transfer that does not decode has its octets, and so has an IE that contains a
    // type when its octets are not of that type.
    let mut undecoded = request.clone();
    inspect::set(&mut undecoded, TRANSFER, tree!({"octets": "ff"})).unwrap();
    let pdu = TOYPDU::from_tree(&undecoded).unwrap();
    assert_eq!(pdu.to_tree(), undecoded);
    let failure = tree!({"message": "SetupFailure", "ies": [{"FailureTransfer": "ff"}]});
    let pdu = TOYPDU::from_tree(&failure).unwrap();
    let shown = pdu.to_tree();
    let octets = tree!({"FailureTransfer": {"octets": "01ff"}});
    assert_eq!(shown["ies"][0], octets);
    assert_eq!(written(&shown).unwrap(), pdu.encode().unwrap());
    // What is in the place of an IE and is none is refused.
    let none = tree!({"message": "SetupFailure", "ies": [
        {"Cause": {"choice-Extensions": {"a": 1, "b": 2}}},
    ]});
    let error = written(&none).unwrap_err().to_string();
    assert!(error.contains("one IE"), "{error}");
}

#[test]
fn the_ies_of_a_transfer_go_by_name_in_its_place() {
    let name = "SetupRequest";
    let tree = sent(&setup_request()).unwrap();
    let address = format!("{TRANSFER}/Tunnel/gTPTunnel/gTP-TEID");
    // By the names of the tree, and in any case.
    for path in [
        address.clone(),
        format!("{TRANSFER}/protocolIEs/0/Tunnel/gTPTunnel/gTP-TEID"),
        format!("{TRANSFER}/protocolIEs/Tunnel/gTPTunnel/gTP-TEID"),
        address.to_lowercase(),
    ] {
        assert_eq!(select(&tree, &path), [tree!(9)], "{path}");
        assert_eq!(inspect::check_path(name, &path), Ok(()), "{path}");
    }
    // An IE of the contained message goes by its name, its position or its identifier.
    for ie in ["TransferKind", "1", "@id=31", "protocolIEs/1/TransferKind"] {
        let kind = select(&tree, &format!("{TRANSFER}/{ie}"));
        assert_eq!(kind, [tree!("ipv4")], "{ie}");
    }
    // `*` is each member of a value, and each entry of a list.
    assert_eq!(select(&tree, &format!("{TRANSFER}/*")).len(), 2);
    let list = "/toy/TransferList";
    assert_eq!(select(&tree, &format!("{list}/*/transfer-ID")), [tree!(1)]);
    // The names of an ENUMERATED are found on the same path.
    let names = inspect::enumerated_at(name, &format!("{TRANSFER}/TransferKind")).unwrap();
    assert_eq!(names, Some(&["ipv4", "ipv6", "ipv4v6"][..]));
    // Edits take the same paths: a member is set, an entry is added and one taken out.
    let mut edited = tree.clone();
    let item = select(&tree, &format!("{list}/0"))[0].clone();
    inspect::set(&mut edited, &format!("{list}/0/transfer-ID"), tree!(5)).unwrap();
    inspect::insert(&mut edited, &format!("{list}/-"), item.clone()).unwrap();
    inspect::insert(&mut edited, &format!("{list}/0"), item).unwrap();
    inspect::set(&mut edited, &format!("{list}/0/transfer-ID"), tree!(7)).unwrap();
    inspect::remove(&mut edited, &format!("{list}/2")).unwrap();
    inspect::set(
        &mut edited,
        &format!("{TRANSFER}/TransferKind"),
        tree!("ipv6"),
    )
    .unwrap();
    let after = sent(&edited).unwrap();
    let ids = select(&after, &format!("{list}/*/transfer-ID"));
    assert_eq!(ids, [tree!(7), tree!(5)]);
    let kind = select(&after, &format!("{TRANSFER}/TransferKind"));
    assert_eq!(kind, [tree!("ipv6")]);
    let failure = format!("{list}/0/failureTransfer");
    inspect::set(&mut edited, &failure, Value::Null).unwrap();
    assert!(select(&edited, &failure).is_empty());
    // A name that the value does not have is refused with its members, and the tree is
    // as it was.
    let before = edited.clone();
    for (path, value) in [
        (
            format!("{list}/0/misspelled"),
            "is not a member of TransferItem, which has transfer-ID",
        ),
        (
            format!("{TRANSFER}/misspelled"),
            "\"misspelled\" is not an IE of TOY",
        ),
        (
            format!("{list}/misspelled"),
            "\"misspelled\" is not a position in a list",
        ),
        (
            "/toy/Areas/0/tAC/valeu".to_string(),
            "\"valeu\" is not a member of this value, which has none",
        ),
    ] {
        for error in [
            inspect::select(&edited, &path)
                .map(|_| ())
                .unwrap_err()
                .to_string(),
            inspect::set(&mut edited, &path, tree!(1))
                .unwrap_err()
                .to_string(),
            inspect::check_path(name, &path).unwrap_err().to_string(),
        ] {
            assert!(error.contains(value), "{path}: {error}");
            assert!(error.ends_with(&format!(", at {path}")), "{path}: {error}");
        }
    }
    assert_eq!(edited, before);
    // A bit string whose size varies has its `value` and its `length`.
    let address = tree!({"value": "0a0000", "length": 24});
    let mut other = tree.clone();
    let place = format!("{TRANSFER}/Tunnel/gTPTunnel/transportLayerAddress");
    inspect::set(&mut other, &place, address.clone()).unwrap();
    let other = sent(&other).unwrap();
    assert_eq!(select(&other, &place), [address]);
    assert_eq!(select(&other, &format!("{place}/value")), [tree!("0a0000")]);
    assert_eq!(select(&other, &format!("{place}/length")), [tree!(24)]);
}

/// The Container of a SETUP REQUEST carries a ContainerBlob, which nothing in the
/// octets says, as a handover container of NGAP and of S1AP carries the container of
/// the node: the tree has its octets until a path that names the type opens them.
#[test]
fn octets_that_carry_a_type_are_written_and_selected_by_that_type() {
    let name = "ContainerBlob";
    let blob = tree!({"tAC": 1, "nodeName": "node-1", "message": "0102"});
    let setup = |container: Value| {
        tree!({"message": "SetupRequest", "ies": [
            {"Areas": areas()},
            {"Container": container},
        ]})
    };
    let pdu = TOYPDU::from_tree(&setup(tree!({name: &blob}))).unwrap();
    // The octets are the APER encoding of the type, which the bindings read.
    let message: SetupRequest = pdu.decode_value().unwrap();
    let octets: Container = oxirush_toy::decode(&message.protocol_ies.0[1].value).unwrap();
    let carried: ContainerBlob = oxirush_toy::decode(&octets.0).unwrap();
    assert_eq!(carried.message.as_deref(), Some(&[1u8, 2][..]));
    assert_eq!(oxirush_toy::encode(&carried).unwrap(), octets.0);
    // Nothing says what the octets carry: a tree has them as they are.
    let mut tree = pdu.to_tree();
    let ie = "/toy/Container";
    let hexadecimal = tree!(hex::encode(&octets.0));
    assert_eq!(select(&tree, ie), std::slice::from_ref(&hexadecimal));
    assert_eq!(tree, setup(hexadecimal.clone()));
    // A path names the type. The types say whether it can select, and the octets are
    // opened before it does.
    let node = format!("{ie}/{name}/nodeName");
    assert_eq!(inspect::check_path("SetupRequest", &node), Ok(()));
    let error = inspect::select(&tree, &node).unwrap_err().to_string();
    assert!(error.contains("open decodes them"), "{error}");
    let error = inspect::set(&mut tree, &node, tree!("node-2"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("open decodes them"), "{error}");
    inspect::open(&mut tree, &node).unwrap();
    assert_eq!(select(&tree, &node), [tree!("node-1")]);
    // What was opened is selected by the type, in any case of its name, and its paths
    // are listed.
    assert_eq!(
        select(&tree, &format!("{ie}/{name}")),
        std::slice::from_ref(&blob)
    );
    let loose = format!("{ie}/{}/node_name", name.to_lowercase());
    assert_eq!(select(&tree, &loose), [tree!("node-1")]);
    let paths = inspect::paths(&tree);
    assert!(paths.iter().any(|(path, _)| *path == node), "{paths:?}");
    for (path, value) in &paths {
        assert_eq!(select(&tree, path), std::slice::from_ref(value), "{path}");
    }
    assert_eq!(TOYPDU::from_tree(&tree).unwrap(), pdu);
    // An edit goes into it.
    inspect::set(&mut tree, &node, tree!("node-2")).unwrap();
    inspect::remove(&mut tree, &format!("{ie}/{name}/message")).unwrap();
    let mut after = sent(&tree).unwrap();
    assert!(select(&after, ie)[0].is_string());
    inspect::open(&mut after, &node).unwrap();
    assert_eq!(select(&after, &node), [tree!("node-2")]);
    assert!(select(&after, &format!("{ie}/{name}/message")).is_empty());
    // Octets that are not of the type are opened as their octets, which a path into
    // them says, and are sent as they came.
    let other = TOYPDU::from_tree(&setup(tree!("ff"))).unwrap();
    let mut tree = other.to_tree();
    inspect::open(&mut tree, &node).unwrap();
    assert_eq!(
        select(&tree, &format!("{ie}/{name}")),
        [tree!({"octets": "ff"})]
    );
    let error = inspect::select(&tree, &node).unwrap_err().to_string();
    assert!(error.contains("the value is its octets"), "{error}");
    assert_eq!(TOYPDU::from_tree(&tree).unwrap(), other);
    // A path that goes through no such name opens nothing.
    let before = tree.clone();
    inspect::open(&mut tree, "/toy/Areas").unwrap();
    assert_eq!(tree, before);
    // The name is one of a type that the IE may carry.
    let error = TOYPDU::from_tree(&setup(tree!({"Counts": {}})))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains(&format!("for a type that they carry: {name}")),
        "{error}"
    );
    let mut tree = other.to_tree();
    for path in [format!("{ie}/NoSuchType/tAC"), format!("{ie}/tAC")] {
        let error = inspect::check_path("SetupRequest", &path).unwrap_err();
        assert!(
            error.to_string().contains(&format!("which has {name}")),
            "{error}"
        );
        assert_eq!(inspect::select(&tree, &path), Err(error.clone()), "{path}");
        assert_eq!(inspect::open(&mut tree, &path), Err(error), "{path}");
    }
}

#[test]
fn the_name_of_another_ie_after_a_position_says_which_ie_the_entry_is() {
    let name = "SetupRequest";
    let mut tree = sent(&setup_request()).unwrap();
    let ies = format!("{TRANSFER}/protocolIEs");
    let endpoint = format!("{ies}/0/Tunnel/gTPTunnel/gTP-TEID");
    assert_eq!(select(&tree, &endpoint), [tree!(9)]);
    let other = format!("{ies}/0/TransferKind");
    let error = inspect::select(&tree, &other).unwrap_err();
    let text = error.to_string();
    assert!(
        text.starts_with("this IE is Tunnel, not TransferKind: "),
        "{error}"
    );
    assert!(text.contains("is not a member of Tunnel"), "{error}");
    assert_eq!(inspect::set(&mut tree, &other, tree!("ipv6")), Err(error));
    // Without a tree, the entry at a position is any IE, and the path is read against
    // the type that the name of the next segment gives it.
    assert_eq!(inspect::check_path(name, &endpoint), Ok(()));
    assert_eq!(inspect::check_path(name, &other), Ok(()));
    let wrong = format!("{ies}/0/Tunnel/gTPTunel");
    let error = inspect::check_path(name, &wrong).unwrap_err().to_string();
    assert!(error.contains("is not a member of Tunnel"), "{error}");
    let kinds = inspect::enumerated_at(name, &other).unwrap();
    assert_eq!(kinds, Some(&["ipv4", "ipv6", "ipv4v6"][..]));
    // An edit takes the same path, and an IE goes out whole.
    inspect::set(&mut tree, &endpoint, tree!(7)).unwrap();
    inspect::remove(&mut tree, &format!("{ies}/1")).unwrap();
    let after = sent(&tree).unwrap();
    assert_eq!(select(&after, &endpoint), [tree!(7)]);
    let transfer = &select(&after, TRANSFER)[0];
    assert_eq!(transfer["protocolIEs"].as_array().unwrap().len(), 1);
    // A member of a value that is named as an IE is that member: a ContainerBlob has a
    // `nodeName`, and NodeName is an IE.
    let blob = tree!({"ContainerBlob": {"tAC": 1, "nodeName": "node-1"}});
    let setup = tree!({"message": "SetupRequest", "ies": [{"Container": blob}]});
    let mut tree = sent(&setup);
    let tree = tree.as_mut().unwrap();
    let node = "/toy/Container/ContainerBlob/nodeName";
    inspect::open(tree, node).unwrap();
    assert_eq!(select(tree, node), [tree!("node-1")]);
    assert_eq!(inspect::check_path("SetupRequest", node), Ok(()));
}

#[test]
fn well_known_values_are_shown_and_taken_as_they_are_usually_written() {
    let wire = TOYPDU::from_tree(&setup_request())
        .unwrap()
        .encode()
        .unwrap();
    let mut tree = TOYPDU::decode(&wire).unwrap().to_tree();
    let area = "/toy/Areas/0";
    let network = format!("{area}/broadcastPLMNs/0");
    assert_eq!(select(&tree, &format!("{area}/tAC")), [tree!(1)]);
    assert_eq!(select(&tree, &network), [tree!("208-93")]);
    assert_eq!(TOYPDU::from_tree(&tree).unwrap().encode().unwrap(), wire);

    // Edited as they are shown, a number in hexadecimal too.
    let before = TOYPDU::decode(&wire).unwrap();
    inspect::set(&mut tree, &format!("{area}/tAC"), tree!("0x60B72F")).unwrap();
    inspect::set(&mut tree, &network, tree!("001-01")).unwrap();
    let edited = TOYPDU::from_tree(&tree).unwrap();
    let shown = edited.to_tree();
    assert_eq!(select(&shown, &format!("{area}/tAC")), [tree!(0x60B72F)]);
    assert_eq!(select(&shown, &network), [tree!("001-01")]);
    // The other IEs keep their octets.
    assert_eq!(octets_of(&edited)[1], octets_of(&before)[1]);

    // As JER writes them, after 0x where they are digits alone.
    inspect::set(&mut tree, &format!("{area}/tAC"), tree!("60b72f")).unwrap();
    inspect::set(&mut tree, &network, tree!("00f110")).unwrap();
    assert_eq!(
        TOYPDU::from_tree(&tree).unwrap().encode().unwrap(),
        edited.encode().unwrap()
    );
    // Digits alone are refused: read in two ways, with both, and where both are one
    // number, with that number as JSON writes it.
    inspect::set(&mut tree, &format!("{area}/tAC"), tree!("000010")).unwrap();
    let error = TOYPDU::from_tree(&tree).unwrap_err().to_string();
    assert!(
        error.contains("the number 10 ") && error.contains("the number 16"),
        "{error}"
    );
    inspect::set(&mut tree, &format!("{area}/tAC"), tree!("000007")).unwrap();
    let error = TOYPDU::from_tree(&tree).unwrap_err().to_string();
    assert!(
        error.contains("\"000007\" is a number in quotes: write 7 without quotes"),
        "{error}"
    );
    // Neither form, and a number that the type has no room for.
    for (path, refused) in [
        ("tAC", tree!("7")),
        ("tAC", tree!(1 << 24)),
        ("tAC", tree!("0x1000000")),
        ("broadcastPLMNs/0", tree!("208-9")),
        ("broadcastPLMNs/0", tree!("20-893")),
    ] {
        let mut tree = TOYPDU::decode(&wire).unwrap().to_tree();
        inspect::set(&mut tree, &format!("{area}/{path}"), refused.clone()).unwrap();
        assert!(TOYPDU::from_tree(&tree).is_err(), "{path} {refused}");
    }
}

/// A BIT STRING of a fixed size is its octets in hexadecimal, the bits after the last
/// one zero, as the identifier of a node, of 20 or 28 bits.
#[test]
fn a_bit_string_of_a_fixed_size_is_its_octets_in_hexadecimal() {
    let node = |node: Value| {
        let setup = tree!({"message": "SetupRequest", "ies": [
            {"Areas": areas()}, {"NodeID": node},
        ]});
        TOYPDU::from_tree(&setup)
    };
    let pdu = node(tree!({"macro-NodeID": "abcde0"})).unwrap();
    let id = pdu.ie::<oxirush_toy::ie::NodeID>().unwrap().unwrap();
    let NodeID::macro_NodeID(bits) = &id else {
        panic!("{id:?}");
    };
    assert_eq!((bits.value(), bits.len()), (0xABCDE, 20));
    let shown = pdu.to_tree();
    assert_eq!(
        select(&shown, "/toy/NodeID/macro-NodeID"),
        [tree!("abcde0")]
    );
    let pdu = node(tree!({"home-NodeID": "fffffff0"})).unwrap();
    let shown = pdu.to_tree();
    assert_eq!(
        select(&shown, "/toy/NodeID/home-NodeID"),
        [tree!("fffffff0")]
    );
    // The macro identifier has twenty bits, in three octets.
    let error = node(tree!({"macro-NodeID": 0xABCDE})).unwrap_err();
    assert_eq!(
        error.to_string(),
        "this is not a value of NodeID: a bit string of 20 bits is expected, in hexadecimal, \
         and 703710 is written, at macro-NodeID"
    );
    assert!(node(tree!({"macro-NodeID": "abcd"})).is_err());
    // Beside them, an alternative of the type NULL, which the bindings read by its index.
    let pdu = node(tree!({"unassigned": null})).unwrap();
    let id = pdu.ie::<oxirush_toy::ie::NodeID>().unwrap().unwrap();
    assert_eq!(id, NodeID::unassigned(()));
    let octets = pdu.encode().unwrap();
    let received = TOYPDU::decode(&octets).unwrap();
    assert_eq!(
        select(&received.to_tree(), "/toy/NodeID/unassigned"),
        [Value::Null]
    );
}

#[test]
fn a_tunnel_endpoint_is_an_ip_address_and_a_number() {
    let written = |tunnel: Value| {
        TOYPDU::from_tree(&tree!({"message": "SetupRequest", "ies": [
            {"Areas": areas()}, {"Tunnel": tunnel},
        ]}))
    };
    let octets = |pdu: &TOYPDU<'_>| octets_of(pdu).remove(1);
    let pdu = written(tunnel()).unwrap();
    let expected = Tunnel::gTPTunnel(GTPTunnel {
        transport_layer_address: TransportLayerAddress(oxirush_toy::BitString::from([10, 0, 0, 1])),
        gtp_teid: GTPTEID::from([0, 0, 0, 9]),
        ie_extensions: None,
    });
    assert_eq!(octets(&pdu), oxirush_toy::encode(&expected).unwrap());
    assert_eq!(select(&pdu.to_tree(), "/toy/Tunnel"), [tunnel()]);
    let address = "/toy/Tunnel/gTPTunnel/transportLayerAddress";
    let with =
        |address: Value| tree!({"gTPTunnel": {"transportLayerAddress": address, "gTP-TEID": 9}});
    for (written_as, hexadecimal) in [
        ("10.0.0.1", "0a000001"),
        ("2001:db8::1", "20010db8000000000000000000000001"),
        (
            "10.0.0.1,2001:db8::1",
            "0a00000120010db8000000000000000000000001",
        ),
    ] {
        let pdu = written(with(tree!(written_as))).unwrap();
        assert_eq!(select(&pdu.to_tree(), address), [tree!(written_as)]);
        // As JER writes it.
        let jer = tree!({"value": hexadecimal, "length": hexadecimal.len() * 4});
        assert_eq!(octets(&written(with(jer)).unwrap()), octets(&pdu));
    }
    // An address of another length is neither: it stays as JER has it.
    let other = tree!({"value": "0a0000", "length": 24});
    let pdu = written(with(other.clone())).unwrap();
    assert_eq!(select(&pdu.to_tree(), address), [other]);
    for refused in ["10.0.0", "2001:db8::1,10.0.0.1", "10.0.0.1,10.0.0.2"] {
        assert!(written(with(tree!(refused))).is_err(), "{refused}");
    }
}

#[test]
fn an_enumerated_value_is_named_whatever_its_case_and_separators() {
    let causes = |cause: &str, kind: &str| {
        TOYPDU::from_tree(&tree!({"message": "ReleaseRequest", "ies": [
            {"Cause": {"radioNetwork": cause}},
            {"TransferKind": kind, "criticality": "ignore"},
        ]}))
    };
    let exact = causes("user-inactivity", "ipv4").unwrap();
    let loose = causes("User_Inactivity", "IPV4").unwrap();
    assert_eq!(loose, exact);
    assert_eq!(loose.to_tree(), exact.to_tree());
    // A name of another type, and no name at all.
    assert!(causes("ipv4", "ipv4").is_err());
    assert!(causes("user-inactivit", "ipv4").is_err());
}

/// The values of an ENUMERATED have the indexes of PER (X.691 §14): those of the root in
/// the order of their numbers, then the additions in their order, whatever the order in
/// which the ASN.1 writes them and the numbers of the additions. TransferKind is
/// `{ ipv6 (10), ipv4 (5), ..., ipv4v6 (7) }`.
#[test]
fn the_values_of_an_enumerated_have_the_indexes_of_per() {
    let names = inspect::enumerated_at("SetupRequest", &format!("{TRANSFER}/TransferKind"));
    assert_eq!(names.unwrap(), Some(&["ipv4", "ipv6", "ipv4v6"][..]));
    for (kind, octets, name) in [
        (TransferKind::ipv4, [0x00], "ipv4"),
        (TransferKind::ipv6, [0x40], "ipv6"),
        (TransferKind::ipv4v6, [0x80], "ipv4v6"),
    ] {
        assert_eq!(oxirush_toy::encode(&kind).unwrap(), octets, "{name}");
        assert_eq!(oxirush_toy::decode::<TransferKind>(&octets).unwrap(), kind);
        let written = tree!({"message": "ReleaseRequest", "ies": [
            {"TransferKind": name, "criticality": "ignore"},
        ]});
        let pdu = TOYPDU::from_tree(&written).unwrap();
        assert_eq!(octets_of(&pdu), [octets.to_vec()], "{name}");
        assert_eq!(pdu.to_tree(), written);
    }
}

#[test]
fn the_ies_of_a_message_have_paths_by_their_names() {
    let tree = release_request().to_tree();
    let printed: Vec<_> = (inspect::paths(&tree).iter())
        .map(|(path, value)| format!("{path} = {value}"))
        .collect();
    // In the order of the tree, which is that of the message.
    assert_eq!(
        printed,
        [
            "/toy/UE-Core-ID = 1",
            "/toy/UE-RAN-ID = 7",
            "/toy/Cause/radioNetwork = \"user-inactivity\"",
        ]
    );
    // A name is taken in any case, and an IE also by its identifier or its position.
    for path in [
        "/toy/UE-RAN-ID",
        "/toy/ue_ran_id",
        "/toy/@id=11",
        "/toy/1",
        "/ies/1",
        "/ies/1/UE-RAN-ID",
    ] {
        assert_eq!(select(&tree, path), [tree!(7)], "{path}");
    }
    assert_eq!(select(&tree, "/toy/*").len(), 3);
    // The criticality of an IE is beside it where the message assigns another one.
    let shown = released(tree!([{"UE-RAN-ID": 7, "criticality": "ignore"}])).unwrap();
    let listed = inspect::paths(&shown);
    let criticality = ("/toy/UE-RAN-ID/criticality".to_string(), tree!("ignore"));
    assert!(listed.contains(&criticality), "{listed:?}");
    // Each path selects its value, and is one that its message can have.
    for (path, value) in inspect::paths(&sent(&setup_request()).unwrap()) {
        assert!(!path.contains("protocolIEs"), "{path}");
        let tree = sent(&setup_request()).unwrap();
        assert_eq!(select(&tree, &path), [value], "{path}");
        assert_eq!(inspect::check_path("SetupRequest", &path), Ok(()), "{path}");
    }
}

#[test]
fn ies_are_added_and_taken_out_at_a_path() {
    let mut tree = release_request().to_tree();
    // An IE by its name, with its value, at the end.
    let name = tree!({"NodeName": "node-1", "criticality": "ignore"});
    inspect::insert(&mut tree, "/toy/-", name).unwrap();
    // Given octets as an IE, before the cause.
    let unknown = tree!({"60000": {"octets": "c0ffee"}, "criticality": "ignore"});
    inspect::insert(&mut tree, "/toy/Cause", unknown).unwrap();
    inspect::remove(&mut tree, "/toy/UE-Core-ID").unwrap();
    inspect::set(&mut tree, "/toy/UE-RAN-ID", tree!({"octets": "400102"})).unwrap();
    let pdu = TOYPDU::from_tree(&tree).unwrap();
    let ids: Vec<_> = (pdu.protocol_ies().unwrap().0.iter())
        .map(|ie| ie.id.0)
        .collect();
    assert_eq!(ids, [11, 60000, 15, 20]);
    let tree = pdu.to_tree();
    assert_eq!(select(&tree, "/toy/UE-RAN-ID"), [tree!(258)]);
    assert_eq!(select(&tree, "/toy/NodeName"), [tree!("node-1")]);
    assert_eq!(select(&tree, "/toy/@id=60000/octets"), [tree!("c0ffee")]);
    // An IE that the message does not have is nothing, not an error.
    assert!(select(&tree, "/toy/UE-Core-ID").is_empty());
}

#[test]
fn the_ies_of_a_contained_message_go_by_name_in_its_place() {
    let mut tree = sent(&setup_request()).unwrap();
    // An IE of the contained message is selected as one of the PDU is, and by the path
    // that names the list.
    let first = select(&tree, &format!("{TRANSFER}/0"));
    for path in [
        "/Tunnel",
        "/tunnel",
        "/0",
        "/protocolIEs/Tunnel",
        "/protocolIEs/0/Tunnel",
    ] {
        assert_eq!(select(&tree, &format!("{TRANSFER}{path}")), first, "{path}");
    }
    // Edits take the same paths: the criticality of one IE, then the IE itself.
    let criticality = format!("{TRANSFER}/Tunnel/criticality");
    inspect::set(&mut tree, &criticality, tree!("ignore")).unwrap();
    assert_eq!(select(&tree, &criticality), [tree!("ignore")]);
    let count = select(&tree, &format!("{TRANSFER}/*")).len();
    inspect::remove(&mut tree, &format!("{TRANSFER}/Tunnel")).unwrap();
    assert_eq!(select(&tree, &format!("{TRANSFER}/*")).len(), count - 1);
    let unknown = tree!({"60000": {"octets": "00"}, "criticality": "ignore"});
    inspect::insert(&mut tree, &format!("{TRANSFER}/-"), unknown).unwrap();
    assert_eq!(
        select(&tree, &format!("{TRANSFER}/@id=60000/octets")),
        [tree!("00")]
    );
    let error = inspect::select(&tree, &format!("{TRANSFER}/misspelled"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("is not an IE of TOY"), "{error}");
}

#[test]
fn a_path_that_names_nothing_is_refused_and_changes_nothing() {
    let mut tree = release_request().to_tree();
    let before = tree.clone();
    for (path, reason) in [
        ("/toy/NoSuchIE", "\"NoSuchIE\" is not an IE of TOY"),
        (
            "/toy/Cause/misspelled",
            "\"misspelled\" is not a member of Cause, which has radioNetwork, nas, protocol, \
             misc, choice-Extensions",
        ),
        (
            "/toy/Cause/radioNetwork/deeper",
            "\"deeper\" is not a member of this value, which has none",
        ),
        (
            "/misspelled",
            "\"misspelled\" is not a member of the tree of a PDU, which has message",
        ),
        ("/toy/+0", "\"+0\" is not an IE of TOY"),
        ("/toy/00", "\"00\" is not an IE of TOY"),
        ("toy/Cause", "a path starts with \"/\""),
        ("/toy/@id=70000", "is not the identifier of an IE"),
    ] {
        let error = inspect::select(&tree, path).unwrap_err().to_string();
        assert!(error.contains(reason), "{path}: {error}");
    }
    for (path, reason) in [
        ("/toy/NoSuchIE", "is not an IE of TOY"),
        (
            "/toy/NAS-PDU",
            "insert {\"NAS-PDU\": …} at /toy/- to add it",
        ),
        ("/toy/7", "the path selects nothing"),
    ] {
        let error = inspect::set(&mut tree, path, tree!(1))
            .unwrap_err()
            .to_string();
        assert!(error.contains(reason), "{path}: {error}");
    }
    assert!(inspect::remove(&mut tree, "/toy/NAS-PDU").is_err());
    assert_eq!(tree, before);
}

#[test]
fn a_message_goes_by_the_name_that_asn1_gives_it() {
    // A name finds its message again, however it is written, and no other does.
    for set in Toy::object_sets() {
        let (name, found) = (set.name, Some((set.direction, set.procedure_code)));
        let named =
            |name: &str| Toy::object_set_named(name).map(|set| (set.direction, set.procedure_code));
        assert_eq!(named(name), found, "{name}");
        assert_eq!(named(&name.to_lowercase()), found);
        assert_eq!(named(&name.replace('-', "")), found);
    }
    assert_eq!(Toy::object_set_named("NoSuchMessage"), None);
    // A tree that is written from its name.
    let mut tree = tree!({"message": "release request", "ies": []});
    inspect::insert(&mut tree, "/toy/-", tree!({"UE-RAN-ID": 7})).unwrap();
    let pdu = TOYPDU::from_tree(&tree).unwrap();
    assert_eq!(name_of(&pdu), Some("ReleaseRequest"));
    assert_eq!(pdu.direction(), Direction::Initiating);
    // The name of an ENUMERATED value of the PDU itself is taken as any other.
    inspect::set(&mut tree, "/criticality", tree!("Reject")).unwrap();
    let pdu = TOYPDU::from_tree(&tree).unwrap();
    assert_eq!(pdu.to_tree()["criticality"], tree!("reject"));
}

#[test]
fn null_takes_an_optional_member_out_and_a_transfer_is_set_as_its_octets() {
    let tree = sent(&setup_request()).unwrap();
    let item = "/toy/TransferList/0";
    // An optional member that is there is taken out, and one that is not is not
    // selected.
    let failure = format!("{item}/failureTransfer");
    let mut edited = tree.clone();
    inspect::set(&mut edited, &failure, Value::Null).unwrap();
    let mut edited = sent(&edited).unwrap();
    assert!(select(&edited, &failure).is_empty());
    let error = inspect::set(&mut edited, &failure, Value::Null)
        .unwrap_err()
        .to_string();
    assert!(error.contains("the path selects nothing"), "{error}");
    // A transfer is set as its octets, as the value of an IE is.
    let mut edited = tree.clone();
    inspect::set(&mut edited, TRANSFER, tree!({"octets": "00"})).unwrap();
    let after = sent(&edited).unwrap();
    assert_eq!(select(&after, &format!("{TRANSFER}/octets")), [tree!("00")]);
    // Its decoded value is not taken out from what holds it as a member that its type
    // always has.
    let mut edited = tree.clone();
    let error = inspect::remove(&mut edited, TRANSFER)
        .unwrap_err()
        .to_string();
    assert!(error.contains("its type always has it"), "{error}");
}

#[test]
fn what_is_nested_deeper_than_the_inspection_goes_keeps_its_octets() {
    // A cause in the extension of a cause, forty times.
    let mut cause = Cause::misc(CauseMisc::unspecified);
    for _ in 0..40 {
        let value = oxirush_toy::encode(&cause).unwrap().into();
        cause = Cause::choice_Extensions(ProtocolIEField {
            id: ProtocolIEID(15),
            criticality: Criticality::Ignore,
            value,
        });
    }
    let pdu = build_toy!(IGNORE ReleaseRequest,
        REJECT UE_Core_ID(1u64),
        REJECT UE_RAN_ID(7u32),
        IGNORE Cause(cause),
    )
    .unwrap();
    let pdu = TOYPDU::decode(&pdu.encode().unwrap()).unwrap().into_owned();
    let tree = pdu.to_tree();
    // A path has 64 segments at most: the deepest value is listed whole, at a path that
    // selects it.
    let paths = inspect::paths(&tree);
    let (deepest, value) = (paths.iter())
        .max_by_key(|(path, _)| path.matches('/').count())
        .unwrap();
    assert_eq!(deepest.matches('/').count(), 64, "{deepest}");
    assert!(
        deepest.matches("choice-Extensions").count() > 20,
        "{deepest}"
    );
    assert_eq!(select(&tree, deepest), std::slice::from_ref(value));
    // The cause at the bottom, deeper than the inspection goes, is its octets.
    assert!(tree.to_string().contains("\"octets\""), "{tree}");
    // The other IEs are read, and the PDU is sent as it came.
    assert_eq!(select(&tree, "/toy/UE-RAN-ID"), [tree!(7)]);
    assert_eq!(TOYPDU::from_tree(&tree).unwrap(), pdu);
}

#[test]
fn a_member_that_is_absent_selects_nothing_and_one_that_its_type_has_not_is_an_error() {
    let tree = release_request().to_tree();
    let select = |path: &str| inspect::select(&tree, path);
    // The alternative of a CHOICE that is there, and another one.
    let cause = tree!("user-inactivity");
    assert_eq!(select("/toy/Cause/radioNetwork").unwrap(), [&cause]);
    assert!(select("/toy/Cause/nas").unwrap().is_empty());
    // What follows an IE or a member that is not there is still a path of its type.
    assert!(
        select("/toy/UE-IDs/uE-ID-pair/uE-RAN-ID")
            .unwrap()
            .is_empty()
    );
    let error = select("/toy/UE-IDs/uE-ID-pair/misspelled")
        .unwrap_err()
        .to_string();
    let members = "is not a member of UEIDPair, which has uE-Core-ID, uE-RAN-ID, iE-Extensions";
    assert!(error.contains(members), "{error}");
    let error = select("/toy/Cause/nas/deeper").unwrap_err().to_string();
    assert!(error.contains("which has none"), "{error}");
    // After `*`, a member is an error when no value can have it.
    assert_eq!(select("/toy/*/radioNetwork").unwrap(), [&cause]);
    assert_eq!(select("/toy/Cause/*").unwrap(), [&cause]);
    let error = select("/toy/*/misspelled").unwrap_err().to_string();
    assert!(error.contains("is not a member of"), "{error}");
    // A list of values has no IEs to name.
    let setup = sent(&setup_request()).unwrap();
    let list = "/toy/TransferList";
    let absent = inspect::select(&setup, &format!("{list}/0/iE-Extensions"));
    assert!(absent.unwrap().is_empty());
    let error = inspect::select(&setup, &format!("{list}/Cause"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("go by position"), "{error}");
    let error = inspect::select(&setup, &format!("{list}/0/transfer-id-typo"))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("is not a member of TransferItem, which has transfer-ID"),
        "{error}"
    );
}

#[test]
fn a_procedure_that_the_specification_does_not_have_has_a_tree_of_its_ies() {
    // The message of a RELEASE REQUEST, under a procedure code that TOY does not have:
    // the second octet of a PDU.
    let mut octets = release_request().encode().unwrap();
    octets[1] = 200;
    let pdu = TOYPDU::decode(&octets).unwrap();
    assert_eq!(pdu.procedure_code(), 200);
    // Its tree has its procedure code and its direction in the place of a name, and the
    // criticality of the PDU and of each IE, which no table assigns: it is the PDU again.
    let mut tree = pdu.to_tree();
    assert_eq!(
        tree,
        tree!({
            "procedure_code": 200,
            "direction": "InitiatingMessage",
            "ies": [{"UE-Core-ID": 1, "criticality": "reject"},
                    {"UE-RAN-ID": 7, "criticality": "reject"},
                    {"Cause": {"radioNetwork": "user-inactivity"}, "criticality": "ignore"}],
            "criticality": "ignore",
        })
    );
    assert_eq!(TOYPDU::from_tree(&tree).unwrap(), pdu);
    // Each IE is read with the type of its identifier, and has its name.
    assert_eq!(select(&tree, "/toy/UE-RAN-ID"), [tree!(7)]);
    let paths = inspect::paths(&tree);
    assert!(
        paths.contains(&("/toy/UE-Core-ID".to_string(), tree!(1))),
        "{paths:?}"
    );
    // An edit of it is sent under the same procedure code.
    inspect::set(&mut tree, "/toy/UE-RAN-ID", tree!(9)).unwrap();
    let edited = TOYPDU::from_tree(&tree).unwrap();
    assert_eq!(edited.procedure_code(), 200);
    assert_eq!(select(&edited.to_tree(), "/toy/UE-RAN-ID"), [tree!(9)]);
    // Without the criticality, which no table has for it, it is refused.
    let mut unsaid = tree.clone();
    unsaid.as_object_mut().unwrap().shift_remove("criticality");
    let error = TOYPDU::from_tree(&unsaid).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Value);
    // A message that is no container of protocol IEs does not decode: one IE is
    // claimed, and none is there. Its tree has its octets.
    let pdu = TOYPDU::decode(&[0x00, 200, 0x00, 0x03, 0x00, 0x00, 0x01]).unwrap();
    let tree = pdu.to_tree();
    assert_eq!(
        tree,
        tree!({"procedure_code": 200, "direction": "InitiatingMessage", "octets": "000001",
               "criticality": "reject"})
    );
    assert_eq!(TOYPDU::from_tree(&tree).unwrap(), pdu);
}

#[test]
fn an_edit_that_one_place_refuses_is_done_at_none() {
    // Tunnel is a CHOICE that has no `radioNetwork`, and Cause has it.
    let mut tree = released(tree!([
        {"Tunnel": tunnel(), "criticality": "ignore"},
        {"Cause": {"radioNetwork": "user-inactivity"}},
    ]))
    .unwrap();
    let before = tree.clone();
    // The cause takes the edit, and the tunnel refuses the path.
    let everywhere = "/toy/*/radioNetwork";
    let error = inspect::set(&mut tree, everywhere, tree!("unspecified"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("is not a member of Tunnel"), "{error}");
    assert_eq!(tree, before);
    assert!(inspect::remove(&mut tree, everywhere).is_err());
    assert_eq!(tree, before);
    // The cause alone takes it.
    inspect::set(&mut tree, "/toy/Cause/radioNetwork", tree!("unspecified")).unwrap();
    assert_eq!(
        select(&tree, "/toy/Cause/radioNetwork"),
        [tree!("unspecified")]
    );
    // An entry that is taken out before another refuses the path is put back where it
    // was.
    let mut tree = before.clone();
    let both = tree["ies"].as_array().unwrap().clone();
    tree["ies"].as_array_mut().unwrap().extend(both);
    let before = tree.clone();
    assert!(inspect::remove(&mut tree, "/toy/*/radioNetwork").is_err());
    assert_eq!(tree, before);
}

#[test]
fn an_edit_names_a_member_that_the_type_has_and_keeps_what_it_always_has() {
    let mut tree = release_request().to_tree();
    let before = tree.clone();
    let error = inspect::set(&mut tree, "/toy/Cause/misspelled", tree!(1))
        .unwrap_err()
        .to_string();
    assert!(error.contains("is not a member of Cause"), "{error}");
    let error = inspect::remove(&mut tree, "/toy/Cause/radioNetwork")
        .unwrap_err()
        .to_string();
    assert!(error.contains("a CHOICE has one alternative"), "{error}");
    assert_eq!(tree, before);
    // An alternative takes the place of the one that is there.
    inspect::set(&mut tree, "/toy/Cause/nas", tree!("normal-release")).unwrap();
    let written = sent(&tree).unwrap();
    assert_eq!(
        select(&written, "/toy/Cause"),
        [tree!({"nas": "normal-release"})]
    );
    // A member that the type always has stays, and an optional one is added by its name.
    let mut setup = sent(&setup_request()).unwrap();
    let item = "/toy/TransferList/0";
    let error = inspect::remove(&mut setup, &format!("{item}/transfer-ID"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("its type always has it"), "{error}");
    let error = inspect::set(&mut setup, &format!("{item}/transfer-ID"), Value::Null)
        .unwrap_err()
        .to_string();
    assert!(error.contains("its type always has it"), "{error}");
    let index = tree!([{"ExtensionIndex": 1, "criticality": "ignore"}]);
    inspect::set(&mut setup, &format!("{item}/ie_extensions"), index).unwrap();
    let written = sent(&setup).unwrap();
    let index = format!("{item}/iE-Extensions/ExtensionIndex");
    assert_eq!(select(&written, &index), [tree!(1)]);
}

#[test]
fn a_name_is_its_letters_and_its_digits() {
    let tree = release_request().to_tree();
    for path in [
        "/toy/UE-RAN-ID",
        "/TOY/ue_ran_id",
        "/Toy/ueranid",
        "/toy/UE RAN ID",
        "/IES/1",
    ] {
        assert_eq!(select(&tree, path), [tree!(7)], "{path}");
    }
    for path in ["/toy/cause/radionetwork", "/toy/Cause/Radio-Network"] {
        assert_eq!(select(&tree, path), [tree!("user-inactivity")], "{path}");
    }
    assert_eq!(select(&tree, "/Message"), select(&tree, "/message"));
    // An edit writes the member by the name that ASN.1 gives it.
    let mut edited = tree.clone();
    inspect::set(&mut edited, "/toy/cause/NAS", tree!("deregister")).unwrap();
    assert_eq!(
        select(&edited, "/toy/Cause"),
        [tree!({"nas": "deregister"})]
    );
    assert!(TOYPDU::from_tree(&edited).is_ok());
    assert_eq!(
        Toy::object_set_named("release_REQUEST").map(|set| (set.direction, set.procedure_code)),
        Some((Direction::Initiating, 2))
    );
}

#[test]
fn a_path_is_checked_against_a_message_without_a_tree() {
    for (message, path) in [
        ("ReleaseRequest", "/toy/Cause/nas"),
        ("release-request", "/toy/cause/radio_network"),
        ("ReleaseRequest", "/toy/*/radioNetwork"),
        ("ReleaseRequest", "/toy/0/criticality"),
        ("ReleaseRequest", "/toy/Cause/criticality"),
        ("ReleaseRequest", "/toy/-"),
        ("ReleaseRequest", "/toy"),
        ("ReleaseRequest", "/criticality"),
        // An identifier that has no type may be anything.
        ("ReleaseRequest", "/toy/@id=60000/anything"),
        ("Notification", "/toy/UE-IDs/uE-ID-pair/uE-RAN-ID"),
        ("SetupRequest", "/toy/Areas/*/broadcastPLMNs/0"),
        ("SetupRequest", "/toy/Areas/0/tAC"),
        ("SetupRequest", &format!("{TRANSFER}/TransferKind")),
        (
            "SetupRequest",
            &format!("{TRANSFER}/Tunnel/gTPTunnel/gTP-TEID"),
        ),
        ("SetupRequest", &format!("{TRANSFER}/protocolIEs/0/Tunnel")),
        ("SetupRequest", &format!("{TRANSFER}/@id=29/gTPTunnel")),
    ] {
        let checked = inspect::check_path(message, path);
        assert!(checked.is_ok(), "{message} {path}: {checked:?}");
    }
    for (message, path, reason) in [
        ("NoSuchMessage", "/toy/Cause", "is not a message of TOY"),
        (
            "ReleaseRequest",
            "/toy/Areas",
            "is not an IE of ReleaseRequest, which has UE-Core-ID, UE-RAN-ID, Cause, NAS-PDU",
        ),
        (
            "ReleaseRequest",
            "/toy/Cause/misspelled",
            "is not a member of Cause, which has radioNetwork, nas, protocol, misc, \
             choice-Extensions, at /toy/Cause/misspelled",
        ),
        ("ReleaseRequest", "/toy/*/misspelled", "is not a member of"),
        ("ReleaseRequest", "/toy/UE-RAN-ID/deeper", "which has none"),
        (
            "ReleaseRequest",
            "/toy/UE-RAN-ID/*/deeper",
            "selects nothing in a message ReleaseRequest",
        ),
        (
            "ReleaseRequest",
            "/toy/Cause/value/nas",
            "a path is the tree's names joined: write `/toy/Cause/nas`",
        ),
        ("ReleaseRequest", "/toy/NoSuchIE", "is not an IE of TOY"),
        (
            "SetupRequest",
            &format!("{TRANSFER}/misspelled"),
            "is not an IE of TOY",
        ),
        (
            "SetupRequest",
            &format!("{TRANSFER}/Tunnel/gTPTunnel/misspelled"),
            "is not a member of GTPTunnel",
        ),
        ("SetupRequest", "/toy/TransferList/Cause", "go by position"),
    ] {
        let error = inspect::check_path(message, path).unwrap_err().to_string();
        assert!(error.contains(reason), "{message} {path}: {error}");
    }
}

/// A path is the names of the tree joined: after the name of an IE, the next name is a
/// member of its value, and `value`, which no tree has there, is refused with the path
/// that the names of the tree write, by every call that takes a path. A value whose type
/// has a member `value`, a bit string with its length, keeps it.
#[test]
fn a_path_is_the_names_of_the_tree_and_value_after_an_ie_is_refused() {
    let joined = |written: &str, path: &str| {
        format!("a path is the tree's names joined: write `{written}`, at {path}")
    };
    let mut tree = release_request().to_tree();
    let before = tree.clone();
    assert_eq!(
        select(&tree, "/toy/Cause/radioNetwork"),
        [tree!("user-inactivity")]
    );
    for (path, written) in [
        ("/toy/Cause/value/radioNetwork", "/toy/Cause/radioNetwork"),
        ("/toy/UE-RAN-ID/value", "/toy/UE-RAN-ID"),
        ("/toy/cause/VALUE/radio-network", "/toy/cause/radio-network"),
        ("/toy/2/value/radioNetwork", "/toy/2/radioNetwork"),
        ("/toy/@id=15/value", "/toy/@id=15"),
        ("/toy/Cause/Cause/value", "/toy/Cause/Cause"),
        ("/toy/*/value", "/toy/*"),
        // An IE that the message does not have, and an identifier that has no type.
        ("/toy/NAS-PDU/value", "/toy/NAS-PDU"),
        ("/toy/@id=60000/value", "/toy/@id=60000"),
    ] {
        let error = inspect::select(&tree, path).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Path, "{path}");
        assert_eq!(error.to_string(), joined(written, path));
        assert!(inspect::select(&tree, written).is_ok(), "{written}");
        for refused in [
            inspect::set(&mut tree, path, tree!(1)),
            inspect::remove(&mut tree, path),
            inspect::insert(&mut tree, path, tree!(1)),
            inspect::open(&mut tree, path),
            inspect::check_path("ReleaseRequest", path),
            inspect::enumerated_at("ReleaseRequest", path).map(|_| ()),
        ] {
            let refused = refused.map_err(|error| error.to_string());
            assert_eq!(refused, Err(joined(written, path)));
        }
    }
    assert_eq!(tree, before);
    // Each `value` after an IE goes: in a transfer, in a list of single containers, and
    // not that of a bit string whose size varies.
    let setup = sent(&setup_request()).unwrap();
    let items = sent(&with_items()).unwrap();
    let address = "gTPTunnel/transportLayerAddress/value";
    for (tree, path, written) in [
        (
            &setup,
            "/toy/TransferList/value/0/setupTransfer/Tunnel/value/gTPTunnel".to_string(),
            format!("{TRANSFER}/Tunnel/gTPTunnel"),
        ),
        (
            &setup,
            format!("{TRANSFER}/Tunnel/value/{address}"),
            format!("{TRANSFER}/Tunnel/{address}"),
        ),
        (
            &items,
            "/toy/Items/0/Item/value/item-ID".to_string(),
            "/toy/Items/0/Item/item-ID".to_string(),
        ),
        (
            &items,
            "/toy/Items/value/0/item-ID".to_string(),
            "/toy/Items/0/item-ID".to_string(),
        ),
    ] {
        let error = inspect::select(tree, &path).unwrap_err().to_string();
        assert_eq!(error, joined(&written, &path));
        assert!(inspect::select(tree, &written).is_ok(), "{written}");
    }
    let tree = with_algorithms(24).to_tree();
    assert_eq!(select(&tree, "/toy/Algorithms/value"), [tree!("c00000")]);
    let checked = inspect::check_path("SetupRequest", "/toy/Algorithms/value");
    assert_eq!(checked, Ok(()));
}

#[test]
fn a_private_message_has_no_ies_under_the_root() {
    let message = PrivateMessage {
        private_ies: PrivateMessagePrivateIEs(vec![AnonymousPrivateMessagePrivateIEs {
            id: PrivateIEID::local(1),
            criticality: Criticality::Ignore,
            value: Octets::from(vec![1, 2]),
        }]),
    };
    let pdu = TOYPDU::initiatingMessage(InitiatingMessage {
        procedure_code: ProcedureCode(4),
        criticality: Criticality::Ignore,
        value: Octets::owned(oxirush_toy::encode(&message).unwrap()),
    });
    let tree = pdu.to_tree();
    let octets = hex::encode(oxirush_toy::encode(&message).unwrap());
    assert_eq!(tree, tree!({"message": "PrivateMessage", "octets": octets}));
    assert_eq!(TOYPDU::from_tree(&tree).unwrap(), pdu);
    let reason = "a message PrivateMessage has no IEs: its tree has its octets";
    let error = inspect::select(&tree, "/toy/Cause")
        .unwrap_err()
        .to_string();
    assert!(error.contains(reason), "{error}");
    let error = inspect::check_path("PrivateMessage", "/toy/Cause")
        .unwrap_err()
        .to_string();
    assert!(error.contains(reason), "{error}");
    assert_eq!(inspect::check_path("PrivateMessage", "/octets"), Ok(()));
}

/// A SETUP REQUEST with counts: the downlink one is an INTEGER (0..4000000000000, ...).
fn counted(downlink: u64) -> TOYPDU<'static> {
    let counts = Counts {
        count_ul: 1,
        counts: None,
        count_dl: downlink,
        ie_extensions: None,
    };
    let area = AreaItem {
        tac: TAC([0, 0, 1]),
        broadcast_plmns: BroadcastPLMNs(vec![PLMNIdentity([0x02, 0xf8, 0x39])]),
        ie_extensions: None,
    };
    build_toy!(SetupRequest, Areas(Areas(vec![area])), Counts(counts),).unwrap()
}

/// An INTEGER is a number (ITU-T X.697 (02/2021) §21). One of 2^63 or more is read,
/// edited and written as any other: the octets are those that the bindings encode. The
/// bindings hold a number in 64 bits: an IE with a larger one keeps the octets that it
/// was received as.
#[test]
fn an_integer_of_two_to_the_63_or_more_is_a_number() {
    let downlink = "/toy/Counts/countDL";
    for count in [(1u64 << 63) - 1, 1 << 63, u64::MAX] {
        // Received, it is the number that it is.
        let wire = counted(count).encode().unwrap();
        let mut tree = TOYPDU::decode(&wire).unwrap().to_tree();
        assert_eq!(select(&tree, downlink), [tree!(count)]);
        assert_eq!(TOYPDU::from_tree(&tree).unwrap().encode().unwrap(), wire);
        // Edited, it is what the bindings encode for the other number.
        inspect::set(&mut tree, downlink, tree!(count - 1)).unwrap();
        assert_eq!(TOYPDU::from_tree(&tree).unwrap(), counted(count - 1));
    }
    // The count has an extension marker, so a later version may have a count of 2^64
    // or more. The bindings hold a number in 64 bits, and refuse that one: its IE is
    // shown as the octets that were received, and is written back as it is. The octets
    // are those of the largest count that the bindings hold, with the nine octets of
    // 2^64 in the place of its own nine.
    let largest = Counts {
        count_ul: 1,
        counts: None,
        count_dl: u64::MAX,
        ie_extensions: None,
    };
    let largest = oxirush_toy::encode(&largest).unwrap();
    let nine = [0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];
    let (start, end) = largest.split_at(largest.len() - nine.len());
    assert_eq!(end, nine);
    let beyond = [start, &[0x01], &[0; 8]].concat();
    let error = oxirush_toy::decode::<Counts>(&beyond).unwrap_err();
    assert_eq!(
        error.to_string(),
        "these octets do not decode as Counts: the number does not fit in 64 bits, in its \
         field `count_dl`"
    );
    let mut pdu = counted(1);
    let mut ies = pdu.protocol_ies().unwrap().into_owned();
    ies.0[1].value = beyond.clone().into();
    pdu.set_protocol_ies(ies).unwrap();
    let wire = pdu.encode().unwrap();
    let tree = TOYPDU::decode(&wire).unwrap().to_tree();
    let octets = tree!({"octets": hex::encode(&beyond)});
    assert_eq!(tree["ies"][1], tree!({"Counts": octets}));
    // The other IEs are read, and the tree gives the octets back.
    assert_eq!(select(&tree, "/toy/Areas/0/tAC"), [tree!(1)]);
    assert_eq!(TOYPDU::from_tree(&tree).unwrap().encode().unwrap(), wire);
    // JSON has no exact number for 2^64, and its decimal digits are no number: a count
    // is written as a number that 64 bits hold.
    let written = |downlink: Value| {
        tree!({"message": "SetupRequest", "ies": [
            {"Areas": areas()},
            {"Counts": {"countUL": 1, "countDL": downlink}},
        ]})
    };
    for digits in ["18446744073709551616", "18446744073709551615"] {
        let error = TOYPDU::from_tree(&written(tree!(digits))).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Value);
        assert_eq!(
            error.to_string(),
            format!(
                "this is not a value of Counts: a number from 0 to 18446744073709551615 is \
                 expected, and \"{digits}\" is written, at countDL"
            )
        );
    }
    let negative = TOYPDU::from_tree(&written(tree!(-1))).unwrap_err();
    assert_eq!(
        negative.to_string(),
        "this is not a value of Counts: a number from 0 to 18446744073709551615 is expected, \
         and -1 is written, at countDL"
    );
}

/// A SETUP REQUEST whose algorithms are this many bits, the first two of them set.
fn with_algorithms(bits: usize) -> TOYPDU<'static> {
    let mut octets = vec![0; bits.div_ceil(8)];
    octets[0] = 0xc0;
    let algorithms = Algorithms(oxirush_toy::BitString::new(octets, bits).unwrap());
    let area = AreaItem {
        tac: TAC([0, 0, 1]),
        broadcast_plmns: BroadcastPLMNs(vec![PLMNIdentity([0x02, 0xf8, 0x39])]),
        ie_extensions: None,
    };
    build_toy!(
        SetupRequest,
        Areas(Areas(vec![area])),
        Algorithms(algorithms),
    )
    .unwrap()
}

/// A BIT STRING whose type gives one size and an extension marker has its length when
/// it is of another size (ITU-T X.697 (02/2021) §24.3), and is its octets alone when it
/// is of that size, as it always was. Both are read, edited and written: the octets
/// are those that the bindings encode.
#[test]
fn a_bit_string_of_another_size_than_its_type_gives_has_its_length() {
    let algorithms = "/toy/Algorithms";
    // The size that the type gives: hexadecimal, read and edited.
    let mut tree = with_algorithms(16).to_tree();
    assert_eq!(select(&tree, algorithms), [tree!("c000")]);
    inspect::set(&mut tree, algorithms, tree!("8000")).unwrap();
    assert_eq!(select(&sent(&tree).unwrap(), algorithms), [tree!("8000")]);
    // 20 bits and 24 bits are the same three octets, and not the same string.
    for bits in [20, 24] {
        let wire = with_algorithms(bits).encode().unwrap();
        let mut tree = TOYPDU::decode(&wire).unwrap().to_tree();
        let shown = tree!({"value": "c00000", "length": bits});
        assert_eq!(select(&tree, algorithms), [shown]);
        // Its two members have paths, which a message is checked against.
        let length = format!("{algorithms}/length");
        assert_eq!(select(&tree, &length), [tree!(bits)]);
        let paths = inspect::paths(&tree);
        assert!(paths.iter().any(|(path, _)| *path == length), "{paths:?}");
        assert_eq!(inspect::check_path("SetupRequest", &length), Ok(()));
        assert_eq!(TOYPDU::from_tree(&tree).unwrap().encode().unwrap(), wire);
        // Edited to the size of the type, it is what the bindings encode for that.
        inspect::set(&mut tree, algorithms, tree!("c000")).unwrap();
        assert_eq!(TOYPDU::from_tree(&tree).unwrap(), with_algorithms(16));
    }
    // From the size of the type to another one, by the whole value and by its length.
    let mut tree = with_algorithms(16).to_tree();
    let longer = tree!({"value": "c00000", "length": 24});
    inspect::set(&mut tree, algorithms, longer).unwrap();
    assert_eq!(TOYPDU::from_tree(&tree).unwrap(), with_algorithms(24));
    inspect::set(&mut tree, &format!("{algorithms}/length"), tree!(20)).unwrap();
    assert_eq!(TOYPDU::from_tree(&tree).unwrap(), with_algorithms(20));
    // What is not the string that it says is refused: three octets without a length
    // are not the 16 bits of the type, and 20 bits are not two octets.
    let written = |algorithms: Value| {
        TOYPDU::from_tree(&tree!({"message": "SetupRequest", "ies": [
            {"Areas": areas()},
            {"Algorithms": algorithms},
        ]}))
    };
    assert!(written(tree!("c000")).is_ok());
    let error = written(tree!("c00000")).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Value);
    assert_eq!(
        error.to_string(),
        "this is not a value of Algorithms: 3 octets are written for a bit string of 16 bits, \
         which has 2"
    );
    assert!(written(tree!({"value": "c000", "length": 20})).is_err());
}

/// id-ExtensionIndex is an IndexOfItem, INTEGER (1..8, ...), where it extends an Item,
/// and an IndexOfTransfer, INTEGER (1..4, ...), where it extends a TransferItem: an IE of
/// an extension container has the type that the type which the container extends gives
/// it, and the two types give the same number other octets.
#[test]
fn an_ie_of_an_extension_has_the_type_of_what_it_extends() {
    let of_item = |index: u64| oxirush_toy::encode(&IndexOfItem(index)).unwrap();
    let of_transfer = |index: u64| oxirush_toy::encode(&IndexOfTransfer(index)).unwrap();
    assert_eq!((of_item(3), of_transfer(3)), (vec![0x20], vec![0x40]));
    let indexed = |item: Value, transfer: Value| {
        tree!({"message": "SetupRequest", "ies": [
            {"Areas": areas()},
            {"Items": [{"Item": {
                "item-ID": 1,
                "iE-Extensions": [{"ExtensionIndex": item, "criticality": "ignore"}],
            }}]},
            {"TransferList": [{
                "transfer-ID": 1,
                "setupTransfer": {"protocolIEs": [{"Tunnel": tunnel()}]},
                "iE-Extensions": [{"ExtensionIndex": transfer, "criticality": "ignore"}],
            }]},
        ]})
    };
    // The octets of each, as the bindings read them.
    let octets = |pdu: &TOYPDU<'_>| {
        let item = pdu
            .items::<oxirush_toy::ie::Items>()
            .next()
            .unwrap()
            .unwrap();
        let ies = pdu.protocol_ies().unwrap();
        let list: TransferList = oxirush_toy::decode(&ies.0[2].value).unwrap();
        let value = |extensions: &Option<ProtocolExtensionContainer>| {
            extensions.as_ref().unwrap().0[0].extension_value.to_vec()
        };
        (value(&item.ie_extensions), value(&list.0[0].ie_extensions))
    };
    let item = "/toy/Items/0/Item/iE-Extensions/ExtensionIndex";
    let transfer = "/toy/TransferList/0/iE-Extensions/ExtensionIndex";
    let tree = indexed(tree!(3), tree!(3));
    let pdu = TOYPDU::from_tree(&tree).unwrap();
    assert_eq!(octets(&pdu), (of_item(3), of_transfer(3)));
    let wire = pdu.encode().unwrap();
    let mut shown = TOYPDU::decode(&wire).unwrap().to_tree();
    assert_eq!(shown, tree);
    assert_eq!(select(&shown, item), [tree!(3)]);
    // Edited, each by its own type.
    inspect::set(&mut shown, item, tree!(8)).unwrap();
    inspect::set(&mut shown, transfer, tree!(1)).unwrap();
    let edited = TOYPDU::from_tree(&shown).unwrap();
    assert_eq!(octets(&edited), (of_item(8), of_transfer(1)));
    // A path is read against the type without a tree, by the name of the IE, by its
    // identifier and after its position.
    for extensions in [
        "/toy/Items/0/Item/iE-Extensions",
        "/toy/TransferList/0/iE-Extensions",
    ] {
        for ie in ["ExtensionIndex", "@id=40", "0/ExtensionIndex"] {
            let value = format!("{extensions}/{ie}");
            let checked = inspect::check_path("SetupRequest", &value);
            assert_eq!(checked, Ok(()), "{value}");
            let names = inspect::enumerated_at("SetupRequest", &value);
            assert_eq!(names, Ok(None), "{value}");
            let member = format!("{value}/member");
            let refused = inspect::check_path("SetupRequest", &member)
                .unwrap_err()
                .to_string();
            assert!(refused.contains("which has none"), "{refused}");
        }
    }
    // Where it extends nothing that gives it a type, it has none: it is sent as octets.
    let alone = |value: Value| {
        tree!({"message": "ReleaseRequest", "ies": [
            {"ExtensionIndex": value, "criticality": "ignore"},
        ]})
    };
    let refused = TOYPDU::from_tree(&alone(tree!(3))).unwrap_err().to_string();
    assert!(
        refused.contains("TOY IE 40 has a type where it extends"),
        "{refused}"
    );
    assert!(refused.contains("none here"), "{refused}");
    let pdu = TOYPDU::from_tree(&alone(tree!({"octets": "20"}))).unwrap();
    assert_eq!(pdu.to_tree(), alone(tree!({"octets": "20"})));
}

/// A value whose octets are not those that the bindings write for it, as a UE-Core-ID
/// whose padding bits are 0011, is its octets in a tree, which are sent as they came.
#[test]
fn a_value_that_is_not_written_as_the_bindings_write_it_is_its_octets() {
    let pdu = released(tree!([
        {"UE-Core-ID": {"octets": "0301"}},
        {"UE-RAN-ID": 7},
    ]))
    .unwrap();
    assert_eq!(select(&pdu, "/toy/UE-Core-ID/octets"), [tree!("0301")]);
    let wire = TOYPDU::from_tree(&pdu).unwrap().encode().unwrap();
    let again = TOYPDU::decode(&wire).unwrap().to_tree();
    assert_eq!(again, pdu);
    // A bit of any IE changed is the same PDU through its tree.
    let request = TOYPDU::from_tree(&setup_request()).unwrap();
    let ies = request.protocol_ies().unwrap().into_owned();
    for at in 0..ies.0.len() {
        for bit in 0..8 {
            let mut changed = ies.clone();
            let mut value = changed.0[at].value.to_vec();
            *value.last_mut().unwrap() ^= 1 << bit;
            changed.0[at].value = Octets::owned(value);
            let mut pdu = request.clone();
            pdu.set_protocol_ies(changed).unwrap();
            let tree = pdu.to_tree();
            assert_eq!(TOYPDU::from_tree(&tree).as_ref(), Ok(&pdu), "{tree}");
        }
    }
}

/// A SETUP REQUEST of its Areas and two Items, a list of single containers.
fn with_items() -> Value {
    tree!({"message": "SetupRequest", "ies": [
        {"Areas": areas()},
        {"Items": [
            {"Item": {"item-ID": 1, "cause": {"nas": "deregister"}}},
            {"Item": {"item-ID": 2}},
        ]},
    ]})
}

#[test]
fn the_items_of_a_list_go_by_their_names_or_their_positions() {
    let name = "SetupRequest";
    let tree = sent(&with_items()).unwrap();
    assert_eq!(tree, with_items());
    let list = "/toy/Items";
    for path in [
        format!("{list}/0"),
        format!("{list}/0").to_uppercase(),
        format!("{list}/0/Item"),
    ] {
        let selected = select(&tree, &format!("{path}/item-ID"));
        assert_eq!(selected, [tree!(1)], "{path}");
        let checked = inspect::check_path(name, &format!("{path}/item-ID"));
        assert_eq!(checked, Ok(()), "{path}");
    }
    // The name of the item and its identifier select each item.
    for each in ["Item", "@id=23"] {
        let ids = select(&tree, &format!("{list}/{each}/item-ID"));
        assert_eq!(ids, [tree!(1), tree!(2)], "{each}");
    }
    // An item has the criticality that its object set assigns, which the tree leaves out.
    assert!(select(&tree, &format!("{list}/0/criticality")).is_empty());
    assert_eq!(select(&tree, &format!("{list}/*")).len(), 2);
    let names = inspect::enumerated_at(name, &format!("{list}/Item/cause/nas")).unwrap();
    assert_eq!(
        names,
        Some(&["normal-release", "deregister", "unspecified"][..])
    );
    assert_eq!(
        inspect::enumerated_at(name, &format!("{list}/@id=23/cause/nas")),
        Ok(names)
    );
    // Edits take the same paths: a member is set, an item is added and one taken out.
    let mut edited = tree.clone();
    let first = select(&tree, &format!("{list}/0"))[0].clone();
    let entry = tree!({"Item": first});
    inspect::set(&mut edited, &format!("{list}/1/item-ID"), tree!(5)).unwrap();
    inspect::insert(&mut edited, &format!("{list}/-"), entry.clone()).unwrap();
    inspect::insert(&mut edited, &format!("{list}/0"), entry).unwrap();
    inspect::set(
        &mut edited,
        &format!("{list}/0/cause/nas"),
        tree!("unspecified"),
    )
    .unwrap();
    inspect::remove(&mut edited, &format!("{list}/3")).unwrap();
    let after = sent(&edited).unwrap();
    let ids = select(&after, &format!("{list}/*/item-ID"));
    assert_eq!(ids, [tree!(1), tree!(1), tree!(5)]);
    let causes = select(&after, &format!("{list}/*/cause/nas"));
    assert_eq!(causes, [tree!("unspecified"), tree!("deregister")]);
    // A name that the value does not have is refused with its members.
    let before = edited.clone();
    for (path, reason) in [
        (
            format!("{list}/Item/misspelled"),
            "is not a member of Item, which has item-ID",
        ),
        (
            format!("{list}/misspelled"),
            "\"misspelled\" is not an IE of TOY",
        ),
    ] {
        for error in [
            inspect::select(&edited, &path)
                .map(|_| ())
                .unwrap_err()
                .to_string(),
            inspect::set(&mut edited, &path, tree!(1))
                .unwrap_err()
                .to_string(),
            inspect::check_path(name, &path).unwrap_err().to_string(),
        ] {
            assert!(error.contains(reason), "{path}: {error}");
        }
    }
    assert_eq!(edited, before);
}

/// A RELEASE REQUEST with a Tunnel, which a tree writes with the criticality of its
/// IE, and a Cause.
fn with_tunnel(tunnel: Value) -> Value {
    tree!({"message": "ReleaseRequest", "ies": [
        {"Tunnel": tunnel, "criticality": "ignore"},
        {"Cause": {"radioNetwork": "user-inactivity"}},
    ]})
}

/// An alternative of the type NULL is a value, `null`, which a path lists and selects
/// as any other, and `null` set there is that alternative. Elsewhere `null` is the
/// absence of a value.
#[test]
fn an_alternative_of_the_type_null_is_a_value() {
    let unassigned = with_tunnel(tree!({"unassigned": null}));
    let tree = sent(&unassigned).unwrap();
    assert_eq!(tree, unassigned);
    assert_eq!(select(&tree, "/toy/Tunnel/unassigned"), [Value::Null]);
    assert_eq!(select(&tree, "/toy/Tunnel/*"), [Value::Null]);
    let paths = inspect::paths(&tree);
    assert!(
        paths.contains(&("/toy/Tunnel/unassigned".to_string(), Value::Null)),
        "{paths:?}"
    );
    // An alternative that is not there selects nothing.
    assert!(select(&tree, "/toy/Tunnel/gTPTunnel").is_empty());
    let checked = inspect::check_path("SetupRequest", "/toy/Tunnel/unassigned");
    assert_eq!(checked, Ok(()));
    // `null` set there is that alternative, in the place of the one there.
    let mut edited = sent(&with_tunnel(tunnel())).unwrap();
    inspect::set(&mut edited, "/toy/Tunnel/unassigned", Value::Null).unwrap();
    assert_eq!(edited, unassigned);
    assert_eq!(sent(&edited).unwrap(), unassigned);
    // Elsewhere, `null` takes an optional member out, which then selects nothing.
    let mut setup = sent(&setup_request()).unwrap();
    let failure = "/toy/TransferList/0/failureTransfer";
    inspect::set(&mut setup, failure, Value::Null).unwrap();
    assert!(select(&setup, failure).is_empty());
}

/// So it is in a procedure that the tables do not have: its message is read as a
/// container of IEs, each of the type of its identifier.
#[test]
fn an_alternative_of_the_type_null_is_a_value_in_a_procedure_that_the_tables_lack() {
    let pdu = TOYPDU::from_tree(&with_tunnel(tree!({"unassigned": null}))).unwrap();
    let mut octets = pdu.encode().unwrap();
    octets[1] = 200;
    let tree = TOYPDU::decode(&octets).unwrap().to_tree();
    assert_eq!(tree["procedure_code"], tree!(200));
    let paths = inspect::paths(&tree);
    assert!(
        paths.contains(&("/toy/Tunnel/unassigned".to_string(), Value::Null)),
        "{paths:?}"
    );
    for (path, value) in &paths {
        assert_eq!(select(&tree, path), std::slice::from_ref(value), "{path}");
    }
}

/// A view of a PDU whose values another implementation wrote in the JSON Encoding Rules,
/// as pycrate writes them, is read as a tree: each IE by its name, and the values of the
/// well-known types in their readable form, where it gives the same value back.
#[test]
fn a_view_in_the_json_encoding_rules_is_read_as_a_tree() {
    let view = tree!({
        "procedure_code": 1,
        "direction": "InitiatingMessage",
        "criticality": "reject",
        "message": {"protocolIEs": [
            {"id": 26, "criticality": "reject", "value": [
                {"tAC": "000001", "broadcastPLMNs": ["02f839"]},
            ]},
            {"id": 20, "criticality": "ignore", "value": "node-1"},
        ]},
    });
    let tree = oxirush_toy::inspect::__tree_of_jer(&view).unwrap();
    assert_eq!(
        tree,
        tree!({"message": "SetupRequest", "ies": [
            {"Areas": [{"tAC": 1, "broadcastPLMNs": ["208-93"]}]},
            {"NodeName": "node-1"},
        ]})
    );
    assert!(TOYPDU::from_tree(&tree).is_ok());
    // A view without its procedure code or its direction is refused.
    let mut without = view.clone();
    without
        .as_object_mut()
        .unwrap()
        .shift_remove("procedure_code");
    assert!(oxirush_toy::inspect::__tree_of_jer(&without).is_err());
}

/// A transfer is the value that it contains in the JSON Encoding Rules, as pycrate writes
/// it, in a member of a value and as the value of an IE: it is read as a tree has it, its
/// decoded value.
#[test]
fn a_transfer_in_the_json_encoding_rules_is_read_as_its_decoded_value() {
    let read = |view: Value| {
        let tree = oxirush_toy::inspect::__tree_of_jer(&view).unwrap();
        TOYPDU::from_tree(&tree).map(|_| tree)
    };
    let failure = tree!({"cause": {"protocol": "semantic-error"}});
    let request = tree!({
        "procedure_code": 1,
        "direction": "InitiatingMessage",
        "criticality": "reject",
        "message": {"protocolIEs": [
            {"id": 26, "criticality": "reject", "value": [
                {"tAC": "000001", "broadcastPLMNs": ["02f839"]},
            ]},
            {"id": 24, "criticality": "reject", "value": [{
                "transfer-ID": 1,
                "setupTransfer": {"protocolIEs": [
                    {"id": 29, "criticality": "reject", "value": {"unassigned": null}},
                ]},
                "failureTransfer": &failure,
            }]},
        ]},
    });
    assert_eq!(
        read(request).unwrap(),
        tree!({"message": "SetupRequest", "ies": [
            {"Areas": areas()},
            {"TransferList": [{
                "transfer-ID": 1,
                "setupTransfer": {"protocolIEs": [{"Tunnel": {"unassigned": null}}]},
                "failureTransfer": &failure,
            }]},
        ]})
    );
    let failed = tree!({
        "procedure_code": 1,
        "direction": "UnsuccessfulOutcome",
        "criticality": "reject",
        "message": {"protocolIEs": [
            {"id": 15, "criticality": "ignore", "value": {"protocol": "semantic-error"}},
            {"id": 34, "criticality": "ignore", "value": &failure},
        ]},
    });
    assert_eq!(
        read(failed).unwrap(),
        tree!({"message": "SetupFailure", "ies": [
            {"Cause": {"protocol": "semantic-error"}},
            {"FailureTransfer": &failure},
        ]})
    );
}

/// A value that no table describes, as one that a tree written by hand gives an IE that
/// the protocol does not have, has the members that the tree gives it: a set adds one
/// as the last segment of its path, and nothing else goes into one that is not there.
#[test]
fn a_value_that_no_table_describes_has_the_members_of_its_tree() {
    let mut tree = tree!({"message": "ReleaseRequest", "ies": [
        {"60000": {"a": 1}, "criticality": "ignore"},
    ]});
    assert_eq!(select(&tree, "/toy/@id=60000/a"), [tree!(1)]);
    inspect::set(&mut tree, "/toy/@id=60000/b", tree!(2)).unwrap();
    assert_eq!(tree["ies"][0]["60000"], tree!({"a": 1, "b": 2}));
    for refused in ["/toy/@id=60000/c/d", "/toy/@id=60000/A/B"] {
        let before = tree.clone();
        assert!(
            inspect::set(&mut tree, refused, tree!(3)).is_err(),
            "{refused}"
        );
        assert_eq!(tree, before);
    }
    let error = inspect::select(&tree, "/toy/@id=60000/c").unwrap_err();
    assert_eq!(
        error.to_string(),
        "\"c\" is not a member of this value, at /toy/@id=60000/c"
    );
    assert!(inspect::remove(&mut tree, "/toy/@id=60000/c").is_err());
    inspect::remove(&mut tree, "/toy/@id=60000/a").unwrap();
    assert_eq!(tree["ies"][0]["60000"], tree!({"b": 2}));
}

/// A bit string of the size that its type gives is its octets in hexadecimal, and has
/// the members of a bit string all the same: its value and its length, which select
/// nothing in it.
#[test]
fn a_bit_string_of_the_size_of_its_type_has_the_members_of_a_bit_string() {
    let tree = sent(&tree!({"message": "SetupRequest", "ies": [
        {"Areas": areas()}, {"Algorithms": "c000"},
    ]}))
    .unwrap();
    assert_eq!(select(&tree, "/toy/Algorithms"), [tree!("c000")]);
    assert!(select(&tree, "/toy/Algorithms/length").is_empty());
    let error = inspect::select(&tree, "/toy/Algorithms/x").unwrap_err();
    assert_eq!(
        error.to_string(),
        "\"x\" is not a member of a bit string, which has value, length, at /toy/Algorithms/x"
    );
}

/// The entry of an IE that a value holds alone, written without the IE, takes the IE that
/// a set gives it, as the last segment of its path; nothing else goes into it.
#[test]
fn an_ie_that_a_value_holds_alone_is_added_by_a_set() {
    let mut tree = tree!({"message": "SetupFailure", "ies": [
        {"Cause": {"choice-Extensions": {"criticality": "ignore"}}},
    ]});
    let held = "/toy/Cause/choice-Extensions";
    assert!(select(&tree, &format!("{held}/ExtendedCause")).is_empty());
    assert!(select(&tree, &format!("{held}/ExtendedCause/x")).is_empty());
    let deeper = inspect::set(&mut tree, &format!("{held}/ExtendedCause/x"), tree!(1));
    assert!(deeper.is_err());
    inspect::set(
        &mut tree,
        &format!("{held}/ExtendedCause"),
        tree!("overload"),
    )
    .unwrap();
    assert_eq!(
        select(&tree, held),
        [tree!({"criticality": "ignore", "ExtendedCause": "overload"})]
    );
    assert!(TOYPDU::from_tree(&tree).is_ok());
    // `*` there is the value of the IE and the criticality beside it, which has no
    // members: a bit string has its length, and the criticality refuses one.
    let bits = tree!({"message": "SetupFailure", "ies": [
        {"Cause": {"choice-Extensions": {"Algorithms": "c000", "criticality": "ignore"}}},
    ]});
    let star = format!("{held}/*");
    assert_eq!(select(&bits, &star), [tree!("c000"), tree!("ignore")]);
    // `check_path` takes the same path without a tree.
    assert_eq!(inspect::check_path("SetupFailure", &star), Ok(()));
    let error = inspect::select(&bits, &format!("{star}/length")).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("\"length\" is not a member of this value"),
        "{error}"
    );
}

/// A set adds a member that its type has and the value has not as the last segment of its
/// path, and goes into none that is not there.
#[test]
fn a_set_adds_a_member_at_the_end_of_its_path_alone() {
    let mut tree = sent(&tree!({"message": "SetupRequest", "ies": [
        {"Areas": areas()},
        {"TransferList": [{"transfer-ID": 1, "setupTransfer": {"protocolIEs": [{"Tunnel": tunnel()}]}}]},
    ]}))
    .unwrap();
    let item = "/toy/TransferList/0";
    let before = tree.clone();
    let through = format!("{item}/failureTransfer/cause/nas");
    assert!(inspect::set(&mut tree, &through, tree!("deregister")).is_err());
    assert!(inspect::remove(&mut tree, &format!("{item}/failureTransfer")).is_err());
    assert_eq!(tree, before);
    let failure = tree!({"cause": {"nas": "deregister"}});
    inspect::set(
        &mut tree,
        &format!("{item}/failureTransfer"),
        failure.clone(),
    )
    .unwrap();
    assert_eq!(select(&tree, &format!("{item}/failureTransfer")), [failure]);
}

/// A member of the name of the IE whose value has it is that member when the path writes
/// it as the member is written, and the IE that the value is otherwise, with a tree and
/// without one: the Counts of a SETUP REQUEST have their `counts`.
#[test]
fn a_member_of_the_name_of_its_ie_is_that_member_as_it_is_written() {
    let counts = tree!({"countUL": 1, "counts": 7, "countDL": 2});
    let tree = sent(&tree!({"message": "SetupRequest", "ies": [
        {"Areas": areas()}, {"Counts": counts},
    ]}))
    .unwrap();
    for path in [
        "/toy/1/counts",
        "/toy/1/Counts/counts",
        "/toy/Counts/counts",
        "/toy/Counts/Counts/counts",
    ] {
        assert_eq!(select(&tree, path), [tree!(7)], "{path}");
        assert_eq!(inspect::check_path("SetupRequest", path), Ok(()), "{path}");
    }
}

/// The name of the IE stays after the IE once, in a list as in a value, with a tree and
/// without one: a second time it is refused.
#[test]
fn the_name_of_a_list_of_ies_stays_after_it_once() {
    let tree = sent(&tree!({"message": "SetupRequest", "ies": [
        {"Areas": areas()},
        {"Items": [{"Item": {"item-ID": 1}}]},
    ]}))
    .unwrap();
    assert_eq!(select(&tree, "/toy/Items/Items/0/item-ID"), [tree!(1)]);
    assert!(select(&tree, "/toy/Items/Items/Items/0").is_empty());
    let checked = |path: &str| inspect::check_path("SetupRequest", path);
    assert_eq!(checked("/toy/Items/Items/0/item-ID"), Ok(()));
    assert_eq!(checked("/toy/Items/Items/Items/0"), Ok(()));
    assert_eq!(checked("/toy/Areas/Areas/0/tAC"), Ok(()));
    assert!(checked("/toy/Areas/Areas/Areas/0").is_err());
}

/// Each path that `paths` lists for `tree` selects the value that it lists with it.
fn each_path_selects_its_value(tree: &Value) {
    for (path, value) in inspect::paths(tree) {
        assert_eq!(select(tree, &path), [value], "{path}");
    }
}

/// An IE that a value holds alone and that has no name is its identifier in digits, in the
/// tree and in a path after the member that holds it, where no segment is a position:
/// `@id=N` selects it too, with a tree and without one.
#[test]
fn an_ie_held_alone_that_has_no_name_goes_by_its_identifier_in_digits() {
    let written = tree!({"message": "SetupFailure", "ies": [
        {"Cause": {"choice-Extensions": {"60000": {"octets": "c0ffee"}, "criticality": "ignore"}}},
    ]});
    let tree = sent(&written).unwrap();
    assert_eq!(tree, written);
    let held = "/toy/Cause/choice-Extensions";
    for path in [
        format!("{held}/60000/octets"),
        format!("{held}/@id=60000/octets"),
    ] {
        assert_eq!(select(&tree, &path), [tree!("c0ffee")], "{path}");
        assert_eq!(inspect::check_path("SetupFailure", &path), Ok(()), "{path}");
    }
    assert_eq!(
        select(&tree, &format!("{held}/criticality")),
        [tree!("ignore")]
    );
    each_path_selects_its_value(&tree);
    let mut edited = tree.clone();
    let octets = tree!({"octets": "00"});
    inspect::set(&mut edited, &format!("{held}/60000"), octets.clone()).unwrap();
    assert_eq!(select(&edited, &format!("{held}/60000")), [octets]);
    assert_eq!(sent(&edited).unwrap(), edited);
}

/// An entry of a list of IEs, or of a container in the place of the value of an IE, that
/// holds the IE whose value the list or the container is goes by its position: the name of
/// that IE after it stays on it, and selects that IE and its criticality, not the entry.
/// So go an Items in the Items of a SETUP REQUEST, as its octets or as a value, and a
/// SetupTransfer in the SetupTransfer of a SETUP RESPONSE.
#[test]
fn an_entry_that_holds_the_ie_whose_value_its_list_is_goes_by_its_position() {
    let request = |entry: Value| {
        tree!({"message": "SetupRequest", "ies": [
            {"Areas": areas()},
            {"Items": [entry, {"Item": {"item-ID": 1}}]},
        ]})
    };
    let octets = request(tree!({"Items": {"octets": "ff"}, "criticality": "ignore"}));
    let items = tree!([{"Item": {"item-ID": 2}}]);
    let value = request(tree!({"Items": items, "criticality": "ignore"}));
    let transfer = tree!({"message": "SetupResponse", "ies": [
        {"SetupTransfer": {"protocolIEs": [
            {"SetupTransfer": {"octets": "ff"}, "criticality": "ignore"},
            {"Tunnel": tunnel()},
        ]}},
    ]});
    let ff = ("0/octets", tree!("ff"));
    let ignore = ("0/criticality", tree!("ignore"));
    let item = ("0/Item/item-ID", tree!(2));
    for (written, list, entry) in [
        (octets, "/toy/Items", [ff.clone(), ignore.clone()]),
        (value, "/toy/Items", [item, ignore.clone()]),
        (transfer, "/toy/SetupTransfer", [ff, ignore]),
    ] {
        let tree = sent(&written).unwrap();
        assert_eq!(tree, written);
        let paths = inspect::paths(&tree);
        for (path, value) in entry {
            let listed = (format!("{list}/{path}"), value);
            assert!(paths.contains(&listed), "{listed:?} in {paths:?}");
        }
        each_path_selects_its_value(&tree);
        // After the IE, its name stays on it: the criticality is that of the IE, which
        // its message assigns, and the entries are those of its list.
        let name = &list["/toy/".len()..];
        assert!(select(&tree, &format!("{list}/{name}/criticality")).is_empty());
        let first = select(&tree, &format!("{list}/0"));
        assert_eq!(select(&tree, &format!("{list}/{name}/0")), first);
    }
}
