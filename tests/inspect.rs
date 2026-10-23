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

//! The paths into the views of an application protocol, on the tables of a protocol of
//! a few types that stands for those of the protocol crates: a cause, a value that holds
//! a cause, a transfer that contains one and an extension container, and a message of
//! two IEs.

use oxirush_aper::inspect::{self, Form, Kind, Member, Message, Protocol, Shape, Typed};
use oxirush_aper::{Error, ErrorKind};
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

fn no_codec(_: &[u8]) -> Result<(Value, bool), Error> {
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

const MESSAGES: &[Message] = &[(
    "InitiatingMessage",
    1,
    "Release",
    typed,
    &[(15, true), (16, false)],
    "crate::test::Release<'a>",
)];

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
    enumerations: &[
        ("CauseNas", &["normal-release", "deregister"]),
        ("CauseRadioNetwork", &["unspecified", "user-inactivity"]),
    ],
    item_criticalities: &[(15, "ignore")],
    message_ie_criticalities: &[("Release", &[(15, "ignore"), (16, "reject")])],
    transfers: &[],
    ie_id,
    kind_of,
    procedure_criticalities: &[(1, "reject")],
    ie_typed: |_| Err(Error::new(ErrorKind::UnknownIe, "no codec in this test")),
    extension_typed: |_, _| None,
    ie_contents: |_| None,
    type_form: |name| (name == "TAC").then_some(Form::Number(24)),
    transfer_fields: &["transfer"],
    transfer_typed: |_| Ok(typed()),
    carried_typed: |_| None,
    any_message: typed,
};

/// The view of a RELEASE with a cause, and a holder whose transfer contains a cause.
fn release() -> Value {
    tree!({
        "procedure_code": 1,
        "direction": "InitiatingMessage",
        "criticality": "reject",
        "message": {"protocolIEs": [
            {"id": 15, "criticality": "ignore", "value": {"radioNetwork": "unspecified"}, "_raw_value": "00"},
            {"id": 16, "criticality": "reject", "value": {
                "transfer": {"decoded": {"radioNetwork": "user-inactivity"}, "_raw_transfer": "10"},
            }, "_raw_value": "4010"},
        ]},
    })
}

#[test]
fn a_segment_that_names_no_member_of_a_transfer_is_taken_in_its_decoded_value() {
    let view = release();
    let select = |path: &str| inspect::select(&TEST, &view, path);
    let transfer = "/test/Holder/transfer";
    assert_eq!(
        select(&format!("{transfer}/radioNetwork")),
        Ok(vec![&tree!("user-inactivity")])
    );
    assert_eq!(
        select(&format!("{transfer}/decoded/radioNetwork")),
        Ok(vec![&tree!("user-inactivity")])
    );
    assert_eq!(select(&format!("{transfer}/nas")), Ok(vec![]));
    // What the transfer has itself comes first.
    assert_eq!(
        select(&format!("{transfer}/octets")),
        Ok(vec![&tree!("10")])
    );
    // A name that neither has is refused with the members of both, with a view and
    // without one.
    let refused = select(&format!("{transfer}/misspelled")).unwrap_err();
    assert_eq!(refused.kind(), ErrorKind::NoSuchMember);
    assert_eq!(
        refused.to_string(),
        "\"misspelled\" is not a member of Cause, which has radioNetwork, nas, misc, \
         choice-Extensions; a transfer itself has decoded, octets, at \
         /test/Holder/transfer/misspelled"
    );
    let checked = inspect::check_path(&TEST, "Release", &format!("{transfer}/misspelled"));
    assert_eq!(checked, Err(refused));
    let names = inspect::enumerated_at(&TEST, "Release", &format!("{transfer}/nas"));
    assert_eq!(names, Ok(Some(&["normal-release", "deregister"][..])));
    // An edit is made in the decoded value, whose octets are then others.
    let mut edited = view.clone();
    let path = format!("{transfer}/nas");
    inspect::set(&TEST, &mut edited, &path, tree!("deregister")).unwrap();
    let transfer = &edited["message"]["protocolIEs"][1]["value"]["transfer"];
    assert_eq!(transfer["decoded"], tree!({"nas": "deregister"}));
    assert_eq!(transfer[inspect::EDITED], tree!(true));
    assert_eq!(
        edited["message"]["protocolIEs"][1][inspect::EDITED],
        tree!(true)
    );
}

