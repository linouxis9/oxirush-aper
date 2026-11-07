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

//! What the fuzz targets do with what they are given: the programs of `fuzz_targets/`
//! call these functions.
//!
//! A target does not only look for a panic. It gives the same octets to the crate and
//! to the oracle of its tests, which has the same types with the codec of rasn, another
//! implementation of aligned PER, and compares what the two make of them: the PDU, the
//! protocol IEs of its message, the message that `decode_value` reads, and the JSON
//! form of each value that a view decodes. What one reads the other reads, as a value
//! that both write as the same octets, and what one refuses the other refuses. And
//! what was read is written and read again as the same.
//!
//! The crate differs from the oracle in one thing, which the comparison of the oracle
//! names where it meets it: its bindings hold an INTEGER in 64 bits.
//!
//! oxirush-ngap and oxirush-s1ap take this file from oxirush-aper. The package of each
//! names its crate `codec`, whose module `hooks` names what is of the protocol.

use codec::hooks::{self, BorrowedPdu as Borrowed, PDU_TYPE, Pdu, ROOT};
use codec::tree::Value;
use codec::{Deviation, Error, ErrorKind, aper, inspect};
use libfuzzer_sys::arbitrary::{self, Arbitrary};
use std::sync::LazyLock;

/// A message that is a container of protocol IEs, as every message but the private one
/// is, and which both protocols have: the codec of rasn reads the protocol IEs of any of
/// them as this one.
const ANY_MESSAGE: &str = "ErrorIndication";

/// What the crate wrote of a value that it read, or why it read none, for the oracle.
fn ours(written: &Result<Vec<u8>, Error>) -> Result<&[u8], String> {
    let written = written.as_ref().map(Vec::as_slice);
    written.map_err(ToString::to_string)
}

/// The octets of the open type that holds the message of a PDU.
fn message(pdu: &Pdu) -> &[u8] {
    match pdu {
        Pdu::initiatingMessage(message) => &message.value,
        Pdu::successfulOutcome(message) => &message.value,
        Pdu::unsuccessfulOutcome(message) => &message.value,
        _ => panic!("a PDU of an alternative that the bindings do not have"),
    }
}

/// A PDU, the protocol IEs of its message and the message as its type: each as the
/// crate reads and writes it, and as the codec of rasn does.
pub fn decode(octets: &[u8]) {
    // The PDU, which both read or both refuse, and write back as the same octets.
    let read = Pdu::decode(octets);
    let written = read.as_ref().map_err(Clone::clone).and_then(Pdu::encode);
    oracle::same_octets(PDU_TYPE, octets, ours(&written));
    let pdu = match read {
        Ok(pdu) => pdu,
        // What `decode` refuses, the codec of the type refuses in the same words.
        Err(error) => {
            assert_eq!(aper::decode::<Borrowed>(octets), Err(error));
            return;
        }
    };
    let written = match written {
        Ok(written) => written,
        Err(error) => panic!("a PDU that was decoded does not encode: {error}: {pdu:?}"),
    };
    assert_eq!(Pdu::decode(&written).as_ref(), Ok(&pdu), "{written:02x?}");

    // A PDU that borrows its message from the octets is the same PDU.
    let borrowed = Borrowed::decode_borrowed(octets);
    assert_eq!(borrowed.as_ref(), Ok(&pdu), "{octets:02x?}");
    assert_eq!(borrowed.unwrap().into_owned(), pdu);
    // `decode` and `encode` take a PDU of the usual layout octet by octet: the codec of
    // the type gives the same PDU and the same octets.
    assert_eq!(aper::decode::<Borrowed>(octets).as_ref(), Ok(&pdu));
    assert_eq!(aper::encode(&pdu).as_ref(), Ok(&written), "{pdu:?}");

    // What the octets of the message have no part in.
    let _ = (pdu.to_string(), pdu.procedure_name(), pdu.kind());

    // The protocol IEs of the message, whatever the message is: rasn reads them as
    // those of a message that is a container of protocol IEs and nothing else.
    let theirs = oracle::read(ANY_MESSAGE, message(&pdu)).unwrap();
    match pdu.protocol_ies() {
        Ok(ies) => {
            // Put back, they are the message that rasn writes of them.
            let ies = ies.into_owned();
            let mut again = pdu.clone();
            again.set_protocol_ies(ies.clone()).unwrap();
            match &theirs {
                Ok(theirs) => assert_eq!(message(&again), &theirs.octets[..], "{pdu:?}"),
                Err(error) => panic!("{pdu:?} has protocol IEs here, and by rasn: {error}"),
            }
            assert_eq!(again.protocol_ies(), Ok(ies));
            let _ = pdu.check();
        }
        // The private message has no protocol IEs, and its octets are not looked at.
        Err(error) if error.kind() == ErrorKind::WrongMessage => {}
        Err(error) => assert!(theirs.is_err(), "{error}: {pdu:?}"),
    }

    // The message as its type.
    if let Some((name, written)) = hooks::message_read_and_written(&pdu) {
        oracle::same_octets(name, message(&pdu), ours(&written));
    }
}

