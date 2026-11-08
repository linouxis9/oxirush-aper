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

//! The trees and the paths of a protocol of a few types with codecs of their own, written
//! by hand, for what TOY, the generated protocol of `protocols/toy/` whose tests are those
//! of the generated crates, cannot have: types that hold one another 256 deep, which the
//! JSON Encoding Rules and a written tree are read and written as deep as, and the types of
//! another protocol, which a program registers, and of one that it does not. Its tables
//! have messages and a transfer of protocol IEs, an IE that extends a type, one whose
//! octets contain a type, one whose octets may carry a type, a list of single containers,
//! a bit string and a well-known type.
//!
//! ```text
//! Setup    reject  InitiatingMessage SetupRequest  [Cause ignore mandatory, Holder reject,
//!                                                   Container ignore, Items reject,
//!                                                   Payload ignore]
//!                  SuccessfulOutcome SetupResponse [Cause ignore mandatory]
//! Release  ignore  InitiatingMessage Release       [Cause ignore mandatory]
//!
//! Cause         ::= CHOICE { radioNetwork, nas, misc NULL, choice-Extensions }   id 15
//! Holder        ::= SEQUENCE { cause, transfer OCTET STRING (CONTAINING Cause), setup
//!                   OCTET STRING (CONTAINING SetupTransfer), flags Flags, tac TAC,
//!                   items Items, iE-Extensions }                                  id 16
//! ExtendedCause: the extension of Holder that is a Cause                         id 17
//! Container     ::= OCTET STRING, which may carry an LteBlob, a Cause, or a
//!                   Wrapper of the protocol OTHER and a GhostBlob of GHOST        id 18
//! Items         ::= SEQUENCE OF ProtocolIE-SingleContainer {{Item reject}}        id 20
//! Payload       ::= OCTET STRING (CONTAINING Cause)                               id 21
//! Item          ::= INTEGER (0..255)                                              id 30
//! SetupTransfer ::= SEQUENCE { protocolIEs {{Cause ignore mandatory}} }
//! Flags ::= BIT STRING (SIZE (1..8)), TAC ::= OCTET STRING (SIZE (3)), Alias ::= Cause
//! ```
//!
//! The codecs are the toy's own. A list of IEs is, for each, its identifier in two octets,
//! its criticality, its length and its octets; 0x80 before it is what a later version adds,
//! which the value has no place for. A Cause is its alternative and the position of its
//! name, and more octets after them are such an addition; 0xff is no Cause, and 0xfe one in
//! another form than the canonical one. A Holder is its members, each a tag, a length and
//! its octets.

use oxirush_aper::Criticality::{self, Ignore, Reject};
use oxirush_aper::Presence::{self, Mandatory, Optional};
use oxirush_aper::inspect::{self, Form, Kind, Member, Message, Protocol, Shape, Typed};
use oxirush_aper::{Direction, Error, ErrorKind, ObjectSet};
use oxirush_utils::tree;
use oxirush_utils::tree::{Map, Value};

// ── The tables ──────────────────────────────────────────────────────────────

const CAUSE: &[Member] = &[
    ("radioNetwork", Shape::Named("CauseRadioNetwork"), false),
    ("nas", Shape::Named("CauseNas"), false),
    ("misc", Shape::Null, false),
    ("choice-Extensions", Shape::Ie(None), false),
];
const HOLDER: &[Member] = &[
    ("cause", Shape::Named("Alias"), true),
    ("transfer", Shape::Transfer("Cause"), true),
    ("setup", Shape::Transfer("SetupTransfer"), true),
    ("flags", Shape::Named("Flags"), true),
    ("tac", Shape::Named("TAC"), true),
    ("items", Shape::Named("Items"), true),
    ("causes", Shape::Named("Causes"), true),
    // Named as the IE Payload, and of another type
    ("payload", Shape::Named("Flags"), true),
    ("inner", Shape::Transfer("Holder"), true),
    ("otherTac", Shape::Named("TacAlias"), true),
    // Octets that contain one of several types
    ("blob", Shape::Transfer(""), true),
    ("iE-Extensions", Shape::Extensions(""), true),
];
/// Node ::= SEQUENCE { tac TAC, next NodeAlias, nodes Nodes, packed OCTET STRING
/// (CONTAINING Node) }, all OPTIONAL: a type that holds itself.
const NODE: &[Member] = &[
    ("tac", Shape::Named("TAC"), true),
    ("next", Shape::Named("NodeAlias"), true),
    ("nodes", Shape::Named("Nodes"), true),
    ("packed", Shape::Transfer("Node"), true),
];
const PAIR: &[Member] = &[
    ("first", Shape::Named("CauseNas"), false),
    ("second", Shape::Named("CauseRadioNetwork"), false),
];
/// Same ::= SEQUENCE { a CauseNas OPTIONAL, b NasAlias OPTIONAL }: one enumeration by two
/// names.
const SAME: &[Member] = &[
    ("a", Shape::Named("CauseNas"), true),
    ("b", Shape::Named("NasAlias"), true),
];
/// PrivateMessage ::= SEQUENCE { tac TAC, kind NasAlias OPTIONAL, pair Pair OPTIONAL,
/// same Same OPTIONAL }: a message without IEs.
const PRIVATE: &[Member] = &[
    ("tac", Shape::Named("TAC"), false),
    ("kind", Shape::Named("NasAlias"), true),
    ("pair", Shape::Named("Pair"), true),
    ("same", Shape::Named("Same"), true),
];
const CONTAINER_OF_IES: &[Member] = &[("protocolIEs", Shape::Ies(None), false)];
static ITEM: Shape = Shape::Ie(None);
static A_CAUSE: Shape = Shape::Named("Cause");
static A_NODE: Shape = Shape::Named("Node");

