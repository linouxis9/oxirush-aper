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

//! What each function of the paths gives on a table of trees and of paths, which
//! `tests/paths_table.txt` records: the trees of a few PDUs of TOY, each path of a list
//! that goes into the places that the other tests of the paths name one at a time, and
//! `select`, `check_path`, `enumerated_at`, `set`, `remove`, `insert` and `open` at
//! each, and `paths` of each tree. A change of what one of them gives at one of these
//! places shows in the file: write it again and read what changed.
//!
//! ```sh
//! PATHS_TABLE=write cargo test -p oxirush-toy --all-features --test paths_table
//! ```

#![cfg(feature = "inspect")]

use oxirush_toy::toy::TOYPDU;
use oxirush_toy::tree::Value;
use oxirush_toy::{Error, inspect, tree};
use std::fmt::Write;

/// What a call gave: its value, or the kind and the words of its error.
fn shown<T: std::fmt::Debug>(result: Result<T, Error>) -> String {
    match result {
        Ok(value) => format!("{value:?}"),
        Err(error) => format!("{:?}: {error}", error.kind()),
    }
}

/// The trees of the table: the name of each, its message, and the tree, which is that
/// of the PDU that it writes.
fn trees() -> Vec<(&'static str, &'static str, Value)> {
    let tunnel = tree!({"gTPTunnel": {"transportLayerAddress": "10.0.0.1", "gTP-TEID": 9}});
    let written = [
        (
            "failure",
            "SetupFailure",
            tree!({"message": "SetupFailure", "ies": [
                {"Cause": {"choice-Extensions": {"ExtendedCause": "congestion", "criticality": "reject"}}},
                {"FailureTransfer": {"cause": {"nas": "deregister"}}},
                {"CriticalityDiagnostics": {"procedureCode": 1, "iEsCriticalityDiagnostics": [
                    {"iECriticality": "reject", "iE-ID": 26, "typeOfError": "missing"},
                ]}},
            ]}),
        ),
        (
            "release",
            "ReleaseRequest",
            tree!({"message": "ReleaseRequest", "ies": [
                {"UE-Core-ID": {"octets": "0301"}},
                {"UE-RAN-ID": 7, "criticality": "ignore"},
                {"Cause": {"nas": "deregister"}, "criticality": "reject"},
                {"60000": {"octets": "c0ffee"}, "criticality": "ignore"},
            ]}),
        ),
        (
            "setup",
            "SetupRequest",
            tree!({"message": "SetupRequest", "ies": [
                {"Areas": [{"tAC": 1, "broadcastPLMNs": ["208-93", "001-01"]}]},
                {"NodeID": {"unassigned": null}},
                {"Items": [{"Item": {"item-ID": 1, "cause": {"nas": "deregister"}}}, {"Item": {"item-ID": 2}}]},
                {"TransferList": [{
                    "transfer-ID": 1,
                    "setupTransfer": {"protocolIEs": [{"Tunnel": &tunnel}, {"TransferKind": "ipv4v6"}]},
                    "failureTransfer": {"cause": {"misc": "unspecified"}},
                    "iE-Extensions": [{"ExtensionIndex": 2, "criticality": "ignore"}],
                }]},
                {"Container": {"ContainerBlob": {"tAC": 1, "nodeName": "node-1", "kinds": {"first": "ipv4", "second": "ipv6"}, "mixed": {"kind": "ipv4", "cause": "deregister"}}}},
                {"Algorithms": {"value": "c00000", "length": 20}},
                {"Tunnel": {"unassigned": null}},
            ]}),
        ),
        (
            "notification",
            "Notification",
            tree!({"message": "Notification", "ies": [
                {"UE-IDs": {"uE-ID-pair": {"uE-Core-ID": 1, "uE-RAN-ID": 2}}},
            ]}),
        ),
        (
            "unknown",
            "ReleaseRequest",
            tree!({"procedure_code": 200, "direction": "InitiatingMessage",
            "criticality": "ignore", "ies": [{"UE-RAN-ID": 7, "criticality": "reject"}]}),
        ),
    ];
    let pdu = |tree: &Value| TOYPDU::from_tree(tree).unwrap().to_tree();
    written
        .into_iter()
        .map(|(name, message, tree)| (name, message, pdu(&tree)))
        .collect()
}