/// `value` without what a view keeps of what was received: the members whose name
/// starts with `_`, at any depth.
fn written(value: &Value) -> Value {
    match value {
        Value::Object(members) => {
            let kept = members.iter().filter(|(name, _)| !name.starts_with('_'));
            let kept = kept.map(|(name, value)| (name.clone(), written(value)));
            Value::Object(kept.collect())
        }
        Value::Array(entries) => Value::Array(entries.iter().map(written).collect()),
        other => other.clone(),
    }
}

/// Whether a view has an IE or a transfer that was not read with its type, and is its
/// octets beside the reason.
fn undecoded(value: &Value) -> bool {
    match value {
        Value::Object(members) => {
            members.contains_key("_decode_error") || members.values().any(undecoded)
        }
        Value::Array(entries) => entries.iter().any(undecoded),
        _ => false,
    }
}

/// What a view decoded as the type that the bindings name `name` is what the oracle
/// reads in the same octets: the same JSON form, or no value for both.
fn observed(name: &'static str, octets: &[u8], ours: &Result<Value, Error>) {
    let ours = ours.as_ref().map(serde_json::Value::from);
    oracle::same_json(name, octets, ours.as_ref().map_err(ToString::to_string));
}

/// The JSON forms of a PDU: its view, which gives the PDU back and is written from its
/// values, its tree, and the paths of its view. Each value that a view shows is
/// compared with what the JSON Encoding Rules of rasn write of the same octets.
pub fn view(octets: &[u8]) {
    let Ok(pdu) = Pdu::decode(octets) else {
        return;
    };
    hooks::observe(Some(observed));
    // Every PDU has a tree, which is the PDU again: what its typed values have no place
    // for, and a message that does not decode, are octets in it.
    let tree = pdu.to_tree();
    assert_eq!(Pdu::from_tree(&tree).as_ref(), Ok(&pdu), "{tree}");
    let Ok(view) = pdu.to_view() else {
        // A message that does not decode has no view, and its tree has its octets.
        assert!(tree.get("octets").is_some(), "{tree}");
        return;
    };

    // A view that nothing changed is the PDU, octet for octet.
    assert_eq!(pdu.with_view(&view).as_ref(), Ok(&pdu), "{view}");
    assert_eq!(Pdu::from_view(&view).as_ref(), Ok(&pdu), "{view}");

    // Without the octets received, a view in which everything was read with its type
    // is written from its values, when they are values that their types write: the PDU
    // that is written has a view, which is written to the same PDU.
    if !undecoded(&view)
        && let Ok(first) = Pdu::from_view(&written(&view))
    {
        let again = match first.to_view() {
            Ok(again) => again,
            Err(error) => panic!("a PDU that a view wrote has no view: {error}: {first:?}"),
        };
        let second = Pdu::from_view(&written(&again));
        assert_eq!(second.as_ref(), Ok(&first), "{view}");
    }

    // Each path that the view has selects its value: every one, as an alternative of the
    // type NULL deep in a message is among the last that are listed.
    let paths = inspect::paths(&view);
    for (path, value) in &paths {
        assert_eq!(inspect::select(&view, path), Ok(vec![value]), "{path}");
    }

    // Where the tree has an IE as its octets the view says why, and so does its path
    // list: octets that do not decode, or that are in another form than the one of X.691,
    // have a `_decode_error`, and those that a typed value has no place for `_lossy`.
    let (Some(Value::Array(shown)), Some(Value::Array(viewed))) = (tree.get("ies"), ies_of(&view))
    else {
        return;
    };
    assert_eq!(shown.len(), viewed.len(), "{tree}");
    let mut why = 0;
    for (shown, viewed) in shown.iter().zip(viewed) {
        let said = says_why(viewed) || viewed.get("value").is_some_and(says_why);
        assert_eq!(
            as_octets(shown),
            said,
            "{shown} is shown so, and viewed as {viewed}"
        );
        why += usize::from(said);
    }
    let listed = paths.iter().filter(|(path, _)| {
        path.starts_with('/') && (path.ends_with("/_decode_error") || path.ends_with("/_lossy"))
    });
    assert!(listed.count() >= why, "{view}");
}