const TYPES: &[(&str, Kind)] = &[
    ("Alias", Kind::Is(Shape::Named("Cause"))),
    ("Cause", Kind::Choice(CAUSE)),
    ("Causes", Kind::Is(Shape::List(&A_CAUSE))),
    ("Flags", Kind::Is(Shape::Bits)),
    ("Holder", Kind::Sequence(HOLDER)),
    ("Items", Kind::Is(Shape::List(&ITEM))),
    ("NasAlias", Kind::Is(Shape::Named("CauseNas"))),
    ("Node", Kind::Sequence(NODE)),
    ("NodeAlias", Kind::Is(Shape::Named("Node"))),
    ("Nodes", Kind::Is(Shape::List(&A_NODE))),
    ("Pair", Kind::Sequence(PAIR)),
    ("PrivateMessage", Kind::Sequence(PRIVATE)),
    ("Release", Kind::Sequence(CONTAINER_OF_IES)),
    ("Same", Kind::Sequence(SAME)),
    ("SetupRequest", Kind::Sequence(CONTAINER_OF_IES)),
    ("SetupTransfer", Kind::Sequence(CONTAINER_OF_IES)),
    ("TacAlias", Kind::Is(Shape::Named("TAC"))),
];

const SETUP_IES: &[(u16, Criticality, Presence)] = &[
    (15, Ignore, Mandatory),
    (16, Reject, Optional),
    (18, Ignore, Optional),
    (20, Reject, Optional),
    (21, Ignore, Optional),
    (22, Ignore, Optional),
    (24, Ignore, Optional),
];

/// A message of the toy protocol whose procedure is of a criticality and whose IEs are
/// these.
const fn set(
    direction: Direction,
    procedure_code: u8,
    name: &'static str,
    criticality: Criticality,
    ies: Option<&'static [(u16, Criticality, Presence)]>,
) -> ObjectSet {
    ObjectSet {
        direction,
        procedure_code,
        name,
        criticality,
        ies,
    }
}

const SETUP_REQUEST: ObjectSet = set(
    Direction::Initiating,
    1,
    "SetupRequest",
    Reject,
    Some(SETUP_IES),
);
const SETUP_RESPONSE: ObjectSet = set(
    Direction::Successful,
    1,
    "SetupResponse",
    Reject,
    Some(&[(15, Ignore, Mandatory)]),
);
const RELEASE: ObjectSet = set(
    Direction::Initiating,
    2,
    "Release",
    Ignore,
    Some(&[(15, Ignore, Mandatory)]),
);
const PRIVATE_MESSAGE: ObjectSet = set(Direction::Initiating, 3, "PrivateMessage", Ignore, None);

const MESSAGES: &[Message] = &[
    (&SETUP_REQUEST, ies_typed, "crate::toy::SetupRequest<'a>"),
    (&SETUP_RESPONSE, ies_typed, "crate::toy::Release<'a>"),
    (&RELEASE, release_typed, "crate::toy::Release<'a>"),
    (
        &PRIVATE_MESSAGE,
        private_typed,
        "crate::toy::PrivateMessage<'a>",
    ),
];

const IE_NAMES: &[(u16, &str)] = &[
    (15, "Cause"),
    (16, "Holder"),
    (17, "ExtendedCause"),
    (18, "Container"),
    (19, "ExtendedFlags"),
    (20, "Items"),
    (21, "Payload"),
    (22, "Nas"),
    (24, "Node"),
    (30, "Item"),
];

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

