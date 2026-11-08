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

//! What a generated crate costs on a message of the most IEs that a container holds:
//! 65,535 of them, of one name. Each call takes a time that grows as the number of IEs
//! does, so each is done within a bound that a call whose time grows as the square of
//! it cannot meet: the listing of the paths once took nine minutes on this message.

#![cfg(feature = "inspect")]

use oxirush_toy::inspect;
use oxirush_toy::toy::*;
use oxirush_toy::{tree, tree::Value};
use std::sync::mpsc;
use std::time::Duration;

/// The most IEs of a container: `maxProtocolIEs`.
const MOST: usize = 65_535;

/// What a call is given at most, on a build without optimisation: the calls of this
/// file take two seconds together there.
const BOUND: Duration = Duration::from_secs(60);

/// A RELEASE REQUEST of `count` IEs of the identifier `id`, each of `value`.
fn message_of(count: usize, id: u16, value: &[u8]) -> TOYPDU<'static> {
    let ie = ProtocolIEField {
        id: ProtocolIEID(id),
        criticality: Criticality::Reject,
        value: value.to_vec().into(),
    };
    let mut pdu = TOYPDU::decode(&hex::decode("0002400100").unwrap())
        .unwrap()
        .into_owned();
    pdu.set_protocol_ies(ProtocolIEContainer(vec![ie; count]))
        .unwrap();
    pdu
}

/// What `call` gives, when it gives it within the bound. The test fails at the bound,
/// without waiting for a call that is not done.
fn within<T: Send + 'static>(what: &str, call: impl FnOnce() -> T + Send + 'static) -> T {
    let (done, result) = mpsc::channel();
    std::thread::spawn(move || done.send(call()));
    match result.recv_timeout(BOUND) {
        Ok(result) => result,
        Err(_) => panic!("{what} is not done after {BOUND:?} on {MOST} IEs"),
    }
}

#[test]
fn each_call_on_the_most_ies_of_one_name_is_done_within_a_bound() {
    // UE-Core-ID, the smallest IE that has a name and a value.
    let pdu = message_of(MOST, 10, &[0x00, 0x01]);
    let octets = pdu.encode().unwrap();
    // Six octets an IE, and the lengths of the message and of its container.
    assert_eq!(octets.len(), 393_224);

    let checked = pdu.clone();
    let deviations = within("check", move || checked.check().unwrap());
    assert!(!deviations.is_empty());

    let written = pdu.clone();
    let tree = within("to_tree", move || written.to_tree());
    assert_eq!(tree["ies"].as_array().unwrap().len(), MOST);

    let listed = tree.clone();
    let paths = within("paths", move || inspect::paths(&listed));
    // An IE whose identifier is there twice goes by its position.
    let last = format!("/toy/{}", MOST - 1);
    // One value of each IE.
    assert_eq!(paths.len(), MOST);
    assert!(paths.contains(&(last.clone(), tree!(1))));

    let (selected, path) = (tree.clone(), last.clone());
    let one = within("select", move || {
        inspect::select(&selected, &path).map(|values| values.len())
    });
    assert_eq!(one.unwrap(), 1);
    // A path selects every IE of a message, which has 65,535 at most.
    let selected = tree.clone();
    let all = within("select by name", move || {
        inspect::select(&selected, "/toy/UE-Core-ID").map(|values| values.len())
    });
    assert_eq!(all.unwrap(), MOST);

    let (mut edited, path) = (tree.clone(), last);
    let edited = within("set", move || {
        inspect::set(&mut edited, &path, tree!(2)).map(|()| edited)
    })
    .unwrap();
    let same = tree.clone();
    let again = within("from_tree", move || TOYPDU::from_tree(&same).unwrap());
    assert_eq!(again.encode().unwrap(), octets);
    let other = within("from_tree of an edit", move || {
        TOYPDU::from_tree(&edited).unwrap()
    });
    let other = other.encode().unwrap();
    assert_eq!(other.len(), octets.len());
    assert_eq!((octets[octets.len() - 1], other[other.len() - 1]), (1, 2));
}

#[test]
fn the_paths_of_ies_without_a_name_are_listed_within_the_bound_too() {
    let pdu = message_of(MOST, 60_000, &[0x00]);
    let tree = within("to_tree", move || pdu.to_tree());
    let paths: Vec<(String, Value)> = within("paths", move || inspect::paths(&tree));
    assert!(paths.len() >= MOST);
}

/// `open` once took the path again from the tree for each value that it opened, ten
/// seconds for 4,096 containers. It opens those of a segment in one walk, by the name of
/// the IE and with `*`, which takes the IEs that have the name of the type and leaves
/// the others.
#[test]
fn open_on_many_containers_is_done_within_a_bound() {
    const CONTAINERS: usize = 8192;
    let container = tree!({"ContainerBlob": {"tAC": 1, "nodeName": "node-1"}});
    let mut ies = vec![tree!({"Areas": [{"tAC": 1, "broadcastPLMNs": ["208-93"]}]})];
    ies.extend((0..CONTAINERS).map(|_| tree!({"Container": &container})));
    let setup = tree!({"message": "SetupRequest", "ies": ies});
    let pdu = TOYPDU::from_tree(&setup).unwrap();
    let tree = within("to_tree", move || pdu.to_tree());
    let name = "/toy/Container/ContainerBlob/nodeName";
    for path in [name.to_string(), "/toy/*/ContainerBlob/tAC".to_string()] {
        let mut opened = tree.clone();
        let opened = within("open", move || {
            inspect::open(&mut opened, &path).unwrap();
            opened
        });
        let names = inspect::select(&opened, name).unwrap();
        assert_eq!(names, vec![&tree!("node-1"); CONTAINERS]);
    }
}
