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

//! The trees and the paths of a protocol of a few types with codecs of their own,
//! which stands for the protocol crates: messages and a transfer of protocol IEs, an IE that
//! extends a type, one whose octets contain a type, one whose octets may carry a type, a list
//! of single containers, a bit string and a well-known type.
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

/// A Holder of a cause and of a transfer of a cause.
const HOLDER_OCTETS: &[u8] = &[0, 2, 1, 0, 1, 2, 0, 1];

/// The message of a SETUP REQUEST of a Cause, a Holder, a Container, Items and a Payload.
fn setup_request() -> Vec<u8> {
    ies(&[
        (15, 1, &[0, 0]),
        (16, 0, HOLDER_OCTETS),
        (18, 1, &[0, 2, 1, 1]),
        (20, 0, &ies(&[(30, 0, &[7])])),
        (21, 1, &[2, 1, 0]),
    ])
}

fn setup_view() -> Value {
    inspect::view(
        &TOY,
        Direction::Initiating,
        1,
        tree!("reject"),
        &setup_request(),
    )
    .unwrap()
}

// ── The decoding ────────────────────────────────────────────────────────────

/// What a tree is decoded through has each IE as its type decodes it, beside what it was
/// received as, and what its octets contain as a transfer.
#[test]
fn each_ie_is_decoded_with_what_its_octets_contain() {
    let view = setup_view();
    let ies = &view["message"]["protocolIEs"];
    assert_eq!(view["procedure_code"], tree!(1));
    assert_eq!(view["direction"], tree!("InitiatingMessage"));
    assert_eq!(view["criticality"], tree!("reject"));
    assert_eq!(view["_raw_message"], tree!((hex(&setup_request()))));
    assert!(view.get("_lossy").is_none());
    assert_eq!(
        ies[0],
        tree!({"id": 15, "criticality": "ignore", "value": {"radioNetwork": "unspecified"},
               "_raw_value": "0000", "_original_id": 15, "_ie_name": "Cause"})
    );
    assert_eq!(
        ies[1]["value"],
        tree!({"cause": {"nas": "normal-release"}, "transfer":
               {"_raw_transfer": "0001", "decoded": {"radioNetwork": "user-inactivity"}}})
    );
    // The octets of the Container are shown as they are, until they are opened
    assert_eq!(ies[2]["value"], tree!("00020101"));
    assert_eq!(
        ies[3]["value"],
        tree!([{"id": 30, "criticality": "reject", "value": 7, "_raw_value": "07",
                "_original_id": 30, "_ie_name": "Item"}])
    );
    assert_eq!(
        ies[4]["value"],
        tree!({"_raw_transfer": "0100", "decoded": {"nas": "normal-release"}})
    );
    assert_eq!(ies[4]["_raw_value"], tree!("020100"));
}

/// What a type has no place for, a value in another form than the canonical one, and
/// octets that are no value: a mark beside the value, or the octets with why.
#[test]
fn what_a_value_has_no_place_for_is_marked_and_what_did_not_decode_kept() {
    let octets = ies(&[
        (15, 1, &[0, 0, 9]),
        (21, 1, &[3, 1, 0, 9]),
        (16, 0, &[0, 3, 0, 1, 9]),
    ]);
    let view = inspect::view(&TOY, Direction::Initiating, 1, tree!("reject"), &octets).unwrap();
    let listed = &view["message"]["protocolIEs"];
    assert_eq!(listed[0]["_lossy"], tree!(true));
    assert_eq!(listed[0]["value"], tree!({"radioNetwork": "unspecified"}));
    assert_eq!(listed[1]["value"]["_lossy"], tree!(true));
    assert!(listed[1].get("_lossy").is_none());
    assert_eq!(listed[2]["_lossy"], tree!(true));
    // A message whose type has no place for what it was received with
    let mut lossy = vec![0x80];
    lossy.extend(ies(&[(15, 1, &[0, 0])]));
    let view = inspect::view(&TOY, Direction::Initiating, 1, tree!("reject"), &lossy).unwrap();
    assert_eq!(view["_lossy"], tree!(true));
    // Octets that are no value, and a value in another form than the canonical one
    let octets = ies(&[(15, 1, &[0xff]), (21, 1, &[2, 0xfe, 0]), (15, 1, &[0xfe])]);
    let view = inspect::view(&TOY, Direction::Initiating, 1, tree!("reject"), &octets).unwrap();
    let listed = &view["message"]["protocolIEs"];
    assert_eq!(listed[0]["value"], tree!("ff"));
    assert_eq!(listed[0]["_decode_error"], tree!("no Cause"));
    assert_eq!(
        listed[1]["value"],
        tree!({"_raw_transfer": "fe00",
               "_decode_error": "a Cause in another form than the canonical one"})
    );
    assert_eq!(listed[2]["value"], tree!("fe"));
    assert!(listed[2]["_decode_error"].is_string());
}

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

/// The IE of the cause `level` deep in the view of a message whose first IE is the cause
/// of `nested`.
fn at_level(view: &Value, level: usize) -> &Value {
    let mut ie = &view["message"]["protocolIEs"][0];
    for _ in 1..level {
        ie = &ie["value"]["choice-Extensions"];
    }
    ie
}

/// An IE is decoded as deep as 64, and one deeper keeps its octets with why.
#[test]
fn an_ie_deeper_than_the_inspection_keeps_its_octets() {
    let octets = ies(&[(15, 1, &nested(40))]);
    let view = inspect::view(&TOY, Direction::Initiating, 1, tree!("reject"), &octets).unwrap();
    // The IE of level n is 2n deep: that of level 32 is 64 deep
    assert!(at_level(&view, 32).get("_decode_error").is_none());
    assert!(at_level(&view, 32)["value"].is_object());
    let too_deep = at_level(&view, 33);
    assert_eq!(
        too_deep["_decode_error"],
        tree!("inspection nesting exceeds 64")
    );
    assert!(too_deep["value"].is_string());
    // The tree has its octets, and gives them back
    let tree = inspect::tree(&TOY, Direction::Initiating, 1, tree!("reject"), &octets);
    let mut cause = &tree["ies"][0]["Cause"];
    for _ in 1..33 {
        cause = &cause["choice-Extensions"]["Cause"];
    }
    assert_eq!(cause, &tree!({"octets": (hex(&nested(8)))}));
    assert_eq!(written(&tree).unwrap(), octets);
}