static TOY: Protocol = Protocol {
    name: "TOY",
    root: "toy",
    types: TYPES,
    messages: MESSAGES,
    ie_names: IE_NAMES,
    ie_types: &[
        (15, "crate::toy::Cause", ""),
        (16, "crate::toy::Holder", ""),
        (18, "crate::toy::Container<'a>", ""),
        (20, "crate::toy::Items<'a>", ""),
        (21, "crate::toy::Payload<'a>", "crate::toy::Cause"),
        (22, "crate::toy::Nas<'a>", "crate::toy::Cause"),
        (24, "crate::toy::Node", ""),
        (30, "crate::toy::Item", ""),
    ],
    extension_types: &[
        (17, "Holder", "crate::toy::Cause"),
        (19, "Holder", "crate::toy::Flags"),
    ],
    carried: &[(18, "LteBlob", "crate::toy::Holder")],
    foreign: &[
        (18, "Wrapper", "OTHER"),
        (18, "Gauge", "SECOND"),
        (18, "GhostBlob", "GHOST"),
    ],
    enumerations: &[
        ("CauseNas", &["normal-release", "deregister"]),
        ("CauseRadioNetwork", &["unspecified", "user-inactivity"]),
    ],
    item_criticalities: &[(15, Ignore), (30, Reject)],
    transfers: &[("SetupTransfer", "SetupTransfer", &[(15, Ignore, Mandatory)])],
    ie_id,
    kind_of,
    ie_typed,
    extension_typed: |id, extended| (id == 17 && extended == "Holder").then(cause_typed),
    ie_contents: |id| matches!(id, 21 | 22).then(cause_typed),
    type_form: |name| (name == "TAC").then_some(Form::Number(24)),
    transfer_fields: &["transfer", "setup", "inner", "packed"],
    transfer_typed: |field| match field {
        "transfer" => Ok(cause_typed()),
        "setup" => Ok(ies_typed()),
        "inner" => Ok(holder_typed()),
        _ => Err(Error::new(ErrorKind::Path, "no codec of a Node")),
    },
    carried_typed: |name| (name == "LteBlob").then(holder_typed),
    any_message: ies_typed,
};

fn ie_typed(id: u16) -> Result<Typed, Error> {
    match id {
        15 => Ok(cause_typed()),
        16 => Ok(holder_typed()),
        18 => Ok(Typed {
            decode: octets_decode,
            encode: octets_encode,
        }),
        21 | 22 => Ok(Typed {
            decode: payload_decode,
            encode: payload_encode,
        }),
        20 => Ok(Typed {
            decode: items_decode,
            encode: items_encode,
        }),
        30 => Ok(Typed {
            decode: item_decode,
            encode: item_encode,
        }),
        _ => Err(Error::new(
            ErrorKind::UnknownIe,
            format!("no type for IE {id}"),
        )),
    }
}

// ── The codecs ──────────────────────────────────────────────────────────────

fn hex(octets: &[u8]) -> String {
    octets.iter().map(|octet| format!("{octet:02x}")).collect()
}

fn unhex(text: &str) -> Result<Vec<u8>, Error> {
    let wrong = || Error::new(ErrorKind::Encode, format!("{text} is no octets"));
    if !text.len().is_multiple_of(2) {
        return Err(wrong());
    }
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&text[at..at + 2], 16).map_err(|_| wrong()))
        .collect()
}

fn decoding(why: &str) -> Error {
    Error::new(ErrorKind::Decode, why.to_string())
}

fn encoding(why: &str) -> Error {
    Error::new(ErrorKind::Encode, why.to_string())
}

const CRITICALITIES: [&str; 3] = ["reject", "ignore", "notify"];

/// The entries of a list of IEs, each with its value as its octets in `field`.
fn entries(mut octets: &[u8], field: &str) -> Result<Vec<Value>, Error> {
    let mut entries = Vec::new();
    while !octets.is_empty() {
        let [high, low, criticality, length, rest @ ..] = octets else {
            return Err(decoding("an IE is cut short"));
        };
        let length = usize::from(*length);
        let value = rest
            .get(..length)
            .ok_or_else(|| decoding("an IE is cut short"))?;
        let criticality = CRITICALITIES
            .get(usize::from(*criticality))
            .ok_or_else(|| decoding("no criticality"))?;
        let mut entry = Map::new();
        entry.insert(
            "id".into(),
            (u16::from(*high) << 8 | u16::from(*low)).into(),
        );
        entry.insert("criticality".into(), (*criticality).into());
        entry.insert(field.into(), hex(value).into());
        entries.push(Value::Object(entry));
        octets = &rest[length..];
    }
    Ok(entries)
}

/// The octets of the entries of a list of IEs, each with its value as its octets in
/// `field`.
fn entries_octets(entries: &Value, field: &str) -> Result<Vec<u8>, Error> {
    let entries = entries.as_array().ok_or_else(|| encoding("no list"))?;
    let mut octets = Vec::new();
    for entry in entries {
        let id = entry["id"].as_u64().ok_or_else(|| encoding("no id"))?;
        let criticality = entry["criticality"].as_str().unwrap_or_default();
        let criticality = CRITICALITIES
            .iter()
            .position(|known| *known == criticality)
            .ok_or_else(|| encoding("no criticality"))?;
        let value = unhex(entry[field].as_str().ok_or_else(|| encoding("no octets"))?)?;
        octets.extend([
            (id >> 8) as u8,
            id as u8,
            criticality as u8,
            value.len() as u8,
        ]);
        octets.extend(value);
    }
    Ok(octets)
}

