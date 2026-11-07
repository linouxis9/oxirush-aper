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

//! What the application protocols of 3GPP that are written in aligned PER have in common,
//! as NGAP (TS 38.413) and S1AP (TS 36.413): a PDU of three directions holds the message of
//! a procedure in an open type, a message is a container of protocol IEs, and each IE is
//! its identifier, its criticality and its value in an open type. The ASN.1 gives each
//! message the object set of the IEs that it can have.
//!
//! The bindings of a protocol implement the traits of this module from their registry, the
//! list of the procedures and of the IEs that the generator writes from the ASN.1: a marker
//! type for the protocol ([`Protocol`]), its PDU ([`Pdu`]), the IE of a container
//! ([`Field`]), each message ([`Message`]) and each transfer of protocol IEs
//! ([`Transfer`]), both of them containers ([`Container`]), and one marker type for each
//! IE ([`Ie`]). The readers and the macros of
//! the protocol crates are written once, here, for every protocol.
//!
//! # Reading the IEs that a program needs
//!
//! [`Ies`] reads every occurrence of one IE in a container, in order, and decodes that IE
//! alone: from the octets of the message of a PDU, which it scans without allocating, or
//! from a message or a transfer that is decoded. The protocol crates give it as `ies` and
//! `ie` of a PDU and of each message, and `items` walks the items of a list of single
//! containers, as the E-RAB lists of S1AP, without building the list.
//!
//! An IE that a message has twice stays visible to `ies`, which gives each occurrence, and
//! `check()` of the PDU reports the second as [`Deviation::RepeatedIe`]: `ie`, and the
//! macro that extracts IEs, take the first.
//!
//! Decoding an IE checks its value, as its type reads it. `check_structure()` of a PDU
//! checks what clause 10 of the specification has a receiver look at in the container of
//! its message, its object set, without decoding the values; `check()` decodes each value
//! too. A program that reads the IEs that it needs and checks the structure of the message
//! pays for those IEs alone. A transfer of protocol IEs has its own object set, which
//! `check` and `check_structure` of its type compare its octets with.
//!
//! [`Deviation::RepeatedIe`]: crate::check::Deviation::RepeatedIe

mod build;
mod errors;
mod inspection;
mod macros;
mod pdu;
mod read;

pub use build::{__Message, __assigned_ie, __ie, __message_of, __message_of_protocol_ies};
pub use errors::{__missing_ie, __undecodable_ie, __wrong_message, ie_name};
pub use pdu::{
    check, check_structure, check_transfer, check_transfer_structure, holds_key, message_octets,
    message_octets_as_its_type, of_message, protocol_ies, set_protocol_ies,
};
pub use read::{
    __Decoded, __Each, __Entry, __Fields, __Holds, __Item, __Source, __Walk, __first, __pdu_holds,
    Ies, Items, Source, items_of_fields,
};

use crate::check::Diagnostics;
use crate::{AsnType, Decode, Encode, Error, Faults, Octets, Reader, Writer};

/// The criticality of an IE or of a procedure in the application protocols of 3GPP, as
/// their ASN.1 has it (`Criticality ::= ENUMERATED { reject, ignore, notify }`): what a
/// receiver does with what it does not comprehend. The bindings of each protocol take
/// this type for theirs. Its `Display`, and the trees, name it as ASN.1 does, as
/// `reject`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Criticality {
    /// The receiver rejects the procedure.
    Reject,
    /// The receiver ignores the IE.
    Ignore,
    /// The receiver ignores the IE, and notifies the sender.
    Notify,
}

impl Criticality {
    /// The three, in the order of their indexes in PER.
    pub const ALL: [Self; 3] = [Self::Reject, Self::Ignore, Self::Notify];
    /// The names that ASN.1 gives them, in the same order.
    pub const NAMES: [&'static str; 3] = ["reject", "ignore", "notify"];

    /// The name that ASN.1 gives it: `reject`, `ignore` or `notify`.
    pub const fn name(self) -> &'static str {
        Self::NAMES[self as usize]
    }
}

impl core::fmt::Display for Criticality {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.name())
    }
}

impl AsnType for Criticality {
    const NAME: &'static str = "Criticality";
}

impl<'a> Decode<'a> for Criticality {
    #[inline]
    fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
        Ok(Self::ALL[reader.enumerated(3, 0, false)? as usize])
    }
}