/// An IE twice as deep in a list of single containers is decoded as deep as 64 too: the
/// entries of a list count in the depth.
#[test]
fn the_entries_of_a_list_count_in_the_depth_of_the_inspection() {
    // The causes in the list of the Items: the IE of level n is 2n + 2 deep
    let octets = ies(&[(20, 0, &ies(&[(15, 1, &nested(40))]))]);
    let view = inspect::view(&TOY, Direction::Initiating, 1, tree!("reject"), &octets).unwrap();
    let mut ie = &view["message"]["protocolIEs"][0]["value"][0];
    for _ in 1..31 {
        ie = &ie["value"]["choice-Extensions"];
    }
    assert!(ie.get("_decode_error").is_none(), "{ie}");
    let too_deep = &ie["value"]["choice-Extensions"];
    assert_eq!(
        too_deep["_decode_error"],
        tree!("inspection nesting exceeds 64")
    );
}

// ── The trees ───────────────────────────────────────────────────────────────

/// The octets of the message that `tree` writes.
fn written(tree: &Value) -> Result<Vec<u8>, Error> {
    Ok(inspect::tree_parts(&TOY, tree)?.message)
}

fn setup_tree() -> Value {
    inspect::tree(
        &TOY,
        Direction::Initiating,
        1,
        tree!("reject"),
        &setup_request(),
    )
}

/// The tree of a PDU writes the octets received, also for what its types have no place
/// for, and for what did not decode, which it has as their octets.
#[test]
fn a_tree_writes_the_octets_received() {
    assert_eq!(written(&setup_tree()).unwrap(), setup_request());
    let octets = ies(&[
        (15, 1, &[0, 0, 9]),
        (21, 1, &[3, 1, 0, 9]),
        (16, 0, &[0, 3, 0, 1, 9]),
        (15, 1, &[0xff]),
        (21, 1, &[2, 0xfe, 0]),
        (15, 1, &[0xfe]),
    ]);
    let tree = inspect::tree(&TOY, Direction::Initiating, 1, tree!("reject"), &octets);
    assert_eq!(
        tree["ies"],
        tree!([
            {"Cause": {"octets": "000009"}},
            {"Payload": {"octets": "03010009"}},
            {"Holder": {"octets": "0003000109"}},
            {"Cause": {"octets": "ff"}},
            {"Payload": {"octets": "02fe00"}},
            {"Cause": {"octets": "fe"}},
        ])
    );
    assert_eq!(written(&tree).unwrap(), octets);
    // and a message that has no place for its own
    let mut lossy = vec![0x80];
    lossy.extend(ies(&[(15, 1, &[0, 0])]));
    let tree = inspect::tree(&TOY, Direction::Successful, 1, tree!("ignore"), &lossy);
    let parts = inspect::tree_parts(&TOY, &tree).unwrap();
    assert_eq!(
        (parts.direction, parts.message),
        (Direction::Successful, lossy)
    );
}

/// An edit is encoded in the IE that it changed, and the IEs that it did not change are
/// sent as they were received, whatever they hold.
#[test]
fn an_edit_is_encoded_in_the_ie_that_it_changed() {
    let mut tree = setup_tree();
    inspect::set(&TOY, &mut tree, "/toy/Cause/nas", tree!("deregister")).unwrap();
    let path = "/toy/Holder/transfer/nas";
    inspect::set(&TOY, &mut tree, path, tree!("normal-release")).unwrap();
    let path = "/toy/Payload/radioNetwork";
    inspect::set(&TOY, &mut tree, path, tree!("user-inactivity")).unwrap();
    let expected = ies(&[
        (15, 1, &[1, 1]),
        (16, 0, &[0, 2, 1, 0, 1, 2, 1, 0]),
        (18, 1, &[0, 2, 1, 1]),
        (20, 0, &ies(&[(30, 0, &[7])])),
        (21, 1, &[2, 0, 1]),
    ]);
    assert_eq!(written(&tree).unwrap(), expected);
    // An IE that is its octets keeps them beside an edit of another, and is no value to
    // edit itself
    let octets = ies(&[
        (15, 1, &[0, 0, 9]),
        (21, 1, &[3, 1, 0, 9]),
        (15, 1, &[1, 0]),
    ]);
    let mut tree = inspect::tree(&TOY, Direction::Initiating, 1, tree!("reject"), &octets);
    let error = inspect::set(&TOY, &mut tree, "/toy/Payload/nas", tree!("deregister"));
    assert_eq!(error.unwrap_err().kind(), ErrorKind::UndecodableIe);
    inspect::set(&TOY, &mut tree, "/toy/2/nas", tree!("deregister")).unwrap();
    let expected = ies(&[
        (15, 1, &[0, 0, 9]),
        (21, 1, &[3, 1, 0, 9]),
        (15, 1, &[1, 1]),
    ]);
    assert_eq!(written(&tree).unwrap(), expected);
    // Its value takes the place of its octets
    inspect::set(&TOY, &mut tree, "/toy/0", tree!({"nas": "deregister"})).unwrap();
    assert_eq!(&written(&tree).unwrap()[..6], ies(&[(15, 1, &[1, 1])]));
    // A message that is its octets has no IEs to edit
    let mut lossy = vec![0x80];
    lossy.extend(ies(&[(15, 1, &[0, 0])]));
    let mut tree = inspect::tree(&TOY, Direction::Initiating, 1, tree!("reject"), &lossy);
    let error = inspect::set(&TOY, &mut tree, "/toy/Cause/nas", tree!("deregister"));
    assert_eq!(error.unwrap_err().kind(), ErrorKind::Path);
}

