//! The protocol IEs of the message of a PDU, whatever the message: read, put back, and
//! compared with the object set that the ASN.1 gives the message. The protocol crates give
//! each as a method of their PDU.

use alloc::vec;
use alloc::vec::Vec;

use super::errors::__undecodable_ie;
use super::read;
use super::{CriticalityOf, Field, Message, Pdu, Protocol, object_set};
use crate::check::{Deviation, Wrong, deviations, structure_deviations};
use crate::{Decode, Error, ErrorKind, Faults, Octets, Reader, Writer};

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
fn listed_ies<P: Protocol>(octets: &[u8]) -> Result<Vec<(u16, CriticalityOf<P>)>, Error> {
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
        let criticality: crate::Criticality = ie.criticality().into();
        let criticality = (criticality as u8) << 6;
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
    criticality: CriticalityOf<M::Protocol>,
) -> Result<<M::Protocol as Protocol>::Pdu<'static>, Error> {
    let octets = Octets::owned(crate::encode(message)?);
    let pdu = <<M::Protocol as Protocol>::Pdu<'static> as Pdu<'static>>::__of(
        M::DIRECTION,
        M::PROCEDURE_CODE,
        criticality,
        octets,
    );
    // The direction of a message is one of the three.
    pdu.ok_or_else(|| Error::new(ErrorKind::Encode, "a message has no direction"))
}

/// Whether the message of a direction and a procedure code may hold a key: its object set
/// has an IE of a type that holds one. `Debug` of its PDU shows the length of the octets
/// of the message, and not the octets.
pub fn holds_key<P: Protocol>(direction: &str, code: u8) -> bool {
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
///   is not;
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
    if let Some(assigned) = X::Protocol::__procedure_criticality(code)
        && criticality != assigned
    {
        found.push(Deviation::PduCriticality {
            criticality,
            assigned,
        });
    }
    let Some(set) = message.ies else {
        return Ok(found);
    };
    if !values {
        let octets = pdu.value().as_slice();
        if octets.is_empty() {
            let none = Error::new(ErrorKind::Decode, "there are no octets");
            return Err(none.of_message(Some(message.name)));
        }
        let listed = listed_ies::<X::Protocol>(octets);
        let listed = listed.map_err(|error| error.of_message(Some(message.name)))?;
        structure_deviations(set, &listed, &mut found);
        return Ok(found);
    }
    let ies = protocol_ies(pdu)?;
    // What of the message itself is in another form than the one of X.691.
    let faults = message_faults::<X::Protocol>(pdu.value());
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
        match X::Protocol::__decodes(id, ie.value()) {
            None => (None, Faults::default()),
            Some(Ok((outside, faults))) => (outside.map(Wrong::OutsideRoot), faults),
            Some(Err(error)) => {
                let error =
                    __undecodable_ie::<X::Protocol>(error, message.name, id, criticality, position);
                (Some(Wrong::Undecodable(error)), Faults::default())
            }
        }
    };
    deviations(set, &listed, &value, &mut found);
    Ok(found)
}
