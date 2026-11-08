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

//! The PDUs of `tests/fixtures/`, which the fuzz targets of `fuzz/` start from, as those
//! of oxirush-ngap and oxirush-s1ap start from theirs. Each is written here, from its tree
//! or as octets, and the files are compared with what is written: each PDU decodes and
//! encodes back to its octets. A change of what a tree writes shows in the files: write
//! them again and read what changed.
//!
//! ```sh
//! FIXTURES=write cargo test -p oxirush-toy --all-features --test fixtures
//! ```

#![cfg(feature = "inspect")]

use oxirush_toy::toy::TOYPDU;
use oxirush_toy::tree;
use oxirush_toy::tree::Value;
use std::fmt::Write;
use std::path::Path;

/// One message of each procedure, with its mandatory IEs and nothing else.
fn messages() -> Vec<(&'static str, Value)> {
    vec![
        (
            "SetupRequest",
            tree!({"message": "SetupRequest", "ies": [
                {"Areas": [{"tAC": 1, "broadcastPLMNs": ["208-93"]}]},
            ]}),
        ),
        (
            "SetupResponse",
            tree!({"message": "SetupResponse", "ies": []}),
        ),
        (
            "SetupFailure",
            tree!({"message": "SetupFailure", "ies": [{"Cause": {"misc": "unspecified"}}]}),
        ),
        (
            "ReleaseRequest",
            tree!({"message": "ReleaseRequest", "ies": [
                {"UE-Core-ID": 1},
                {"UE-RAN-ID": 2},
                {"Cause": {"radioNetwork": "user-inactivity"}},
            ]}),
        ),
        (
            "ErrorIndication",
            tree!({"message": "ErrorIndication", "ies": []}),
        ),
        (
            "Notification",
            tree!({"message": "Notification", "ies": [{"UE-IDs": {"uE-Core-ID": 3}}]}),
        ),
    ]
}