/// A tree written by hand is encoded from its values: a transfer as its decoded value or
/// as its octets, and a type that the octets of an IE carry by its name.
#[test]
fn a_tree_written_by_hand_is_encoded_from_its_values() {
    let message = |ies: Value| tree!({"message": "SetupRequest", "ies": (ies)});
    let holder = |transfer: Value| {
        message(tree!([{"Holder": {"cause": {"nas": "deregister"}, "transfer": (transfer)}}]))
    };
    let expected = ies(&[(16, 0, &[0, 2, 1, 1, 1, 2, 0, 1])]);
    for transfer in [
        tree!({"radioNetwork": "user-inactivity"}),
        tree!({"octets": "0001"}),
    ] {
        let octets = written(&holder(transfer.clone()));
        assert_eq!(octets.unwrap(), expected, "{transfer}");
    }
    for refused in [
        tree!({"decoded": {"radioNetwork": "unspecified"}}),
        tree!({"radioNetwork": "unspecified", "octets": "0000"}),
    ] {
        let error = written(&holder(refused.clone())).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Value, "{refused}: {error}");
    }
    // The octets of the Container, written as an LteBlob, and as another name
    let carried = message(tree!([{"Container": {"LteBlob": {"cause": {"nas": "deregister"}}}}]));
    assert_eq!(written(&carried).unwrap(), ies(&[(18, 1, &[0, 2, 1, 1])]));
    let other = message(tree!([{"Container": {"Other": "0101"}}]));
    assert_eq!(written(&other).unwrap_err().kind(), ErrorKind::Value);
    // An entry that is no IE
    let empty = message(tree!([{"criticality": "ignore"}]));
    assert_eq!(written(&empty).unwrap_err().kind(), ErrorKind::Value);
}

/// `open` shows the octets of the Container as the LteBlob that they carry, which is
/// written back, as it was or edited.
#[test]
fn opened_octets_are_written_back_as_they_were_or_edited() {
    let mut tree = setup_tree();
    inspect::open(&TOY, &mut tree, "/toy/Container/LteBlob").unwrap();
    assert_eq!(
        tree["ies"][2],
        tree!({"Container": {"LteBlob": {"cause": {"nas": "deregister"}}}})
    );
    assert_eq!(written(&tree).unwrap(), setup_request());
    let path = "/toy/Container/LteBlob/cause/nas";
    inspect::set(&TOY, &mut tree, path, tree!("normal-release")).unwrap();
    let mut expected = setup_request();
    let at = expected
        .windows(8)
        .position(|ie| ie == [0, 18, 1, 4, 0, 2, 1, 1])
        .unwrap();
    expected[at + 7] = 0;
    assert_eq!(written(&tree).unwrap(), expected);
}

