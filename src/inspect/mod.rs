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

//! What the bindings of the application protocols of 3GPP share with the `inspect`
//! feature: the paths into the view of a PDU, read against the types and the object
//! sets of the protocol, on the engine of `oxirush_utils::tree`.
//!
//! A protocol crate gives its tables, which its generator writes, as a [`Protocol`],
//! and its own calls take it: `select`, `check_path`, `enumerated_at`, `paths`, `set`,
//! `remove`, `insert` and `open` of `oxirush-ngap` and of `oxirush-s1ap` are these
//! functions with the tables of the protocol. The rules are those of a view: the root
//! is short for the IEs of the message, an IE goes by its name, its position or
//! `@id=N`, a segment that names no member of an IE is taken in its value and one that
//! names no member of a transfer in its decoded value, and the name of an IE stays on
//! the entry that it is.
//!
//! It is the plumbing of the generated crates, which takes their tables as they are
//! written today: hidden from the documentation, and no part of the API that a version
//! number of this crate promises. A program calls the functions of oxirush-ngap and of
//! oxirush-s1ap, which are.

mod paths;
mod views;

use alloc::vec::Vec;
use oxirush_utils::tree::Value;

use crate::Error;

pub use crate::json::Form;
pub use paths::{
    EDITED, Members, check_path, enumerated_at, extended, insert, listed_ies, member_shape,
    named_ies, of_a_transfer, open, paths, remove, select, set, written_ie,
};
pub use views::{Parts, of_the_pdu, tree, tree_parts, view, view_of_jer, view_parts};

/// What a type of a protocol makes of octets: the JSON form of their value, whether that
/// value encodes back to them, and why a reader that takes the one encoding of X.691
/// alone refuses them. The value does not encode back to octets that hold what a later
/// version of the type adds, which the value has no place for, nor to octets in another
/// form than the canonical one, which that reader refuses.
pub type Decoded = (Value, bool, Option<Error>);

/// The functions of a type of a protocol: the JSON form of the value that octets are,
/// and the octets of the value that a JSON form writes.
#[derive(Clone, Copy)]
pub struct Typed {
    /// The JSON form of the value of the type that the octets are, and whether that
    /// value encodes back to them.
    pub decode: fn(&[u8]) -> Result<Decoded, Error>,
    /// The octets of the value of the type that the JSON form writes.
    pub encode: fn(&Value) -> Result<Vec<u8>, Error>,
}

/// A message of a protocol, as its tables have it: the direction, the procedure code,
/// the name that ASN.1 gives it, the functions of its type, the IEs of its object set
/// with their identifiers and whether each is mandatory, and the path of its type in
/// the bindings.
pub type Message = (
    &'static str,
    u8,
    &'static str,
    fn() -> Typed,
    &'static [(u16, bool)],
    &'static str,
);

/// A transfer of protocol IEs of a protocol, as its tables have it: the name that ASN.1
/// gives it, the name of its type in the bindings, the IEs of its object set with their
/// identifiers and whether each is mandatory, and the criticality that the set assigns
/// each, in the order of the set.
pub type Transfer = (
    &'static str,
    &'static str,
    &'static [(u16, bool)],
    &'static [(u16, &'static str)],
);

/// The tables of an application protocol that the paths read, which its generator
/// writes from the ASN.1 of the specification. Each list is in the order that its
/// documentation says, which the lookups take.
pub struct Protocol {
    /// The name of the protocol: `NGAP`.
    pub name: &'static str,
    /// The first segment of the paths into the IEs of the message of a PDU: `ngap`.
    pub root: &'static str,
    /// Each SEQUENCE with its members and each CHOICE with its alternatives, by the
    /// names that ASN.1 gives them, and each type that is a list or a bit string, in
    /// the order of the names of the types.
    pub types: &'static [(&'static str, Kind)],
    /// The messages.
    pub messages: &'static [Message],
    /// The identifier of each IE with the name that ASN.1 gives it after `id-`, in the
    /// order of the identifiers.
    pub ie_names: &'static [(u16, &'static str)],
    /// The path of the type of each IE that has one type, and that of the type that its
    /// octets contain, which is empty for octets that contain none.
    pub ie_types: &'static [(u16, &'static str, &'static str)],
    /// The IEs of an extension container that have the type which the type that they
    /// extend gives them: the identifier, the name of the type that is extended and the
    /// path of the type that the IE has there.
    pub extension_types: &'static [(u16, &'static str, &'static str)],
    /// The IEs whose octets may carry a type that the specification names and the
    /// ASN.1 does not: the identifier, the name that ASN.1 gives the type, and its path.
    pub carried: &'static [(u16, &'static str, &'static str)],
    /// The names of the values of each ENUMERATED, as the specification spells them, in
    /// the order of the names of the types.
    pub enumerations: &'static [(&'static str, &'static [&'static str])],
    /// The criticality that ASN.1 assigns to each IE that a value holds alone, in a
    /// single container.
    pub item_criticalities: &'static [(u16, &'static str)],
    /// The criticality that ASN.1 assigns to the IEs of each message, by the name that
    /// it gives the message.
    pub message_ie_criticalities: &'static [(&'static str, &'static [(u16, &'static str)])],
    /// The transfers of protocol IEs, each with its object set.
    pub transfers: &'static [Transfer],
    /// The identifier of the IE of a name, however the name is written: the protocol
    /// keeps the index that it reads, made once.
    pub ie_id: fn(&str) -> Option<u16>,
    /// What a type of [`types`](Self::types) is, by its name: the protocol keeps the
    /// index that it reads, made once.
    pub kind_of: fn(&str) -> Option<&'static Kind>,
    /// The criticality that ASN.1 assigns to each procedure, by its code.
    pub procedure_criticalities: &'static [(u8, &'static str)],
    /// The functions of the type of each IE that has one type, by its identifier: an
    /// error that says why for one that has none.
    pub ie_typed: fn(u16) -> Result<Typed, Error>,
    /// The functions of the type of an IE of an extension container where it extends
    /// the type that the bindings give this name; `None` for an IE that has its type
    /// whatever it extends.
    pub extension_typed: fn(u16, &str) -> Option<Typed>,
    /// The functions of the type that the octets of an IE contain, for an IE whose type
    /// is `OCTET STRING (CONTAINING ...)`.
    pub ie_contents: fn(u16) -> Option<Typed>,
    /// The readable form of each well-known type, by the name that the bindings give it,
    /// which its tree shows: what the JSON Encoding Rules write otherwise.
    pub type_form: fn(&str) -> Option<Form>,
    /// The members that hold a transfer: octets that contain a value of a type.
    pub transfer_fields: &'static [&'static str],
    /// The functions of the type that a transfer of a member contains; an error for a
    /// member that has several types, or none.
    pub transfer_typed: fn(&str) -> Result<Typed, Error>,
    /// The functions of a type that the octets of an IE may carry, by the name that
    /// ASN.1 gives it.
    pub carried_typed: fn(&str) -> Option<Typed>,
    /// The functions of the message of a procedure that the protocol does not have in
    /// that direction, which is read and written as what every message of the protocol
    /// but the private one is: a container of protocol IEs and nothing else.
    pub any_message: fn() -> Typed,
}