/// Messages with content: every IE of each object set, the alternatives of the CHOICE
/// types, extensions, transfers and an IE of no object set.
fn vectors() -> Vec<(&'static str, Value)> {
    let tunnel = tree!({"gTPTunnel": {"transportLayerAddress": "10.0.0.1", "gTP-TEID": 9}});
    let key = "00".repeat(32);
    vec![
        (
            "SetupRequest/every-ie",
            tree!({"message": "SetupRequest", "ies": [
                {"Areas": [
                    {"tAC": 1, "broadcastPLMNs": ["208-93", "001-01"]},
                    {"tAC": 16777214, "broadcastPLMNs": ["310-410"]},
                ]},
                {"NodeName": "node-1"},
                {"NodeID": {"macro-NodeID": "123450"}},
                {"Items": [
                    {"Item": {"item-ID": 1, "cause": {"nas": "deregister"}}},
                    {"Item": {"item-ID": 2, "iE-Extensions": [{"ExtensionIndex": 8, "criticality": "ignore"}]}},
                ]},
                {"TransferList": [{
                    "transfer-ID": 1,
                    "setupTransfer": {"protocolIEs": [{"Tunnel": &tunnel}, {"TransferKind": "ipv4v6"}]},
                    "failureTransfer": {"cause": {"misc": "unspecified"}},
                    "iE-Extensions": [{"ExtensionIndex": 2, "criticality": "ignore"}],
                }]},
                {"Container": {"ContainerBlob": {
                    "tAC": 1,
                    "nodeName": "node-1",
                    "message": "7e0041",
                    "kinds": {"first": "ipv4", "second": "ipv6"},
                    "mixed": {"kind": "ipv4", "cause": "deregister"},
                }}},
                {"SecurityKey": &key},
                {"Algorithms": {"value": "c00000", "length": 20}},
                {"Counts": {"countUL": 18446744073709551615u64, "counts": 255, "countDL": 4000000000000u64}},
                {"Tunnel": {"unassigned": null}},
            ]}),
        ),
        (
            "SetupRequest/node-ids",
            tree!({"message": "SetupRequest", "ies": [
                {"Areas": [{"tAC": 1, "broadcastPLMNs": ["208-93"]}]},
                {"NodeID": {"home-NodeID": "12345670"}},
                {"Tunnel": &tunnel},
            ]}),
        ),
        (
            "SetupResponse/every-ie",
            tree!({"message": "SetupResponse", "ies": [
                {"NodeName": "node-2"},
                {"SecurityContext": {"nextHopChainingCount": 7, "nextHopNH": &key}},
                {"SetupTransfer": {"protocolIEs": [{"Tunnel": {"unassigned": null}}]}},
                {"CriticalityDiagnostics": {
                    "procedureCode": 1,
                    "triggeringMessage": "initiating-message",
                    "procedureCriticality": "reject",
                    "iEsCriticalityDiagnostics": [
                        {"iECriticality": "ignore", "iE-ID": 20, "typeOfError": "not-understood"},
                    ],
                }},
            ]}),
        ),
        (
            "SetupFailure/every-ie",
            tree!({"message": "SetupFailure", "ies": [
                {"Cause": {"choice-Extensions": {"ExtendedCause": "congestion", "criticality": "reject"}}},
                {"FailureTransfer": {"cause": {"nas": "deregister"}}},
                {"CriticalityDiagnostics": {"procedureCode": 1, "iEsCriticalityDiagnostics": [
                    {"iECriticality": "reject", "iE-ID": 26, "typeOfError": "missing"},
                ]}},
            ]}),
        ),
        (
            "ReleaseRequest/every-ie",
            tree!({"message": "ReleaseRequest", "ies": [
                {"UE-Core-ID": 1099511627775u64},
                {"UE-RAN-ID": 4294967295u64, "criticality": "ignore"},
                {"Cause": {"protocol": "semantic-error"}, "criticality": "reject"},
                {"NAS-PDU": "7e004501000bf202f839010040c0000001"},
                {"60000": {"octets": "c0ffee"}, "criticality": "ignore"},
            ]}),
        ),
        (
            "ErrorIndication/every-ie",
            tree!({"message": "ErrorIndication", "ies": [
                {"UE-Core-ID": 4},
                {"UE-RAN-ID": 5},
                {"Cause": {"nas": "normal-release"}},
                {"CriticalityDiagnostics": {"procedureCode": 2, "triggeringMessage": "successful-outcome"}},
            ]}),
        ),
        (
            "Notification/every-ie",
            tree!({"message": "Notification", "ies": [
                {"UE-IDs": {"uE-ID-pair": {"uE-Core-ID": 1, "uE-RAN-ID": 2}}},
                {"Cause": {"misc": "control-processing-overload"}},
            ]}),
        ),
        (
            "ReleaseRequest/unknown-procedure",
            tree!({"procedure_code": 200, "direction": "InitiatingMessage",
            "criticality": "ignore", "ies": [{"UE-RAN-ID": 7, "criticality": "reject"}]}),
        ),
    ]
}

/// PDUs as octets: a cause outside the values of its type, and a container of no IE.
const OCTETS: &[(&str, &str)] = &[
    (
        "ReleaseRequest/spoiled-cause",
        "00024014000003000a0002002a000b00020007000f4001e4",
    ),
    ("ReleaseRequest/no-ie", "0002400100"),
];

/// A message whose type has, after the extension marker, additions of a later version
/// (X.691 §19.7 to §19.9): the octets with them, and without.
fn later() -> Vec<(&'static str, Vec<u8>, Vec<u8>)> {
    let tree = tree!({"message": "ReleaseRequest", "ies": [
        {"UE-Core-ID": 42},
        {"UE-RAN-ID": 7},
        {"Cause": {"nas": "deregister"}},
    ]});
    let pdu = TOYPDU::from_tree(&tree).expect("a tree of TOY");
    let without = pdu.encode().expect("a PDU encodes");
    let TOYPDU::initiatingMessage(mut held) = pdu else {
        panic!("an initiating message");
    };
    let mut message = held.value.to_vec();
    message[0] |= 0x80;
    message.extend([0x01, 0x01, 0xaa]);
    held.value = message.into();
    let with = TOYPDU::initiatingMessage(held)
        .encode()
        .expect("a PDU encodes");
    vec![("ReleaseRequest/later", with, without)]
}

fn hex(octets: &[u8]) -> String {
    octets.iter().map(|octet| format!("{octet:02x}")).collect()
}

/// The octets of a PDU, which decode and encode back to themselves.
fn written(name: &str, tree: &Value) -> Vec<u8> {
    let pdu = TOYPDU::from_tree(tree).unwrap_or_else(|error| panic!("{name}: {error}"));
    let octets = pdu
        .encode()
        .unwrap_or_else(|error| panic!("{name}: {error}"));
    read_back(name, &octets);
    octets
}