impl Encode for Criticality {
    #[inline]
    fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
        writer.index(*self as u64, 3, false);
        Ok(())
    }
}

/// The tree of a criticality is its name, as ASN.1 gives it.
#[cfg(feature = "inspect")]
impl crate::json::Tree for Criticality {
    const NAME: &'static str = "Criticality";

    fn to_tree(&self) -> crate::json::Value {
        crate::json::text(self.name())
    }

    fn from_tree(
        value: &crate::json::Value,
        reading: &mut crate::json::Reading,
    ) -> Result<Self, crate::json::Refused> {
        Ok(Self::ALL[reading.named(value, &Self::NAMES)?])
    }
}

#[cfg(feature = "inspect")]
oxirush_utils::impl_serde!(Criticality);

/// The direction of a PDU, the alternative of the PDU that holds a message: the
/// initiating message of a procedure, or its successful or unsuccessful outcome. Its
/// `Display`, and the trees, name it as ASN.1 does, as `InitiatingMessage`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    /// The initiating message of a procedure.
    Initiating,
    /// The successful outcome of a procedure.
    Successful,
    /// The unsuccessful outcome of a procedure.
    Unsuccessful,
}

impl Direction {
    /// The three directions, in the order of the alternatives of a PDU.
    pub const ALL: [Self; 3] = [Self::Initiating, Self::Successful, Self::Unsuccessful];

    /// The name that ASN.1 gives the alternative of the PDU: `InitiatingMessage`,
    /// `SuccessfulOutcome` or `UnsuccessfulOutcome`.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Initiating => "InitiatingMessage",
            Self::Successful => "SuccessfulOutcome",
            Self::Unsuccessful => "UnsuccessfulOutcome",
        }
    }

    /// The direction of a name, as [`name`](Self::name) gives it.
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|direction| direction.name() == name)
    }
}

impl core::fmt::Display for Direction {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.name())
    }
}

/// A protocol, as the marker type that its bindings give it: `oxirush_ngap::Ngap`,
/// `oxirush_s1ap::S1ap`. Its registry implements it, with the tables that the readers of
/// this module read.
pub trait Protocol: Diagnostics + Copy + core::fmt::Debug + 'static {
    /// The name of the protocol, as `NGAP`.
    const NAME: &'static str;
    /// The PDU of the bindings, which borrows its octets for `'a`.
    type Pdu<'a>: Pdu<'a, Protocol = Self>;
    /// The IE of a container of protocol IEs, `ProtocolIEField` of the bindings.
    type Field<'a>: Field<'a, Protocol = Self>;
    /// The object set of each message.
    #[doc(hidden)]
    const __OBJECT_SETS: &'static [ObjectSet];
    /// Where each message is in the object sets, by its direction and its procedure code.
    #[doc(hidden)]
    const __OBJECT_SET_INDEX: &'static [[u16; 256]; 3];
    /// The identifier of each IE with the name that ASN.1 gives it after `id-`, in the
    /// order of the identifiers.
    #[doc(hidden)]
    const __IE_NAMES: &'static [(u16, &'static str)];
    /// The identifiers of the IEs whose type holds a key.
    #[doc(hidden)]
    const __KEY_IES: &'static [u16];
    /// Whether `octets`, the value of the IE `id`, decode as the type that the registry
    /// gives the IE: the error of the codec when they do not, and `None` for an
    /// identifier that has no one type. A value that decodes may be outside the root of
    /// an extensible constraint of its type, which is the error of an encoding that
    /// refuses it, and it has what of its encoding is in another form than the one of
    /// X.691.
    #[doc(hidden)]
    #[allow(clippy::type_complexity)]
    fn __decodes(id: u16, octets: &[u8]) -> Option<Result<(Option<Error>, Faults), Error>>;
    /// The criticality that ASN.1 assigns to the procedure of a code.
    #[doc(hidden)]
    fn __procedure_criticality(code: u8) -> Option<Criticality>;
}

/// What the registry has of a message: the direction and the procedure code of its PDU,
/// the name that ASN.1 gives it, and the IEs of its object set in the order of the set,
/// each as its identifier, the criticality that the set assigns it and whether its
/// presence is mandatory. A message that has no container of protocol IEs, as a private
/// message, has no object set.
#[doc(hidden)]
#[derive(Debug)]
pub struct ObjectSet {
    /// The direction of its PDU.
    pub direction: Direction,
    /// The code of its procedure.
    pub procedure_code: u8,
    /// The name that ASN.1 gives the message.
    pub name: &'static str,
    /// The IEs of its object set.
    pub ies: Option<&'static [(u16, Criticality, bool)]>,
}

