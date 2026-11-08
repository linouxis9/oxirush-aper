//! The protocol IEs of the message of a PDU, whatever the message: read, put back, and
//! compared with the object set that the ASN.1 gives the message. The protocol crates give
//! each as a method of their PDU.

use alloc::vec;
use alloc::vec::Vec;

use super::errors::__undecodable_ie;
use super::read;
use super::{Direction, Field, Message, Pdu, Protocol, Transfer, object_set};
use crate::check::{Deviation, Wrong, deviations, structure_deviations};
use crate::{Criticality, Decode, Error, ErrorKind, Faults, Octets, Presence, Reader, Writer};

/// The protocol IEs of a message, which the ASN.1 has as a SEQUENCE of one container of
/// protocol IEs, which a later release may add to: the additions are skipped.
fn message_ies<'o, P: Protocol>(octets: &'o [u8]) -> Result<Vec<P::Field<'o>>, Error> {
    let mut reader = Reader::new(octets);
    let extended = reader.bit()?;
    let ies = (|| {
        let count = reader.size(0, 65535)?;
        let mut list = reader.list(count);
        for _ in 0..count {
            list.push(P::Field::decode(&mut reader)?);
        }
        Ok::<_, Error>(list)
    })();
    let ies = ies.map_err(|error| error.in_field("protocol_ies"))?;
    if extended {
        reader.skip_additions()?;
    }
    reader.finish()?;
    Ok(ies)
}

/// What of the message is in another form than the one that X.691 gives it, around the
/// values of its IEs, which are not read here: the faults of a reader that notes them, as
/// the codec of a message that is a container of protocol IEs and nothing else reads it.
fn message_faults<P: Protocol>(octets: &[u8]) -> Faults {
    let mut reader = Reader::noting(octets);
    let read = (|| {
        let extended = reader.bit()?;
        let count = reader.size(0, 65535)?;
        for _ in 0..count {
            P::Field::decode(&mut reader)?;
        }
        if extended {
            reader.skip_additions()?;
        }
        // The bits after the message, up to the end of its last octet, are padding.
        reader.align();
        reader.finish()
    })();
    match read {
        Ok(()) => *reader.faults(),
        Err(_) => Faults::default(),
    }
}

/// The identifier and the criticality of each IE of the octets of a message, in their
/// order, without their values: the walk of the readers, which the codec of the
/// container reads alike.
fn listed_ies<P: Protocol>(octets: &[u8]) -> Result<Vec<(u16, Criticality)>, Error> {
    let (extended, count) = read::head(octets)?;
    // No more IEs than the octets can hold, of four octets each at least.
    let mut listed = Vec::with_capacity(count.min(octets.len() / 4));
    let mut at = 3;
    for _ in 0..count {
        let (id, criticality, _, next) =
            read::field::<P>(octets, at).map_err(|error| error.in_field("protocol_ies"))?;
        listed.push((id, criticality));
        at = next;
    }
    read::tail(octets, at, extended)?;
    Ok(listed)
}

/// The octets of the message that has these protocol IEs, written octet by octet: the
/// extension bit of the message in an octet, the count of its IEs in two, and for each IE
/// its identifier in two octets, its criticality in the next, and its value in its open
/// type. It is what the codec of the type writes, which the tests of the protocol crates
/// hold it to.
pub fn message_octets<'f, F: Field<'f>>(ies: &[F]) -> Result<Vec<u8>, Error> {
    // More IEs than a container has are refused by the codec of the type.
    let Ok(count) = u16::try_from(ies.len()) else {
        return message_octets_as_its_type(ies);
    };
    let values: usize = ies.iter().map(|ie| ie.value().len() + 5).sum();
    let mut octets = Vec::with_capacity(3 + values);
    octets.push(0);
    octets.extend_from_slice(&count.to_be_bytes());
    for ie in ies {
        let [high, low] = ie.id().to_be_bytes();
        let criticality = (ie.criticality() as u8) << 6;
        let value: &[u8] = ie.value();
        // The length of the value in one octet or in two, as nearly every IE has it.
        match value.len() {
            length @ 0..=0x7f => octets.extend_from_slice(&[high, low, criticality, length as u8]),
            length @ 0x80..=0x3fff => {
                let [first, second] = (length as u16 | 0x8000).to_be_bytes();
                octets.extend_from_slice(&[high, low, criticality, first, second]);
            }
            // A longer value is in fragments, which the writer writes.
            _ => {
                octets.extend_from_slice(&[high, low, criticality]);
                Writer::new(&mut octets).open_type(value);
                continue;
            }
        }
        octets.extend_from_slice(value);
    }
    Ok(octets)
}

/// The octets of the message that has these protocol IEs, as the codec of the type writes
/// them.
pub fn message_octets_as_its_type<'f, F: Field<'f>>(ies: &[F]) -> Result<Vec<u8>, Error> {
    let values: usize = ies.iter().map(|ie| ie.value().len() + 5).sum();
    let mut octets = Vec::with_capacity(3 + values);
    let mut writer = Writer::new(&mut octets);
    writer.bit(false);
    writer.size(ies.len(), 0, 65535)?;
    for ie in ies {
        ie.encode(&mut writer)?;
    }
    Ok(octets)
}

/// The name that ASN.1 gives the message of `pdu`, when the specification has its
/// procedure in that direction; an error for a message that has no container of protocol
/// IEs.
fn message_of_protocol_ies<'p, X: Pdu<'p>>(pdu: &X) -> Result<Option<&'static str>, Error> {
    let set = object_set::<X::Protocol>(pdu.direction(), pdu.procedure_code());
    match set {
        Some(set) if set.ies.is_none() => Err(Error::no_protocol_ies(set.name)),
        set => Ok(set.map(|set| set.name)),
    }
}