#[test]
fn a_transfer_that_did_not_decode_or_that_is_its_octets_has_no_decoded_value() {
    let mut view = release();
    let holder = &mut view["message"]["protocolIEs"][1]["value"];
    holder["transfer"] = tree!({"_raw_transfer": "ff", "_decode_error": "why"});
    let path = "/test/Holder/transfer/radioNetwork";
    let error = inspect::select(&TEST, &view, path).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::UndecodableIe);
    assert_eq!(
        error.to_string(),
        "the transfer did not decode: why, at /test/Holder/transfer/radioNetwork"
    );
    // An edit changes nothing there.
    let before = view.clone();
    let error = inspect::set(&TEST, &mut view, path, tree!("unspecified")).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::NothingSelected);
    assert_eq!(view, before);
    // A transfer that an edit replaced by its octets: what its type has selects nothing.
    view["message"]["protocolIEs"][1]["value"]["transfer"] = tree!("00");
    assert_eq!(inspect::select(&TEST, &view, path), Ok(vec![]));
    let misspelled = inspect::select(&TEST, &view, "/test/Holder/transfer/misspelled");
    assert_eq!(misspelled.unwrap_err().kind(), ErrorKind::NoSuchMember);
}

#[test]
fn an_edit_that_one_place_refuses_is_done_at_none() {
    // The cause takes the edit, the holder, after it, has no `radioNetwork`.
    let mut view = release();
    let before = view.clone();
    let everywhere = "/test/*/value/radioNetwork";
    assert_eq!(inspect::select(&TEST, &view, everywhere).unwrap().len(), 1);
    let error = inspect::set(&TEST, &mut view, everywhere, tree!("user-inactivity")).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::NoSuchMember);
    assert!(
        error.to_string().contains("is not a member of Holder"),
        "{error}"
    );
    assert_eq!(view, before);
    // A CHOICE has one alternative: another takes its place, and none is taken out.
    inspect::set(&TEST, &mut view, "/test/Cause/nas", tree!("deregister")).unwrap();
    let cause = &view["message"]["protocolIEs"][0];
    assert_eq!(cause["value"], tree!({"nas": "deregister"}));
    let refused = inspect::remove(&TEST, &mut view, "/test/Cause/nas").unwrap_err();
    assert_eq!(refused.kind(), ErrorKind::Edit);
    // The value of an IE is not taken out, and its octets are set in its place.
    let refused = inspect::remove(&TEST, &mut view, "/test/Cause/value").unwrap_err();
    assert_eq!(refused.kind(), ErrorKind::Edit);
    inspect::set(&TEST, &mut view, "/test/Cause/octets", tree!("40")).unwrap();
    let cause = &view["message"]["protocolIEs"][0];
    assert_eq!(
        cause,
        &tree!({"id": 15, "criticality": "ignore", "_raw_value": "40"})
    );
    let refused = inspect::set(&TEST, &mut view, "/test/Cause/octets", tree!(64)).unwrap_err();
    assert_eq!(refused.kind(), ErrorKind::Edit);
}

#[test]
fn the_segment_after_the_root_selects_among_the_ies() {
    let view = release();
    for path in ["/test/NoSuchIe/value", "/test/value"] {
        let error = inspect::select(&TEST, &view, path).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::UnknownIe, "{path}");
    }
    // Without the root, the list has positions.
    let error = inspect::select(&TEST, &view, "/message/protocolIEs/Cause").unwrap_err();
    assert_eq!(error.kind(), ErrorKind::NoSuchMember);
    assert_eq!(
        inspect::select(&TEST, &view, "/message/protocolIEs/0/id"),
        Ok(vec![&tree!(15)])
    );
    // An IE that the object set has and the view has not selects nothing, and one that
    // the set has not is refused without a view.
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
    let mut view = release();
    let holder = &mut view["message"]["protocolIEs"][1]["value"];
    holder["iE-Extensions"] = tree!([
        {"id": 17, "criticality": "ignore", "extensionValue": {"nas": "deregister"}},
    ]);
    let ies = view["message"]["protocolIEs"].clone();
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
    // An entry of the extension container that does not say its criticality is refused.
    let unsaid = tree!([{"Holder": {"iE-Extensions": [{"ExtendedCause": {"nas": "deregister"}}]}}]);
    let error = inspect::listed_ies(&TEST, "Release", &unsaid).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Form);
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
fn the_paths_of_a_view_name_the_ies_and_select_their_values() {
    let view = release();
    let paths = inspect::paths(&TEST, &view);
    let listed: Vec<String> = paths.iter().map(|(path, _)| path.clone()).collect();
    assert_eq!(
        listed,
        [
            "/procedure_code",
            "/direction",
            "/criticality",
            "/test/Cause/id",
            "/test/Cause/criticality",
            "/test/Cause/value/radioNetwork",
            "/test/Holder/id",
            "/test/Holder/criticality",
            "/test/Holder/value/transfer/decoded/radioNetwork",
        ]
    );
    for (path, value) in &paths {
        assert_eq!(
            inspect::select(&TEST, &view, path),
            Ok(vec![value]),
            "{path}"
        );
    }
}