fn ies_decode(octets: &[u8]) -> Result<inspect::Decoded, Error> {
    // What a later version adds, which the value has no place for
    let (octets, exact) = match octets.split_first() {
        Some((0x80, rest)) => (rest, false),
        _ => (octets, true),
    };
    let mut message = Map::new();
    message.insert(
        "protocolIEs".into(),
        Value::Array(entries(octets, "value")?),
    );
    Ok((Value::Object(message), exact, None))
}

fn ies_encode(value: &Value) -> Result<Vec<u8>, Error> {
    entries_octets(&value["protocolIEs"], "value")
}

fn ies_typed() -> Typed {
    Typed {
        decode: ies_decode,
        encode: ies_encode,
    }
}

/// A RELEASE is its list of IEs after an `R`, which no other message has.
fn release_decode(octets: &[u8]) -> Result<inspect::Decoded, Error> {
    match octets.split_first() {
        Some((b'R', rest)) => ies_decode(rest),
        _ => Err(decoding("no Release")),
    }
}

fn release_encode(value: &Value) -> Result<Vec<u8>, Error> {
    let mut octets = vec![b'R'];
    octets.extend(ies_encode(value)?);
    Ok(octets)
}

fn release_typed() -> Typed {
    Typed {
        decode: release_decode,
        encode: release_encode,
    }
}

const NAMES: [&[&str]; 2] = [
    &["unspecified", "user-inactivity"],
    &["normal-release", "deregister"],
];
const ALTERNATIVES: [&str; 2] = ["radioNetwork", "nas"];

fn cause_decode(octets: &[u8]) -> Result<inspect::Decoded, Error> {
    let mut cause = Map::new();
    let refused = octets.first() == Some(&0xfe);
    let (value, used) = match octets {
        [2, ..] => (Value::Null, 1),
        [3, rest @ ..] => {
            let [extension] = &entries(rest, "value")?[..] else {
                return Err(decoding("a Cause extension is one IE"));
            };
            cause.insert("choice-Extensions".into(), extension.clone());
            return Ok((Value::Object(cause), true, None));
        }
        [alternative @ (0 | 1), name, ..] => {
            let names = NAMES[usize::from(*alternative)];
            let name = names
                .get(usize::from(*name))
                .ok_or_else(|| decoding("no such cause"))?;
            cause.insert(
                ALTERNATIVES[usize::from(*alternative)].into(),
                (*name).into(),
            );
            return Ok((Value::Object(cause), octets.len() == 2, None));
        }
        [0xfe, ..] => (Value::from("unspecified"), octets.len()),
        _ => return Err(decoding("no Cause")),
    };
    let alternative = if refused { "radioNetwork" } else { "misc" };
    cause.insert(alternative.into(), value);
    let refusal = refused.then(|| decoding("a Cause in another form than the canonical one"));
    Ok((Value::Object(cause), octets.len() == used, refusal))
}

fn cause_encode(value: &Value) -> Result<Vec<u8>, Error> {
    let cause = value.as_object().ok_or_else(|| encoding("no Cause"))?;
    let (name, chosen) = cause.iter().next().ok_or_else(|| encoding("no Cause"))?;
    if name == "misc" && chosen.is_null() {
        return Ok(vec![2]);
    }
    if name == "choice-Extensions" {
        let mut octets = vec![3];
        octets.extend(entries_octets(
            &Value::Array(vec![chosen.clone()]),
            "value",
        )?);
        return Ok(octets);
    }
    let alternative = ALTERNATIVES
        .iter()
        .position(|known| known == name)
        .ok_or_else(|| encoding("no such alternative"))?;
    let position = NAMES[alternative]
        .iter()
        .position(|known| Some(*known) == chosen.as_str())
        .ok_or_else(|| encoding("no such cause"))?;
    Ok(vec![alternative as u8, position as u8])
}

fn cause_typed() -> Typed {
    Typed {
        decode: cause_decode,
        encode: cause_encode,
    }
}

const HOLDER_TAGS: [&str; 8] = [
    "cause",
    "transfer",
    "setup",
    "flags",
    "tac",
    "items",
    "iE-Extensions",
    "inner",
];

fn holder_decode(mut octets: &[u8]) -> Result<inspect::Decoded, Error> {
    let mut holder = Map::new();
    let mut exact = true;
    while let [tag, length, rest @ ..] = octets {
        let contents = (rest.get(..usize::from(*length))).ok_or_else(|| decoding("cut short"))?;
        let name = HOLDER_TAGS
            .get(usize::from(*tag))
            .ok_or_else(|| decoding("no such member"))?;
        let value = match *name {
            "cause" => {
                let (cause, whole, _) = cause_decode(contents)?;
                exact &= whole;
                cause
            }
            "transfer" | "setup" | "inner" => hex(contents).into(),
            "flags" => tree!({"value": (hex(&contents[1..])), "length": (contents[0])}),
            "tac" => contents
                .iter()
                .fold(0_u64, |n, octet| n << 8 | u64::from(*octet))
                .into(),
            "items" => Value::Array(entries(contents, "value")?),
            _ => Value::Array(entries(contents, "extensionValue")?),
        };
        holder.insert((*name).into(), value);
        octets = &rest[usize::from(*length)..];
    }
    Ok((Value::Object(holder), exact, None))
}

