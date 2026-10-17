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

//! How a PDU of an application protocol of 3GPP compares with what its ASN.1 assigns its
//! message: the criticality of its procedure, and the object set of the IEs of the
//! message. The bindings of each protocol tell their deviations with their own types,
//! which they give through [`Diagnostics`]: `oxirush_ngap::Deviation` is
//! `Deviation<oxirush_ngap::Ngap>`.

use alloc::vec;
use alloc::vec::Vec;
use core::fmt;

use crate::{Criticality, Error, ErrorKind, Fault, Faults};

/// What the bindings of a protocol give the deviations of their PDUs: their own types for
/// the criticality, the type of an error and the protocol cause of an answer and the
/// item of its Criticality Diagnostics, and the names of their IEs.
pub trait Diagnostics {
    /// The criticality of the protocol, which converts to and from that of the crate.
    type Criticality: Copy + Eq + fmt::Debug + Into<Criticality> + From<Criticality>;
    /// The `TypeOfError` of the Criticality Diagnostics of the protocol.
    type TypeOfError;
    /// The `CauseProtocol` of the protocol.
    type Cause;
    /// The item of the Criticality Diagnostics that tells of an IE.
    type Item;

    /// The name that ASN.1 gives the IE of an identifier, after `id-`.
    fn name(id: u16) -> Option<&'static str>;

    /// The type of error of an IE that the receiver does not comprehend, or of one that
    /// is missing.
    fn type_of_error(missing: bool) -> Self::TypeOfError;

    /// The protocol cause that is this one.
    fn cause(cause: ProtocolCause) -> Self::Cause;

    /// The item of the Criticality Diagnostics of the IE `id`, of a criticality and a
    /// type of error.
    fn item(
        criticality: Self::Criticality,
        id: u16,
        type_of_error: Self::TypeOfError,
    ) -> Self::Item;
}

/// The protocol causes that a receiver answers a message with for its deviations: those
/// that the clauses of the criticality of TS 38.413 and TS 36.413 give.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtocolCause {
    /// An IE whose value does not decode.
    TransferSyntaxError,
    /// An IE of the criticality `reject` that the receiver does not comprehend or that
    /// is missing.
    AbstractSyntaxErrorReject,
    /// An IE of the criticality `notify` that the receiver does not comprehend or that
    /// is missing.
    AbstractSyntaxErrorIgnoreAndNotify,
    /// An IE that is there twice or in a wrong order.
    AbstractSyntaxErrorFalselyConstructedMessage,
}