/// Octets that carry a Holder are opened as one, its extension decoded as the Holder
/// gives it its type.
#[test]
fn opened_octets_have_the_type_that_they_carry() {
    let carried = [&[6, 6][..], &ies(&[(17, 1, &[1, 1])])].concat();
    let octets = ies(&[(18, 1, &carried)]);
    let mut tree = inspect::tree(&TOY, Direction::Initiating, 1, tree!("reject"), &octets);
    inspect::open(&TOY, &mut tree, "/toy/Container/LteBlob").unwrap();
    let opened = &tree["ies"][0]["Container"]["LteBlob"];
    assert_eq!(
        opened["iE-Extensions"][0],
        tree!({"ExtendedCause": {"nas": "deregister"}, "criticality": "ignore"})
    );
    assert_eq!(written(&tree).unwrap(), octets);
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

/// Each message has the type of its direction and its procedure code, and a procedure that
/// the tables lack in that direction has that of any message.
#[test]
fn each_message_has_the_type_of_its_procedure() {
    let octets = [b"R".as_slice(), &ies(&[(15, 1, &[1, 0])])].concat();
    let tree = inspect::tree(&TOY, Direction::Initiating, 2, tree!("ignore"), &octets);
    assert_eq!(tree["ies"][0], tree!({"Cause": {"nas": "normal-release"}}));
    assert_eq!(written(&tree).unwrap(), octets);
    // Not in another direction or of another code
    for (direction, code) in [(Direction::Successful, 2), (Direction::Initiating, 9)] {
        let view = inspect::view(&TOY, direction, code, tree!("ignore"), &octets);
        assert_eq!(
            view.unwrap_err().kind(),
            ErrorKind::Decode,
            "{direction} {code}"
        );
        let tree = inspect::tree(&TOY, direction, code, tree!("ignore"), &octets);
        assert_eq!(tree["octets"], tree!((hex(&octets))), "{direction} {code}");
    }
}

/// A tree has the name of the message, its IEs by name, and its criticality where it is
/// not that of its procedure; a procedure that the tables lack has its code and its
/// direction, and a message that is not read as IEs its octets.
#[test]
fn a_tree_names_the_message_and_its_ies() {
    let tree = inspect::tree(
        &TOY,
        Direction::Initiating,
        1,
        tree!("reject"),
        &setup_request(),
    );
    assert_eq!(
        tree,
        tree!({"message": "SetupRequest", "ies": [
            {"Cause": {"radioNetwork": "unspecified"}},
            {"Holder": {"cause": {"nas": "normal-release"}, "transfer": {"radioNetwork": "user-inactivity"}}},
            {"Container": "00020101"},
            {"Items": [{"Item": 7}]},
            {"Payload": {"nas": "normal-release"}},
        ]})
    );
    let tree = inspect::tree(
        &TOY,
        Direction::Initiating,
        1,
        tree!("ignore"),
        &setup_request(),
    );
    assert_eq!(tree["criticality"], tree!("ignore"));
    let release = [b"R".as_slice(), &ies(&[(15, 1, &[1, 0])])].concat();
    let tree = inspect::tree(&TOY, Direction::Initiating, 2, tree!("ignore"), &release);
    assert_eq!(
        tree,
        tree!({"message": "Release", "ies": [{"Cause": {"nas": "normal-release"}}]})
    );
    let response = ies(&[(15, 0, &[1, 0])]);
    let tree = inspect::tree(&TOY, Direction::Successful, 1, tree!("reject"), &response);
    assert_eq!(
        tree,
        tree!({"message": "SetupResponse", "ies": [{"Cause": {"nas": "normal-release"},
               "criticality": "reject"}]})
    );
    let tree = inspect::tree(&TOY, Direction::Unsuccessful, 7, tree!("notify"), &response);
    assert_eq!(
        tree,
        tree!({"procedure_code": 7, "direction": "UnsuccessfulOutcome", "ies":
               [{"Cause": {"nas": "normal-release"}, "criticality": "reject"}],
               "criticality": "notify"})
    );
    let lossy = [&[0x80][..], &response].concat();
    let tree = inspect::tree(&TOY, Direction::Successful, 1, tree!("ignore"), &lossy);
    assert_eq!(
        tree,
        tree!({"message": "SetupResponse", "octets": (hex(&lossy)), "criticality": "ignore"})
    );
}

/// The parts that a tree writes: the message by its name, or a procedure that the tables
/// lack by its code and its direction, with the criticality of its procedure unless the
/// tree says another, and its IEs by name or its octets.
#[test]
fn the_parts_that_a_tree_writes() {
    let tree = inspect::tree(
        &TOY,
        Direction::Initiating,
        1,
        tree!("reject"),
        &setup_request(),
    );
    let parts = inspect::tree_parts(&TOY, &tree).unwrap();
    assert_eq!(
        (parts.direction, parts.code, parts.criticality),
        (Direction::Initiating, 1, tree!("reject"))
    );
    assert_eq!(parts.message, setup_request());
    let release = tree!({"message": "release", "ies": [{"Cause": {"nas": "deregister"}}]});
    let parts = inspect::tree_parts(&TOY, &release).unwrap();
    assert_eq!((parts.code, parts.criticality), (2, tree!("ignore")));
    assert_eq!(
        parts.message,
        [b"R".as_slice(), &ies(&[(15, 1, &[1, 1])])].concat()
    );
    let octets = tree!({"message": "SetupResponse", "octets": "000f010201 00"});
    assert_eq!(
        inspect::tree_parts(&TOY, &octets).err().unwrap().kind(),
        ErrorKind::Value
    );
    let octets = tree!({"message": "SetupResponse", "octets": "000f01020100"});
    let parts = inspect::tree_parts(&TOY, &octets).unwrap();
    assert_eq!(parts.direction, Direction::Successful);
    assert_eq!(parts.message, ies(&[(15, 1, &[1, 0])]));
    let unknown = tree!({"procedure_code": 7, "direction": "UnsuccessfulOutcome",
                         "criticality": "notify", "ies": [{"Cause": {"nas": "deregister"},
                         "criticality": "ignore"}]});
    let parts = inspect::tree_parts(&TOY, &unknown).unwrap();
    assert_eq!(
        (parts.direction, parts.code, parts.criticality),
        (Direction::Unsuccessful, 7, tree!("notify"))
    );
    assert_eq!(parts.message, ies(&[(15, 1, &[1, 1])]));
    for refused in [
        tree!({"message": "SetupRequest", "ies": {}}),
        tree!({"message": "SetupRequest", "ies": [], "octets": "00"}),
        tree!({"message": "SetupRequest", "other": []}),
        tree!({"procedure_code": 1, "direction": "Sideways", "ies": []}),
    ] {
        let error = inspect::tree_parts(&TOY, &refused).err().unwrap();
        assert_eq!(error.kind(), ErrorKind::Value, "{refused}: {error}");
    }
}

/// A view in the JSON Encoding Rules is a tree with its values of a well-known type in
/// their readable form; a view without a procedure code is refused.
#[test]
fn a_view_in_the_json_encoding_rules_is_read_as_a_tree() {
    let jer = tree!({"procedure_code": 1, "direction": "InitiatingMessage", "criticality": "reject",
                     "message": {"protocolIEs": [{"id": 16, "criticality": "reject",
                                                  "value": {"tac": "000102"}}]}});
    let tree = inspect::tree_of_jer(&TOY, &jer).unwrap();
    assert_eq!(
        tree,
        tree!({"message": "SetupRequest", "ies": [{"Holder": {"tac": 258}}]})
    );
    let mut without = jer.clone();
    without["procedure_code"] = Value::Null;
    let error = inspect::tree_of_jer(&TOY, &without).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Value);
}

/// A message without IEs has the members of its type, which the JSON Encoding Rules write
/// as they write them, and a tree has its octets.
#[test]
fn a_message_without_ies_has_the_members_of_its_type() {
    let view = inspect::view(&TOY, Direction::Initiating, 3, tree!("ignore"), &[0, 1, 2]).unwrap();
    assert_eq!(view["message"], tree!({"tac": 258}));
    let tree = inspect::tree(&TOY, Direction::Initiating, 3, tree!("ignore"), &[0, 1, 2]);
    assert_eq!(
        tree,
        tree!({"message": "PrivateMessage", "octets": "000102"})
    );
    let mut jer = view.clone();
    jer["message"]["tac"] = tree!("000102");
    assert_eq!(inspect::tree_of_jer(&TOY, &jer).unwrap(), tree);
}

/// A transfer that contains a Holder has the extensions of a Holder, received or written:
/// they are read, edited and written as the Holder gives them their type.
#[test]
fn a_transfer_of_a_holder_has_its_extensions() {
    let inner = [&[6, 6][..], &ies(&[(17, 1, &[1, 1])])].concat();
    let holder = [&[7, inner.len() as u8][..], &inner].concat();
    let octets = ies(&[(16, 0, &holder)]);
    let mut tree = inspect::tree(&TOY, Direction::Initiating, 1, tree!("reject"), &octets);
    assert_eq!(written(&tree).unwrap(), octets);
    let path = "/toy/Holder/inner/iE-Extensions/ExtendedCause/nas";
    assert_eq!(
        inspect::select(&TOY, &tree, path),
        Ok(vec![&tree!("deregister")])
    );
    inspect::set(&TOY, &mut tree, path, tree!("normal-release")).unwrap();
    let mut edited = octets.clone();
    *edited.last_mut().unwrap() = 0;
    assert_eq!(written(&tree).unwrap(), edited);
    // Written by hand
    let written_by_hand = tree!({"message": "SetupRequest", "ies": [{"Holder": {"inner":
        {"iE-Extensions": [{"ExtendedCause": {"nas": "deregister"}, "criticality": "ignore"}]}}}]});
    assert_eq!(written(&written_by_hand).unwrap(), octets);
}