/// Whether the entry of a view says why it is its octets, or has no place for all of them.
fn says_why(entry: &Value) -> bool {
    entry.get("_decode_error").is_some() || entry.get("_lossy").is_some()
}

/// Whether the entry of a tree has its IE as the octets that it was received as.
fn as_octets(entry: &Value) -> bool {
    let mut members = entry.as_object().into_iter().flatten();
    let value = members
        .find(|(name, _)| *name != "criticality")
        .map(|(_, value)| value);
    value
        .and_then(Value::as_object)
        .is_some_and(|value| value.len() == 1 && value.contains_key("octets"))
}

/// The functions that take a path, on the view of a PDU: `select`, `set`, `insert`,
/// `remove`, `open`, `check_path` and `enumerated_at`, with a path and a value of the
/// octets. No path and no value makes one of them panic, and a view that one of them
/// changed is a PDU or an error.
///
/// The first octets say what is done: the function, a path of the view to start from
/// and where it is cut, then a length and that many octets to add to the path, and a
/// length and that many octets of JSON for the value. The rest is the PDU.
pub fn paths(octets: &[u8]) {
    let [function, pick, cut, ref rest @ ..] = *octets else {
        return;
    };
    let Some((extra, rest)) = counted(rest) else {
        return;
    };
    let Some((value, rest)) = counted(rest) else {
        return;
    };
    let Ok(pdu) = Pdu::decode(rest) else {
        return;
    };
    let Ok(mut view) = pdu.to_view() else {
        return;
    };

    // A path that the view has, cut short, with what the octets add to it.
    let known = inspect::paths(&view);
    let start = match known.get(usize::from(pick) % known.len().max(1)) {
        Some((path, _)) => path.as_str(),
        None => ROOT,
    };
    let start: String = start.chars().take(usize::from(cut)).collect();
    let path = format!("{start}{}", String::from_utf8_lossy(extra));
    let value = match serde_json::from_slice(value) {
        Ok(value) => value,
        Err(_) => Value::String(String::from_utf8_lossy(value).into_owned()),
    };

    let name = inspect::message_name(&pdu).unwrap_or("no message");
    let changed = match function % 7 {
        0 => inspect::select(&view, &path).is_err(),
        1 => inspect::set(&mut view, &path, value).is_ok(),
        2 => inspect::insert(&mut view, &path, value).is_ok(),
        3 => inspect::remove(&mut view, &path).is_ok(),
        4 => inspect::open(&mut view, &path).is_ok(),
        5 => inspect::check_path(name, &path).is_err(),
        _ => inspect::enumerated_at(name, &path).is_err(),
    };
    if !changed {
        return;
    }

    // What an edit left is a PDU or an error, and a PDU that it is encodes or is an
    // error: an edit may write a value that its type does not have.
    let _ = inspect::paths(&view);
    if let Ok(edited) = pdu.with_view(&view) {
        assert_eq!(Pdu::from_view(&view).as_ref(), Ok(&edited), "{path}");
        if let Ok(octets) = edited.encode() {
            assert_eq!(Pdu::decode(&octets).as_ref(), Ok(&edited), "{path}");
        }
    }
}

