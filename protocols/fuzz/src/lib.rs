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
//! A target does not only look for a panic: what the crate read is written and read
//! again as the same, the PDU, the protocol IEs of its message, its message as its type
//! and its tree.
//!
//! oxirush-ngap and oxirush-s1ap take this file from oxirush-aper. The package of each
//! names its crate `codec`, whose module `hooks` names what is of the protocol.

use codec::hooks::{self, BorrowedPdu as Borrowed, Pdu, ROOT};
use codec::tree::Value;
use codec::{Deviation, ErrorKind, aper, inspect};
use libfuzzer_sys::arbitrary::{self, Arbitrary};
use std::sync::LazyLock;

/// The octets of the open type that holds the message of a PDU.
fn message<'p>(pdu: &'p Borrowed<'_>) -> &'p [u8] {
    match pdu {
        Borrowed::initiatingMessage(message) => &message.value,
        Borrowed::successfulOutcome(message) => &message.value,
        Borrowed::unsuccessfulOutcome(message) => &message.value,
        _ => panic!("a PDU of an alternative that the bindings do not have"),
    }
}

/// A PDU, the protocol IEs of its message and the message as its type, each as the crate
/// reads and writes it.
pub fn decode(octets: &[u8]) {
    // The PDU, which is written back as octets that are read as the same PDU.
    let Ok(pdu) = Borrowed::decode(octets) else {
        return;
    };
    let written = match pdu.encode() {
        Ok(written) => written,
        Err(error) => panic!("a PDU that was decoded does not encode: {error}: {pdu:?}"),
    };
    assert_eq!(
        Borrowed::decode(&written).as_ref(),
        Ok(&pdu),
        "{written:02x?}"
    );

    // What the octets of the message have no part in.
    let _ = (pdu.to_string(), pdu.kind());

    // The protocol IEs of the message, whatever the message is, which are put back as
    // they were.
    match pdu.protocol_ies() {
        Ok(ies) => {
            let ies = ies.into_owned();
            let mut again = pdu.clone();
            again.set_protocol_ies(ies.clone()).unwrap();
            assert_eq!(again.protocol_ies(), Ok(ies));
            let _ = pdu.check();
        }
        // The private message has no protocol IEs, and its octets are not looked at.
        Err(error) if error.kind() == ErrorKind::WrongMessage => {}
        Err(_) => {}
    }

    // The message as its type, which `decode_value` reads and the codec writes.
    let _ = hooks::message_read_and_written(&pdu);
}

/// The tree of a PDU, which gives the PDU back, and its paths.
pub fn json(octets: &[u8]) {
    let Ok(pdu) = Borrowed::decode(octets) else {
        return;
    };
    // Every PDU has a tree, which is the PDU again: what its typed values have no place
    // for, and a message that does not decode, are octets in it.
    let tree = pdu.to_tree();
    assert_eq!(Pdu::from_tree(&tree).as_ref(), Ok(&pdu), "{tree}");

    // Each path that the tree has selects its value: every one, as an alternative of the
    // type NULL deep in a message is among the last that are listed.
    for (path, value) in inspect::paths(&tree) {
        assert_eq!(inspect::select(&tree, &path), Ok(vec![&value]), "{path}");
    }
}

/// The functions that take a path, on the tree of a PDU: `select`, `set`, `insert`,
/// `remove`, `open`, `check_path` and `enumerated_at`, with a path and a value of the
/// octets. No path and no value makes one of them panic, and a tree that one of them
/// changed is a PDU or an error.
///
/// The first octets say what is done: the function, a path of the tree to start from
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
    let Ok(pdu) = Borrowed::decode(rest) else {
        return;
    };
    let mut tree = pdu.to_tree();

    // A path that the tree has, cut short, with what the octets add to it.
    let known = inspect::paths(&tree);
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

    let set = aper::protocol::object_set::<<Pdu as aper::protocol::InProtocol>::Protocol>(
        pdu.direction(),
        pdu.procedure_code(),
    );
    let name = set.map_or("no message", |set| set.name);
    let changed = match function % 7 {
        0 => inspect::select(&tree, &path).is_err(),
        1 => inspect::set(&mut tree, &path, value).is_ok(),
        2 => inspect::insert(&mut tree, &path, value).is_ok(),
        3 => inspect::remove(&mut tree, &path).is_ok(),
        4 => inspect::open(&mut tree, &path).is_ok(),
        5 => inspect::check_path(name, &path).is_err(),
        _ => inspect::enumerated_at(name, &path).is_err(),
    };
    if !changed {
        return;
    }

    // What an edit left is a PDU or an error, and a PDU that it is encodes or is an
    // error: an edit may write a value that its type does not have.
    let _ = inspect::paths(&tree);
    if let Ok(edited) = Pdu::from_tree(&tree)
        && let Ok(octets) = edited.encode()
    {
        assert_eq!(Borrowed::decode(&octets).as_ref(), Ok(&edited), "{path}");
    }
}