/// The object set of the message of a direction and a procedure code, when the
/// specification has one.
#[doc(hidden)]
pub fn object_set<P: Protocol>(direction: Direction, code: u8) -> Option<&'static ObjectSet> {
    let at = P::__OBJECT_SET_INDEX[direction as usize][usize::from(code)];
    P::__OBJECT_SETS.get(usize::from(at))
}

/// For the registry of a protocol: where each message is in its object sets, by its
/// direction and its procedure code; `u16::MAX` where the protocol has no message.
#[doc(hidden)]
pub const fn __object_set_index(sets: &[ObjectSet]) -> [[u16; 256]; 3] {
    let mut index = [[u16::MAX; 256]; 3];
    let mut at = 0;
    while at < sets.len() {
        let set = &sets[at];
        index[set.direction as usize][set.procedure_code as usize] = at as u16;
        at += 1;
    }
    index
}

/// The name that ASN.1 gives the IE of an identifier, after `id-`.
#[doc(hidden)]
pub fn name<P: Protocol>(id: u16) -> Option<&'static str> {
    let names = P::__IE_NAMES;
    let at = names.binary_search_by_key(&id, |(known, _)| *known);
    at.ok().map(|at| names[at].1)
}

/// The protocol of a type of its bindings: of a PDU, of an IE of a container, of a
/// message and of a transfer, whatever the octets that it borrows.
pub trait InProtocol {
    /// The protocol.
    type Protocol: Protocol;
}

/// The criticality that the object set of the message of a direction and a procedure code
/// assigns to the IE `id`: `None` for an IE that the set does not have.
#[doc(hidden)]
pub fn __assigned<P: Protocol>(direction: Direction, code: u8, id: u16) -> Option<Criticality> {
    __assigned_in(object_set::<P>(direction, code)?.ies?, id)
}

/// The criticality that the IEs of an object set assign to the IE `id`: `None` for an IE
/// that the set does not have.
#[doc(hidden)]
#[inline]
pub fn __assigned_in(ies: &[(u16, Criticality, bool)], id: u16) -> Option<Criticality> {
    let found = ies.iter().find(|(known, ..)| *known == id);
    found.map(|(_, assigned, _)| *assigned)
}

/// The name of a type of the bindings, of its path as the registry writes it: without the
/// modules before it and the lifetime after it.
#[doc(hidden)]
pub fn __type_name(path: &'static str) -> &'static str {
    let name = path.rsplit(':').next().unwrap_or(path);
    name.split('<').next().unwrap_or(name).trim()
}

/// A PDU of a protocol: one of the three directions, with the code of its procedure, its
/// criticality and its message in an open type. `'a` is how long the octets live that it
/// borrows its message from.
pub trait Pdu<'a>: Decode<'a> + Encode + InProtocol + Sized {
    /// Its direction.
    fn direction(&self) -> Direction;
    /// The code of its procedure.
    fn procedure_code(&self) -> u8;
    /// Its criticality.
    fn criticality(&self) -> Criticality;
    /// The octets of the open type that holds its message.
    fn value(&self) -> &Octets<'a>;
    /// Make `value` the octets of its message.
    #[doc(hidden)]
    fn __set_value(&mut self, value: Octets<'a>);
    /// The PDU of a direction, with the code of its procedure, its criticality and the
    /// octets of its message.
    #[doc(hidden)]
    fn __of(
        direction: Direction,
        procedure_code: u8,
        criticality: Criticality,
        value: Octets<'a>,
    ) -> Self;
}

/// An IE of a container of protocol IEs, `ProtocolIEField` of the bindings: its
/// identifier, its criticality, and its value in an open type.
pub trait Field<'a>: Decode<'a> + Encode + InProtocol + Clone {
    /// Its identifier.
    fn id(&self) -> u16;
    /// Its criticality.
    fn criticality(&self) -> Criticality;
    /// The octets of its value.
    fn value(&self) -> &Octets<'a>;
    /// The IE of an identifier, a criticality and the octets of a value.
    #[doc(hidden)]
    fn __new(id: u16, criticality: Criticality, value: Octets<'a>) -> Self;
}

/// A type whose value is a container of protocol IEs, whose IEs the readers and the
/// macros read, change and build: the type of a message, or of a transfer whose octets
/// hold such a container, as `PDUSessionResourceSetupRequestTransfer` of NGAP. The
/// registry implements it for each message and each transfer.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is no container of protocol IEs",
    label = "the type of a message, or of a transfer of protocol IEs, is expected here",
    note = "the macros take the type of a message, as `UEContextReleaseRequest`, or of a \
            transfer of protocol IEs; an IE of a list of single containers is built without \
            the name of a type"
)]
pub trait Container<'a>: Decode<'a> + Encode + AsnType + InProtocol {
    /// Whether the type is a container of protocol IEs: every message but a private one
    /// is, and every transfer.
    #[doc(hidden)]
    const __OF_PROTOCOL_IES: bool;
    /// The criticality that the object set of the type assigns to the IE `id`: `None`
    /// for an IE that the set does not have.
    #[doc(hidden)]
    fn __assigned(id: u16) -> Option<Criticality>;
    /// The IEs of the container, in their order: none for a private message.
    #[doc(hidden)]
    fn __fields<'s>(&'s self) -> &'s [<Self::Protocol as Protocol>::Field<'s>];
}

