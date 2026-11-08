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

//! The paths into the trees of an application protocol, on the tables of a protocol of
//! a few types that stands for those of the protocol crates: a cause, a value that holds
//! a cause, a transfer that contains one and an extension container, and a message of
//! two IEs.

use oxirush_aper::inspect::{self, Form, Kind, Member, Message, Protocol, Shape, Typed};
use oxirush_aper::{Criticality, Direction, Error, ErrorKind, ObjectSet, Presence};
use oxirush_utils::tree;
use oxirush_utils::tree::Value;

/// Cause ::= CHOICE { radioNetwork, nas, choice-Extensions }
const CAUSE: &[Member] = &[
    ("radioNetwork", Shape::Named("CauseRadioNetwork"), false),
    ("nas", Shape::Named("CauseNas"), false),
    ("misc", Shape::Null, false),
    ("choice-Extensions", Shape::Ie(None), false),
];
/// Holder ::= SEQUENCE { cause Cause OPTIONAL, transfer OCTET STRING (CONTAINING Cause)
/// OPTIONAL, iE-Extensions OPTIONAL }
const HOLDER: &[Member] = &[
    ("cause", Shape::Named("Cause"), true),
    ("transfer", Shape::Transfer("Cause"), true),
    ("tac", Shape::Named("TAC"), true),
    ("iE-Extensions", Shape::Extensions(""), true),
];
/// Release ::= SEQUENCE { protocolIEs }
const RELEASE: &[Member] = &[("protocolIEs", Shape::Ies(None), false)];

const TYPES: &[(&str, Kind)] = &[
    ("Cause", Kind::Choice(CAUSE)),
    ("Holder", Kind::Sequence(HOLDER)),
    ("Release", Kind::Sequence(RELEASE)),
];

fn no_codec(_: &[u8]) -> Result<oxirush_aper::inspect::Decoded, Error> {
    Err(Error::new(ErrorKind::Decode, "no codec in this test"))
}

fn no_encoder(_: &Value) -> Result<Vec<u8>, Error> {
    Err(Error::new(ErrorKind::Encode, "no codec in this test"))
}

fn typed() -> Typed {
    Typed {
        decode: no_codec,
        encode: no_encoder,
    }
}

const RELEASE_SET: ObjectSet = ObjectSet {
    direction: Direction::Initiating,
    procedure_code: 1,
    name: "Release",
    criticality: Criticality::Reject,
    ies: Some(&[
        (15, Criticality::Ignore, Presence::Mandatory),
        (16, Criticality::Reject, Presence::Optional),
    ]),
};

const MESSAGES: &[Message] = &[(&RELEASE_SET, typed, "crate::test::Release<'a>")];

const IE_NAMES: &[(u16, &str)] = &[(15, "Cause"), (16, "Holder"), (17, "ExtendedCause")];

fn ie_id(name: &str) -> Option<u16> {
    let mut names = IE_NAMES.iter();
    names
        .find(|(_, known)| inspect::same_name(known, name))
        .map(|(id, _)| *id)
}

fn kind_of(name: &str) -> Option<&'static Kind> {
    let at = TYPES.binary_search_by(|(known, _)| (*known).cmp(name));
    at.ok().map(|at| &TYPES[at].1)
}

static TEST: Protocol = Protocol {
    name: "TEST",
    root: "test",
    types: TYPES,
    messages: MESSAGES,
    ie_names: IE_NAMES,
    ie_types: &[
        (15, "crate::test::Cause", ""),
        (16, "crate::test::Holder", ""),
    ],
    extension_types: &[(17, "Holder", "crate::test::Cause")],
    carried: &[],
    foreign: &[],
    enumerations: &[
        ("CauseNas", &["normal-release", "deregister"]),
        ("CauseRadioNetwork", &["unspecified", "user-inactivity"]),
    ],
    item_criticalities: &[(15, Criticality::Ignore)],
    transfers: &[],
    ie_id,
    kind_of,
    ie_typed: |_| Err(Error::new(ErrorKind::UnknownIe, "no codec in this test")),
    extension_typed: |_, _| None,
    ie_contents: |_| None,
    type_form: |name| (name == "TAC").then_some(Form::Number(24)),
    transfer_fields: &["transfer"],
    transfer_typed: |_| Ok(typed()),
    carried_typed: |_| None,
    any_message: typed,
};