fn holder_encode(value: &Value) -> Result<Vec<u8>, Error> {
    let holder = value.as_object().ok_or_else(|| encoding("no Holder"))?;
    let mut octets = Vec::new();
    for (name, member) in holder {
        let tag = HOLDER_TAGS
            .iter()
            .position(|known| known == name)
            .ok_or_else(|| encoding("no such member"))?;
        let contents = match name.as_str() {
            "cause" => cause_encode(member)?,
            "transfer" | "setup" | "inner" => {
                unhex(member.as_str().ok_or_else(|| encoding("no octets"))?)?
            }
            "flags" => {
                let mut flags = vec![member["length"].as_u64().unwrap_or_default() as u8];
                flags.extend(unhex(member["value"].as_str().unwrap_or_default())?);
                flags
            }
            "tac" => {
                let tac = member.as_u64().ok_or_else(|| encoding("no TAC"))?;
                vec![(tac >> 16) as u8, (tac >> 8) as u8, tac as u8]
            }
            "items" => entries_octets(member, "value")?,
            _ => entries_octets(member, "extensionValue")?,
        };
        octets.extend([tag as u8, contents.len() as u8]);
        octets.extend(contents);
    }
    Ok(octets)
}

fn holder_typed() -> Typed {
    Typed {
        decode: holder_decode,
        encode: holder_encode,
    }
}

/// An OCTET STRING (CONTAINING Cause): its length, then its octets.
fn payload_decode(octets: &[u8]) -> Result<inspect::Decoded, Error> {
    match octets.split_first() {
        Some((length, contents)) if usize::from(*length) == contents.len() => {
            Ok((hex(contents).into(), true, None))
        }
        _ => Err(decoding("no octets of their length")),
    }
}

fn payload_encode(value: &Value) -> Result<Vec<u8>, Error> {
    let contents = unhex(value.as_str().ok_or_else(|| encoding("no octets"))?)?;
    Ok([&[contents.len() as u8][..], &contents].concat())
}

/// A PRIVATE MESSAGE is its TAC.
fn private_decode(octets: &[u8]) -> Result<inspect::Decoded, Error> {
    let tac = octets
        .iter()
        .fold(0_u64, |n, octet| n << 8 | u64::from(*octet));
    Ok((tree!({"tac": tac}), true, None))
}

fn private_encode(value: &Value) -> Result<Vec<u8>, Error> {
    let tac = value["tac"].as_u64().ok_or_else(|| encoding("no TAC"))?;
    Ok(vec![(tac >> 16) as u8, (tac >> 8) as u8, tac as u8])
}

fn private_typed() -> Typed {
    Typed {
        decode: private_decode,
        encode: private_encode,
    }
}

fn octets_decode(octets: &[u8]) -> Result<inspect::Decoded, Error> {
    Ok((hex(octets).into(), true, None))
}

fn octets_encode(value: &Value) -> Result<Vec<u8>, Error> {
    unhex(value.as_str().ok_or_else(|| encoding("no octets"))?)
}

fn items_decode(octets: &[u8]) -> Result<inspect::Decoded, Error> {
    Ok((Value::Array(entries(octets, "value")?), true, None))
}

fn items_encode(value: &Value) -> Result<Vec<u8>, Error> {
    entries_octets(value, "value")
}

fn item_decode(octets: &[u8]) -> Result<inspect::Decoded, Error> {
    match octets {
        [item] => Ok(((*item).into(), true, None)),
        _ => Err(decoding("no Item")),
    }
}

fn item_encode(value: &Value) -> Result<Vec<u8>, Error> {
    let item = value.as_u64().filter(|item| *item < 256);
    Ok(vec![item.ok_or_else(|| encoding("no Item"))? as u8])
}

// ── The PDUs ────────────────────────────────────────────────────────────────

/// The octets of a list of IEs: each identifier with its criticality and its octets.
fn ies(listed: &[(u16, u8, &[u8])]) -> Vec<u8> {
    let mut octets = Vec::new();
    for (id, criticality, value) in listed {
        octets.extend([(id >> 8) as u8, *id as u8, *criticality, value.len() as u8]);
        octets.extend_from_slice(value);
    }
    octets
}

// ── The decoding ────────────────────────────────────────────────────────────

/// The causes of the toy that hold one another through their extension, `levels` deep: the
/// last one a radio network cause.
fn nested(levels: usize) -> Vec<u8> {
    let mut cause = vec![0, 0];
    for _ in 1..levels {
        let mut outer = vec![3];
        outer.extend(ies(&[(15, 1, &cause)]));
        cause = outer;
    }
    cause
}

// ── The trees ───────────────────────────────────────────────────────────────

