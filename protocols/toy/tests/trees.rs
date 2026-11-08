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

//! The paths of the tree of a PDU of TOY, by the names of its messages and its IEs, as
//! those of every generated crate read them.

#![cfg(feature = "inspect")]

use oxirush_toy::inspect::{check_path, enumerated_at, insert, remove, select, set};
use oxirush_toy::{ErrorKind, Toy, tree};

#[test]
fn an_enumerated_value_has_the_names_of_its_type_wherever_a_path_finds_it() {
    let names = |message: &str, path: &str| enumerated_at(message, &format!("/toy/{path}"));
    let release = |path: &str| names("ReleaseRequest", path).unwrap();
    // An alternative of a CHOICE, however the path is written.
    let nas = release("Cause/nas").expect("an ENUMERATED");
    assert_eq!(nas, ["normal-release", "deregister", "unspecified"]);
    assert_eq!(release("cause/Nas"), Some(nas));
    // A CHOICE.
    assert_eq!(release("Cause"), None);
    // A member of the entries of a list, through an optional member.
    let error = names(
        "ErrorIndication",
        "CriticalityDiagnostics/iEsCriticalityDiagnostics/0/typeOfError",
    );
    assert_eq!(error.unwrap(), Some(&["not-understood", "missing"][..]));
    // What `check_path` refuses is refused.
    assert!(names("ReleaseRequest", "Cause/nsa").is_err());
    assert!(names("NoSuchMessage", "Cause/nas").is_err());
}

#[test]
fn the_name_of_an_ie_selects_its_value() {
    let value = tree!({"radioNetwork": "unspecified"});
    let entry = tree!({"Cause": &value, "criticality": "reject"});
    let mut tree = tree!({"message": "ReleaseRequest", "ies": [&entry]});
    let path = |rest: &str| format!("/toy/Cause/{rest}");
    assert_eq!(select(&tree, "/toy/Cause").unwrap(), [&value]);
    assert_eq!(
        select(&tree, &path("radioNetwork")).unwrap(),
        [&tree!("unspecified")]
    );
    assert!(select(&tree, &path("nas")).unwrap().is_empty());
    // A name that the value cannot have is refused with its members, with a tree and
    // without one.
    let refused = select(&tree, &path("misspelled")).unwrap_err();
    assert_eq!(refused.kind(), ErrorKind::Path);
    let text = refused.to_string();
    assert!(
        text.contains("is not a member of Cause, which has radioNetwork"),
        "{text}"
    );
    let checked = |rest: &str| check_path("ReleaseRequest", &path(rest));
    assert_eq!(checked("misspelled"), Err(refused.clone()));
    assert_eq!(checked("nas"), Ok(()));
    let deeper = checked("radioNetwork/deeper").unwrap_err().to_string();
    assert!(deeper.contains("which has none"), "{deeper}");
    // An edit takes the same path: the value changes, and its criticality stays.
    set(&mut tree, &path("nas"), tree!("normal-release")).unwrap();
    let edited = tree!({"Cause": {"nas": "normal-release"}, "criticality": "reject"});
    assert_eq!(tree["ies"][0], edited);
    let before = tree.clone();
    assert_eq!(set(&mut tree, &path("misspelled"), tree!(1)), Err(refused));
    let kept = remove(&mut tree, &path("nas")).unwrap_err();
    assert_eq!(kept.kind(), ErrorKind::Value);
    assert!(
        kept.to_string().contains("a CHOICE has one alternative"),
        "{kept}"
    );
    assert_eq!(tree, before);
    // An IE goes out whole, and one that the message does not have is inserted.
    remove(&mut tree, "/toy/Cause").unwrap();
    assert_eq!(tree["ies"], tree!([]));
    insert(&mut tree, "/toy/-", entry.clone()).unwrap();
    assert_eq!(tree["ies"], tree!([&entry]));
}

#[test]
fn the_name_of_the_ie_stays_after_its_position() {
    let value = tree!({"radioNetwork": "unspecified"});
    let entry = tree!({"Cause": &value});
    let mut tree = tree!({"message": "ReleaseRequest", "ies": [&entry]});
    let path = |rest: &str| format!("/toy/{rest}");
    // After its position, and after its name, which selects it already.
    for named in ["0", "0/Cause", "0/cause", "Cause/Cause", "*/Cause"] {
        assert_eq!(select(&tree, &path(named)).unwrap(), [&value], "{named}");
        let member = select(&tree, &path(&format!("{named}/radioNetwork"))).unwrap();
        assert_eq!(member, [&tree!("unspecified")], "{named}");
    }
    // The name of another IE is no member of its value.
    let other = Toy::ie_names().iter().find(|(_, name)| *name != "Cause");
    let other = path(&format!("0/{}", other.unwrap().1));
    let refused = select(&tree, &other).unwrap_err();
    assert!(
        refused.to_string().contains("is not a member of Cause"),
        "{refused}"
    );
    let checked = |rest: &str| check_path("ReleaseRequest", &path(rest));
    assert_eq!(checked("Cause/Cause/nas"), Ok(()));
    assert!(checked("Cause/Cause/misspelled").is_err());
    let names = |rest: &str| enumerated_at("ReleaseRequest", &path(rest));
    assert_eq!(names("*/Cause/nas"), names("Cause/nas"));
    assert!(names("*/Cause/nas").unwrap().is_some());
    // An edit takes the same path, and an insertion or a removal the entry.
    set(&mut tree, &path("0/Cause/nas"), tree!("normal-release")).unwrap();
    let edited = select(&tree, &path("Cause")).unwrap();
    assert_eq!(edited, [&tree!({"nas": "normal-release"})]);
    insert(&mut tree, &path("0"), entry.clone()).unwrap();
    assert_eq!(select(&tree, &path("*/Cause")).unwrap().len(), 2);
    remove(&mut tree, &path("1")).unwrap();
    assert_eq!(select(&tree, &path("Cause")).unwrap(), [&value]);
}
