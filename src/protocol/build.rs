//! What the macros that build a PDU and an IE call: the octets of a message, written IE by
//! IE, and the criticality that an object set assigns.

use alloc::format;
use alloc::vec::Vec;
use core::marker::PhantomData;

use super::{Container, CriticalityOf, Field, Message, Pdu, Protocol};
use crate::{Encode, Error, ErrorKind, Octets, Writer};

/// For the macro that builds an IE: one entry of the IEs of a message, which is the
/// identifier of an IE, its criticality and its value in an open type, or why the value
/// does not encode.
#[doc(hidden)]
pub fn __ie<P: Protocol, T: Encode>(
    id: u16,
    criticality: CriticalityOf<P>,
    value: &T,
) -> Result<P::Field<'static>, Error> {
    let octets = crate::encode(value).map(Octets::owned)?;
    Ok(P::Field::__new(id, criticality, octets))
}

/// The criticality that the object set of `M` assigns to the IE `id`, which the macros
/// name `name`: an error of the kind `Argument` for one that the set does not have.
fn assigned<'a, M: Container<'a>>(
    id: u16,
    name: &str,
) -> Result<CriticalityOf<M::Protocol>, Error> {
    M::__assigned(id).map(Into::into).ok_or_else(|| {
        let why = format!(
            "{name} is no IE of the object set of {}, which assigns it no criticality: write \
             its criticality",
            M::NAME
        );
        Error::new(ErrorKind::Argument, why)
    })
}

/// For the macro that builds one IE of a message without its criticality: the IE `id`
/// with the criticality that the object set of `M` assigns to it.
#[doc(hidden)]
pub fn __assigned_ie<'a, M: Container<'a>, T: Encode>(
    id: u16,
    name: &str,
    value: &T,
) -> Result<<M::Protocol as Protocol>::Field<'static>, Error> {
    __ie::<M::Protocol, T>(id, assigned::<M>(id, name)?, value)
}

/// For the macro that builds a PDU: the message that is being written, a container of
/// protocol IEs. Each IE is written into the octets of the message, with its value in its
/// open type, so that a PDU is built without octets for each IE.
#[doc(hidden)]
pub struct __Message<P: Protocol> {
    octets: Vec<u8>,
    count: usize,
    /// Why the first IE that did not encode did not.
    error: Option<Error>,
    protocol: PhantomData<P>,
}

impl<P: Protocol> __Message<P> {
    /// A message without an IE yet: its extension bit, and the place of the count of its
    /// IEs, which is two octets (X.691 (02/2021) §19.1, §20.6, §11.5.7.3).
    #[doc(hidden)]
    #[allow(clippy::new_without_default)]
    #[inline]
    pub fn new() -> Self {
        // Room for a message of the usual size, so that its octets seldom move as the IEs
        // are written.
        let mut octets = Vec::with_capacity(256);
        octets.extend_from_slice(&[0, 0, 0]);
        Self {
            octets,
            count: 0,
            error: None,
            protocol: PhantomData,
        }
    }

    /// One more IE: its identifier, its criticality, and its value in an open type.
    #[doc(hidden)]
    #[inline]
    pub fn ie<T: Encode>(&mut self, id: u16, criticality: CriticalityOf<P>, value: &T) {
        if self.error.is_some() {
            return;
        }
        let start = self.octets.len();
        let criticality: crate::Criticality = criticality.into();
        let mut writer = Writer::new(&mut self.octets);
        writer.octets(&id.to_be_bytes());
        writer.index(criticality as u64, 3, false);
        let value_start = writer.begin_open_type();
        match value.encode(&mut writer) {
            Ok(()) => {
                writer.end_open_type(value_start);
                self.count += 1;
            }
            Err(error) => {
                self.octets.truncate(start);
                self.error = Some(error.encoding(T::NAME));
            }
        }
    }

    /// One more IE, of the criticality that the object set of the message `M` assigns to
    /// it: an error of the kind `Argument` for an IE, which the macros name `name`, that
    /// the set does not have.
    #[doc(hidden)]
    #[inline]
    pub fn ie_assigned<'a, M: Container<'a, Protocol = P>, T: Encode>(
        &mut self,
        id: u16,
        name: &str,
        value: &T,
    ) {
        if self.error.is_some() {
            return;
        }
        match assigned::<M>(id, name) {
            Ok(criticality) => self.ie(id, criticality, value),
            Err(error) => self.error = Some(error),
        }
    }

    /// The PDU of the message `M` of these IEs, with the direction and the procedure code
    /// of that message, or the first error of an IE that did not encode.
    #[doc(hidden)]
    pub fn pdu<'a, M: Message<'a, Protocol = P>>(
        mut self,
        criticality: CriticalityOf<P>,
    ) -> Result<P::Pdu<'static>, Error> {
        if let Some(error) = self.error {
            return Err(error);
        }
        // A container has 65535 IEs at most.
        let Ok(count) = u16::try_from(self.count) else {
            let size = Error::new(ErrorKind::Encode, "the value does not encode").because(format!(
                "a size of {} is outside its constraint (0..65535)",
                self.count
            ));
            return Err(size.of_protocol_ies(self.count));
        };
        self.octets[1..3].copy_from_slice(&count.to_be_bytes());
        let value = Octets::owned(self.octets);
        // The direction of a message is one of the three.
        <P::Pdu<'static> as Pdu<'static>>::__of(M::DIRECTION, M::PROCEDURE_CODE, criticality, value)
            .ok_or_else(|| Error::new(ErrorKind::Encode, "a message has no direction"))
    }
}

/// For the macro that builds one IE: `M` is a message, or a transfer, and a container of
/// protocol IEs.
#[doc(hidden)]
#[inline]
pub fn __message_of_protocol_ies<'a, M: Container<'a>>() {
    const {
        assert!(
            M::__OF_PROTOCOL_IES,
            "the message has private IEs, and no container of protocol IEs"
        )
    };
}

/// For the macro that builds a PDU: `M` is the message that the procedure of `Q` has in
/// its direction, which is the type `Q` names.
#[doc(hidden)]
#[inline]
pub fn __message_of<M, Q>(_: fn(M) -> Q) {}
