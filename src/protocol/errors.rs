//! The errors that name what the registry of a protocol has: the message of a PDU, and an
//! IE by its identifier.

use alloc::format;
use alloc::string::{String, ToString};

use super::{Container, CriticalityOf, Protocol, name, object_set};
use crate::{Error, ErrorIe};

/// The IE of an identifier, as an error names it: the name that ASN.1 gives it after
/// `id-`, or its number for one that the protocol does not have.
pub fn ie_name<P: Protocol>(id: u16) -> String {
    match name::<P>(id) {
        Some(name) => name.to_string(),
        None => format!("IE {id}"),
    }
}

/// A PDU of the message of `direction` and `code`, which is read as the message that
/// ASN.1 names `expected`.
#[doc(hidden)]
pub fn __wrong_message<P: Protocol>(expected: &str, direction: &str, code: u8) -> Error {
    let held = object_set::<P>(direction, code).map(|set| set.name);
    Error::wrong_message(expected, held, direction, code)
}

/// The IE `id`, which the field `field` requires, is not in a message, or a transfer, of
/// the type `M`. The error is about that IE, with the criticality that the object set of
/// the message assigns it.
#[doc(hidden)]
pub fn __missing_ie<'a, M: Container<'a>>(id: u16, field: &str) -> Error {
    let error = Error::missing_ie(M::NAME, &ie_name::<M::Protocol>(id), field);
    match M::__assigned(id) {
        Some(assigned) => error.about_ie(ErrorIe::new(id, assigned, None)),
        // An IE that the object set of the message does not have, or of a transfer.
        None => error,
    }
}

/// `error` is why the value of the IE `id` does not decode, in a message that ASN.1 names
/// `message`. The error is about that IE, with the criticality that it was received with
/// and its position in the message.
#[doc(hidden)]
pub fn __undecodable_ie<P: Protocol>(
    error: Error,
    message: &str,
    id: u16,
    criticality: CriticalityOf<P>,
    position: usize,
) -> Error {
    let about = ErrorIe::new(id, criticality.into(), Some(position));
    error
        .undecodable_ie(message, &ie_name::<P>(id))
        .about_ie(about)
}