/// One way in which a PDU is not as the specification has a sender construct it: what
/// `check()` of a PDU reports.
///
/// The check compares the criticality of the PDU with the one that the specification
/// assigns to its procedure, and the IEs of its message with the object set that the
/// ASN.1 gives the message: which IEs the message has, in which order, with which
/// criticality, and whether the value of each decodes as its type. It also says what
/// of the message and of these values is written in another form than the one that
/// ITU-T X.691 gives it. An IE is told by its identifier, by the name that ASN.1 gives
/// it after `id-` and by its position in the message, which starts at 0, and `Display`
/// says each deviation in a sentence.
///
/// [`ie`](Self::ie), [`type_of_error`](Self::type_of_error),
/// [`diagnostics_item`](Self::diagnostics_item) and [`cause`](Self::cause) say what
/// the receiver of the message answers, as clause 10 of the specification has it.
///
/// More variants may come with a later version: leave the others to `_`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Deviation<P: Diagnostics> {
    /// The criticality of the PDU is not the one that the specification assigns to its
    /// procedure.
    PduCriticality {
        /// The criticality that the PDU has.
        criticality: P::Criticality,
        /// The criticality of the procedure.
        assigned: P::Criticality,
    },
    /// An IE that the message cannot have: the object set of the message does not have
    /// its identifier.
    UnexpectedIe {
        /// The identifier of the IE.
        id: u16,
        /// Its name; `None` for an identifier that no object set of the protocol has.
        name: Option<&'static str>,
        /// Its position in the message.
        position: usize,
        /// The criticality that the IE has.
        criticality: P::Criticality,
    },
    /// An IE that the message has already: an IE is in a message once.
    RepeatedIe {
        /// The identifier of the IE.
        id: u16,
        /// Its name.
        name: &'static str,
        /// The position of this one.
        position: usize,
        /// The position of the first one.
        first: usize,
    },
    /// An IE that comes after one that the object set has after it. The IEs that the
    /// object set does not have are left out of the order, and so is an IE that is
    /// repeated, after its first.
    OutOfOrder {
        /// The identifier of the IE.
        id: u16,
        /// Its name.
        name: &'static str,
        /// Its position in the message.
        position: usize,
        /// The identifier of an IE before it in the message that the object set has
        /// after it: the one that the object set has last.
        after: u16,
        /// The name of that IE.
        after_name: &'static str,
    },
    /// An IE whose criticality is not the one that the object set assigns it.
    IeCriticality {
        /// The identifier of the IE.
        id: u16,
        /// Its name.
        name: &'static str,
        /// Its position in the message.
        position: usize,
        /// The criticality that the IE has.
        criticality: P::Criticality,
        /// The criticality that the object set assigns it.
        assigned: P::Criticality,
    },
    /// An IE whose presence is mandatory, and that the message does not have.
    MissingIe {
        /// The identifier of the IE.
        id: u16,
        /// Its name.
        name: &'static str,
        /// The criticality that the object set assigns it.
        criticality: P::Criticality,
    },
    /// An IE whose value is one of a later version of its type: a number or a size
    /// outside the root of a constraint that has an extension marker, as a name of no
    /// character where the type has `SIZE (1..150, ...)`. It decodes, a sender of this
    /// version of the specification does not send it, and a receiver of this version
    /// does not comprehend it.
    OutsideRoot {
        /// The identifier of the IE.
        id: u16,
        /// Its name.
        name: &'static str,
        /// Its position in the message.
        position: usize,
        /// The criticality that the IE has.
        criticality: P::Criticality,
        /// What is outside its root: the error of an encoding that refuses it.
        error: Error,
    },
    /// An IE of the message whose value does not decode as its type: its octets end
    /// before the value does, go on after it, or hold a value outside a constraint.
    UndecodableIe {
        /// The identifier of the IE.
        id: u16,
        /// Its name.
        name: &'static str,
        /// Its position in the message.
        position: usize,
        /// The criticality that the IE has.
        criticality: P::Criticality,
        /// Why its value does not decode: the error that the macro which extracts
        /// the IE returns.
        error: Error,
    },
    /// An encoding that ITU-T X.691 gives one form for, in another: a value that is
    /// whole, which a sender by the clause writes otherwise, as a length of 127 or less
    /// in two octets or padding bits that are not zero. It is read, and a receiver
    /// answers nothing for it: a tool that tests a peer says that the encoder of the
    /// peer is not canonical.
    NotCanonical {
        /// The IE in whose value it is, as its identifier, its name and its position
        /// in the message. `None` for what is around the values: the container of the
        /// message, and the identifier, the criticality and the length of each IE.
        ie: Option<(u16, &'static str, usize)>,
        /// What is in another form, which has the clause that gives its form.
        fault: Fault,
        /// The bit at which it is: of the value of the IE, or of the octets of the
        /// message.
        bit: usize,
    },
}

impl<P: Diagnostics> Deviation<P> {
    /// The IE that the deviation is about, as its identifier and the criticality that
    /// the receiver acts on: the one that the IE has, when it is in the message, and
    /// the one that the object set assigns it, when it is missing. `None` for the
    /// criticality of the PDU, and for an IE that is there twice or in a wrong order:
    /// the receiver rejects the procedure for it, whatever its criticality.
    pub fn ie(&self) -> Option<(u16, P::Criticality)> {
        match self {
            Self::PduCriticality { .. } => None,
            Self::UnexpectedIe {
                id, criticality, ..
            }
            | Self::IeCriticality {
                id, criticality, ..
            }
            | Self::MissingIe {
                id, criticality, ..
            }
            | Self::OutsideRoot {
                id, criticality, ..
            }
            | Self::UndecodableIe {
                id, criticality, ..
            } => Some((*id, *criticality)),
            Self::RepeatedIe { .. } | Self::OutOfOrder { .. } | Self::NotCanonical { .. } => None,
        }
    }

    /// How the Criticality Diagnostics of an answer say what is wrong with the IE:
    /// `not_understood` for an IE that the message cannot have, for one whose value
    /// does not decode and for one whose value is of a later version of its type, and
    /// `missing` for one that is. `None` for a deviation that is no fault of one IE
    /// which a receiver reports so.
    pub fn type_of_error(&self) -> Option<P::TypeOfError> {
        match self {
            Self::UnexpectedIe { .. } | Self::UndecodableIe { .. } | Self::OutsideRoot { .. } => {
                Some(P::type_of_error(false))
            }
            Self::MissingIe { .. } => Some(P::type_of_error(true)),
            _ => None,
        }
    }