/// The octets of the message that `tree` writes.
fn written(tree: &Value) -> Result<Vec<u8>, Error> {
    Ok(inspect::tree_parts(&TOY, tree)?.message)
}

/// A tree that goes as deep as 256 is written, and one deeper is refused.
#[test]
fn a_tree_deeper_than_256_is_not_written() {
    // The value of the cause of level n, written by hand: its IE is 2n deep
    let hand = |levels: usize, last: Value| {
        let mut cause = last;
        for _ in 1..levels {
            cause = tree!({"choice-Extensions": {"Cause": (cause)}});
        }
        tree!({"message": "SetupRequest", "ies": [{"Cause": (cause)}]})
    };
    let deepest = hand(127, tree!({"radioNetwork": "unspecified"}));
    assert!(written(&deepest).is_ok());
    let deeper = hand(129, tree!({"octets": "0000"}));
    assert_eq!(written(&deeper).unwrap_err().kind(), ErrorKind::Value);
    let deeper = hand(130, tree!({"radioNetwork": "unspecified"}));
    assert_eq!(written(&deeper).unwrap_err().kind(), ErrorKind::Value);
    // The tree of 40 levels, 80 deep, is written back
    let octets = ies(&[(15, 1, &nested(40))]);
    let tree = inspect::tree(&TOY, Direction::Initiating, 1, tree!("reject"), &octets);
    assert_eq!(written(&tree).unwrap(), octets);
}

// ── The paths ───────────────────────────────────────────────────────────────

/// A chain of `levels` nodes, each the next of the one before, the last with a TAC; each
/// link as `link` makes it of the node that it holds.
fn chain(levels: usize, link: impl Fn(Value) -> Value) -> Value {
    let mut node = tree!({"tac": "000102"});
    for _ in 1..levels {
        node = link(node);
    }
    node
}

/// The node `levels` deep in a chain whose links `step` takes.
fn deepest(mut node: &Value, levels: usize, step: fn(&Value) -> &Value) -> &Value {
    for _ in 1..levels {
        node = step(node);
    }
    node
}

fn by_next(node: &Value) -> &Value {
    &node["next"]
}

fn by_nodes(node: &Value) -> &Value {
    &node["nodes"][0]
}

/// The JSON Encoding Rules of a value are read as deep as 256, through its members, the
/// types that another names, its lists and its transfers.
#[test]
fn the_json_encoding_rules_are_read_as_deep_as_256() {
    let read = |mut value: Value| {
        TOY.from_jer(&mut value, Shape::Named("Node"));
        value
    };
    // Each next is one deeper: the TAC of the node of level n is n deep
    let next = |node: Value| tree!({"next": (node)});
    let value = read(chain(256, next));
    assert_eq!(deepest(&value, 256, by_next)["tac"], tree!(258));
    let value = read(chain(257, next));
    assert_eq!(deepest(&value, 257, by_next)["tac"], tree!("000102"));
    // Each list of nodes is two deeper: the TAC of level n is 2n - 1 deep
    let nodes = |node: Value| tree!({"nodes": [(node)]});
    let value = read(chain(128, nodes));
    assert_eq!(deepest(&value, 128, by_nodes)["tac"], tree!(258));
    let value = read(chain(129, nodes));
    assert_eq!(deepest(&value, 129, by_nodes)["tac"], tree!("000102"));
    // A type that another names, as far as a well-known type
    let value = read(tree!({"next": {"tac": "000001"}}));
    assert_eq!(value, tree!({"next": {"tac": 1}}));
    let mut holder = tree!({"otherTac": "000003"});
    TOY.from_jer(&mut holder, Shape::Named("Holder"));
    assert_eq!(holder, tree!({"otherTac": 3}));
}

/// A tree that is written is read as deep as 256: through the members, the transfers, the
/// lists and the IEs that a value holds.
#[test]
fn a_written_tree_is_read_as_deep_as_256() {
    let listed = |node: Value| {
        let tree = tree!([{"Node": (node)}]);
        inspect::listed_ies(&TOY, "SetupRequest", &tree)
    };
    let limit = |result: Result<Value, Error>| result.unwrap_err().kind() == ErrorKind::Value;
    // The IE's value is 1 deep, each next one deeper: the TAC of level n is n + 1 deep
    let next = |node: Value| tree!({"next": (node)});
    assert!(listed(chain(255, next)).is_ok());
    assert!(limit(listed(chain(256, next))));
    // Each list of nodes two deeper: the TAC of level n is 2n deep
    let nodes = |node: Value| tree!({"nodes": [(node)]});
    assert!(listed(chain(128, nodes)).is_ok());
    assert!(limit(listed(chain(129, nodes))));
    // A transfer is two deeper, written as its decoded value
    let packed = |node: Value| tree!({"packed": (node)});
    assert!(listed(chain(128, packed)).is_ok());
    assert!(limit(listed(chain(129, packed))));
    // The IE that a cause holds is two deeper: the name of the alternative of the cause of
    // level n is 2n deep
    let causes = |levels: usize, link: &dyn Fn(Value) -> Value| {
        let mut cause = tree!({"nas": "deregister"});
        for _ in 1..levels {
            cause = link(cause);
        }
        inspect::listed_ies(&TOY, "SetupRequest", &tree!([{"Cause": (cause)}]))
    };
    let by_name = |cause: Value| tree!({"choice-Extensions": {"Cause": (cause)}});
    assert!(causes(128, &by_name).is_ok());
    assert!(limit(causes(129, &by_name)));
}