/// The octets that a length of one octet counts, and those after them.
fn counted(octets: &[u8]) -> Option<(&[u8], &[u8])> {
    let (length, rest) = octets.split_first()?;
    rest.split_at_checked(usize::from(*length))
}

/// Whether `view` has a member `name`, at any depth.
fn has_member(value: &Value, name: &str) -> bool {
    match value {
        Value::Object(members) => {
            members.contains_key(name) || members.values().any(|value| has_member(value, name))
        }
        Value::Array(entries) => entries.iter().any(|value| has_member(value, name)),
        _ => false,
    }
}

/// What `check` says of a PDU, against what the other ways of reading it say.
///
/// `check_structure` says what `check` says, but for the values that it does not decode:
/// no `UndecodableIe`, `OutsideRoot` or `NotCanonical`. Each deviation tells its IE,
/// its cause and the item of its Criticality Diagnostics, and the Criticality
/// Diagnostics of all of them, for an ERROR INDICATION and for a response, are values
/// that encode and decode as themselves.
///
/// A PDU of protocol IEs of which `check` finds nothing to say is written as X.691
/// writes it, octet for octet: the codec of the type of its message writes the octets of
/// its message, and,
/// when its view read every value and transfer with its type, the PDU that the values of
/// its view write alone is the PDU.
fn checked(pdu: &Pdu) {
    let (deviations, structure) = match (pdu.check(), pdu.check_structure()) {
        (Ok(deviations), Ok(structure)) => (deviations, structure),
        (Err(all), Err(structure)) => {
            assert_eq!(all, structure, "{pdu:?}");
            return;
        }
        (all, structure) => panic!("check: {all:?}, check_structure: {structure:?}, of {pdu:?}"),
    };
    let of_the_structure = |deviation: &&Deviation| {
        !matches!(
            deviation,
            Deviation::UndecodableIe { .. }
                | Deviation::OutsideRoot { .. }
                | Deviation::NotCanonical { .. }
        )
    };
    let expected: Vec<Deviation> = deviations
        .iter()
        .filter(of_the_structure)
        .cloned()
        .collect();
    assert_eq!(structure, expected, "{pdu:?}");
    for deviation in &deviations {
        let _ = (deviation.to_string(), deviation.ie(), deviation.cause());
        let _ = (deviation.type_of_error(), deviation.diagnostics_item());
    }
    for diagnostics in [
        pdu.criticality_diagnostics(&deviations),
        pdu.criticality_diagnostics_for_response(&deviations),
    ] {
        let octets = match hooks::bindings::encode_open_type(&diagnostics) {
            Ok(octets) => octets,
            Err(error) => {
                panic!("Criticality Diagnostics that do not encode: {error}: {diagnostics:?}")
            }
        };
        let read =
            hooks::bindings::decode_open_type::<hooks::bindings::CriticalityDiagnostics>(&octets);
        assert_eq!(read.as_ref(), Ok(&diagnostics), "{octets:02x?}");
    }

    // The private message has no protocol IEs, and its octets are not looked at
    if !deviations.is_empty() || pdu.protocol_ies().is_err() {
        return;
    }
    if let Some((name, written)) = hooks::message_read_and_written(pdu) {
        match written {
            Ok(written) => assert_eq!(written, message(pdu), "{name} of {pdu:?}"),
            Err(error) => panic!("check finds nothing, and {name} does not decode: {error}"),
        }
    }
    let Ok(view) = pdu.to_view() else {
        return;
    };
    if !has_member(&view, "_decode_error") && !has_member(&view, "_lossy") {
        let alone = Pdu::from_view(&written(&view));
        assert_eq!(alone.as_ref(), Ok(pdu), "{view}");
    }
}