/// The tree of a RELEASE with a cause, and a holder whose transfer contains a cause.
fn release() -> Value {
    tree!({
        "message": "Release",
        "ies": [
            {"Cause": {"radioNetwork": "unspecified"}},
            {"Holder": {"transfer": {"radioNetwork": "user-inactivity"}}},
        ],
    })
}

#[test]
fn a_transfer_is_its_decoded_value() {
    let tree = release();
    let select = |path: &str| inspect::select(&TEST, &tree, path);
    let transfer = "/test/Holder/transfer";
    assert_eq!(
        select(&format!("{transfer}/radioNetwork")),
        Ok(vec![&tree!("user-inactivity")])
    );
    assert_eq!(select(&format!("{transfer}/nas")), Ok(vec![]));
    // A name that the type does not have is refused with its members, with a tree and
    // without one.
    let refused = select(&format!("{transfer}/misspelled")).unwrap_err();
    assert_eq!(refused.kind(), ErrorKind::Path);
    assert_eq!(
        refused.to_string(),
        "\"misspelled\" is not a member of Cause, which has radioNetwork, nas, misc, \
         choice-Extensions, at /test/Holder/transfer/misspelled"
    );
    let checked = inspect::check_path(&TEST, "Release", &format!("{transfer}/misspelled"));
    assert_eq!(checked, Err(refused));
    let names = inspect::enumerated_at(&TEST, "Release", &format!("{transfer}/nas"));
    assert_eq!(names, Ok(Some(&["normal-release", "deregister"][..])));
    // An edit is made in the decoded value, where an alternative takes the place of the
    // one there.
    let mut edited = tree.clone();
    let path = format!("{transfer}/nas");
    inspect::set(&TEST, &mut edited, &path, tree!("deregister")).unwrap();
    let holder = tree!({"Holder": {"transfer": {"nas": "deregister"}}});
    assert_eq!(edited["ies"][1], holder);
}

#[test]
fn a_value_that_is_its_octets_has_nothing_under_it() {
    let mut tree = release();
    tree["ies"][1]["Holder"]["transfer"] = tree!({"octets": "ff"});
    let path = "/test/Holder/transfer/radioNetwork";
    let error = inspect::select(&TEST, &tree, path).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::UndecodableIe);
    assert_eq!(
        error.to_string(),
        "\"radioNetwork\" is in no value here: the value is its octets, as it did not decode \
         or is written as it is sent, at /test/Holder/transfer/radioNetwork"
    );
    let octets = inspect::select(&TEST, &tree, "/test/Holder/transfer/octets");
    assert_eq!(octets, Ok(vec![&tree!("ff")]));
    // An edit changes nothing there.
    let before = tree.clone();
    let error = inspect::set(&TEST, &mut tree, path, tree!("unspecified")).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::UndecodableIe);
    assert_eq!(tree, before);
    // The value of an IE is set as its octets as any other value, and is then they.
    inspect::set(&TEST, &mut tree, "/test/Cause", tree!({"octets": "40"})).unwrap();
    assert_eq!(tree["ies"][0], tree!({"Cause": {"octets": "40"}}));
    let octets = inspect::select(&TEST, &tree, "/test/Cause/octets");
    assert_eq!(octets, Ok(vec![&tree!("40")]));
    // Its name stays on it, as on any value of an IE.
    let named = inspect::select(&TEST, &tree, "/test/0/Cause/octets");
    assert_eq!(named, Ok(vec![&tree!("40")]));
}

#[test]
fn an_edit_that_one_place_refuses_is_done_at_none() {
    // The cause takes the edit, the holder, after it, has no `radioNetwork`.
    let mut tree = release();
    let before = tree.clone();
    let everywhere = "/test/*/radioNetwork";
    assert_eq!(inspect::select(&TEST, &tree, everywhere).unwrap().len(), 1);
    let error = inspect::set(&TEST, &mut tree, everywhere, tree!("user-inactivity")).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Path);
    assert!(
        error.to_string().contains("is not a member of Holder"),
        "{error}"
    );
    assert_eq!(tree, before);
    // A CHOICE has one alternative: another takes its place, and none is taken out.
    inspect::set(&TEST, &mut tree, "/test/Cause/nas", tree!("deregister")).unwrap();
    assert_eq!(tree["ies"][0], tree!({"Cause": {"nas": "deregister"}}));
    let refused = inspect::remove(&TEST, &mut tree, "/test/Cause/nas").unwrap_err();
    assert_eq!(refused.kind(), ErrorKind::Value);
    // An IE that the message does not have is inserted, not set.
    tree["ies"].as_array_mut().unwrap().pop();
    let before = tree.clone();
    let holder = tree!({"cause": {"misc": null}});
    let refused = inspect::set(&TEST, &mut tree, "/test/Holder", holder.clone()).unwrap_err();
    assert_eq!(refused.kind(), ErrorKind::Path);
    assert!(refused.to_string().contains("at /test/-"), "{refused}");
    assert_eq!(tree, before);
    let entry = tree!({"Holder": holder});
    inspect::insert(&TEST, &mut tree, "/test/-", entry.clone()).unwrap();
    assert_eq!(tree["ies"][1], entry);
    // An IE goes out with its criticality, and a member of its value alone.
    inspect::remove(&TEST, &mut tree, "/test/Holder/cause").unwrap();
    assert_eq!(tree["ies"][1], tree!({"Holder": {}}));
    inspect::remove(&TEST, &mut tree, "/test/Cause").unwrap();
    assert_eq!(tree["ies"], tree!([{"Holder": {}}]));
}