// ── The paths ───────────────────────────────────────────────────────────────

/// A tree of a SETUP REQUEST as the paths read it, written by hand.
fn written_tree() -> Value {
    tree!({"message": "SetupRequest", "ies": [
        {"Cause": {"radioNetwork": "unspecified"}},
        {"Holder": {
            "cause": {"nas": "deregister"},
            "transfer": {"radioNetwork": "user-inactivity"},
            "setup": {"protocolIEs": [{"Cause": {"nas": "normal-release"}}]},
            "flags": {"value": "80", "length": 1},
            "items": [{"Item": 7}],
            "causes": [{"nas": "deregister"}, {"radioNetwork": "unspecified"}],
            "iE-Extensions": [{"ExtendedCause": {"nas": "deregister"}, "criticality": "ignore"}],
        }},
        {"Nas": {"nas": "deregister"}},
        {"99": {"abc": 1, "list": [{"Cause": {"octets": "0000"}}]}, "criticality": "ignore"},
    ]})
}

fn select<'v>(tree: &'v Value, path: &str) -> Result<Vec<&'v Value>, Error> {
    inspect::select(&TOY, tree, path)
}

fn checked(path: &str) -> Result<(), Error> {
    inspect::check_path(&TOY, "SetupRequest", path)
}

fn names_at(message: &str, path: &str) -> Result<Option<&'static [&'static str]>, Error> {
    inspect::enumerated_at(&TOY, message, path)
}

const NAS: &[&str] = &["normal-release", "deregister"];

/// What is no path, and a path of more segments than a path has, are refused as such.
#[test]
fn what_is_no_path_is_refused_as_such() {
    let tree = written_tree();
    assert_eq!(
        select(&tree, "toy/Cause").unwrap_err().kind(),
        ErrorKind::Path
    );
    let long = "/toy/Cause".to_string() + &"/a".repeat(70);
    assert_eq!(select(&tree, &long).unwrap_err().kind(), ErrorKind::Path);
}

/// A tree without its IEs is read by the tables of its message: the object set of its
/// message says which IEs it can have.
#[test]
fn a_tree_without_its_ies_is_read_by_the_tables_of_its_message() {
    let tree = tree!({"message": "SetupResponse"});
    let error = select(&tree, "/toy/Holder").unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Path);
    assert!(
        error
            .to_string()
            .starts_with("\"Holder\" is not an IE of SetupResponse, which has Cause"),
        "{error}"
    );
    assert_eq!(select(&tree, "/toy/Cause/nas"), Ok(vec![]));
}

/// The value of an IE has the type of its identifier, of the type that its octets contain,
/// or of the one that the type which its extension container extends gives it there and
/// nowhere else.
#[test]
fn the_value_of_an_ie_has_the_type_of_its_identifier_and_its_container() {
    assert_eq!(names_at("SetupRequest", "/toy/Cause/nas"), Ok(Some(NAS)));
    assert_eq!(names_at("SetupRequest", "/toy/Payload/nas"), Ok(Some(NAS)));
    let extension = "/toy/Holder/iE-Extensions/ExtendedCause/nas";
    assert_eq!(names_at("SetupRequest", extension), Ok(Some(NAS)));
    let flags = "/toy/Holder/iE-Extensions/ExtendedFlags/value";
    assert_eq!(names_at("SetupRequest", flags), Ok(None));
    assert_eq!(checked(flags), Ok(()));
    // An IE of an extension, where nothing is extended, has no type
    assert_eq!(names_at("SetupRequest", "/toy/@id=17/nas"), Ok(None));
    assert_eq!(checked("/toy/Holder/iE-Extensions/ExtendedCause"), Ok(()));
}

/// The IEs of a transfer of protocol IEs are those of its object set, and go by name in
/// the place of the transfer.
#[test]
fn the_ies_of_a_transfer_are_those_of_its_object_set() {
    let error = checked("/toy/Holder/setup/Holder").unwrap_err();
    assert!(
        error
            .to_string()
            .starts_with("\"Holder\" is not an IE of SetupTransfer, which has Cause"),
        "{error}"
    );
    assert_eq!(checked("/toy/Holder/setup/Cause/nas"), Ok(()));
    let tree = written_tree();
    assert_eq!(
        select(&tree, "/toy/Holder/setup/Cause/nas"),
        Ok(vec![&tree!("normal-release")])
    );
    assert_eq!(
        select(&tree, "/toy/Holder/setup/protocolIEs/0/nas"),
        Ok(vec![&tree!("normal-release")])
    );
}

/// The name of an IE, written as its name is or otherwise, stays on its value once; a
/// member of its value of that name is that member, unless the name is written as the
/// IE's; another IE is refused with the one that the entry is.
#[test]
fn the_name_of_an_ie_stays_on_its_value() {
    let tree = written_tree();
    let cause = &tree["ies"][0]["Cause"];
    assert_eq!(select(&tree, "/toy/0/Cause"), Ok(vec![cause]));
    assert_eq!(select(&tree, "/toy/0/cause"), Ok(vec![cause]));
    assert_eq!(
        select(&tree, "/toy/0/Cause/radioNetwork"),
        Ok(vec![&tree!("unspecified")])
    );
    let error = select(&tree, "/toy/0/Holder").unwrap_err();
    assert!(
        error
            .to_string()
            .starts_with("this IE is Cause, not Holder: "),
        "{error}"
    );
    assert!(select(&tree, "/toy/Cause/Cause/Cause").is_err());
    assert_eq!(
        select(&tree, "/toy/Nas/nas"),
        Ok(vec![&tree!("deregister")])
    );
    assert_eq!(
        select(&tree, "/toy/Nas/Nas"),
        Ok(vec![&tree["ies"][2]["Nas"]])
    );
}