    /// The item of the Criticality Diagnostics IE that tells of this deviation: the
    /// criticality of [`ie`](Self::ie), its identifier and
    /// [`type_of_error`](Self::type_of_error). `None` where there is no type of error,
    /// and for an IE of the criticality `ignore`.
    ///
    /// A receiver reports the IEs that it does not comprehend and those that are
    /// missing when their criticality is `reject` or `notify`, and ignores the others
    /// (clauses 10.3.4.2 and 10.3.5 of TS 38.413 and of TS 36.413): the criticality of an
    /// item is never `ignore` (TS 38.413 clause 9.3.1.3, TS 36.413 clause 9.2.1.21).
    pub fn diagnostics_item(&self) -> Option<P::Item> {
        let (id, criticality) = self.ie()?;
        if matches!(criticality.into(), Criticality::Ignore) {
            return None;
        }
        let type_of_error = self.type_of_error()?;
        Some(P::item(criticality, id, type_of_error))
    }

    /// The protocol cause that the specification has a receiver answer the message
    /// with, for this deviation alone:
    ///
    /// - `transfer_syntax_error` for an IE whose value does not decode;
    /// - `abstract_syntax_error_reject` and `abstract_syntax_error_ignore_and_notify`
    ///   for an IE that the message cannot have and for one whose value is of a later
    ///   version of its type, by the criticality that it has, and for an IE that is
    ///   missing, by the criticality that the object set assigns it;
    /// - `abstract_syntax_error_falsely_constructed_message` for an IE that is there
    ///   twice or in a wrong order.
    ///
    /// `None` where the receiver goes on: for an IE of the criticality `ignore` that
    /// it does not comprehend or that is missing, for a criticality that is not the
    /// assigned one, and for an encoding that is not the canonical one.
    pub fn cause(&self) -> Option<P::Cause> {
        let by_criticality = |criticality: &P::Criticality| match (*criticality).into() {
            Criticality::Reject => Some(ProtocolCause::AbstractSyntaxErrorReject),
            Criticality::Notify => Some(ProtocolCause::AbstractSyntaxErrorIgnoreAndNotify),
            Criticality::Ignore => None,
        };
        let cause = match self {
            Self::UndecodableIe { .. } => Some(ProtocolCause::TransferSyntaxError),
            Self::UnexpectedIe { criticality, .. }
            | Self::MissingIe { criticality, .. }
            | Self::OutsideRoot { criticality, .. } => by_criticality(criticality),
            Self::RepeatedIe { .. } | Self::OutOfOrder { .. } => {
                Some(ProtocolCause::AbstractSyntaxErrorFalselyConstructedMessage)
            }
            Self::PduCriticality { .. }
            | Self::IeCriticality { .. }
            | Self::NotCanonical { .. } => None,
        };
        cause.map(P::cause)
    }

    /// The deviation that an error of the macros tells of: the IE that a `req` field
    /// misses, with the criticality that the object set assigns it, and the IE whose
    /// value does not decode, with the criticality that it has. `None` for an error
    /// that is about no IE.
    pub fn of(error: &Error) -> Option<Self> {
        let ie = error.ie()?;
        let (id, criticality) = (ie.id, P::Criticality::from(ie.criticality));
        let name = P::name(id).unwrap_or_default();
        match (error.kind(), ie.position) {
            (ErrorKind::MissingIe, None) => Some(Self::MissingIe {
                id,
                name,
                criticality,
            }),
            (ErrorKind::UndecodableIe, Some(position)) => Some(Self::UndecodableIe {
                id,
                name,
                position,
                criticality,
                error: error.clone(),
            }),
            _ => None,
        }
    }
}