/// The IEs of the message of a view.
fn ies_of(view: &Value) -> Option<&Value> {
    view.get("message")?.get("protocolIEs")
}

/// The octets of a PDU, what `check` says of it and how the crate writes it again.
///
/// A PDU that decodes encodes to the octets that it was decoded from, unless its reader
/// notes a form that X.691 does not give around its message, and only then; the
/// protocol IEs of its message, put back, are the octets of its message unless `check`
/// finds such a form around their values, or does not know its procedure. What `check` says agrees with the other
/// readings ([`checked`]).
pub fn exact(octets: &[u8]) {
    let Ok(pdu) = Pdu::decode(octets) else {
        return;
    };
    let written = match pdu.encode() {
        Ok(written) => written,
        Err(error) => panic!("a PDU that was decoded does not encode: {error}: {pdu:?}"),
    };
    // The PDU keeps its message as its octets, and writes again what is around it: the
    // same octets where the reader of the PDU notes no form that X.691 does not give
    let faults = match aper::decode_with_faults::<Borrowed>(octets) {
        Ok((_, faults)) => faults,
        Err(error) => panic!("decode reads what decode_with_faults refuses: {error}"),
    };
    assert_eq!(
        written == octets,
        faults.is_empty(),
        "{faults:?} of {pdu:?}"
    );
    let deviations = pdu.check();
    // `check` looks at the container of the message of a procedure that it knows
    let around = deviations.as_ref().is_ok_and(|deviations| {
        deviations.iter().any(|deviation| {
            matches!(
                deviation,
                Deviation::NotCanonical { ie: None, .. } | Deviation::UnknownProcedure { .. }
            )
        })
    });
    if let Ok(ies) = pdu.protocol_ies() {
        let mut again = pdu.clone();
        again.set_protocol_ies(ies.into_owned()).unwrap();
        if deviations.is_ok() && !around {
            assert_eq!(message(&again), message(&pdu), "{pdu:?}");
        }
    }
    checked(&pdu);
}

/// The PDUs of the fixtures of the protocol, which the targets that build their input
/// with `arbitrary` start from, and what their trees and views hold.
struct Pool {
    pdus: Vec<Pdu>,
    /// Each IE of the trees, as a tree has it: its name with its value
    ies: Vec<Value>,
    /// The values of the views, by the last segment of their paths
    values: std::collections::BTreeMap<String, Vec<Value>>,
    /// Each text of the views: the names of values above all
    names: Vec<String>,
}

/// The vectors of the protocol, the PDUs of other implementations and those of a later
/// version, of the repository whose fuzz package this is.
const FIXTURES: [&str; 4] = [
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/fixtures/messages.tsv"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/fixtures/vectors.tsv"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/fixtures/later.tsv"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/fixtures/captured.tsv"
    )),
];

static POOL: LazyLock<Pool> = LazyLock::new(|| {
    let unhex = |text: &str| -> Vec<u8> {
        let digit = |c: u8| (c as char).to_digit(16).unwrap_or(0) as u8;
        (text.as_bytes().chunks(2))
            .map(|pair| digit(pair[0]) << 4 | pair.get(1).map_or(0, |low| digit(*low)))
            .collect()
    };
    let rows = FIXTURES.iter().flat_map(|text| text.lines());
    let pdus: Vec<Pdu> = rows
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.split('\t').nth(1))
        .filter_map(|octets| Pdu::decode(&unhex(octets)).ok())
        .collect();
    let mut ies = Vec::new();
    let mut values = std::collections::BTreeMap::<String, Vec<Value>>::new();
    let mut names = Vec::new();
    for pdu in &pdus {
        if let Some(Value::Array(entries)) = pdu.to_tree().get("ies") {
            ies.extend(entries.iter().cloned());
        }
        let Ok(view) = pdu.to_view() else { continue };
        for (path, value) in inspect::paths(&view) {
            let last = path.rsplit('/').next().unwrap_or_default().to_owned();
            if let Value::String(text) = &value {
                names.push(text.clone());
            }
            let seen = values.entry(last).or_default();
            if seen.len() < 64 && !seen.contains(&value) {
                seen.push(value);
            }
        }
    }
    ies.dedup();
    names.sort();
    names.dedup();
    Pool {
        pdus,
        ies,
        values,
        names,
    }
});