/// The paths of the table.
const PATHS: &[&str] = &[
    "",
    "/toy",
    "/toy/-",
    "/toy/*",
    "/toy/0",
    "/toy/9",
    "/criticality",
    "/message",
    "/octets",
    "/toy/Cause",
    "/toy/Cause/*",
    "/toy/Cause/nas",
    "/toy/Cause/Cause",
    "/toy/Cause/Cause/Cause",
    "/toy/Cause/criticality",
    "/toy/Cause/choice-Extensions",
    "/toy/Cause/choice-Extensions/*",
    "/toy/Cause/choice-Extensions/criticality",
    "/toy/Cause/choice-Extensions/ExtendedCause",
    "/toy/Cause/choice-Extensions/@id=41",
    "/toy/Cause/choice-Extensions/Cause",
    "/toy/Cause/choice-Extensions/ExtendedCause/ExtendedCause",
    "/toy/FailureTransfer",
    "/toy/FailureTransfer/*",
    "/toy/FailureTransfer/cause/nas",
    "/toy/FailureTransfer/criticality",
    "/toy/FailureTransfer/misspelled",
    "/toy/FailureTransfer/octets",
    "/toy/FailureTransfer/FailureTransfer/cause",
    "/toy/CriticalityDiagnostics/*",
    "/toy/CriticalityDiagnostics/iEsCriticalityDiagnostics/*/iECriticality",
    "/toy/CriticalityDiagnostics/iEsCriticalityDiagnostics/0/criticality",
    "/toy/UE-Core-ID",
    "/toy/UE-Core-ID/octets",
    "/toy/UE-Core-ID/UE-Core-ID/octets",
    "/toy/UE-Core-ID/UE-Core-ID/UE-Core-ID",
    "/toy/UE-Core-ID/criticality",
    "/toy/UE-RAN-ID",
    "/toy/UE-RAN-ID/*",
    "/toy/UE-RAN-ID/criticality",
    "/toy/UE-RAN-ID/UE-RAN-ID",
    "/toy/UE-RAN-ID/UE-RAN-ID/UE-RAN-ID",
    "/toy/@id=60000",
    "/toy/@id=60000/octets",
    "/toy/@id=60000/criticality",
    "/toy/@id=60000/x",
    "/toy/NAS-PDU",
    "/toy/NAS-PDU/criticality",
    "/toy/Areas/0/criticality",
    "/toy/Areas/*/broadcastPLMNs/*",
    "/toy/Areas/0/tAC/tAC",
    "/toy/NodeID/unassigned",
    "/toy/NodeID/*",
    "/toy/NodeID/macro-NodeID",
    "/toy/Items/*",
    "/toy/Items/Item",
    "/toy/Items/Item/cause/nas",
    "/toy/Items/@id=23/item-ID",
    "/toy/Items/0/criticality",
    "/toy/Items/1/Item/item-ID",
    "/toy/Items/Cause",
    "/toy/TransferList/0/setupTransfer/*",
    "/toy/TransferList/0/setupTransfer/TransferKind",
    "/toy/TransferList/0/setupTransfer/protocolIEs/1/Tunnel",
    "/toy/TransferList/0/setupTransfer/Tunnel/criticality",
    "/toy/TransferList/0/iE-Extensions/*",
    "/toy/TransferList/0/iE-Extensions/ExtensionIndex",
    "/toy/TransferList/0/iE-Extensions/0/criticality",
    "/toy/TransferList/0/failureTransfer/cause",
    "/toy/Container/ContainerBlob/nodeName",
    "/toy/Container/ContainerBlob/*",
    "/toy/Algorithms/length",
    "/toy/Algorithms/*",
    "/toy/Tunnel/*",
    "/toy/Tunnel/unassigned",
    "/toy/UE-IDs/uE-ID-pair/*",
    "/toy/UE-IDs/*/uE-RAN-ID",
    "/toy/UE-IDs/uE-Core-ID",
    "/toy/x",
    "/toy/UE-RAN-ID/x",
    "/toy/UE-RAN-ID/criticality/x",
    "/toy/Cause/criticality/nas",
    "/toy/Cause/nas/x",
    "/toy/Algorithms/x",
    "/toy/Algorithms/length/x",
    "/criticality/x",
    "/toy/Cause/choice-Extensions/criticality/x",
    "/toy/Cause/choice-Extensions/x",
    "/toy/Cause/choice-Extensions/ExtendedCause/x",
    "/toy/FailureTransfer/cause/x",
    "/toy/TransferList/0/x",
    "/toy/TransferList/0/failureTransfer/x",
    "/toy/NodeID/x",
    "/toy/TransferList/0/setupTransfer/x",
    "/toy/TransferList/0/setupTransfer/Tunnel/x",
    "/toy/Container/ContainerBlob/x",
    "/toy/Container/ContainerBlob/kinds/*",
    "/toy/Container/ContainerBlob/mixed/*",
    "/toy/Container/ContainerBlob/kinds/second",
    "/toy/Container/ContainerBlob/kinds/second/x",
    "/toy/UE-IDs/x",
    "/toy/UE-IDs/uE-Core-ID/x",
    "/toy/Items/0/Item/x",
    "/toy/Items/0/x",
    "/toy/Items/0/Item/Item/item-ID",
    "/toy/Items/Item/Item/Item",
    "/toy/UE-RAN-ID/UE-RAN-ID/criticality",
];