// ── A type of another protocol ──────────────────────────────────────────────

/// The SETUP REQUEST of a Container that carries a Wrapper of Speed 7 and Mode fast.
fn wrapped() -> Vec<u8> {
    ies(&[(18, 1, &ies(&[(15, 0, &[7]), (16, 1, &[1])]))])
}

/// The protocol OTHER, which the register of protocols, of the standard library, holds.
#[cfg(feature = "std")]
mod another_protocol {
    use super::*;

    /// OTHER, a protocol whose IE 40 may carry a Wrapper: a transfer of its protocol IEs,
    /// Speed (INTEGER (0..255), id 15, mandatory, reject) and Mode (ENUMERATED { slow, fast },
    /// id 16, ignore). The Container of TOY may carry a Wrapper as a type of OTHER, whose
    /// IE 15 is no Cause: only the tables of OTHER read it.
    const WRAPPER_IES: &[(u16, Criticality, Presence)] =
        &[(15, Reject, Mandatory), (16, Ignore, Optional)];

    const OTHER_TABLES: Protocol = Protocol {
        name: "OTHER",
        root: "other",
        types: &[("Wrapper", Kind::Sequence(CONTAINER_OF_IES))],
        messages: &[],
        ie_names: &[(15, "Speed"), (16, "Mode"), (40, "Blob")],
        ie_types: &[
            (15, "crate::other::Speed", ""),
            (16, "crate::other::Mode", ""),
            (40, "crate::other::Blob<'a>", ""),
        ],
        extension_types: &[],
        carried: &[(40, "Wrapper", "crate::other::Wrapper")],
        foreign: &[],
        enumerations: &[("Mode", &["slow", "fast"])],
        item_criticalities: &[],
        transfers: &[("Wrapper", "Wrapper", WRAPPER_IES)],
        ie_id: |name| match name {
            _ if inspect::same_name(name, "Speed") => Some(15),
            _ if inspect::same_name(name, "Mode") => Some(16),
            _ if inspect::same_name(name, "Blob") => Some(40),
            _ => None,
        },
        kind_of: |name| (name == "Wrapper").then_some(&Kind::Sequence(CONTAINER_OF_IES)),
        ie_typed: |id| match id {
            15 => Ok(Typed {
                decode: item_decode,
                encode: item_encode,
            }),
            16 => Ok(Typed {
                decode: mode_decode,
                encode: mode_encode,
            }),
            40 => Ok(Typed {
                decode: octets_decode,
                encode: octets_encode,
            }),
            _ => Err(Error::new(
                ErrorKind::UnknownIe,
                format!("no type for IE {id}"),
            )),
        },
        extension_typed: |_, _| None,
        ie_contents: |_| None,
        type_form: |_| None,
        transfer_fields: &[],
        transfer_typed: |_| Err(Error::new(ErrorKind::Path, "no transfer")),
        carried_typed: |name| inspect::same_name(name, "Wrapper").then(ies_typed),
        any_message: ies_typed,
    };
    static OTHER: Protocol = OTHER_TABLES;
    /// SECOND, whose IE 40 may carry a Gauge, a transfer of the IEs of OTHER: a program
    /// registers it beside OTHER.
    static SECOND: Protocol = Protocol {
        name: "SECOND",
        root: "second",
        types: &[("Gauge", Kind::Sequence(CONTAINER_OF_IES))],
        carried: &[(40, "Gauge", "crate::second::Gauge")],
        transfers: &[("Gauge", "Gauge", WRAPPER_IES)],
        kind_of: |name| (name == "Gauge").then_some(&Kind::Sequence(CONTAINER_OF_IES)),
        carried_typed: |name| inspect::same_name(name, "Gauge").then(ies_typed),
        ..OTHER_TABLES
    };

    const MODES: [&str; 2] = ["slow", "fast"];

    fn mode_decode(octets: &[u8]) -> Result<inspect::Decoded, Error> {
        match octets {
            [mode] => MODES
                .get(usize::from(*mode))
                .map(|name| (Value::from(*name), true, None))
                .ok_or_else(|| decoding("no Mode")),
            _ => Err(decoding("no Mode")),
        }
    }

    fn mode_encode(value: &Value) -> Result<Vec<u8>, Error> {
        let mode = MODES.iter().position(|name| Some(*name) == value.as_str());
        Ok(vec![mode.ok_or_else(|| encoding("no Mode"))? as u8])
    }