/// A message of a protocol: the type of what the PDU of one procedure holds in one
/// direction. `'a` is how long the octets live that the message borrows from.
///
/// The registry of a protocol implements it for the type of each message that the ASN.1
/// gives a procedure, and for no other type. So `decode_value` of a PDU takes the type of
/// a message, and not that of an IE or of a transfer, which would decode from the same
/// octets as another value.
///
/// The name that ASN.1 gives the message, as `UEContextReleaseRequest`, is the `NAME` of
/// its type, which every type of the bindings has.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is no message of the protocol",
    label = "the type of a message is expected here",
    note = "a message is the type of what a PDU holds, as `UEContextReleaseRequest`; an IE of a \
            list of single containers is built without the name of a type"
)]
pub trait Message<'a>: Container<'a> {
    /// The direction of the PDU that holds it.
    const DIRECTION: Direction;
    /// The code of its procedure.
    const PROCEDURE_CODE: u8;
    /// The criticality that the specification assigns to its procedure, which the PDU
    /// that holds it has: what `of_message` is given for it, and what the macro that
    /// builds a PDU gives one whose criticality it is not given.
    const CRITICALITY: Criticality;
    /// The message that `octets` hold, which borrows nothing from them: what it holds of
    /// octets is copied.
    #[doc(hidden)]
    fn __decode_owned(octets: &[u8]) -> Result<Self, Error>;
}

/// A transfer of protocol IEs: the type of octets that an IE or a member of a value holds,
/// which contain a container of protocol IEs, as `PDUSessionResourceSetupRequestTransfer`
/// of NGAP. The ASN.1 gives it an object set as it gives a message one, and the registry
/// of a protocol implements it for each such type, with that set: the macros give its IEs
/// the criticality that the set assigns, and [`check_transfer`] compares its octets with
/// the set.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is no transfer of protocol IEs",
    label = "the type of a transfer of protocol IEs is expected here",
    note = "a transfer of protocol IEs is the type of octets that hold a container of protocol \
            IEs, as `PDUSessionResourceSetupRequestTransfer` of NGAP"
)]
pub trait Transfer<'a>: Container<'a> {
    /// The IEs of its object set, in the order of the set, each as its identifier, the
    /// criticality that the set assigns it and whether its presence is mandatory.
    #[doc(hidden)]
    const __OBJECT_SET: &'static [(u16, Criticality, bool)];
}