#[test]
fn the_segment_after_the_root_selects_among_the_ies() {
    let tree = release();
    for path in ["/test/NoSuchIe", "/test/value"] {
        let error = inspect::select(&TEST, &tree, path).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::UnknownIe, "{path}");
    }
    // The root is short for the IEs of the message, which a path may say.
    for path in [
        "/test/Cause",
        "/test/0",
        "/ies/Cause",
        "/ies/0",
        "/test/@id=15",
    ] {
        let selected = inspect::select(&TEST, &tree, path);
        assert_eq!(
            selected,
            Ok(vec![&tree!({"radioNetwork": "unspecified"})]),
            "{path}"
        );
    }
    // An IE that the object set has and the tree has not selects nothing, and one that
    // the set has not is refused without a tree.
    let checked = |path: &str| inspect::check_path(&TEST, "Release", path);
    assert_eq!(checked("/test/Holder/cause/nas"), Ok(()));
    let error = checked("/test/ExtendedCause").unwrap_err();
    assert_eq!(
        error.to_string(),
        "\"ExtendedCause\" is not an IE of Release, which has Cause, Holder, at \
         /test/ExtendedCause"
    );
    let error = inspect::check_path(&TEST, "NoSuchMessage", "/test/Cause").unwrap_err();
    assert_eq!(error.kind(), ErrorKind::UnknownMessage);
}

#[test]
fn the_ies_that_a_value_holds_are_written_by_name_and_read_back() {
    // An IE of an extension container, which has the type that the type it extends
    // gives it, and whose criticality no table has: it is written beside its name.
    let mut ies = tree!([
        {"id": 15, "criticality": "ignore", "value": {"radioNetwork": "unspecified"}, "_raw_value": "00"},
        {"id": 16, "criticality": "reject", "value": {
            "transfer": {"decoded": {"radioNetwork": "user-inactivity"}, "_raw_transfer": "10"},
        }, "_raw_value": "4010"},
    ]);
    ies[1]["value"]["iE-Extensions"] = tree!([
        {"id": 17, "criticality": "ignore", "extensionValue": {"nas": "deregister"}},
    ]);
    let shown = inspect::named_ies(&TEST, "Release", ies).unwrap();
    assert_eq!(
        shown,
        tree!([
            {"Cause": {"radioNetwork": "unspecified"}},
            {"Holder": {
                "transfer": {"radioNetwork": "user-inactivity"},
                "iE-Extensions": [{"ExtendedCause": {"nas": "deregister"}, "criticality": "ignore"}],
            }},
        ])
    );
    let listed = inspect::listed_ies(&TEST, "Release", &shown).unwrap();
    assert_eq!(
        listed,
        tree!([
            {"id": 15, "criticality": "ignore", "value": {"radioNetwork": "unspecified"}},
            {"id": 16, "criticality": "reject", "value": {
                "transfer": {"radioNetwork": "user-inactivity"},
                "iE-Extensions": [
                    {"id": 17, "criticality": "ignore", "extensionValue": {"nas": "deregister"}},
                ],
            }},
        ])
    );
    // A path goes into the IEs of an extension container by their names.
    let mut tree = release();
    tree["ies"] = shown;
    let path = "/test/Holder/iE-Extensions/ExtendedCause/nas";
    assert_eq!(
        inspect::select(&TEST, &tree, path),
        Ok(vec![&tree!("deregister")])
    );
    assert_eq!(inspect::check_path(&TEST, "Release", path), Ok(()));
    // An entry of the extension container that does not say its criticality is refused.
    let unsaid = tree!([{"Holder": {"iE-Extensions": [{"ExtendedCause": {"nas": "deregister"}}]}}]);
    let error = inspect::listed_ies(&TEST, "Release", &unsaid).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Value);
    // A transfer that did not decode is its octets in a tree, and sent as they are.
    let undecoded = tree!([{"id": 16, "criticality": "reject", "value": {
        "transfer": {"_raw_transfer": "ff", "_decode_error": "why"},
    }}]);
    let shown = inspect::named_ies(&TEST, "Release", undecoded).unwrap();
    assert_eq!(shown, tree!([{"Holder": {"transfer": {"octets": "ff"}}}]));
    let listed = inspect::listed_ies(&TEST, "Release", &shown).unwrap();
    assert_eq!(
        listed,
        tree!([{"id": 16, "criticality": "reject", "value": {"transfer": {"octets": "ff"}}}])
    );
}