fn read_back(name: &str, octets: &[u8]) {
    let pdu = TOYPDU::decode(octets).unwrap_or_else(|error| panic!("{name}: {error}"));
    assert_eq!(pdu.encode().as_deref(), Ok(octets), "{name}");
}

/// A row: the name, the octets, the procedure code and the alternative of the PDU.
fn row(text: &mut String, name: &str, octets: &[u8]) {
    write!(
        text,
        "{name}\t{}\t{}\t{}",
        hex(octets),
        octets[1],
        octets[0] >> 5
    )
    .unwrap();
}

#[test]
fn the_fixtures_are_the_pdus_written_here() {
    let header = "# TOY, the protocol of the tests of oxirush-aper, whose fixtures tests/fixtures.rs\n\
                  # writes: FIXTURES=write cargo test -p oxirush-toy --all-features --test fixtures\n";
    let mut messages_tsv = String::from(header);
    messages_tsv += "# One message of each procedure, with its mandatory IEs and nothing else.\n\
                     # Message\twire bytes\tprocedure code\tPDU alternative\n";
    for (name, tree) in messages() {
        row(&mut messages_tsv, name, &written(name, &tree));
        messages_tsv.push('\n');
    }
    let mut vectors_tsv = String::from(header);
    vectors_tsv += "# Messages with content: every IE of each object set, the alternatives of the\n\
                    # CHOICE types, extensions, transfers, an IE of no object set and values that\n\
                    # the types do not have.\n\
                    # Message/variant\twire bytes\tprocedure code\tPDU alternative\n";
    for (name, tree) in vectors() {
        row(&mut vectors_tsv, name, &written(name, &tree));
        vectors_tsv.push('\n');
    }
    for (name, text) in OCTETS {
        let octets = (0..text.len())
            .step_by(2)
            .map(|at| u8::from_str_radix(&text[at..at + 2], 16).unwrap())
            .collect::<Vec<_>>();
        read_back(name, &octets);
        row(&mut vectors_tsv, name, &octets);
        vectors_tsv.push('\n');
    }
    let mut later_tsv = String::from(header);
    later_tsv += "# A message with additions of a later version after the extension marker of its\n\
                  # type, which a receiver of this version skips.\n\
                  # Message/variant\twire bytes\tprocedure code\tPDU alternative\twire bytes without the additions\n";
    for (name, with, without) in later() {
        read_back(name, &with);
        read_back(name, &without);
        row(&mut later_tsv, name, &with);
        writeln!(later_tsv, "\t{}", hex(&without)).unwrap();
    }
    let captured_tsv = format!(
        "{header}# No implementation but this one speaks TOY: no PDU is captured.\n\
         # Message/variant\twire bytes\tprocedure code\tPDU alternative\n"
    );
    let fuzzed_tsv = format!(
        "{header}# PDUs that a fuzzer found a fault with, which the tests read as they read the vectors.\n\
         # name\toctets\n"
    );

    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let files = [
        ("messages.tsv", messages_tsv),
        ("vectors.tsv", vectors_tsv),
        ("later.tsv", later_tsv),
        ("captured.tsv", captured_tsv),
    ];
    if std::env::var_os("FIXTURES").is_some_and(|mode| mode == "write") {
        std::fs::create_dir_all(&directory).unwrap();
        for (name, text) in &files {
            std::fs::write(directory.join(name), text).unwrap();
        }
        if !directory.join("fuzzed.tsv").exists() {
            std::fs::write(directory.join("fuzzed.tsv"), fuzzed_tsv).unwrap();
        }
    }
    for (name, text) in &files {
        let recorded = std::fs::read_to_string(directory.join(name)).unwrap_or_default();
        assert_eq!(
            &recorded, text,
            "tests/fixtures/{name}: FIXTURES=write writes it again"
        );
    }
    // A PDU that a fuzzer found a fault with stays as it was found
    let fuzzed = std::fs::read_to_string(directory.join("fuzzed.tsv")).unwrap();
    for line in fuzzed.lines().filter(|line| !line.starts_with('#')) {
        let (name, octets) = line.split_once('\t').expect("a name and octets");
        assert!(octets.len() % 2 == 0, "{name}");
    }
}