/// A value that the fuzzer writes in the place of another.
#[derive(Arbitrary, Debug)]
pub enum Leaf {
    Unsigned(u64),
    Signed(i64),
    Small(u8),
    Text(String),
    Hex(Vec<u8>),
    Bool(bool),
    Null,
    /// A text of a view of the fixtures: the name of a value above all
    Name(u16),
}

impl Leaf {
    fn value(&self) -> Value {
        match self {
            Leaf::Unsigned(number) => Value::from(*number),
            Leaf::Signed(number) => Value::from(*number),
            Leaf::Small(number) => Value::from(*number),
            Leaf::Text(text) => Value::from(text.as_str()),
            Leaf::Hex(octets) => Value::from(hex(octets)),
            Leaf::Bool(flag) => Value::from(*flag),
            Leaf::Null => Value::Null,
            Leaf::Name(number) => {
                let names = &POOL.names;
                match names.get(usize::from(*number) % names.len().max(1)) {
                    Some(name) => Value::from(name.as_str()),
                    None => Value::Null,
                }
            }
        }
    }
}

fn hex(octets: &[u8]) -> String {
    octets.iter().map(|octet| format!("{octet:02x}")).collect()
}

/// Replace the leaf of `value` that `walk` goes to, a member or an entry by its place
/// at each step, with `leaf`.
fn change(value: &mut Value, walk: &[u16], leaf: Value) {
    let Some((&step, rest)) = walk.split_first() else {
        *value = leaf;
        return;
    };
    match value {
        Value::Array(entries) if !entries.is_empty() => {
            let at = usize::from(step) % entries.len();
            change(&mut entries[at], rest, leaf);
        }
        Value::Object(members) if !members.is_empty() => {
            let at = usize::from(step) % members.len();
            if let Some(member) = members.values_mut().nth(at) {
                change(member, rest, leaf);
            }
        }
        _ => *value = leaf,
    }
}

/// What holds of a PDU that the crate wrote, from a view or a tree: it encodes, its
/// octets decode to it, its tree writes it again, and what `check` says of it agrees
/// with the other readings ([`checked`]).
fn written_pdu(pdu: &Pdu) {
    let octets = match pdu.encode() {
        Ok(octets) => octets,
        Err(error) => panic!("a PDU that the crate wrote does not encode: {error}: {pdu:?}"),
    };
    assert_eq!(Pdu::decode(&octets).as_ref(), Ok(pdu), "{octets:02x?}");
    let tree = pdu.to_tree();
    assert_eq!(Pdu::from_tree(&tree).as_ref(), Ok(pdu), "{tree}");
    checked(pdu);
}

/// A PDU of the fixtures and what is done to its view.
#[derive(Arbitrary, Debug)]
pub struct Edits {
    base: u16,
    edits: Vec<Edit>,
}

/// One edit of a view. A path is one of those of `inspect::paths`, which the number
/// picks.
#[derive(Arbitrary, Debug)]
enum Edit {
    /// A value at the path
    Set { path: u16, to: Leaf },
    /// The value of another path of the view
    Copy { path: u16, from: u16 },
    /// A value that a path of a view of the fixtures that ends as this one has, with
    /// one of its leaves changed or not
    Seen {
        path: u16,
        value: u16,
        walk: Vec<u16>,
        to: Option<Leaf>,
    },
    /// What the path is in: a member, an entry of a list or an IE
    Remove { path: u16 },
    /// A copy of the entry of a list that the path is in, before it
    Insert { path: u16 },
}

/// The IE of the message that a path of a view is under: the position of the IE in a
/// path of `/message/protocolIEs`, or its name in a path of the root of the protocol.
fn ie_of(path: &str) -> Option<String> {
    let mut segments = path.split('/').skip(1);
    match (segments.next(), segments.next(), segments.next()) {
        (Some("message"), Some("protocolIEs"), Some(position)) => Some(position.to_owned()),
        (Some(root), Some(name), _) if ROOT.ends_with(root) => Some(name.to_owned()),
        _ => None,
    }
}