impl<P: Diagnostics> fmt::Display for Deviation<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PduCriticality {
                criticality,
                assigned,
            } => write!(
                formatter,
                "the criticality of the PDU is {criticality:?}, and that of its procedure \
                 is {assigned:?}"
            ),
            Self::UnexpectedIe {
                id, name, position, ..
            } => match name {
                Some(name) => write!(
                    formatter,
                    "{name} (IE {id}), at position {position}, is not an IE of the message"
                ),
                None => write!(
                    formatter,
                    "IE {id}, at position {position}, is not an IE of the message, nor of \
                     the protocol"
                ),
            },
            Self::RepeatedIe {
                id,
                name,
                position,
                first,
            } => write!(
                formatter,
                "{name} (IE {id}), at position {position}, is in the message already, at \
                 position {first}"
            ),
            Self::OutOfOrder {
                id,
                name,
                position,
                after,
                after_name,
            } => write!(
                formatter,
                "{name} (IE {id}), at position {position}, comes after {after_name} (IE \
                 {after}), which the object set of the message has after it"
            ),
            Self::IeCriticality {
                id,
                name,
                position,
                criticality,
                assigned,
            } => write!(
                formatter,
                "the criticality of {name} (IE {id}), at position {position}, is \
                 {criticality:?}, and the message assigns it {assigned:?}"
            ),
            Self::MissingIe { id, name, .. } => write!(
                formatter,
                "{name} (IE {id}) is mandatory, and the message does not have it"
            ),
            Self::UndecodableIe {
                id,
                name,
                position,
                error,
                ..
            } => write!(
                formatter,
                "{name} (IE {id}), at position {position}, does not decode: {}",
                error.reason()
            ),
            Self::OutsideRoot {
                id,
                name,
                position,
                error,
                ..
            } => write!(
                formatter,
                "{name} (IE {id}), at position {position}, has a value of a later version \
                 of its type: {}",
                error.reason()
            ),
            Self::NotCanonical {
                ie: Some((id, name, position)),
                fault,
                bit,
            } => write!(
                formatter,
                "the value of {name} (IE {id}), at position {position}, is not written as \
                 its sender is to write it: at bit {bit}, {fault}"
            ),
            Self::NotCanonical {
                ie: None,
                fault,
                bit,
            } => write!(
                formatter,
                "the message is not written as its sender is to write it: at bit {bit}, {fault}"
            ),
        }
    }
}

/// What is wrong with the value of an IE, against the type that the bindings give it.
#[derive(Debug)]
pub enum Wrong {
    /// The octets do not decode: why, as the macro that extracts the IE says it.
    Undecodable(Error),
    /// The value is outside the root of an extensible constraint: the error of an
    /// encoding that refuses it.
    OutsideRoot(Error),
}

/// Add to `found` how `ies`, the IEs of a message in their order, each as its identifier
/// and its criticality, deviate from `set`, the IEs of the object set of that message,
/// each as its identifier, the criticality that the set assigns it and whether its
/// presence is mandatory: those of the IEs in the order of the message, then the
/// mandatory IEs that are missing in the order of the set. `value` says what is wrong
/// with the value of the IE at a position, for an IE of the set whose value is not one
/// of its type, and what of its encoding is in another form than the one of X.691.
pub fn deviations<P: Diagnostics>(
    set: &[(u16, P::Criticality, bool)],
    ies: &[(u16, P::Criticality)],
    value: &dyn Fn(usize) -> (Option<Wrong>, Faults),
    found: &mut Vec<Deviation<P>>,
) {
    let name = P::name;
    // Where the message has each IE of the set first, and the IE that the set has last
    // among those that the message had so far.
    let mut first: Vec<Option<usize>> = vec![None; set.len()];
    let mut last: Option<usize> = None;
    for (position, (id, criticality)) in ies.iter().enumerate() {
        let (id, criticality) = (*id, *criticality);
        let Some(index) = set.iter().position(|(known, ..)| *known == id) else {
            found.push(Deviation::UnexpectedIe {
                id,
                name: name(id),
                position,
                criticality,
            });
            continue;
        };
        let named = || name(id).unwrap_or_default();
        match (first[index], last) {
            (Some(first), _) => found.push(Deviation::RepeatedIe {
                id,
                name: named(),
                position,
                first,
            }),
            (None, Some(last)) if index < last => {
                first[index] = Some(position);
                let (after, ..) = set[last];
                found.push(Deviation::OutOfOrder {
                    id,
                    name: named(),
                    position,
                    after,
                    after_name: name(after).unwrap_or_default(),
                });
            }
            (None, _) => {
                first[index] = Some(position);
                last = Some(index);
            }
        }
        let (_, assigned, _) = set[index];
        if criticality != assigned {
            found.push(Deviation::IeCriticality {
                id,
                name: named(),
                position,
                criticality,
                assigned,
            });
        }
        let (wrong, faults) = value(position);
        match wrong {
            Some(Wrong::Undecodable(error)) => found.push(Deviation::UndecodableIe {
                id,
                name: named(),
                position,
                criticality,
                error,
            }),
            Some(Wrong::OutsideRoot(error)) => found.push(Deviation::OutsideRoot {
                id,
                name: named(),
                position,
                criticality,
                error,
            }),
            None => {}
        }
        found.extend(faults.iter().map(|noted| Deviation::NotCanonical {
            ie: Some((id, named(), position)),
            fault: noted.fault,
            bit: noted.bit,
        }));
    }
    for ((id, criticality, mandatory), first) in set.iter().zip(first) {
        if *mandatory && first.is_none() {
            found.push(Deviation::MissingIe {
                id: *id,
                name: name(*id).unwrap_or_default(),
                criticality: *criticality,
            });
        }
    }
}