/// What a value of a view is, as its ASN.1 type has it: what a path can select in it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Shape {
    /// A value that the tables do not describe: it has the members that the view has.
    #[default]
    Unknown,
    /// The octets of a value that did not decode: nothing is selected in them.
    Undecoded,
    /// The view of a PDU, of this message when the tables have it.
    Pdu(Option<Sent>),
    /// The message of a PDU.
    Message(Sent),
    /// The message of a procedure that the tables do not have: a container of IEs
    /// without an object set, which each message of a protocol but the private one is.
    AnyMessage,
    /// A value without members.
    Plain,
    /// The one value of the type NULL, which a CHOICE has as an alternative: `null`,
    /// which is a value at a place of this shape, where it is the absence of a value
    /// elsewhere.
    Null,
    /// A bit string whose size varies: its `value` and its `length`, unless the view
    /// shows it as it is usually written.
    Bits,
    /// A SEQUENCE or a CHOICE, or a type that is a list or a bit string, by the name
    /// that the bindings give it. A name that the list of the types does not have is
    /// that of a value without members: an ENUMERATED, which has its names.
    Named(&'static str),
    /// A list.
    List(&'static Shape),
    /// The IEs of a container: those of the object set of this message, or of a set
    /// that the tables do not have.
    Ies(Option<Sent>),
    /// The IEs of an extension container, which extends the type that the bindings give
    /// this name. No name: the tables do not say what the container extends.
    Extensions(&'static str),
    /// An IE of a container, of this identifier when the path says it.
    Ie(Option<u16>),
    /// An IE of an extension container, of this identifier when the path says it, where
    /// the container extends the type of this name.
    Extension(Option<u16>, &'static str),
    /// Octets that contain a value of the type of this name, which the view shows
    /// decoded beside them. No name: the octets have one of several types.
    Transfer(&'static str),
    /// The octets of the IE of this identifier, which may carry a type that the
    /// specification names and the ASN.1 does not. The view shows them as they are, or
    /// as the transfer of that type under its name, once they are written or opened so.
    Carrier(u16),
}

/// A message, as the paths need it: the name that ASN.1 gives it, the name of its type
/// in the bindings, and the IEs of its object set.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sent {
    pub(crate) name: &'static str,
    pub(crate) ty: &'static str,
    pub(crate) ies: &'static [(u16, bool)],
}

/// A member of a SEQUENCE or an alternative of a CHOICE: the name that ASN.1 gives it,
/// the shape of its value, and whether a value may be without it.
pub type Member = (&'static str, Shape, bool);

/// What a type of the list of the types is.
#[derive(Debug)]
pub enum Kind {
    /// A SEQUENCE, with its members.
    Sequence(&'static [Member]),
    /// A CHOICE, with its alternatives.
    Choice(&'static [Member]),
    /// A type that is a list, a bit string, or another type.
    Is(Shape),
}

/// Whether two names are the same: they have the same letters and digits, whatever their
/// case and whatever is between them. A name without a letter or a digit is no name.
pub fn same_name(a: &str, b: &str) -> bool {
    oxirush_utils::tree::same_name(a, b)
}

/// The last segment of the path of a type in the bindings, without the lifetime after
/// it: its name.
pub fn type_name(path: &'static str) -> &'static str {
    let name = path.rsplit(':').next().unwrap_or(path);
    name.split('<').next().unwrap_or(name).trim()
}