/// The protocol IEs of the message of `pdu`, whatever the message is: each IE as it was
/// received, with its identifier, its criticality and the octets of its value, which are
/// borrowed from the PDU. An IE that the message cannot have, one that the protocol does
/// not have and one that is there twice are all there, in the order of the message.
///
/// # Errors
///
/// An error of the kind `WrongMessage` for a private message, which has private IEs and no
/// container of protocol IEs, and one of the kind `Decode` when the octets of the message
/// are not a container of protocol IEs.
pub fn protocol_ies<'s, 'p: 's, X: Pdu<'p>>(
    pdu: &'s X,
) -> Result<Vec<<X::Protocol as Protocol>::Field<'s>>, Error> {
    let name = message_of_protocol_ies(pdu)?;
    let value: &'s [u8] = pdu.value().as_slice();
    if value.is_empty() {
        let none = Error::new(ErrorKind::Decode, "there are no octets");
        return Err(none.of_message(name));
    }
    message_ies::<X::Protocol>(value).map_err(|error| error.of_message(name))
}

/// Make `ies` the protocol IEs of the message of `pdu`: the PDU keeps its direction, its
/// procedure code and its criticality, and its message is these IEs, in their order and
/// as they are. Nothing is checked.
///
/// # Errors
///
/// An error of the kind `WrongMessage` for a private message, which has no container of
/// protocol IEs, and one of the kind `Encode` for more IEs than a container has, which is
/// 65535. The PDU is as it was after an error.
pub fn set_protocol_ies<'p, 'f, X: Pdu<'p>>(
    pdu: &mut X,
    ies: &[<X::Protocol as Protocol>::Field<'f>],
) -> Result<(), Error> {
    message_of_protocol_ies(pdu)?;
    let octets = message_octets(ies).map_err(|error| error.of_protocol_ies(ies.len()))?;
    pdu.__set_value(Octets::owned(octets));
    Ok(())
}

/// The PDU that holds `message`: of the direction and the procedure code of the message,
/// and of the criticality `criticality`. The message is encoded as it is, whatever its IEs
/// are.
///
/// # Errors
///
/// A message that does not encode: a value outside a constraint of its type.
pub fn of_message<'m, M: Message<'m>>(
    message: &M,
    criticality: Criticality,
) -> Result<<M::Protocol as Protocol>::Pdu<'static>, Error> {
    let octets = Octets::owned(crate::encode(message)?);
    Ok(
        <<M::Protocol as Protocol>::Pdu<'static> as Pdu<'static>>::__of(
            M::DIRECTION,
            M::PROCEDURE_CODE,
            criticality,
            octets,
        ),
    )
}

/// Whether the message of a direction and a procedure code may hold a key: its object set
/// has an IE of a type that holds one. `Debug` of its PDU shows the length of the octets
/// of the message, and not the octets.
pub fn holds_key<P: Protocol>(direction: Direction, code: u8) -> bool {
    let ies = object_set::<P>(direction, code).and_then(|set| set.ies);
    ies.is_some_and(|ies| ies.iter().any(|(id, ..)| P::__KEY_IES.contains(id)))
}

/// How `pdu` deviates from what the ASN.1 assigns its message, the value of each IE
/// included: the criticality of its procedure, the object set of its IEs, the value of
/// each IE of the set, which is decoded as its type, and what is written in another form
/// than the one that ITU-T X.691 gives it. The list is empty for a PDU that is constructed
/// as the specification has a sender construct it.
///
/// | [`Deviation`] | Is |
/// | --- | --- |
/// | `PduCriticality` | A criticality of the PDU that is not that of its procedure |
/// | `UnexpectedIe` | An IE that the object set of the message does not have |
/// | `RepeatedIe` | An IE that the message has already |
/// | `OutOfOrder` | An IE that comes after one that the object set has after it |
/// | `IeCriticality` | A criticality of an IE that is not the one that the object set assigns it |
/// | `MissingIe` | An IE whose presence is mandatory, and that the message does not have |
/// | `UndecodableIe` | An IE of the object set whose value does not decode as its type |
/// | `OutsideRoot` | An IE whose value is of a later version of its type: outside the root of an extensible constraint |
/// | `NotCanonical` | What is written in another form than the one that ITU-T X.691 gives it |
/// | `UnknownProcedure` | A procedure that the specification does not have in the direction of the PDU |
///
/// The criticality of the PDU comes first, then what each IE deviates in, in the order of
/// the message, then the IEs that are missing, in the order of the object set.
///
/// It reports, and refuses nothing. The macros build a message whatever its IEs are, a
/// tree writes one, and the codec encodes and decodes it: a test of what a peer does with
/// such a message needs to send it. Clause 10.3 of TS 38.413 and of TS 36.413 has what a
/// receiver does: with an IE that it does not comprehend, by the criticality that the IE
/// has (10.3.4), with one that is missing, by the criticality that the specification
/// assigns it (10.3.5), and with IEs in a wrong order or with too many occurrences, which
/// reject the procedure (10.3.6).
///
/// A value in another form than the one that ITU-T X.691 gives it is whole, and the
/// encoder of a peer may write it: a receiver reads it and answers nothing for it. The
/// octets before the message, which are the PDU itself, were read when the PDU was
/// decoded.
///
/// What it does not look at:
///
/// - a conditional IE, which is reported neither when it is missing nor when it is there:
///   its condition is in the prose of the specification;
/// - the IEs that a value holds, in a list, in a transfer or in an extension container:
///   the value of an IE of the message is decoded as its type, and what it holds as octets
///   is not. [`check_transfer`] compares a transfer of protocol IEs with its object set;
/// - the IEs of a private message, which has no object set: the criticality of its PDU
///   alone is compared;
/// - the message of a procedure that the specification does not have in that direction,
///   as one that a later release adds: it is the one deviation `UnknownProcedure`, which a
///   receiver answers by the criticality of the PDU (clause 10.3.4.1).
///
/// [`check_structure`] looks at the container alone, and decodes no value: what a receiver
/// that reads the IEs that it needs, which checks their values as it decodes them, checks
/// of the message.
///
/// # Errors
///
/// An error of the kind `Decode` when the octets of the message are not a container of
/// protocol IEs.
pub fn check<'p, X: Pdu<'p>>(pdu: &X) -> Result<Vec<Deviation<X::Protocol>>, Error> {
    checked(pdu, true)
}