/// An IE of a protocol, as a marker type: `oxirush_ngap::ie::AMF_UE_NGAP_ID`. The
/// registry of the protocol writes one for each IE that has one type, by the names that
/// the macros take it by, and implements this trait for it: the identifier of the IE,
/// the name that ASN.1 gives it, and the type of its value.
///
/// `ies::<T>()` and `ie::<T>()` of a PDU and of a message take it, as the macros take its
/// name.
pub trait Ie: Copy + core::fmt::Debug + 'static {
    /// The protocol of the IE.
    type Protocol: Protocol;
    /// Its identifier.
    const ID: u16;
    /// The name that ASN.1 gives it after `id-`, as `AMF-UE-NGAP-ID`.
    const NAME: &'static str;
    /// The type of its value, which borrows octets for `'a`.
    type Value<'a>: Decode<'a> + Encode;

    /// The value of the IE that `value` converts into, as the macros that build an IE
    /// take it: a number for an identifier, as `AMF_UE_NGAP_ID(1u64)`.
    #[inline]
    fn value<'a>(value: impl Into<Self::Value<'a>>) -> Self::Value<'a> {
        value.into()
    }

    /// The value of the IE that `octets` hold, which borrows nothing from them, for
    /// octets that came in fragments and were gathered.
    #[doc(hidden)]
    // The marker of an IE names the lifetime of the octets, which its type borrows.
    #[allow(clippy::needless_lifetimes)]
    fn __owned<'a, 'b>(octets: &'a [u8]) -> Result<Self::Value<'b>, Error>;
}

/// An IE whose value is a list of single containers, each of which holds one IE, its
/// item: as `E-RABToBeSetupListCtxtSUReq` of S1AP, whose items are each an
/// `E-RABToBeSetupItemCtxtSUReq`. The registry declares it, with the size that the list
/// has.
pub trait List: Ie {
    /// The IE of each item.
    type Item: Ie<Protocol = Self::Protocol>;
    /// The size constraint of the list: how many items it has, at least and at most.
    #[doc(hidden)]
    const __SIZE: (usize, usize);
}

/// For the marker types of the IEs: `into_owned()` of a value whose type borrows nothing,
/// which is the value itself. A type that borrows has its own, which a call prefers.
#[doc(hidden)]
pub trait __IntoOwned: Sized {
    /// The value itself.
    #[inline]
    fn into_owned(self) -> Self {
        self
    }
}

impl<T> __IntoOwned for T {}

/// What a field of the macro that extracts IEs is given of the value of an IE: the value
/// itself, for a field of the type of the IE, and the one value that the type of the IE
/// is a name for, for a field of the type of that value. The bindings implement it for
/// each type that is a name for one value.
#[diagnostic::on_unimplemented(
    message = "a field of the type `{T}` does not take the value of this IE, which is a `{Self}`",
    label = "the value of the IE is a `{Self}`",
    note = "declare the field as `{Self}`, or as the type of the one value that a `{Self}` holds, \
            or write `=> expression` after the IE to make the field of its value"
)]
#[doc(hidden)]
pub trait __Field<T> {
    /// The field.
    fn __field(self) -> T;
}

impl<T> __Field<T> for T {
    #[inline]
    fn __field(self) -> T {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{Criticality, Direction};
    use alloc::string::ToString;
    use alloc::vec::Vec;

    #[test]
    fn a_criticality_and_a_direction_have_the_names_of_asn1() {
        let names: Vec<_> = Criticality::ALL.iter().map(ToString::to_string).collect();
        assert_eq!(names, Criticality::NAMES);
        assert_eq!(names, ["reject", "ignore", "notify"]);
        let names: Vec<_> = Direction::ALL.iter().map(ToString::to_string).collect();
        assert_eq!(
            names,
            [
                "InitiatingMessage",
                "SuccessfulOutcome",
                "UnsuccessfulOutcome"
            ]
        );
        for direction in Direction::ALL {
            assert_eq!(Direction::named(direction.name()), Some(direction));
        }
        assert_eq!(Direction::named("initiatingMessage"), None);
    }

    #[test]
    fn a_criticality_is_its_index_in_two_bits() {
        for (index, criticality) in Criticality::ALL.into_iter().enumerate() {
            let octets = crate::encode(&criticality).unwrap();
            assert_eq!(octets, [(index as u8) << 6]);
            assert_eq!(crate::decode::<Criticality>(&octets), Ok(criticality));
        }
        // The fourth value of the two bits is none.
        assert!(crate::decode::<Criticality>(&[0xc0]).is_err());
    }

    #[cfg(feature = "inspect")]
    #[test]
    fn the_tree_of_a_criticality_is_its_name() {
        use crate::json::{Tree, Value, from_value};
        for criticality in Criticality::ALL {
            let tree = criticality.to_tree();
            assert_eq!(tree, Value::from(criticality.name()));
            assert_eq!(from_value::<Criticality>(&tree).unwrap(), criticality);
        }
        assert!(from_value::<Criticality>(&Value::from("rejected")).is_err());
    }
}