#[test]
fn the_paths_of_a_tree_name_the_ies_and_select_their_values() {
    let mut tree = release();
    tree["ies"][1]["criticality"] = tree!("ignore");
    let paths = inspect::paths(&TEST, &tree);
    let listed: Vec<String> = paths.iter().map(|(path, _)| path.clone()).collect();
    assert_eq!(
        listed,
        [
            "/test/Cause/radioNetwork",
            "/test/Holder/transfer/radioNetwork",
            "/test/Holder/criticality",
        ]
    );
    for (path, value) in &paths {
        assert_eq!(
            inspect::select(&TEST, &tree, path),
            Ok(vec![value]),
            "{path}"
        );
    }
    // An IE that is there twice goes by its position.
    let cause = tree["ies"][0].clone();
    tree["ies"].as_array_mut().unwrap().push(cause);
    let listed: Vec<String> = inspect::paths(&TEST, &tree)
        .into_iter()
        .map(|(path, _)| path)
        .collect();
    assert_eq!(
        listed,
        [
            "/test/0/radioNetwork",
            "/test/Holder/transfer/radioNetwork",
            "/test/Holder/criticality",
            "/test/2/radioNetwork",
        ]
    );
}

#[test]
fn the_criticality_of_an_ie_is_beside_its_value() {
    let mut tree = release();
    // A tree writes it where it is not the one that the message assigns.
    let criticality = |tree: &Value, path: &str| inspect::select(&TEST, tree, path).unwrap().len();
    assert_eq!(criticality(&tree, "/test/Cause/criticality"), 0);
    assert_eq!(
        inspect::check_path(&TEST, "Release", "/test/Cause/criticality"),
        Ok(())
    );
    inspect::set(&TEST, &mut tree, "/test/Cause/criticality", tree!("reject")).unwrap();
    assert_eq!(
        tree["ies"][0],
        tree!({"Cause": {"radioNetwork": "unspecified"}, "criticality": "reject"})
    );
    assert_eq!(
        inspect::select(&TEST, &tree, "/test/0/Cause/criticality"),
        Ok(vec![&tree!("reject")])
    );
    assert_eq!(criticality(&tree, "/test/*/criticality"), 1);
    // The value stays when the criticality goes, and the reverse.
    inspect::remove(&TEST, &mut tree, "/test/Cause/criticality").unwrap();
    assert_eq!(
        tree["ies"][0],
        tree!({"Cause": {"radioNetwork": "unspecified"}})
    );
    // Beside a value that is its octets too.
    tree["ies"][1] = tree!({"Holder": {"octets": "00"}, "criticality": "ignore"});
    assert_eq!(
        inspect::select(&TEST, &tree, "/test/Holder/criticality"),
        Ok(vec![&tree!("ignore")])
    );
}

/// The first finding of the review of the move of the paths to oxirush-utils: an
/// alternative of the type NULL was read as absent, so that a path neither listed nor
/// selected it.
#[test]
fn an_alternative_of_the_type_null_is_a_value() {
    let mut tree = release();
    tree["ies"][0]["Cause"] = tree!({"misc": null});
    assert_eq!(
        inspect::select(&TEST, &tree, "/test/Cause/misc"),
        Ok(vec![&Value::Null])
    );
    assert_eq!(
        inspect::select(&TEST, &tree, "/test/Cause/*"),
        Ok(vec![&Value::Null])
    );
    let paths = inspect::paths(&TEST, &tree);
    assert!(
        paths.contains(&("/test/Cause/misc".to_string(), Value::Null)),
        "{paths:?}"
    );
    // An alternative that is not there selects nothing, as before.
    assert_eq!(inspect::select(&TEST, &tree, "/test/Cause/nas"), Ok(vec![]));
    assert_eq!(
        inspect::check_path(&TEST, "Release", "/test/Cause/misc"),
        Ok(())
    );
    // `null` set there is that alternative, in the place of the one there.
    let mut edited = release();
    inspect::set(&TEST, &mut edited, "/test/Cause/misc", Value::Null).unwrap();
    assert_eq!(edited["ies"][0], tree!({"Cause": {"misc": null}}));
}