    /// A tree of TOY writes the Container as a Wrapper of OTHER by the names of OTHER, and
    /// opens it and edits it by the tables of OTHER, once OTHER is registered: twice is
    /// once.
    #[test]
    fn a_type_of_another_protocol_is_read_and_written_by_the_tables_of_that_protocol() {
        inspect::register(&OTHER);
        inspect::register(&OTHER);
        inspect::register(&SECOND);
        let tree = tree!({"message": "SetupRequest", "ies": [
            {"Container": {"Wrapper": {"protocolIEs": [{"Speed": 7}, {"Mode": "fast"}]}}},
        ]});
        let parts = inspect::tree_parts(&TOY, &tree).unwrap();
        assert_eq!(parts.message, wrapped());
        // and as a Gauge of SECOND, which is registered too
        let gauge = tree!({"message": "SetupRequest", "ies": [
            {"Container": {"Gauge": {"protocolIEs": [{"Speed": 7}, {"Mode": "fast"}]}}},
        ]});
        assert_eq!(
            inspect::tree_parts(&TOY, &gauge).unwrap().message,
            wrapped()
        );
        // Its octets
        let octets = tree!({"octets": hex(&ies(&[(15, 0, &[7]), (16, 1, &[1])]))});
        let tree =
            tree!({"message": "SetupRequest", "ies": [{"Container": {"Wrapper": (octets)}}]});
        assert_eq!(inspect::tree_parts(&TOY, &tree).unwrap().message, wrapped());
        // A name of TOY is no IE of OTHER
        let cause = tree!({"message": "SetupRequest", "ies": [
            {"Container": {"Wrapper": {"protocolIEs": [{"Cause": {"misc": null}}]}}},
        ]});
        assert_eq!(
            inspect::tree_parts(&TOY, &cause).err().unwrap().kind(),
            ErrorKind::UnknownIe
        );

        // The paths go through the name of the type into the IEs of OTHER
        let mode = "/toy/Container/Wrapper/Mode";
        assert_eq!(inspect::check_path(&TOY, "SetupRequest", mode), Ok(()));
        let names = inspect::enumerated_at(&TOY, "SetupRequest", mode).unwrap();
        assert_eq!(names, Some(&MODES[..]));
        let mut tree = inspect::tree(&TOY, Direction::Initiating, 1, tree!("reject"), &wrapped());
        let speed = "/toy/Container/Wrapper/Speed";
        assert_eq!(
            inspect::select(&TOY, &tree, speed).unwrap_err().kind(),
            ErrorKind::Path
        );
        inspect::open(&TOY, &mut tree, speed).unwrap();
        assert_eq!(inspect::select(&TOY, &tree, speed).unwrap(), [&tree!(7)]);
        assert_eq!(
            tree["ies"][0]["Container"],
            tree!({"Wrapper": {"protocolIEs": [{"Speed": 7}, {"Mode": "fast"}]}})
        );
        // Each value that the listing has, named by OTHER, is selected by its path
        let listed = inspect::paths(&TOY, &tree);
        assert!(
            listed
                .iter()
                .any(|(path, value)| path == speed && *value == tree!(7)),
            "{listed:?}"
        );
        for (path, value) in &listed {
            assert_eq!(
                inspect::select(&TOY, &tree, path).unwrap(),
                [value],
                "{path}"
            );
        }
        assert_eq!(written(&tree).unwrap(), wrapped());
        // An edit is written by the type of OTHER
        inspect::set(&TOY, &mut tree, mode, tree!("medium")).unwrap();
        assert!(written(&tree).is_err());
        inspect::set(&TOY, &mut tree, mode, tree!("slow")).unwrap();
        let edited = ies(&[(18, 1, &ies(&[(15, 0, &[7]), (16, 1, &[0])]))]);
        assert_eq!(written(&tree).unwrap(), edited);
    }
}

/// The type of a protocol that is not registered is refused, and the error says the call
/// that registers it.
#[test]
fn a_type_of_a_protocol_that_is_not_registered_is_refused_with_the_call_that_registers_it() {
    const CALL: &str = "GhostBlob is a type of GHOST: call oxirush_ghost::inspect::register()";
    let tree = tree!({"message": "SetupRequest", "ies": [{"Container": {"GhostBlob": {"a": 1}}}]});
    let error = inspect::tree_parts(&TOY, &tree).err().unwrap();
    assert!(error.to_string().contains(CALL), "{error}");
    let path = "/toy/Container/GhostBlob/a";
    let error = inspect::check_path(&TOY, "SetupRequest", path).unwrap_err();
    assert!(error.to_string().contains(CALL), "{error}");
    let mut tree = inspect::tree(&TOY, Direction::Initiating, 1, tree!("reject"), &wrapped());
    inspect::open(&TOY, &mut tree, "/toy/Container/GhostBlob").unwrap();
    let error = inspect::select(&TOY, &tree, path).unwrap_err();
    assert!(error.to_string().contains(CALL), "{error}");
}