/// The value of an IE that the tables do not have has the members that the tree gives it,
/// and the next segment after an unknown IE is read by what it names.
#[test]
fn an_unknown_ie_has_the_members_that_the_tree_gives_it() {
    let refused = |path: &str| checked(path).unwrap_err().kind();
    assert_eq!(
        refused("/toy/Holder/items/Item/misspelled"),
        ErrorKind::Path
    );
    let extension = "/toy/Holder/iE-Extensions/ExtendedCause/misspelled";
    assert_eq!(refused(extension), ErrorKind::Path);
    // An entry by its position is any IE, whose value may have any member
    assert_eq!(checked("/toy/Holder/items/0/misspelled"), Ok(()));
    assert_eq!(checked("/toy/Holder/iE-Extensions/0/misspelled"), Ok(()));
    assert_eq!(checked("/toy/Cause/Cause/radioNetwork"), Ok(()));
    let error = checked("/toy/Cause/Holder").unwrap_err();
    assert!(
        error
            .to_string()
            .starts_with("this IE is Cause, not Holder: "),
        "{error}"
    );
    // An IE that the tables do not have is read as the tree has it
    assert_eq!(checked("/toy/@id=99/anything"), Ok(()));
}

/// A name that a value cannot have is refused with what it has: a tree, a message of the
/// tables, and a bit string.
#[test]
fn a_name_that_a_value_cannot_have_is_refused_with_what_it_has() {
    let tree = written_tree();
    let error = |tree: &Value, path: &str| select(tree, path).unwrap_err().to_string();
    assert_eq!(
        error(&tree, "/misspelled"),
        "\"misspelled\" is not a member of the tree of a PDU, which has message, \
         procedure_code, direction, criticality, ies, octets, at /misspelled"
    );
    assert_eq!(
        error(&tree, "/toy/Holder/flags/misspelled"),
        "\"misspelled\" is not a member of a bit string, which has value, length, at \
         /toy/Holder/flags/misspelled"
    );
}

/// A name that no IE or message has says the nearest names, where some are near.
#[test]
fn a_name_that_nothing_has_says_the_nearest() {
    let tree = written_tree();
    assert_eq!(
        select(&tree, "/toy/Holdr").unwrap_err().to_string(),
        "\"Holdr\" is not an IE of TOY: the nearest name is Holder, at /toy/Holdr"
    );
    assert_eq!(
        select(&tree, "/toy/Caus").unwrap_err().to_string(),
        "\"Caus\" is not an IE of TOY: the nearest names are Cause and Nas, at /toy/Caus"
    );
    assert_eq!(
        select(&tree, "/toy/Zzzzzzzz").unwrap_err().to_string(),
        "\"Zzzzzzzz\" is not an IE of TOY, at /toy/Zzzzzzzz"
    );
    let error = inspect::check_path(&TOY, "SetupRequst", "/toy/Cause").unwrap_err();
    assert_eq!(
        error.to_string(),
        "\"SetupRequst\" is not a message of TOY: the nearest name is SetupRequest"
    );
}

/// A member is named whatever the case of its name, also in a value that no table
/// describes, where a member that is not there is refused and one that an edit sets is
/// added.
#[test]
fn a_member_is_named_whatever_its_case() {
    let mut tree = written_tree();
    assert_eq!(
        select(&tree, "/toy/Cause/RadioNetwork"),
        Ok(vec![&tree!("unspecified")])
    );
    assert_eq!(select(&tree, "/toy/@id=99/abc"), Ok(vec![&tree!(1)]));
    assert_eq!(select(&tree, "/toy/@id=99/ABC"), Ok(vec![&tree!(1)]));
    assert_eq!(
        select(&tree, "/toy/@id=99/list/0"),
        Ok(vec![&tree["ies"][3]["99"]["list"][0]])
    );
    let error = select(&tree, "/toy/@id=99/missing").unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Path);
    inspect::set(&TOY, &mut tree, "/toy/@id=99/new", tree!(2)).unwrap();
    assert_eq!(tree["ies"][3]["99"]["new"], tree!(2));
}

/// The entries of a list have the type of the list, and the enumerations that are the
/// same where several places are selected are those of each.
#[test]
fn the_entries_of_a_list_have_its_type_and_enumerations_are_compared() {
    assert_eq!(
        names_at("SetupRequest", "/toy/Holder/causes/0/nas"),
        Ok(Some(NAS))
    );
    let radio: &[&str] = &["unspecified", "user-inactivity"];
    assert_eq!(
        names_at("SetupRequest", "/toy/*/radioNetwork"),
        Ok(Some(radio))
    );
    // A value before a segment that has no members
    let error = checked("/toy/Cause/radioNetwork/*").unwrap_err();
    assert_eq!(
        error.to_string(),
        "/toy/Cause/radioNetwork/* selects nothing in a message SetupRequest: a value before \
         \"*\" has no members"
    );
}

/// `null` is the absence of a value, but in the alternative of the type NULL.
#[test]
fn null_is_the_absence_of_a_value() {
    let mut tree = written_tree();
    tree["ies"][1]["Holder"]["cause"] = Value::Null;
    assert_eq!(select(&tree, "/toy/Holder/cause"), Ok(vec![]));
}

/// `*` of an edit is each entry of a list, and of no value that is no list: the member that
/// each entry lacks is given to each.
#[test]
fn an_edit_takes_star_in_a_list_alone() {
    let mut tree = written_tree();
    let error = inspect::set(&TOY, &mut tree, "/toy/Holder/*", tree!(1)).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Path);
    inspect::set(&TOY, &mut tree, "/toy/Holder/causes/*/misc", Value::Null).unwrap();
    assert_eq!(
        tree["ies"][1]["Holder"]["causes"],
        tree!([{"misc": null}, {"misc": null}])
    );
}