/// The path of the entry of a list that `path` is in, the deepest one.
fn entry_of(path: &str) -> Option<String> {
    let segments: Vec<&str> = path.split('/').collect();
    let last = segments.iter().rposition(|segment| {
        !segment.is_empty() && segment.bytes().all(|octet| octet.is_ascii_digit())
    })?;
    Some(segments[..=last].join("/"))
}

/// A PDU of the fixtures, edited through its view by the paths that `inspect::paths`
/// lists, with values that views of the fixtures have, values of the view itself and
/// values of the fuzzer, as a user edits one.
///
/// An edit that the view or the PDU refuses is an error, never a panic. The PDU that
/// `with_view` writes is the one that `from_view` writes of the same view, and one that
/// the crate wrote ([`written_pdu`]). Only what was edited changes: each IE of the
/// message that no path of an edit was under has the entry that it had in the view.
pub fn edit(input: Edits) {
    let pool = &*POOL;
    let Some(pdu) = pool
        .pdus
        .get(usize::from(input.base) % pool.pdus.len().max(1))
    else {
        return;
    };
    let Ok(before) = pdu.to_view() else {
        return;
    };
    let paths = inspect::paths(&before);
    if paths.is_empty() {
        return;
    }
    let pick = |number: u16| paths[usize::from(number) % paths.len()].clone();
    let mut view = before.clone();
    let mut edited = Vec::new();
    for edit in input.edits.iter().take(8) {
        let (path, done) = match edit {
            Edit::Set { path, to } => {
                let (path, _) = pick(*path);
                let done = inspect::set(&mut view, &path, to.value()).is_ok();
                (path, done)
            }
            Edit::Copy { path, from } => {
                let ((path, _), (_, value)) = (pick(*path), pick(*from));
                let done = inspect::set(&mut view, &path, value).is_ok();
                (path, done)
            }
            Edit::Seen {
                path,
                value,
                walk,
                to,
            } => {
                let (path, _) = pick(*path);
                let last = path.rsplit('/').next().unwrap_or_default();
                let Some(seen) = pool.values.get(last).filter(|seen| !seen.is_empty()) else {
                    continue;
                };
                let mut value = seen[usize::from(*value) % seen.len()].clone();
                if let Some(to) = to {
                    change(&mut value, walk, to.value());
                }
                let done = inspect::set(&mut view, &path, value).is_ok();
                (path, done)
            }
            Edit::Remove { path } => {
                let (path, _) = pick(*path);
                let parent = path[..path.rfind('/').unwrap_or(0)].to_owned();
                let done = inspect::remove(&mut view, &parent).is_ok();
                (parent, done)
            }
            Edit::Insert { path } => {
                let (path, _) = pick(*path);
                let Some(entry) = entry_of(&path) else {
                    continue;
                };
                let Some(copy) = inspect::select(&view, &entry)
                    .ok()
                    .and_then(|selected| selected.first().map(|value| (*value).clone()))
                else {
                    continue;
                };
                let done = inspect::insert(&mut view, &entry, copy).is_ok();
                (entry, done)
            }
        };
        if done {
            edited.push(path);
        }
    }

    let Ok(changed) = pdu.with_view(&view) else {
        return;
    };
    assert_eq!(Pdu::from_view(&view).as_ref(), Ok(&changed), "{view}");
    written_pdu(&changed);

    // Only what was edited changes, IE by IE, where the edits kept every IE where it was
    let Some(touched) = edited
        .iter()
        .map(|path| ie_of(path))
        .collect::<Option<Vec<_>>>()
    else {
        return;
    };
    let (Some(Value::Array(was)), Ok(after)) = (ies_of(&before), changed.to_view()) else {
        return;
    };
    let Some(Value::Array(is)) = ies_of(&after) else {
        return;
    };
    if was.len() != is.len() {
        return;
    }
    for (position, (was, is)) in was.iter().zip(is).enumerate() {
        let name = was
            .get("_ie_name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if touched
            .iter()
            .any(|ie| *ie == position.to_string() || ie == name)
        {
            continue;
        }
        assert_eq!(
            was, is,
            "IE {position} changed with the edits of {edited:?}"
        );
    }
}

/// A PDU of the fixtures and what is done to the IEs of its tree.
#[derive(Arbitrary, Debug)]
pub struct Build {
    base: u16,
    ies: Vec<Change>,
}

/// What is done to the list of IEs of a tree. A position is one of the list, which the
/// number picks.
#[derive(Arbitrary, Debug)]
enum Change {
    /// The IE at a position is taken out
    Remove(u16),
    /// An IE of a tree of the fixtures, with one of its leaves changed or not, at a
    /// position, in the place of the IE there or before it
    Seen {
        at: u16,
        ie: u16,
        walk: Vec<u16>,
        to: Option<Leaf>,
        replace: bool,
    },
    /// The value of the IE at a position, with one of its leaves changed
    Change { at: u16, walk: Vec<u16>, to: Leaf },
    /// The IE at a position as octets, which the tree writes as they are
    Octets { at: u16, octets: Vec<u8> },
    /// The IE at a position again, after it
    Repeat(u16),
    /// Two IEs swap their places
    Swap(u16, u16),
}

/// A PDU written from a tree, as `from_tree` writes one, whose IEs the fuzzer chooses
/// among those of the fixtures, with their values deep in each IE, and adds, removes,
/// repeats and puts in another order: the PDUs of every message, which a receiver
/// checks, and not only octets that a decoder refuses at the header.
///
/// A tree that the message or a value refuses is an error, never a panic, and a PDU
/// that `from_tree` writes is one that the crate wrote ([`written_pdu`]).
pub fn tree(input: Build) {
    let pool = &*POOL;
    let Some(pdu) = pool
        .pdus
        .get(usize::from(input.base) % pool.pdus.len().max(1))
    else {
        return;
    };
    let mut tree = pdu.to_tree();
    let Some(Value::Array(ies)) = tree.get_mut("ies") else {
        return;
    };
    for step in input.ies.iter().take(16) {
        let at = |number: u16, length: usize| usize::from(number) % length.max(1);
        match step {
            Change::Remove(position) if !ies.is_empty() => {
                ies.remove(at(*position, ies.len()));
            }
            Change::Seen {
                at: position,
                ie,
                walk,
                to,
                replace,
            } if !pool.ies.is_empty() => {
                let mut ie = pool.ies[usize::from(*ie) % pool.ies.len()].clone();
                if let Some(to) = to {
                    change(&mut ie, walk, to.value());
                }
                let position = at(*position, ies.len() + 1);
                match (*replace, position < ies.len()) {
                    (true, true) => ies[position] = ie,
                    _ => ies.insert(position, ie),
                }
            }
            Change::Change {
                at: position,
                walk,
                to,
            } if !ies.is_empty() => {
                let position = at(*position, ies.len());
                change(&mut ies[position], walk, to.value());
            }
            Change::Octets {
                at: position,
                octets,
            } if !ies.is_empty() => {
                let position = at(*position, ies.len());
                if let Some(Value::Object(members)) = ies.get_mut(position)
                    && let Some((_, value)) =
                        members.iter_mut().find(|(name, _)| *name != "criticality")
                {
                    let mut entry = codec::tree::Map::new();
                    entry.insert("octets".into(), Value::from(hex(octets)));
                    *value = Value::Object(entry);
                }
            }
            Change::Repeat(position) if !ies.is_empty() => {
                let position = at(*position, ies.len());
                let again = ies[position].clone();
                ies.insert(position + 1, again);
            }
            Change::Swap(first, second) if !ies.is_empty() => {
                let (first, second) = (at(*first, ies.len()), at(*second, ies.len()));
                ies.swap(first, second);
            }
            _ => {}
        }
    }
    if let Ok(written) = Pdu::from_tree(&tree) {
        written_pdu(&written);
    }
}