/// How `pdu` deviates from what the ASN.1 assigns its message, in the container of its
/// IEs alone: the criticality of its procedure, and the object set of its IEs, their
/// presence, their order, their repetition and their criticality, which is what clause
/// 10.3 has a receiver answer. The values of the IEs are not decoded: what [`check`]
/// reports of them, `UndecodableIe`, `OutsideRoot` and `NotCanonical`, is not here. A
/// receiver that decodes the IEs that it reads finds which of them does not decode then,
/// as the error of the kind `UndecodableIe` of the macro or of `ie::<T>()`, which
/// [`Deviation::of`] makes a deviation of.
///
/// # Errors
///
/// An error of the kind `Decode` when the octets of the message are not a container of
/// protocol IEs.
pub fn check_structure<'p, X: Pdu<'p>>(pdu: &X) -> Result<Vec<Deviation<X::Protocol>>, Error> {
    checked(pdu, false)
}

/// `check` with the values of the IEs, and `check_structure` without them.
fn checked<'p, X: Pdu<'p>>(pdu: &X, values: bool) -> Result<Vec<Deviation<X::Protocol>>, Error> {
    let (direction, code) = (pdu.direction(), pdu.procedure_code());
    let criticality = pdu.criticality();
    let Some(message) = object_set::<X::Protocol>(direction, code) else {
        return Ok(vec![Deviation::UnknownProcedure {
            procedure_code: code,
            direction,
            criticality,
        }]);
    };
    let mut found = Vec::new();
    if criticality != message.criticality {
        found.push(Deviation::PduCriticality {
            criticality,
            assigned: message.criticality,
        });
    }
    let Some(set) = message.ies else {
        return Ok(found);
    };
    let octets = pdu.value().as_slice();
    let checked = container_deviations(message.name, set, octets, values, &mut found);
    checked.map_err(|error| error.of_message(Some(message.name)))?;
    Ok(found)
}

/// How the transfer `T` that `octets` hold deviates from its object set, the value of each
/// IE included: what [`check`] reports of the IEs of a message, without the criticality of
/// a procedure, which a transfer has none of. The list is empty for a transfer that is
/// constructed as the specification has a sender construct it.
///
/// It looks at the container of the transfer as [`check`] looks at that of a message, and
/// not at what the values of its IEs hold.
///
/// # Errors
///
/// An error of the kind `Decode` when the octets are not a container of protocol IEs.
pub fn check_transfer<'a, T: Transfer<'a>>(
    octets: &[u8],
) -> Result<Vec<Deviation<T::Protocol>>, Error> {
    transfer_checked::<T>(octets, true)
}

/// How the transfer `T` that `octets` hold deviates from its object set, in its container
/// alone: the presence, the order, the repetition and the criticality of its IEs, as
/// [`check_structure`] has those of a message. The values of the IEs are not decoded.
///
/// # Errors
///
/// An error of the kind `Decode` when the octets are not a container of protocol IEs.
pub fn check_transfer_structure<'a, T: Transfer<'a>>(
    octets: &[u8],
) -> Result<Vec<Deviation<T::Protocol>>, Error> {
    transfer_checked::<T>(octets, false)
}

/// `check_transfer` with the values of the IEs, and `check_transfer_structure` without
/// them.
fn transfer_checked<'a, T: Transfer<'a>>(
    octets: &[u8],
    values: bool,
) -> Result<Vec<Deviation<T::Protocol>>, Error> {
    let mut found = Vec::new();
    let checked = container_deviations(T::NAME, T::__OBJECT_SET, octets, values, &mut found);
    checked.map_err(|error| error.decoding(T::NAME))?;
    Ok(found)
}

/// What the IEs of the container that `octets` hold deviate in from its object set `set`,
/// added to `found`: the container of the message or of the transfer that ASN.1 names
/// `name`, with the value of each IE when `values` says so.
fn container_deviations<P: Protocol>(
    name: &'static str,
    set: &'static [(u16, Criticality, Presence)],
    octets: &[u8],
    values: bool,
    found: &mut Vec<Deviation<P>>,
) -> Result<(), Error> {
    if octets.is_empty() {
        return Err(Error::new(ErrorKind::Decode, "there are no octets"));
    }
    if !values {
        let listed = listed_ies::<P>(octets)?;
        structure_deviations(set, &listed, found);
        return Ok(());
    }
    let ies = message_ies::<P>(octets)?;
    // What of the container itself is in another form than the one of X.691.
    let faults = message_faults::<P>(octets);
    found.extend(faults.iter().map(|noted| Deviation::NotCanonical {
        ie: None,
        fault: noted.fault,
        bit: noted.bit,
    }));
    let listed: Vec<_> = ies.iter().map(|ie| (ie.id(), ie.criticality())).collect();
    // What is wrong with the value of the IE at a position: why it does not decode, as
    // the macro that extracts it says it, or what of it is outside the root of an
    // extensible constraint. And what of its encoding is in another form than the one of
    // X.691.
    let value = |position: usize| {
        let ie = &ies[position];
        let (id, criticality) = (ie.id(), ie.criticality());
        match P::__decodes(id, ie.value()) {
            None => (None, Faults::default()),
            Some(Ok((outside, faults))) => (outside.map(Wrong::OutsideRoot), faults),
            Some(Err(error)) => {
                let error = __undecodable_ie::<P>(error, name, id, criticality, position);
                (Some(Wrong::Undecodable(error)), Faults::default())
            }
        }
    };
    deviations(set, &listed, &value, found);
    Ok(())
}