/// The first finding of the review of the move of the paths to oxirush-utils: an
/// alternative of the type NULL was read as absent, so that a path neither listed nor
/// selected it.
#[test]
fn an_alternative_of_the_type_null_is_a_value() {
    let mut view = release();
    view["message"]["protocolIEs"][0]["value"] = tree!({"misc": null});
    assert_eq!(
        inspect::select(&TEST, &view, "/test/Cause/misc"),
        Ok(vec![&Value::Null])
    );
    assert_eq!(
        inspect::select(&TEST, &view, "/test/Cause/value/*"),
        Ok(vec![&Value::Null])
    );
    let paths = inspect::paths(&TEST, &view);
    assert!(
        paths.contains(&("/test/Cause/value/misc".to_string(), Value::Null)),
        "{paths:?}"
    );
    // An alternative that is not there selects nothing, as before.
    assert_eq!(inspect::select(&TEST, &view, "/test/Cause/nas"), Ok(vec![]));
    assert_eq!(
        inspect::check_path(&TEST, "Release", "/test/Cause/misc"),
        Ok(())
    );
    // `null` set there is that alternative, in the place of the one there.
    let mut edited = release();
    inspect::set(&TEST, &mut edited, "/test/Cause/misc", Value::Null).unwrap();
    assert_eq!(
        edited["message"]["protocolIEs"][0]["value"],
        tree!({"misc": null})
    );
    // And a name of a tree, which shows it as a view does.
    let shown = inspect::named_ies(&TEST, "Release", view["message"]["protocolIEs"].clone());
    assert_eq!(shown.unwrap()[0], tree!({"Cause": {"misc": null}}));
}

/// The review of the release candidate found the first finding of the review of the move
/// still true of a procedure that the tables do not have: its message is read as a
/// container of IEs, and its paths listed an alternative of the type NULL that a
/// selection did not take. Its IEs have the shapes of their identifiers there too.
#[test]
fn an_alternative_of_the_type_null_is_a_value_in_a_procedure_that_the_tables_lack() {
    let mut view = release();
    view["procedure_code"] = tree!(254);
    view["message"]["protocolIEs"][0]["value"] = tree!({"misc": null});
    let paths = inspect::paths(&TEST, &view);
    assert!(
        paths.contains(&("/test/Cause/value/misc".to_string(), Value::Null)),
        "{paths:?}"
    );
    for (path, value) in &paths {
        assert_eq!(
            inspect::select(&TEST, &view, path),
            Ok(vec![value]),
            "{path}"
        );
    }
    assert_eq!(
        inspect::select(&TEST, &view, "/test/Cause/misc"),
        Ok(vec![&Value::Null])
    );
    assert_eq!(
        inspect::select(&TEST, &view, "/test/Holder/value/transfer/radioNetwork"),
        Ok(vec![&tree!("user-inactivity")])
    );
}

/// The fourth finding of the review of the move of the paths to oxirush-utils: `open` with
/// `*` was refused at the first IE that had no member of the next name, where a selection
/// takes those that have it.
#[test]
fn open_takes_what_each_place_has_as_a_selection_does() {
    let mut view = release();
    let before = view.clone();
    // The Cause has no transfer, the Holder has one: nothing to open in either.
    assert_eq!(
        inspect::open(&TEST, &mut view, "/test/*/value/transfer"),
        Ok(())
    );
    assert_eq!(view, before);
    assert_eq!(inspect::open(&TEST, &mut view, "/test/*/transfer"), Ok(()));
    assert_eq!(view, before);
    // A name that no place has is still refused.
    let error = inspect::open(&TEST, &mut view, "/test/*/value/misspelled").unwrap_err();
    assert_eq!(error.kind(), ErrorKind::NoSuchMember);
}

/// The second finding of the review of the move of the paths to oxirush-utils: an IE or a
/// transfer whose typed value has no place for what it was received with is its octets in a
/// tree, which gives them back.
#[test]
fn what_a_typed_value_has_no_place_for_keeps_its_octets_in_a_tree() {
    let mut view = release();
    view["message"]["protocolIEs"][0]["_lossy"] = tree!(true);
    let transfer = &mut view["message"]["protocolIEs"][1]["value"]["transfer"];
    transfer["_lossy"] = tree!(true);
    let shown = inspect::named_ies(&TEST, "Release", view["message"]["protocolIEs"].clone());
    assert_eq!(
        shown.unwrap(),
        tree!([{"Cause": {"octets": "00"}}, {"Holder": {"transfer": {"octets": "10"}}}])
    );
    // A view keeps the typed value beside it, which a path selects.
    assert_eq!(
        inspect::select(&TEST, &view, "/test/Cause/radioNetwork"),
        Ok(vec![&tree!("unspecified")])
    );
    assert!(
        inspect::paths(&TEST, &view)
            .iter()
            .all(|(path, _)| !path.contains("lossy"))
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