/// The octets that a length of one octet counts, and those after them.
fn counted(octets: &[u8]) -> Option<(&[u8], &[u8])> {
    let (length, rest) = octets.split_first()?;
    rest.split_at_checked(usize::from(*length))
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
/// its message.
fn checked(pdu: &Borrowed<'_>) {
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
        let octets = match aper::encode(&diagnostics) {
            Ok(octets) => octets,
            Err(error) => {
                panic!("Criticality Diagnostics that do not encode: {error}: {diagnostics:?}")
            }
        };
        let read = aper::decode::<hooks::bindings::CriticalityDiagnostics>(&octets);
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
}

/// The octets of a PDU, what `check` says of it and how the crate writes it again.
///
/// A PDU that decodes encodes to the octets that it was decoded from, unless its reader
/// notes a form that X.691 does not give around its message, and only then; the
/// protocol IEs of its message, put back, are the octets of its message unless `check`
/// finds such a form around their values, or does not know its procedure. What `check` says agrees with the other
/// readings ([`checked`]).
pub fn exact(octets: &[u8]) {
    let Ok(pdu) = Borrowed::decode(octets) else {
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
/// with `arbitrary` start from, and what their trees hold.
struct Pool {
    pdus: Vec<Pdu>,
    /// Each IE of the trees, as a tree has it: its name with its value
    ies: Vec<Value>,
    /// The values of the trees, by the last segment of their paths
    values: std::collections::BTreeMap<String, Vec<Value>>,
    /// Each text of the trees: the names of values above all
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
        .filter_map(|octets| {
            Borrowed::decode(&unhex(octets))
                .ok()
                .map(Borrowed::into_owned)
        })
        .collect();
    let mut ies = Vec::new();
    let mut values = std::collections::BTreeMap::<String, Vec<Value>>::new();
    let mut names = Vec::new();
    for pdu in &pdus {
        let tree = pdu.to_tree();
        if let Some(Value::Array(entries)) = tree.get("ies") {
            ies.extend(entries.iter().cloned());
        }
        for (path, value) in inspect::paths(&tree) {
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
    /// A text of a tree of the fixtures: the name of a value above all
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

/// What holds of a PDU that the crate wrote from a tree: it encodes, its
/// octets decode to it, its tree writes it again, and what `check` says of it agrees
/// with the other readings ([`checked`]).
fn written_pdu(pdu: &Borrowed<'_>) {
    let octets = match pdu.encode() {
        Ok(octets) => octets,
        Err(error) => panic!("a PDU that the crate wrote does not encode: {error}: {pdu:?}"),
    };
    assert_eq!(Borrowed::decode(&octets).as_ref(), Ok(pdu), "{octets:02x?}");
    let tree = pdu.to_tree();
    assert_eq!(Pdu::from_tree(&tree).as_ref(), Ok(pdu), "{tree}");
    checked(pdu);
}

/// A PDU of the fixtures and what is done to its tree.
#[derive(Arbitrary, Debug)]
pub struct Edits {
    base: u16,
    edits: Vec<Edit>,
}

/// One edit of a tree. A path is one of those of `inspect::paths`, which the number
/// picks.
#[derive(Arbitrary, Debug)]
enum Edit {
    /// A value at the path
    Set { path: u16, to: Leaf },
    /// The value of another path of the tree
    Copy { path: u16, from: u16 },
    /// A value that a path of a tree of the fixtures that ends as this one has, with
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

/// The IE of the message that a path of a tree is under: its name or its position, after
/// the root of the protocol.
fn ie_of(path: &str) -> Option<String> {
    let mut segments = path.split('/').skip(1);
    match (segments.next(), segments.next()) {
        (Some(root), Some(name)) if ROOT.ends_with(root) => Some(name.to_owned()),
        _ => None,
    }
}

/// Whether two names are one: their letters and their digits, whatever their case.
fn same_name(one: &str, other: &str) -> bool {
    let letters = |name: &str| -> String {
        let kept = name.chars().filter(char::is_ascii_alphanumeric);
        kept.map(|letter| letter.to_ascii_lowercase()).collect()
    };
    letters(one) == letters(other)
}

/// The name of the IE that the entry of a list of IEs holds.
fn held(entry: &Value) -> Option<&str> {
    let members = entry.as_object()?;
    let mut names = members.keys().filter(|name| *name != "criticality");
    names.next().map(String::as_str)
}

/// The path of the entry of a list that `path` is in, the deepest one.
fn entry_of(path: &str) -> Option<String> {
    let segments: Vec<&str> = path.split('/').collect();
    let last = segments.iter().rposition(|segment| {
        !segment.is_empty() && segment.bytes().all(|octet| octet.is_ascii_digit())
    })?;
    Some(segments[..=last].join("/"))
}

/// A PDU of the fixtures, edited through its tree by the paths that `inspect::paths`
/// lists, with values that trees of the fixtures have, values of the tree itself and
/// values of the fuzzer, as a user edits one.
///
/// An edit that the tree or the PDU refuses is an error, never a panic, and a PDU that
/// `from_tree` writes is one that the crate wrote ([`written_pdu`]). Only what was edited
/// changes: where no edit took an IE of the message out or put one in, each IE that no
/// path of an edit was under has the entry that it had in the tree.
pub fn edit(input: Edits) {
    let pool = &*POOL;
    let Some(pdu) = pool
        .pdus
        .get(usize::from(input.base) % pool.pdus.len().max(1))
    else {
        return;
    };
    let before = pdu.to_tree();
    let paths = inspect::paths(&before);
    if paths.is_empty() {
        return;
    }
    let pick = |number: u16| paths[usize::from(number) % paths.len()].clone();
    let count = |tree: &Value| tree.get("ies").and_then(Value::as_array).map(Vec::len);
    let mut tree = before.clone();
    let mut edited = Vec::new();
    // Whether an edit took an IE of the message out or put one in, which moves those
    // after it: a removal and an insertion leave as many IEs, in other places.
    let mut moved = false;
    for edit in input.edits.iter().take(8) {
        let ies = count(&tree);
        let (path, done) = match edit {
            Edit::Set { path, to } => {
                let (path, _) = pick(*path);
                let done = inspect::set(&mut tree, &path, to.value()).is_ok();
                (path, done)
            }
            Edit::Copy { path, from } => {
                let ((path, _), (_, value)) = (pick(*path), pick(*from));
                let done = inspect::set(&mut tree, &path, value).is_ok();
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
                let done = inspect::set(&mut tree, &path, value).is_ok();
                (path, done)
            }
            Edit::Remove { path } => {
                let (path, _) = pick(*path);
                let parent = path[..path.rfind('/').unwrap_or(0)].to_owned();
                let done = inspect::remove(&mut tree, &parent).is_ok();
                (parent, done)
            }
            Edit::Insert { path } => {
                let (path, _) = pick(*path);
                let Some(entry) = entry_of(&path) else {
                    continue;
                };
                // An IE of the message is copied with its name, and any other entry as
                // the path selects it.
                let copy = match ie_of(&entry).and_then(|ie| ie.parse::<usize>().ok()) {
                    Some(position) if entry.matches('/').count() == 2 => tree["ies"]
                        .as_array()
                        .and_then(|ies| ies.get(position))
                        .cloned(),
                    _ => inspect::select(&tree, &entry)
                        .ok()
                        .and_then(|selected| selected.first().map(|value| (*value).clone())),
                };
                let Some(copy) = copy else {
                    continue;
                };
                let done = inspect::insert(&mut tree, &entry, copy).is_ok();
                (entry, done)
            }
        };
        moved |= count(&tree) != ies;
        if done {
            edited.push(path);
        }
    }

    let Ok(changed) = Pdu::from_tree(&tree) else {
        return;
    };
    written_pdu(&changed);

    // Only what was edited changes, IE by IE, where the edits kept every IE where it was
    if moved {
        return;
    }
    let Some(touched) = edited
        .iter()
        .map(|path| ie_of(path))
        .collect::<Option<Vec<_>>>()
    else {
        return;
    };
    let after = changed.to_tree();
    let (Some(Value::Array(was)), Some(Value::Array(is))) = (before.get("ies"), after.get("ies"))
    else {
        return;
    };
    if was.len() != is.len() {
        return;
    }
    for (position, (was, is)) in was.iter().zip(is).enumerate() {
        let name = held(was).unwrap_or_default();
        if touched
            .iter()
            .any(|ie| *ie == position.to_string() || same_name(ie, name))
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

/// A PDU of the fixtures and the value of its tree that the fuzzer's octets take the
/// place of.
#[derive(Arbitrary, Debug)]
pub struct Transfer {
    base: u16,
    path: u16,
    octets: Vec<u8>,
}

/// A PDU of the fixtures whose transfer, or another value that holds values, is the
/// fuzzer's octets: an OCTET STRING that contains a transfer, the open type of an IE, an
/// entry of a list deep in an IE. The octets are read as the type of that place, and not
/// only where the octets of a whole PDU around them decode.
///
/// A place that takes no octets is an error, never a panic, and a PDU that `from_tree`
/// writes with them is one that the crate wrote ([`written_pdu`]): what the octets are
/// read as is written and read again as the same.
pub fn transfer(input: Transfer) {
    let pool = &*POOL;
    let Some(pdu) = pool
        .pdus
        .get(usize::from(input.base) % pool.pdus.len().max(1))
    else {
        return;
    };
    let mut tree = pdu.to_tree();
    // The places that hold the values that `paths` lists: an IE, a member, an entry. Each
    // once, in the order of the paths: a list of 65535 entries has as many places, and a
    // search of those found before for each of them is longer than a timeout.
    let mut places: Vec<String> = Vec::new();
    let mut found = std::collections::HashSet::new();
    for (path, _) in inspect::paths(&tree) {
        let Some(below) = path.strip_prefix(ROOT) else {
            continue;
        };
        let mut place = ROOT.to_owned();
        for segment in below.split('/').skip(1) {
            place.push('/');
            place.push_str(segment);
            if place.len() < path.len() && found.insert(place.clone()) {
                places.push(place.clone());
            }
        }
    }
    let Some(path) = places.get(usize::from(input.path) % places.len().max(1)) else {
        return;
    };
    let mut octets = codec::tree::Map::new();
    octets.insert("octets".into(), Value::from(hex(&input.octets)));
    if inspect::set(&mut tree, path, Value::Object(octets)).is_err() {
        return;
    }
    if let Ok(written) = Pdu::from_tree(&tree) {
        written_pdu(&written);
    }
}

/// A PDU of the fixtures as another implementation writes it in the JSON Encoding Rules
/// (ITU-T X.697), and the leaves that the fuzzer changes in it.
#[derive(Arbitrary, Debug)]
pub struct Jer {
    base: u16,
    changes: Vec<(Vec<u16>, Leaf)>,
}

/// A view of a PDU as another implementation writes one: its procedure code, its
/// direction, its criticality and each IE by its identifier, its criticality and its
/// value, the values those of the tree of a PDU of the fixtures, with leaves that the
/// fuzzer changes anywhere in it, the identifiers and the criticalities too.
///
/// `tree_of_jer` takes any view without a panic, and a PDU that `from_tree` writes from
/// the tree that it gives is one that the crate wrote ([`written_pdu`]).
pub fn jer(input: Jer) {
    use codec::aper::protocol::Field;
    let pool = &*POOL;
    let Some(pdu) = pool
        .pdus
        .get(usize::from(input.base) % pool.pdus.len().max(1))
    else {
        return;
    };
    let Ok(fields) = pdu.protocol_ies() else {
        return;
    };
    let tree = pdu.to_tree();
    let values = tree.get("ies").and_then(Value::as_array);
    let mut ies = Vec::new();
    for (position, field) in fields.0.iter().enumerate() {
        // The value of the IE in the tree, the member that is not its criticality
        let value = values
            .and_then(|values| values.get(position))
            .and_then(Value::as_object)
            .and_then(|entry| entry.iter().find(|(name, _)| *name != "criticality"))
            .map_or(Value::Null, |(_, value)| value.clone());
        let mut ie = codec::tree::Map::new();
        ie.insert("id".into(), Value::from(field.id()));
        ie.insert(
            "criticality".into(),
            Value::from(field.criticality().name()),
        );
        ie.insert("value".into(), value);
        ies.push(Value::Object(ie));
    }
    let mut message = codec::tree::Map::new();
    message.insert("protocolIEs".into(), Value::Array(ies));
    let mut view = codec::tree::Map::new();
    view.insert("procedure_code".into(), Value::from(pdu.procedure_code()));
    view.insert("direction".into(), Value::from(pdu.direction().name()));
    view.insert("criticality".into(), Value::from(pdu.criticality().name()));
    view.insert("message".into(), Value::Object(message));
    let mut view = Value::Object(view);
    for (walk, leaf) in input.changes.iter().take(8) {
        change(&mut view, walk, leaf.value());
    }
    let Ok(tree) = inspect::__tree_of_jer(&view) else {
        return;
    };
    if let Ok(written) = Pdu::from_tree(&tree) {
        written_pdu(&written);
    }
}