/// The octets of an IE or of a transfer are set in the place of its value, which is
/// then they.
#[test]
fn octets_are_set_in_the_place_of_a_value() {
    let mut tree = written_tree();
    inspect::set(&TOY, &mut tree, "/toy/Cause", tree!({"octets": "0101"})).unwrap();
    assert_eq!(tree["ies"][0], tree!({"Cause": {"octets": "0101"}}));
    assert_eq!(select(&tree, "/toy/Cause/octets"), Ok(vec![&tree!("0101")]));
    let path = "/toy/Holder/transfer";
    inspect::set(&TOY, &mut tree, path, tree!({"octets": "0001"})).unwrap();
    assert_eq!(
        select(&tree, "/toy/Holder/transfer/octets"),
        Ok(vec![&tree!("0001")])
    );
    // A value that is not its octets has none to select, and none to open
    assert!(select(&tree, "/toy/Nas/octets").is_err());
    let before = tree.clone();
    assert_eq!(inspect::open(&TOY, &mut tree, "/toy/Cause/octets"), Ok(()));
    assert_eq!(tree, before);
}

/// An optional member that is not there is read by its type, and an edit that would take it
/// out selects nothing.
#[test]
fn a_member_that_is_not_there_is_read_by_its_type() {
    let mut tree = written_tree();
    let error = select(&tree, "/toy/Holder/tac/misspelled").unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Path);
    let error = inspect::remove(&TOY, &mut tree, "/toy/Holder/tac").unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Path);
    // An IE that the message does not have, likewise
    tree["ies"].as_array_mut().unwrap().remove(1);
    let error = select(&tree, "/toy/Holder/misspelled").unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Path);
    assert_eq!(
        select(&tree, "/toy/Cause/radioNetwork"),
        Ok(vec![&tree!("unspecified")])
    );
}

/// A refusal in the value of an IE is said as it is.
#[test]
fn a_refusal_in_the_value_of_an_ie_is_said_as_it_is() {
    let mut tree = written_tree();
    let error = inspect::remove(&TOY, &mut tree, "/toy/Cause/radioNetwork").unwrap_err();
    assert_eq!(
        error.to_string(),
        "radioNetwork is not taken out: a CHOICE has one alternative, set another in its \
         place, at /toy/Cause/radioNetwork"
    );
}

/// What an edit cannot take out, and an insertion at a member, are refused.
#[test]
fn what_an_edit_cannot_do_is_refused() {
    let mut tree = written_tree();
    let error = |result: Result<(), Error>| {
        let error = result.unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Value, "{error}");
        error.to_string()
    };
    assert!(
        error(inspect::remove(&TOY, &mut tree, "/toy/Holder/flags/length"))
            .starts_with("length is not taken out: its type always has it")
    );
    let insert = inspect::insert(
        &TOY,
        &mut tree,
        "/toy/Holder/cause",
        tree!({"nas": "deregister"}),
    );
    let text = error(insert);
    assert!(text.starts_with("insert adds to a list"), "{text}");
    inspect::remove(&TOY, &mut tree, "/toy/Holder/cause").unwrap();
    assert!(tree["ies"][1]["Holder"].get("cause").is_none());
}

/// An IE is inserted at the end of a list or before an entry, by its name or its number,
/// with its octets in the place of its value.
#[test]
fn an_ie_is_inserted_at_the_end_or_before_an_entry() {
    let mut tree = written_tree();
    let ie = tree!({"Nas": {"octets": "020101"}});
    inspect::insert(&TOY, &mut tree, "/toy/-", ie.clone()).unwrap();
    assert_eq!(tree["ies"].as_array().unwrap().last(), Some(&ie));
    let ie = tree!({"15": {"octets": "0101"}});
    inspect::insert(&TOY, &mut tree, "/toy/0", ie.clone()).unwrap();
    assert_eq!(tree["ies"][0], ie);
    assert_eq!(
        tree["ies"][1],
        tree!({"Cause": {"radioNetwork": "unspecified"}})
    );
    // Before the IE that a name selects, and each IE that `*` selects
    let ie = tree!({"Nas": {"nas": "normal-release"}});
    inspect::insert(&TOY, &mut tree, "/toy/Holder", ie.clone()).unwrap();
    assert_eq!(tree["ies"][2], ie);
    assert_eq!(
        select(&tree, "/toy/Holder/cause/nas"),
        Ok(vec![&tree!("deregister")])
    );
}

/// What a value without members selects: nothing for `*` and the members of a bit string
/// written as usual, and a type that octets carry once they are opened.
#[test]
fn what_a_value_without_members_selects() {
    let mut tree = written_tree();
    let holder = &mut tree["ies"][1]["Holder"];
    holder["flags"] = tree!("80");
    holder["blob"] = tree!("0a");
    tree["ies"]
        .as_array_mut()
        .unwrap()
        .push(tree!({"Container": "00020101"}));
    assert_eq!(select(&tree, "/toy/Cause/radioNetwork/*"), Ok(vec![]));
    assert_eq!(select(&tree, "/toy/Holder/flags/value"), Ok(vec![]));
    let error = select(&tree, "/toy/Holder/flags/misspelled").unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Path);
    assert_eq!(
        select(&tree, "/toy/Holder/blob/x").unwrap_err().to_string(),
        "\"x\" is not a member of this value, which has none, at /toy/Holder/blob/x"
    );
    let error = select(&tree, "/toy/Container/LteBlob").unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Path);
}