/// The methods of the PDU of a protocol and its `Display`, written once: the protocol
/// calls it in its crate with the type of its PDU, its marker type, the name of its crate
/// and of the module of its bindings, its specification with the clauses that its
/// documentation cites, the octets of a UE CONTEXT RELEASE REQUEST and what `Display`
/// shows of it, and the IEs of the identifiers of a UE, which `Display` shows: the two,
/// then the IE of a pair of them with its CHOICE, the alternative of the pair and its
/// members, and the alternative of the first alone.
#[doc(hidden)]
#[macro_export]
// A protocol calls it in its crate, whose items `crate::` names.
#[allow(clippy::crate_in_macro_def)]
macro_rules! __pdu {
    (
        pdu $pdu:ident, protocol $protocol:ident, name $name:ident, bindings $bindings:ident,
        specification $specification:literal, construction $construction:literal,
        diagnostics $diagnostics:literal,
        release $release:literal, display $display:literal,
        ue $first:ident $second:ident,
        pair $pair:ident $choice:ident $both:ident($first_member:ident, $second_member:ident) $alone:ident
    ) => {
        impl<'a> crate::$bindings::$pdu<'a> {
            /// The PDU that `bytes` are, in aligned PER: the octets of one PDU, whole, and
            /// no octet after it. The octets of its message are borrowed from `bytes`, and
            /// not looked into: what does not decode in them is an error of
            /// `decode_value`, of `protocol_ies` and of `check`. `into_owned()` gives the
            /// same PDU, which owns its octets and is kept after `bytes`.
            ///
            /// The crate has no limit on the octets of a PDU, and a call on a PDU takes time
            /// and memory in proportion to them: the caller bounds what it reads from a peer.
            ///
            /// ```
            #[doc = concat!("use ", stringify!($name), "::{Direction, ErrorKind, ", stringify!($bindings), "::*};")]
            ///
            /// // A UE CONTEXT RELEASE REQUEST
            #[doc = concat!("let octets = hex::decode(\"", $release, "\").unwrap();")]
            #[doc = concat!("let pdu = ", stringify!($pdu), "::decode(&octets)?;")]
            /// assert_eq!(pdu.direction(), Direction::Initiating);
            /// assert_eq!(pdu.procedure_name(), "UEContextReleaseRequest");
            ///
            /// // The octets of a PDU that end early
            #[doc = concat!("let error = ", stringify!($pdu), "::decode(&octets[..octets.len() - 1]).unwrap_err();")]
            /// assert_eq!(error.kind(), ErrorKind::Decode);
            ///
            /// // A PDU that is kept after the octets that it was read from
            #[doc = concat!("let kept: ", stringify!($pdu), "<'static> = pdu.into_owned();")]
            /// drop(octets);
            /// assert_eq!(kept.procedure_name(), "UEContextReleaseRequest");
            #[doc = concat!("# Ok::<(), ", stringify!($name), "::Error>(())")]
            /// ```
            pub fn decode(bytes: &'a [u8]) -> Result<Self, $crate::Error> {
                match Self::usual(bytes) {
                    Some((alternative, code, criticality, message)) => Ok(Self::of_alternative(
                        alternative,
                        code,
                        criticality,
                        $crate::Octets::borrowed(message),
                    )),
                    None => $crate::decode(bytes),
                }
            }

            /// A PDU of the usual layout, read octet by octet: its alternative, its procedure
            /// code and its criticality, in an octet each, then its message in one piece up
            /// to the last octet. `None` for octets of another layout and for octets that are
            /// no PDU, which the codec of the type reads or refuses: this is a shorter way to
            /// what it gives, and the tests hold the two to the same PDU.
            #[inline]
            fn usual(bytes: &'a [u8]) -> Option<(u8, u8, $crate::Criticality, &'a [u8])> {
                let [alternative, code, criticality, ref rest @ ..] = *bytes else {
                    return None;
                };
                // No extension bit, and one of the three alternatives.
                if alternative & 0x80 != 0 || alternative >> 5 > 2 {
                    return None;
                }
                let criticality = *$crate::Criticality::ALL.get(usize::from(criticality >> 6))?;
                // The length of the message in one octet or in two, and no fragment.
                let message = match *rest {
                    [length @ 1..=0x7f, ref message @ ..]
                        if message.len() == usize::from(length) =>
                    {
                        message
                    }
                    [high @ 0x80..=0xbf, low, ref message @ ..]
                        if !message.is_empty()
                            && message.len() == usize::from(high & 0x3f) << 8 | usize::from(low) =>
                    {
                        message
                    }
                    _ => return None,
                };
                Some((alternative >> 5, code, criticality, message))
            }

            /// The PDU of an alternative, which is one of the three.
            #[inline]
            fn of_alternative(
                alternative: u8,
                code: u8,
                criticality: $crate::Criticality,
                value: $crate::Octets<'a>,
            ) -> Self {
                let direction = match alternative {
                    0 => $crate::Direction::Initiating,
                    1 => $crate::Direction::Successful,
                    _ => $crate::Direction::Unsuccessful,
                };
                <Self as $crate::protocol::Pdu<'a>>::__of(direction, code, criticality, value)
            }

            /// The octets of this PDU, in aligned PER.
            pub fn encode(&self) -> Result<alloc::vec::Vec<u8>, $crate::Error> {
                let value: &[u8] = <Self as $crate::protocol::Pdu<'a>>::value(self);
                let mut octets = alloc::vec::Vec::with_capacity(value.len() + 6);
                self.encode_into(&mut octets)?;
                Ok(octets)
            }

            /// Append the octets of this PDU, in aligned PER, to `octets`: a buffer that is
            /// kept from one PDU to the next is allocated once. After an error, `octets`
            /// are as they were.
            pub fn encode_into(&self, octets: &mut alloc::vec::Vec<u8>) -> Result<(), $crate::Error> {
                // Octet by octet, as `decode` reads the usual layout: the alternative, the
                // procedure code and the criticality, then the message in its open type. It
                // is what the codec of the type writes, which the tests hold it to.
                let alternative = match self.direction() {
                    $crate::Direction::Initiating => 0,
                    $crate::Direction::Successful => 1,
                    $crate::Direction::Unsuccessful => 2,
                };
                let criticality = (self.criticality() as u8) << 6;
                octets.extend_from_slice(&[alternative << 5, self.procedure_code(), criticality]);
                $crate::Writer::new(octets).open_type(<Self as $crate::protocol::Pdu<'a>>::value(self));
                Ok(())
            }

            /// The message that this PDU holds, as its type `T`, which borrows what it holds
            /// of octets from the PDU: `into_owned()` of the message keeps it after the PDU.
            ///
            /// `T` is the type of the message that `kind()` says. For the type of another
            /// message the error has the kind `WrongMessage`, as the octets of one message
            /// may decode as another:
            ///
            /// ```
            #[doc = concat!("use ", stringify!($name), "::{ErrorKind, ", stringify!($bindings), "::*};")]
            ///
            /// // A UE CONTEXT RELEASE REQUEST
            #[doc = concat!("let octets = hex::decode(\"", $release, "\").unwrap();")]
            #[doc = concat!("let pdu = ", stringify!($pdu), "::decode(&octets)?;")]
            /// let request: UEContextReleaseRequest = pdu.decode_value()?;
            /// assert_eq!(request.protocol_ies.0.len(), 3);
            ///
            /// let other = pdu.decode_value::<ErrorIndication>().unwrap_err();
            /// assert_eq!(other.kind(), ErrorKind::WrongMessage);
            #[doc = concat!("# Ok::<(), ", stringify!($name), "::Error>(())")]
            /// ```
            ///
            /// A type that is no message, as that of a transfer or an IE, does not compile.
            #[doc = concat!("[`", stringify!($name), "::decode`] decodes any type from the octets of an open type.")]
            ///
            /// ```compile_fail
            #[doc = concat!("use ", stringify!($name), "::", stringify!($bindings), "::*;")]
            ///
            #[doc = concat!("fn held(pdu: &", stringify!($pdu), ") {")]
            ///     let _ = pdu.decode_value::<Cause>();
            /// }
            /// ```
            pub fn decode_value<'s, T: $crate::protocol::Message<'s, Protocol = crate::$protocol>>(
                &'s self,
            ) -> Result<T, $crate::Error> {
                let (direction, code) = (self.direction(), self.procedure_code());
                if (direction, code) != (T::DIRECTION, T::PROCEDURE_CODE) {
                    return Err($crate::protocol::__wrong_message::<crate::$protocol>(
                        T::NAME, direction, code,
                    ));
                }
                $crate::decode(<Self as $crate::protocol::Pdu<'a>>::value(self))
            }

            /// The procedure code of this PDU.
            pub fn procedure_code(&self) -> u8 {
                match self {
                    Self::initiatingMessage(message) => message.procedure_code.0,
                    Self::successfulOutcome(message) => message.procedure_code.0,
                    Self::unsuccessfulOutcome(message) => message.procedure_code.0,
                }
            }

            /// The direction of this PDU: an initiating message, a successful outcome or an
            /// unsuccessful outcome.
            pub fn direction(&self) -> $crate::Direction {
                match self {
                    Self::initiatingMessage(_) => $crate::Direction::Initiating,
                    Self::successfulOutcome(_) => $crate::Direction::Successful,
                    Self::unsuccessfulOutcome(_) => $crate::Direction::Unsuccessful,
                }
            }

            /// The criticality of this PDU, which the specification assigns to its procedure
            /// (`Message::CRITICALITY` of its message) in a PDU that is constructed as it has
            /// a sender construct one.
            pub fn criticality(&self) -> $crate::Criticality {
                <Self as $crate::protocol::Pdu<'a>>::criticality(self)
            }

            /// The protocol IEs of the message of this PDU, whatever the message is: the
            /// container of the bindings, in which each IE is as it was received, with its
            /// identifier, its criticality and the octets of its value, which are borrowed
            /// from the PDU. An IE that the message cannot have, one that the protocol does
            /// not have and one that is there twice are all in it, in the order of the
            /// message.
            ///
            #[doc = concat!("Every message of ", $specification, " but the private one is a container of")]
            /// protocol IEs, so this needs no type: `decode_value` takes the type of the
            /// message that the PDU holds, and refuses another one. The message of a
            /// procedure that the specification does not have in that direction is read as
            /// a container of protocol IEs too.
            ///
            /// ```
            #[doc = concat!("use ", stringify!($name), "::", stringify!($bindings), "::*;")]
            ///
            /// // The Cause of a PDU, whatever its message, when it has one
            #[doc = concat!("fn cause(pdu: &", stringify!($pdu), ") -> Result<Option<Cause>, ", stringify!($name), "::Error> {")]
            ///     let ies = pdu.protocol_ies()?;
            ///     let Some(ie) = ies.0.iter().find(|ie| ie.id == ID_CAUSE) else {
            ///         return Ok(None);
            ///     };
            #[doc = concat!("    Ok(Some(", stringify!($name), "::decode(&ie.value)?))")]
            /// }
            ///
            /// // A UE CONTEXT RELEASE REQUEST, which has one
            #[doc = concat!("let octets = hex::decode(\"", $release, "\").unwrap();")]
            #[doc = concat!("let pdu = ", stringify!($pdu), "::decode(&octets)?;")]
            /// assert!(matches!(cause(&pdu)?, Some(Cause::radioNetwork(_))));
            #[doc = concat!("# Ok::<(), ", stringify!($name), "::Error>(())")]
            /// ```
            ///
            /// # Errors
            ///
            /// An error of the kind `WrongMessage` for the private message, which has
            /// private IEs and no container of protocol IEs, and one of the kind `Decode`
            /// when the octets of the message are not a container of protocol IEs.
            pub fn protocol_ies(&self) -> Result<crate::$bindings::ProtocolIEContainer<'_>, $crate::Error> {
                $crate::protocol::protocol_ies(self).map(crate::$bindings::ProtocolIEContainer)
            }

            /// Make `protocol_ies` the protocol IEs of the message of this PDU: the PDU keeps
            /// its direction, its procedure code and its criticality, and its message is
            /// these IEs, in their order and as they are. With
            /// [`protocol_ies`](Self::protocol_ies), this amends a message of any type:
            ///
            /// ```
            #[doc = concat!("use ", stringify!($name), "::{Ie, ie, ", stringify!($bindings), "::*};")]
            ///
            /// // A UE CONTEXT RELEASE REQUEST
            #[doc = concat!("let octets = hex::decode(\"", $release, "\").unwrap();")]
            #[doc = concat!("let mut pdu = ", stringify!($pdu), "::decode(&octets)?;")]
            ///
            /// // Its IEs, with one more: they are owned, as the PDU is changed
            /// let mut ies = pdu.protocol_ies()?.into_owned();
            /// ies.0.push(ie::TimeToWait::field(Criticality::Ignore, TimeToWait::v5s)?);
            /// pdu.set_protocol_ies(ies)?;
            ///
            /// let request: UEContextReleaseRequest = pdu.decode_value()?;
            /// assert_eq!(request.protocol_ies.0.len(), 4);
            #[doc = concat!("# Ok::<(), ", stringify!($name), "::Error>(())")]
            /// ```
            ///
            /// Nothing is checked: the IEs are those of any message, in any order.
            ///
            /// # Errors
            ///
            /// An error of the kind `WrongMessage` for the private message, which has no
            /// container of protocol IEs, and one of the kind `Encode` for more IEs than a
            /// container has, which is 65535. The PDU is as it was after an error.
            pub fn set_protocol_ies(
                &mut self,
                protocol_ies: crate::$bindings::ProtocolIEContainer<'_>,
            ) -> Result<(), $crate::Error> {
                $crate::protocol::set_protocol_ies(self, &protocol_ies.0)
            }

            /// Every occurrence of the IE `T` in the message of this PDU, in its order, each
            /// decoded as the type of `T`. The octets of the message are read where they
            /// are, without `decode_value` and without allocating, and an IE of another
            /// identifier is passed over and not decoded: a value that does not decode is an
            /// error of the kind `UndecodableIe`, and octets that are no container of
            /// protocol IEs an error of the kind `Decode`, which ends the iterator.
            ///
            /// An IE that the message has twice is there twice: `check()` reports the second
            /// as [`Deviation::RepeatedIe`](oxirush_aper::check::Deviation::RepeatedIe), and
            /// [`ie`](Self::ie) takes the first.
            ///
            /// ```
            #[doc = concat!("use ", stringify!($name), "::{ie, ", stringify!($bindings), "::*};")]
            ///
            /// // A UE CONTEXT RELEASE REQUEST, with its Cause twice
            #[doc = concat!("let octets = hex::decode(\"", $release, "\").unwrap();")]
            #[doc = concat!("let mut pdu = ", stringify!($pdu), "::decode(&octets)?.into_owned();")]
            /// let mut ies = pdu.protocol_ies()?.into_owned();
            /// ies.0.push(ies.0[2].clone());
            /// pdu.set_protocol_ies(ies)?;
            ///
            /// let causes: Vec<_> = pdu.ies::<ie::Cause>().collect::<Result<_, _>>()?;
            /// assert_eq!(causes.len(), 2);
            #[doc = concat!("# Ok::<(), ", stringify!($name), "::Error>(())")]
            /// ```
            #[inline]
            pub fn ies<T: $crate::protocol::Ie<Protocol = crate::$protocol>>(
                &self,
            ) -> $crate::protocol::Ies<'_, T> {
                $crate::protocol::Source::__of_pdu(self).ies::<T>()
            }

            /// The first occurrence of the IE `T` in the message of this PDU, decoded as the
            /// type of `T`; `None` when the message does not have it. The octets of the
            /// message are read up to that IE, and nothing else is decoded: see
            /// [`ies`](Self::ies).
            ///
            /// ```
            #[doc = concat!("use ", stringify!($name), "::{ie, ", stringify!($bindings), "::*};")]
            ///
            /// // The Cause of a UE CONTEXT RELEASE REQUEST, read in the buffer that it came in
            #[doc = concat!("let octets = hex::decode(\"", $release, "\").unwrap();")]
            #[doc = concat!("let pdu = ", stringify!($pdu), "::decode(&octets)?;")]
            /// let cause = pdu.ie::<ie::Cause>().unwrap()?;
            /// assert!(matches!(cause, Cause::radioNetwork(_)));
            ///
            /// // An IE that the message does not have
            /// assert!(pdu.ie::<ie::TimeToWait>().is_none());
            #[doc = concat!("# Ok::<(), ", stringify!($name), "::Error>(())")]
            /// ```
            #[inline]
            pub fn ie<T: $crate::protocol::Ie<Protocol = crate::$protocol>>(
                &self,
            ) -> Option<Result<T::Value<'_>, $crate::Error>> {
                self.ies::<T>().next()
            }

            /// The items of the list `L` in the message of this PDU, a list of single
            /// containers, each decoded as the IE of its single container, from the first
            /// occurrence of `L`: the list is walked where it is, and not built. Nothing when
            /// the message has no such list, which [`ie`](Self::ie) tells.
            #[inline]
            pub fn items<L: $crate::protocol::List<Protocol = crate::$protocol>>(
                &self,
            ) -> $crate::protocol::Items<'_, L> {
                $crate::protocol::Source::__of_pdu(self).items::<L>()
            }

            /// The PDU that holds `message`: of the direction and the procedure code of the
            /// message, and of the criticality `criticality`, which is that of its procedure
            #[doc = concat!("for a PDU as ", $specification, " clause ", $construction, " has a sender construct it.")]
            /// The message is encoded as it is, whatever its IEs are, as the macro that
            /// builds a PDU builds one from its IEs.
            ///
            /// ```
            #[doc = concat!("use ", stringify!($name), "::", stringify!($bindings), "::*;")]
            ///
            /// // A message that is at hand, as one that a test changed after it was decoded
            #[doc = concat!("let octets = hex::decode(\"", $release, "\").unwrap();")]
            #[doc = concat!("let pdu = ", stringify!($pdu), "::decode(&octets)?;")]
            /// let mut message: UEContextReleaseRequest = pdu.decode_value()?;
            /// message.protocol_ies.0.swap(0, 1);
            ///
            #[doc = concat!("let again = ", stringify!($pdu), "::of_message(&message, Criticality::Ignore)?;")]
            /// assert_eq!(again.procedure_name(), "UEContextReleaseRequest");
            /// assert_eq!(again.decode_value::<UEContextReleaseRequest>()?, message);
            #[doc = concat!("# Ok::<(), ", stringify!($name), "::Error>(())")]
            /// ```
            ///
            /// # Errors
            ///
            /// A message that does not encode: a value outside a constraint of its type.
            pub fn of_message<'m, M: $crate::protocol::Message<'m, Protocol = crate::$protocol>>(
                message: &M,
                criticality: $crate::Criticality,
            ) -> Result<crate::$bindings::$pdu<'static>, $crate::Error> {
                $crate::protocol::of_message(message, criticality)
            }

            /// How this PDU deviates from what the ASN.1 assigns its message: the criticality
            /// of its procedure, and the object set of its IEs. The list is empty for a PDU
            #[doc = concat!("that is constructed as ", $specification, " clause ", $construction, " has a sender construct it.")]
            ///
            /// The deviations, and what `check` does not look at, are those of
            /// [`oxirush_aper::protocol::check`], where each variant
            /// of [`Deviation`](oxirush_aper::check::Deviation) is told. A deviation says what the
            /// receiver of the message answers: the cause of
            /// [`Deviation::cause`](oxirush_aper::check::Deviation::cause), and for the Criticality
            /// Diagnostics the item of
            /// [`Deviation::diagnostics_item`](oxirush_aper::check::Deviation::diagnostics_item),
            /// which [`criticality_diagnostics`](Self::criticality_diagnostics) gathers into
            #[doc = concat!("the IE of the answer. Clause 10.3 of ", $specification, " has what a receiver does.")]
            ///
            /// `check` decodes the value of each IE of the object set. A receiver that reads
            /// the IEs that it needs, which their decoding checks, calls
            /// [`check_structure`](Self::check_structure), which does not.
            ///
            /// ```
            #[doc = concat!("use ", stringify!($name), "::{Deviation, ", stringify!($bindings), "::*};")]
            ///
            /// // A UE CONTEXT RELEASE REQUEST without its Cause, and with its two identifiers
            /// // in the other order
            #[doc = concat!("let octets = hex::decode(\"", $release, "\").unwrap();")]
            #[doc = concat!("let mut pdu = ", stringify!($pdu), "::decode(&octets)?.into_owned();")]
            /// let mut ies = pdu.protocol_ies()?.into_owned();
            /// ies.0.pop();
            /// ies.0.swap(0, 1);
            /// pdu.set_protocol_ies(ies)?;
            ///
            /// let deviations = pdu.check()?;
            /// assert_eq!(deviations.len(), 2);
            /// assert!(matches!(deviations[0], Deviation::OutOfOrder { .. }));
            /// assert!(matches!(deviations[1], Deviation::MissingIe { name: "Cause", .. }));
            /// assert_eq!(
            ///     deviations[1].to_string(),
            ///     format!("Cause (IE {}) is mandatory, and the message does not have it", ID_CAUSE.0)
            /// );
            #[doc = concat!("# Ok::<(), ", stringify!($name), "::Error>(())")]
            /// ```
            ///
            /// # Errors
            ///
            /// An error of the kind `Decode` when the octets of the message are not a
            /// container of protocol IEs.
            pub fn check(&self) -> Result<alloc::vec::Vec<crate::Deviation>, $crate::Error> {
                $crate::protocol::check(self)
            }

            /// How this PDU deviates from what the ASN.1 assigns its message, in the
            /// container of its IEs alone: the criticality of its procedure, and the object
            /// set of its IEs, the presence, the order, the repetition and the criticality of
            #[doc = concat!("each, which is what clause 10.3 of ", $specification, " has a receiver answer. The values of")]
            /// the IEs are not decoded, and what [`check`](Self::check) reports of them is not
            /// here: `UndecodableIe`, `OutsideRoot` and `NotCanonical`.
            ///
            /// A receiver that checks the structure and reads the IEs that it needs, with the
            /// macro that extracts IEs or [`ie`](Self::ie), pays for those IEs alone: the
            /// decoding of each checks its value, and an IE that does not decode is the error
            /// of the kind `UndecodableIe` there, which
            /// [`Deviation::of`](oxirush_aper::check::Deviation::of) makes a deviation of. `check`
            /// decodes every IE of the object set, for a tool that looks at every value.
            ///
            /// ```
            #[doc = concat!("use ", stringify!($name), "::", stringify!($bindings), "::*;")]
            ///
            /// // A UE CONTEXT RELEASE REQUEST without its Cause
            #[doc = concat!("let octets = hex::decode(\"", $release, "\").unwrap();")]
            #[doc = concat!("let mut pdu = ", stringify!($pdu), "::decode(&octets)?.into_owned();")]
            /// let mut ies = pdu.protocol_ies()?.into_owned();
            /// ies.0.pop();
            /// pdu.set_protocol_ies(ies)?;
            ///
            /// let deviations = pdu.check_structure()?;
            /// let missing = format!("Cause (IE {}) is mandatory", ID_CAUSE.0);
            /// assert!(deviations[0].to_string().starts_with(&missing));
            #[doc = concat!("# Ok::<(), ", stringify!($name), "::Error>(())")]
            /// ```
            ///
            /// # Errors
            ///
            /// An error of the kind `Decode` when the octets of the message are not a
            /// container of protocol IEs.
            pub fn check_structure(&self) -> Result<alloc::vec::Vec<crate::Deviation>, $crate::Error> {
                $crate::protocol::check_structure(self)
            }

            /// The Criticality Diagnostics IE of the ERROR INDICATION with which the receiver
            /// of this PDU answers it for `deviations`, which [`check`](Self::check) gave:
            /// the procedure code, the triggering message and the procedure criticality of
            #[doc = concat!("this PDU, which ", $specification, " clause ", $diagnostics, " has in an ERROR INDICATION alone,")]
            /// as clause 10.3.4.1 has them for a procedure that the receiver does not
            /// comprehend, and the item of each deviation that has one, as clauses 10.3.4.2
            /// and 10.3.5 have them for an IE, 256 at most. No list of IEs where no deviation
            /// has an item. The response of the procedure of this PDU has
            /// [`criticality_diagnostics_for_response`](Self::criticality_diagnostics_for_response).
            ///
            /// ```
            #[doc = concat!("use ", stringify!($name), "::", stringify!($bindings), "::*;")]
            ///
            /// // A UE CONTEXT RELEASE REQUEST without its second identifier, which is
            /// // mandatory and of the criticality reject
            #[doc = concat!("let octets = hex::decode(\"", $release, "\").unwrap();")]
            #[doc = concat!("let mut pdu = ", stringify!($pdu), "::decode(&octets)?.into_owned();")]
            /// let mut ies = pdu.protocol_ies()?.into_owned();
            /// let second = ies.0.remove(1);
            /// pdu.set_protocol_ies(ies)?;
            ///
            /// let deviations = pdu.check()?;
            /// let diagnostics = pdu.criticality_diagnostics(&deviations);
            /// assert_eq!(diagnostics.procedure_code, Some(ProcedureCode(pdu.procedure_code())));
            /// assert_eq!(diagnostics.triggering_message, Some(TriggeringMessage::initiating_message));
            /// assert_eq!(diagnostics.procedure_criticality, Some(pdu.criticality()));
            /// let items = diagnostics.ies_criticality_diagnostics.unwrap();
            /// assert_eq!(items.0[0].ie_id, second.id);
            /// assert_eq!(items.0[0].type_of_error, TypeOfError::missing);
            #[doc = concat!("# Ok::<(), ", stringify!($name), "::Error>(())")]
            /// ```
            pub fn criticality_diagnostics(
                &self,
                deviations: &[crate::Deviation],
            ) -> crate::$bindings::CriticalityDiagnostics {
                $crate::check::criticality_diagnostics(
                    self.procedure_code(),
                    self.direction(),
                    self.criticality(),
                    deviations,
                )
            }

            /// The Criticality Diagnostics IE of the response of the procedure of this PDU,
            /// with which its receiver answers it for `deviations`: the procedure criticality
            /// of this PDU and the item of each deviation that has one, without the procedure
            #[doc = concat!("code and the triggering message, which ", $specification, " clause ", $diagnostics, " keeps for an")]
            /// ERROR INDICATION.
            ///
            /// ```
            #[doc = concat!("use ", stringify!($name), "::", stringify!($bindings), "::*;")]
            ///
            /// // A UE CONTEXT RELEASE REQUEST without its second identifier
            #[doc = concat!("let octets = hex::decode(\"", $release, "\").unwrap();")]
            #[doc = concat!("let mut pdu = ", stringify!($pdu), "::decode(&octets)?.into_owned();")]
            /// let mut ies = pdu.protocol_ies()?.into_owned();
            /// let second = ies.0.remove(1);
            /// pdu.set_protocol_ies(ies)?;
            ///
            /// let deviations = pdu.check()?;
            /// let diagnostics = pdu.criticality_diagnostics_for_response(&deviations);
            /// assert_eq!(diagnostics.procedure_code, None);
            /// assert_eq!(diagnostics.triggering_message, None);
            /// assert_eq!(diagnostics.procedure_criticality, Some(pdu.criticality()));
            /// let items = diagnostics.ies_criticality_diagnostics.unwrap();
            /// assert_eq!(items.0[0].ie_id, second.id);
            #[doc = concat!("# Ok::<(), ", stringify!($name), "::Error>(())")]
            /// ```
            pub fn criticality_diagnostics_for_response(
                &self,
                deviations: &[crate::Deviation],
            ) -> crate::$bindings::CriticalityDiagnostics {
                $crate::check::criticality_diagnostics_for_response(self.criticality(), deviations)
            }
        }

        /// The name of the message, its procedure code and its direction, then the
        /// identifiers of the UE that the message has, by the names of their IEs, also
        /// where one IE holds both or the first alone:
        ///
        /// ```
        #[doc = concat!("use ", stringify!($name), "::", stringify!($bindings), "::", stringify!($pdu), ";")]
        ///
        #[doc = concat!("let octets = hex::decode(\"", $release, "\").unwrap();")]
        #[doc = concat!("let pdu = ", stringify!($pdu), "::decode(&octets)?;")]
        #[doc = concat!("assert_eq!(pdu.to_string(), \"", $display, "\");")]
        #[doc = concat!("# Ok::<(), ", stringify!($name), "::Error>(())")]
        /// ```
        ///
        /// A procedure that the specification does not have in that direction is an
        /// unknown message, with its procedure code.
        impl core::fmt::Display for crate::$bindings::$pdu<'_> {
            fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                use $crate::protocol::Ie;
                let direction = match self.direction() {
                    $crate::Direction::Initiating => "initiating message",
                    $crate::Direction::Successful => "successful outcome",
                    $crate::Direction::Unsuccessful => "unsuccessful outcome",
                };
                let code = self.procedure_code();
                let set = $crate::protocol::object_set::<crate::$protocol>(self.direction(), code);
                match set.map(|set| set.name) {
                    Some(name) => write!(formatter, "{name} (procedure {code}, {direction})")?,
                    None => write!(formatter, "an unknown message (procedure {code}, {direction})")?,
                }
                let (first, second) = (crate::ie::$first::NAME, crate::ie::$second::NAME);
                if let Some(Ok(id)) = self.ie::<crate::ie::$first>() {
                    write!(formatter, ", {first} {}", id.0)?;
                }
                if let Some(Ok(id)) = self.ie::<crate::ie::$second>() {
                    write!(formatter, ", {second} {}", id.0)?;
                }
                match self.ie::<crate::ie::$pair>() {
                    Some(Ok(crate::$bindings::$choice::$both(pair))) => {
                        let (one, other) = (pair.$first_member.0, pair.$second_member.0);
                        write!(formatter, ", {first} {one}, {second} {other}")?;
                    }
                    Some(Ok(crate::$bindings::$choice::$alone(id))) => {
                        write!(formatter, ", {first} {}", id.0)?;
                    }
                    _ => {}
                }
                Ok(())
            }
        }
    };
}