/// The review of the release candidate found the first finding of the review of the move
/// still true of a procedure that the tables do not have: its message is read as a
/// container of IEs, and its paths listed an alternative of the type NULL that a
/// selection did not take. Its IEs have the shapes of their identifiers there too.
#[test]
fn an_alternative_of_the_type_null_is_a_value_in_a_procedure_that_the_tables_lack() {
    let mut tree = release();
    let tree = tree.as_object_mut().unwrap();
    tree.shift_remove("message");
    tree.insert("procedure_code".into(), tree!(254));
    tree.insert("direction".into(), tree!("InitiatingMessage"));
    tree["ies"][0]["Cause"] = tree!({"misc": null});
    let tree = Value::Object(tree.clone());
    let paths = inspect::paths(&TEST, &tree);
    assert!(
        paths.contains(&("/test/Cause/misc".to_string(), Value::Null)),
        "{paths:?}"
    );
    for (path, value) in &paths {
        assert_eq!(
            inspect::select(&TEST, &tree, path),
            Ok(vec![value]),
            "{path}"
        );
    }
    assert_eq!(
        inspect::select(&TEST, &tree, "/test/Holder/transfer/radioNetwork"),
        Ok(vec![&tree!("user-inactivity")])
    );
}

/// The fourth finding of the review of the move of the paths to oxirush-utils: `open` with
/// `*` was refused at the first IE that had no member of the next name, where a selection
/// takes those that have it.
#[test]
fn open_takes_what_each_place_has_as_a_selection_does() {
    let mut tree = release();
    let before = tree.clone();
    // The Cause has no transfer, the Holder has one: nothing to open in either.
    assert_eq!(inspect::open(&TEST, &mut tree, "/test/*/transfer"), Ok(()));
    assert_eq!(tree, before);
    // A name that no place has is still refused.
    let error = inspect::open(&TEST, &mut tree, "/test/*/misspelled").unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Path);
}

/// The second finding of the review of the move of the paths to oxirush-utils: an IE or a
/// transfer whose typed value has no place for what it was received with is its octets in a
/// tree, which gives them back.
#[test]
fn what_a_typed_value_has_no_place_for_keeps_its_octets_in_a_tree() {
    let ies = tree!([
        {"id": 15, "criticality": "ignore", "value": {"radioNetwork": "unspecified"}, "_raw_value": "00", "_lossy": true},
        {"id": 16, "criticality": "reject", "value": {
            "transfer": {"decoded": {"radioNetwork": "user-inactivity"}, "_raw_transfer": "10", "_lossy": true},
        }, "_raw_value": "4010"},
    ]);
    let shown = inspect::named_ies(&TEST, "Release", ies);
    assert_eq!(
        shown.unwrap(),
        tree!([{"Cause": {"octets": "00"}}, {"Holder": {"transfer": {"octets": "10"}}}])
    );
}

/// The sixth finding of the review of the move of the paths to oxirush-utils: a value of a
/// well-known type has the readable form of the trees alone too, and the tests compare the
/// form of the JSON Encoding Rules, which `jer` gives back.
#[test]
fn the_form_of_the_json_encoding_rules_is_given_by_its_own_name() {
    let holder = tree!({"cause": {"nas": "deregister"}, "tac": 1, "transfer": "00"});
    assert_eq!(
        TEST.jer(holder, "Holder"),
        tree!({"cause": {"nas": "deregister"}, "tac": "000001", "transfer": "00"})
    );
    assert_eq!(TEST.jer(tree!(258), "TAC"), tree!("000102"));
    // What is in that form already, and what is no value of it, stays as it is.
    assert_eq!(TEST.jer(tree!("000102"), "TAC"), tree!("000102"));
    assert_eq!(
        TEST.jer(tree!({"misc": null}), "Cause"),
        tree!({"misc": null})
    );
}