/// An IE that a value holds alone goes by its name after the member that holds it, with
/// its criticality beside it.
#[test]
fn an_ie_held_alone_goes_by_its_name() {
    let mut tree = written_tree();
    let extension = tree!({"Cause": {"nas": "deregister"}, "criticality": "reject"});
    tree["ies"][0]["Cause"] = tree!({"choice-Extensions": &extension});
    let path = "/toy/Cause/choice-Extensions/Cause/nas";
    assert_eq!(select(&tree, path), Ok(vec![&tree!("deregister")]));
    let criticality = "/toy/Cause/choice-Extensions/criticality";
    assert_eq!(select(&tree, criticality), Ok(vec![&tree!("reject")]));
    let listed: Vec<String> = inspect::paths(&TOY, &tree)
        .into_iter()
        .map(|(path, _)| path)
        .collect();
    assert!(listed.contains(&path.to_string()) && listed.contains(&criticality.to_string()));
    assert_eq!(checked(path), Ok(()));
    assert_eq!(checked("/toy/Cause/choice-Extensions/Holder/cause"), Ok(()));
    let error = select(&tree, "/toy/Cause/choice-Extensions/Holder").unwrap_err();
    assert!(
        error
            .to_string()
            .contains("is not the IE here, which is Cause"),
        "{error}"
    );
    // An edit goes into it, and the IE is written by its name
    inspect::set(&TOY, &mut tree, path, tree!("normal-release")).unwrap();
    tree["ies"].as_array_mut().unwrap().truncate(1);
    let cause = ies(&[(15, 0, &[1, 0])]);
    assert_eq!(
        written(&tree).unwrap(),
        ies(&[(15, 1, &[&[3][..], &cause].concat())])
    );
}

/// The paths of a tree list the IEs by name, those of a transfer of IEs by name in its
/// place, and the octets of what is its octets.
#[test]
fn the_paths_of_a_tree_list_ies_by_name() {
    let mut tree = written_tree();
    tree["ies"][0]["Cause"] = tree!({"octets": "0000"});
    let paths: Vec<String> = inspect::paths(&TOY, &tree)
        .into_iter()
        .map(|(path, _)| path)
        .collect();
    for path in [
        "/toy/Cause/octets",
        "/toy/Holder/transfer/radioNetwork",
        "/toy/Holder/setup/Cause/nas",
        "/toy/Holder/items/Item",
        "/toy/Holder/iE-Extensions/ExtendedCause/nas",
        "/toy/Holder/iE-Extensions/ExtendedCause/criticality",
        "/toy/3/criticality",
    ] {
        assert!(paths.contains(&path.to_string()), "{path} in {paths:?}");
    }
    for (path, value) in inspect::paths(&TOY, &tree) {
        assert_eq!(select(&tree, &path), Ok(vec![&value]), "{path}");
    }
}

/// The IEs that a tree has by name are entries of a view, with the criticality that the
/// object set of their message or transfer gives them, or the one that the tree says; a
/// value that holds an IE alone holds it by name, and an IE that did not decode is its
/// octets.
#[test]
fn the_ies_of_a_tree_and_of_a_view() {
    let tree = tree!([
        {"Cause": {"choice-Extensions": {"Cause": {"nas": "deregister"}}}},
        {"Holder": {"setup": {"protocolIEs": [{"Cause": {"nas": "normal-release"}}]}}},
        {"Payload": {"octets": "020101"}},
    ]);
    let listed = inspect::listed_ies(&TOY, "SetupRequest", &tree).unwrap();
    assert_eq!(
        listed,
        tree!([
            {"id": 15, "criticality": "ignore", "value": {"choice-Extensions":
                {"id": 15, "criticality": "ignore", "value": {"nas": "deregister"}}}},
            {"id": 16, "criticality": "reject", "value": {"setup": {"protocolIEs": [
                {"id": 15, "criticality": "ignore", "value": {"nas": "normal-release"}}]}}},
            {"id": 21, "criticality": "ignore", "octets": "020101"},
        ])
    );
    let ies = tree!([
        {"id": 15, "criticality": "ignore", "value": {"choice-Extensions":
            {"id": 15, "criticality": "reject", "value": {"nas": "deregister"}}}},
        {"id": 16, "criticality": "reject", "value": {"setup": {"decoded": {"protocolIEs": [
            {"id": 15, "criticality": "ignore", "value": {"nas": "normal-release"},
             "_raw_value": "0100"},
        ]}, "_raw_transfer": "000f01020100"}}},
    ]);
    let named = inspect::named_ies(&TOY, "SetupRequest", ies).unwrap();
    assert_eq!(
        named[0],
        tree!({"Cause": {"choice-Extensions": {"Cause": {"nas": "deregister"},
                                               "criticality": "reject"}}})
    );
    assert_eq!(
        named[1]["Holder"]["setup"],
        tree!({"protocolIEs": [{"Cause": {"nas": "normal-release"}}]})
    );
    // An IE whose transfer did not decode is the octets of the IE
    let undecoded = tree!([{"id": 21, "criticality": "ignore", "_raw_value": "02fe00",
        "value": {"_raw_transfer": "fe00", "_decode_error": "a Cause in another form"}}]);
    let named = inspect::named_ies(&TOY, "SetupRequest", undecoded).unwrap();
    assert_eq!(named, tree!([{"Payload": {"octets": "02fe00"}}]));
}

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

/// The JSON Encoding Rules write a value as deep as 256, through its members, the types that
/// another names and its lists.
#[test]
fn the_json_encoding_rules_write_as_deep_as_256() {
    let jer = |value: Value| TOY.jer(value, "Node");
    let readable = |levels: usize, link: &dyn Fn(Value) -> Value| {
        let mut node = tree!({"tac": 258});
        for _ in 1..levels {
            node = link(node);
        }
        node
    };
    // Each next is two deeper, through the type that names the node: the TAC of level n
    // is 2n - 1 deep
    let next = |node: Value| tree!({"next": (node)});
    let value = jer(readable(128, &next));
    assert_eq!(deepest(&value, 128, by_next)["tac"], tree!("000102"));
    let value = jer(readable(129, &next));
    assert_eq!(deepest(&value, 129, by_next)["tac"], tree!(258));
    // Each list of nodes is three deeper: the TAC of level n is 3n - 2 deep
    let nodes = |node: Value| tree!({"nodes": [(node)]});
    let value = jer(readable(86, &nodes));
    assert_eq!(deepest(&value, 86, by_nodes)["tac"], tree!("000102"));
    let value = jer(readable(87, &nodes));
    assert_eq!(deepest(&value, 87, by_nodes)["tac"], tree!(258));
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