#[test]
fn the_functions_of_the_paths_give_what_the_table_records() {
    let mut table = String::new();
    for (name, message, tree) in trees() {
        writeln!(table, "## {name}: {tree}").unwrap();
        for (path, value) in inspect::paths(&tree) {
            writeln!(table, "paths {path} = {value}").unwrap();
        }
        for path in PATHS {
            writeln!(table, "\n{path}").unwrap();
            writeln!(table, "  select {}", shown(inspect::select(&tree, path))).unwrap();
            writeln!(
                table,
                "  check {}",
                shown(inspect::check_path(message, path))
            )
            .unwrap();
            writeln!(
                table,
                "  enumerated {}",
                shown(inspect::enumerated_at(message, path))
            )
            .unwrap();
            for value in [
                tree!("ignore"),
                Value::Null,
                tree!({"octets": "00"}),
                tree!(1),
                tree!({"ExtendedCause": "overload"}),
            ] {
                let mut edited = tree.clone();
                let done = shown(inspect::set(&mut edited, path, value.clone()));
                let sent = TOYPDU::from_tree(&edited).map(|pdu| pdu.to_tree() == edited);
                writeln!(table, "  set {value}: {done}, sent {}", shown(sent)).unwrap();
            }
            let mut edited = tree.clone();
            let done = shown(inspect::remove(&mut edited, path));
            writeln!(table, "  remove: {done}, left {}", edited != tree).unwrap();
            let mut edited = tree.clone();
            let entry = tree!({"NodeName": "node-2", "criticality": "ignore"});
            let done = shown(inspect::insert(&mut edited, path, entry));
            writeln!(table, "  insert: {done}, grown {}", edited != tree).unwrap();
            let mut edited = tree.clone();
            let done = shown(inspect::open(&mut edited, path));
            writeln!(table, "  open: {done}, opened {}", edited != tree).unwrap();
        }
        table.push('\n');
    }
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/paths_table.txt");
    if std::env::var("PATHS_TABLE").as_deref() == Ok("write") {
        std::fs::write(path, &table).unwrap();
    }
    let recorded = std::fs::read_to_string(path).unwrap_or_default();
    assert!(
        recorded == table,
        "tests/paths_table.txt is not what the paths give: write it with PATHS_TABLE=write and          read what changed"
    );
}
