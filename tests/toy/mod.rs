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

// A protocol of two procedures, written as the generator of the repository writes the
// bindings and the registry of oxirush-ngap and oxirush-s1ap: the tests of
// `oxirush_aper::protocol` and the README read and build its PDUs with the macros and the
// readers of the crate, as those crates do.
//
// ```text
// Setup         reject  InitiatingMessage SetupRequest    [Identity reject mandatory,
//                                                          Name ignore optional,
//                                                          Items reject optional]
//                       SuccessfulOutcome SetupResponse   [Identity reject mandatory]
// Notification  ignore  InitiatingMessage Notification    [Identity ignore mandatory]
// Private       ignore  InitiatingMessage PrivateMessage  (no container of protocol IEs)
//
// Identity ::= INTEGER (0..4294967295)       id 1
// Name     ::= OCTET STRING                  id 2
// Items    ::= SEQUENCE (SIZE (1..16)) OF ProtocolIE-SingleContainer {{ItemIEs}}  id 3
// Item     ::= INTEGER (0..255)              id 10, reject, in the single containers of Items
// ```


pub use registry::{Deviation, Toy, ToyPduKind, ie};

/// The bindings, as the generator writes them for a protocol.
#[allow(dead_code, non_camel_case_types, clippy::new_without_default)]
pub mod toy {
    use oxirush_aper::{AsnType, Decode, Encode, Error, Octets, Reader, UNBOUNDED, Writer};

    /// Read a SEQUENCE of a container of protocol IEs, with an extension marker.
    macro_rules! message {
        ($name:ident $asn:literal) => {
            #[derive(Debug, Clone, PartialEq, Eq)]
            pub struct $name<'a> {
                pub protocol_ies: ProtocolIEContainer<'a>,
            }
            impl<'a> $name<'a> {
                pub fn new(protocol_ies: ProtocolIEContainer<'a>) -> Self {
                    Self { protocol_ies }
                }
                pub fn into_owned(self) -> $name<'static> {
                    $name {
                        protocol_ies: self.protocol_ies.into_owned(),
                    }
                }
            }
            impl AsnType for $name<'_> {
                const NAME: &'static str = $asn;
            }
            impl<'a> Decode<'a> for $name<'a> {
                fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
                    let extended = reader.bit()?;
                    let protocol_ies = ProtocolIEContainer::decode(reader)
                        .map_err(|error| error.in_field("protocol_ies"))?;
                    if extended {
                        reader.skip_additions()?;
                    }
                    Ok(Self { protocol_ies })
                }
            }
            impl Encode for $name<'_> {
                fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
                    writer.bit(false);
                    self.protocol_ies
                        .encode(writer)
                        .map_err(|error| error.in_field("protocol_ies"))
                }
            }
        };
    }

    /// `Criticality ::= ENUMERATED { reject, ignore, notify }`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Criticality {
        reject,
        ignore,
        notify,
    }
    impl AsnType for Criticality {
        const NAME: &'static str = "Criticality";
    }
    impl<'a> Decode<'a> for Criticality {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            const VALUES: [Criticality; 3] =
                [Criticality::reject, Criticality::ignore, Criticality::notify];
            Ok(VALUES[reader.enumerated(3, 0, false)? as usize])
        }
    }
    impl Encode for Criticality {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.index(*self as u64, 3, false);
            Ok(())
        }
    }

    /// `ProtocolIE-ID ::= INTEGER (0..65535)`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ProtocolIEID(pub u16);
    impl AsnType for ProtocolIEID {
        const NAME: &'static str = "ProtocolIE-ID";
    }
    impl<'a> Decode<'a> for ProtocolIEID {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self(reader.unsigned(0, 65535)? as u16))
        }
    }
    impl Encode for ProtocolIEID {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.unsigned(u64::from(self.0), 0, 65535)
        }
    }

    /// `ProtocolIE-Field`: an IE of a container, and a `ProtocolIE-SingleContainer`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ProtocolIEField<'a> {
        pub id: ProtocolIEID,
        pub criticality: Criticality,
        pub value: Octets<'a>,
    }
    impl<'a> ProtocolIEField<'a> {
        pub fn new(id: u16, criticality: Criticality, value: Octets<'a>) -> Self {
            let id = ProtocolIEID(id);
            Self {
                id,
                criticality,
                value,
            }
        }
        pub fn into_owned(self) -> ProtocolIEField<'static> {
            let (id, criticality) = (self.id, self.criticality);
            let value = self.value.into_owned();
            ProtocolIEField {
                id,
                criticality,
                value,
            }
        }
    }
    impl AsnType for ProtocolIEField<'_> {
        const NAME: &'static str = "ProtocolIE-Field";
    }
    impl<'a> Decode<'a> for ProtocolIEField<'a> {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let id = ProtocolIEID::decode(reader).map_err(|error| error.in_field("id"))?;
            let criticality =
                Criticality::decode(reader).map_err(|error| error.in_field("criticality"))?;
            let value = reader.open_type().map_err(|error| error.in_field("value"))?;
            Ok(Self {
                id,
                criticality,
                value,
            })
        }
    }
    impl Encode for ProtocolIEField<'_> {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            self.id.encode(writer)?;
            self.criticality.encode(writer)?;
            writer.open_type(&self.value);
            Ok(())
        }
    }

    /// `ProtocolIE-Container ::= SEQUENCE (SIZE (0..65535)) OF ProtocolIE-Field`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ProtocolIEContainer<'a>(pub Vec<ProtocolIEField<'a>>);
    impl ProtocolIEContainer<'_> {
        pub fn into_owned(self) -> ProtocolIEContainer<'static> {
            ProtocolIEContainer(self.0.into_iter().map(ProtocolIEField::into_owned).collect())
        }
    }
    impl AsnType for ProtocolIEContainer<'_> {
        const NAME: &'static str = "ProtocolIE-Container";
    }
    impl<'a> Decode<'a> for ProtocolIEContainer<'a> {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let count = reader.size(0, 65535)?;
            let mut list = reader.list(count);
            for _ in 0..count {
                list.push(ProtocolIEField::decode(reader)?);
            }
            Ok(Self(list))
        }
    }
    impl Encode for ProtocolIEContainer<'_> {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.size(self.0.len(), 0, 65535)?;
            self.0.iter().try_for_each(|field| field.encode(writer))
        }
    }

    message!(SetupRequest "SetupRequest");
    message!(SetupResponse "SetupResponse");
    message!(Notification "Notification");

    /// `PrivateMessage ::= SEQUENCE { privateIEs OCTET STRING, ... }`: no container of
    /// protocol IEs.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct PrivateMessage<'a> {
        pub private_ies: Octets<'a>,
    }
    impl PrivateMessage<'_> {
        pub fn into_owned(self) -> PrivateMessage<'static> {
            PrivateMessage {
                private_ies: self.private_ies.into_owned(),
            }
        }
    }
    impl AsnType for PrivateMessage<'_> {
        const NAME: &'static str = "PrivateMessage";
    }
    impl<'a> Decode<'a> for PrivateMessage<'a> {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let extended = reader.bit()?;
            let private_ies = reader.octet_string(0, UNBOUNDED)?;
            if extended {
                reader.skip_additions()?;
            }
            Ok(Self { private_ies })
        }
    }
    impl Encode for PrivateMessage<'_> {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.bit(false);
            writer.octet_string(&self.private_ies, 0, UNBOUNDED)
        }
    }

    /// `Identity ::= INTEGER (0..4294967295)`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Identity(pub u32);
    impl AsnType for Identity {
        const NAME: &'static str = "Identity";
    }
    impl<'a> Decode<'a> for Identity {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self(reader.unsigned(0, 4294967295)? as u32))
        }
    }
    impl Encode for Identity {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.unsigned(u64::from(self.0), 0, 4294967295)
        }
    }
    impl From<u32> for Identity {
        fn from(value: u32) -> Self {
            Self(value)
        }
    }
    impl oxirush_aper::protocol::__Field<u32> for Identity {
        fn __field(self) -> u32 {
            self.0
        }
    }

    /// `Name ::= OCTET STRING`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Name<'a>(pub Octets<'a>);
    impl Name<'_> {
        pub fn into_owned(self) -> Name<'static> {
            Name(self.0.into_owned())
        }
    }
    impl AsnType for Name<'_> {
        const NAME: &'static str = "Name";
    }
    impl<'a> Decode<'a> for Name<'a> {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self(reader.octet_string(0, UNBOUNDED)?))
        }
    }
    impl Encode for Name<'_> {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.octet_string(&self.0, 0, UNBOUNDED)
        }
    }
    impl From<&'static str> for Name<'static> {
        fn from(value: &'static str) -> Self {
            Self(Octets::from(value.as_bytes()))
        }
    }
    impl From<Vec<u8>> for Name<'static> {
        fn from(value: Vec<u8>) -> Self {
            Self(Octets::from(value))
        }
    }

    /// `Items ::= SEQUENCE (SIZE (1..16)) OF ProtocolIE-SingleContainer {{ItemIEs}}`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Items<'a>(pub Vec<ProtocolIEField<'a>>);
    impl Items<'_> {
        pub fn into_owned(self) -> Items<'static> {
            Items(self.0.into_iter().map(ProtocolIEField::into_owned).collect())
        }
    }
    impl AsnType for Items<'_> {
        const NAME: &'static str = "Items";
    }
    impl<'a> Decode<'a> for Items<'a> {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let count = reader.size(1, 16)?;
            let mut list = reader.list(count);
            for _ in 0..count {
                list.push(ProtocolIEField::decode(reader)?);
            }
            Ok(Self(list))
        }
    }
    impl Encode for Items<'_> {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.size(self.0.len(), 1, 16)?;
            self.0.iter().try_for_each(|field| field.encode(writer))
        }
    }
    impl<'a> From<Vec<ProtocolIEField<'a>>> for Items<'a> {
        fn from(value: Vec<ProtocolIEField<'a>>) -> Self {
            Self(value)
        }
    }

    /// `Item ::= INTEGER (0..255)`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Item(pub u8);
    impl AsnType for Item {
        const NAME: &'static str = "Item";
    }
    impl<'a> Decode<'a> for Item {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self(reader.unsigned(0, 255)? as u8))
        }
    }
    impl Encode for Item {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.unsigned(u64::from(self.0), 0, 255)
        }
    }
    impl From<u8> for Item {
        fn from(value: u8) -> Self {
            Self(value)
        }
    }

    /// `ProcedureCode ::= INTEGER (0..255)`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ProcedureCode(pub u8);

    /// `TriggeringMessage ::= ENUMERATED { initiating-message, successful-outcome,
    /// unsuccessful-outcome }`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum TriggeringMessage {
        initiating_message,
        successful_outcome,
        unsuccessful_outcome,
    }

    /// `TypeOfError ::= ENUMERATED { not-understood, missing, ... }`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum TypeOfError {
        not_understood,
        missing,
    }

    /// `CauseProtocol`, of the causes that a receiver answers its deviations with.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum CauseProtocol {
        transfer_syntax_error,
        abstract_syntax_error_reject,
        abstract_syntax_error_ignore_and_notify,
        abstract_syntax_error_falsely_constructed_message,
        unspecified,
    }

    /// `CriticalityDiagnostics-IE-Item`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct CriticalityDiagnosticsIEItem {
        pub ie_criticality: Criticality,
        pub ie_id: ProtocolIEID,
        pub type_of_error: TypeOfError,
    }
    impl CriticalityDiagnosticsIEItem {
        pub fn new(
            ie_criticality: Criticality,
            ie_id: ProtocolIEID,
            type_of_error: TypeOfError,
            _: Option<()>,
        ) -> Self {
            Self {
                ie_criticality,
                ie_id,
                type_of_error,
            }
        }
    }

    /// `CriticalityDiagnostics-IE-List`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct CriticalityDiagnosticsIEList(pub Vec<CriticalityDiagnosticsIEItem>);

    /// `CriticalityDiagnostics`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct CriticalityDiagnostics {
        pub procedure_code: Option<ProcedureCode>,
        pub triggering_message: Option<TriggeringMessage>,
        pub procedure_criticality: Option<Criticality>,
        pub ies_criticality_diagnostics: Option<CriticalityDiagnosticsIEList>,
    }
    impl CriticalityDiagnostics {
        pub fn new(
            procedure_code: Option<ProcedureCode>,
            triggering_message: Option<TriggeringMessage>,
            procedure_criticality: Option<Criticality>,
            ies_criticality_diagnostics: Option<CriticalityDiagnosticsIEList>,
            _: Option<()>,
        ) -> Self {
            Self {
                procedure_code,
                triggering_message,
                procedure_criticality,
                ies_criticality_diagnostics,
            }
        }
    }

    /// A message of a PDU, of one direction.
    macro_rules! direction {
        ($name:ident) => {
            #[derive(Debug, Clone, PartialEq, Eq)]
            pub struct $name<'a> {
                pub procedure_code: ProcedureCode,
                pub criticality: Criticality,
                pub value: Octets<'a>,
            }
        };
    }
    direction!(InitiatingMessage);
    direction!(SuccessfulOutcome);
    direction!(UnsuccessfulOutcome);

    /// `TOY-PDU ::= CHOICE { initiatingMessage, successfulOutcome, unsuccessfulOutcome, ... }`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum TOYPDU<'a> {
        initiatingMessage(InitiatingMessage<'a>),
        successfulOutcome(SuccessfulOutcome<'a>),
        unsuccessfulOutcome(UnsuccessfulOutcome<'a>),
    }
    /// A PDU that owns its octets.
    pub type TOY_PDU = TOYPDU<'static>;
    impl AsnType for TOYPDU<'_> {
        const NAME: &'static str = "TOY-PDU";
    }
    impl<'a> Decode<'a> for TOYPDU<'a> {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let alternative = reader.choice(3, 0, true)?;
            let procedure_code = ProcedureCode(reader.unsigned(0, 255)? as u8);
            let criticality = Criticality::decode(reader)?;
            let value = reader.open_type()?;
            Ok(match alternative {
                0 => Self::initiatingMessage(InitiatingMessage {
                    procedure_code,
                    criticality,
                    value,
                }),
                1 => Self::successfulOutcome(SuccessfulOutcome {
                    procedure_code,
                    criticality,
                    value,
                }),
                _ => Self::unsuccessfulOutcome(UnsuccessfulOutcome {
                    procedure_code,
                    criticality,
                    value,
                }),
            })
        }
    }
    impl Encode for TOYPDU<'_> {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            let (alternative, code, criticality, value) = match self {
                Self::initiatingMessage(m) => (0, m.procedure_code, m.criticality, &m.value),
                Self::successfulOutcome(m) => (1, m.procedure_code, m.criticality, &m.value),
                Self::unsuccessfulOutcome(m) => (2, m.procedure_code, m.criticality, &m.value),
            };
            writer.bit(false);
            writer.index(alternative, 3, false);
            writer.unsigned(u64::from(code.0), 0, 255)?;
            criticality.encode(writer)?;
            writer.open_type(value);
            Ok(())
        }
    }
    impl<'a> TOYPDU<'a> {
        /// The PDU of octets.
        pub fn decode(octets: &'a [u8]) -> Result<Self, Error> {
            oxirush_aper::decode(octets)
        }
        /// The octets of the PDU.
        pub fn encode(&self) -> Result<Vec<u8>, Error> {
            oxirush_aper::encode(self)
        }
        /// The direction of the PDU.
        pub fn direction(&self) -> &'static str {
            match self {
                Self::initiatingMessage(_) => "InitiatingMessage",
                Self::successfulOutcome(_) => "SuccessfulOutcome",
                Self::unsuccessfulOutcome(_) => "UnsuccessfulOutcome",
            }
        }
        /// The procedure code of the PDU.
        pub fn procedure_code(&self) -> u8 {
            match self {
                Self::initiatingMessage(m) => m.procedure_code.0,
                Self::successfulOutcome(m) => m.procedure_code.0,
                Self::unsuccessfulOutcome(m) => m.procedure_code.0,
            }
        }
        /// The message of the PDU, as its type.
        pub fn decode_value<'s, T: oxirush_aper::protocol::Message<'s, Protocol = crate::Toy>>(
            &'s self,
        ) -> Result<T, Error> {
            let (direction, code) = (self.direction(), self.procedure_code());
            if (direction, code) != (T::DIRECTION, T::PROCEDURE_CODE) {
                let expected = <T as AsnType>::NAME;
                return Err(oxirush_aper::protocol::__wrong_message::<crate::Toy>(
                    expected, direction, code,
                ));
            }
            oxirush_aper::decode(<Self as oxirush_aper::protocol::Pdu<'a>>::value(self))
        }

        /// Every occurrence of the IE `T` in the message of this PDU.
        pub fn ies<T: oxirush_aper::protocol::Ie<Protocol = crate::Toy>>(
            &self,
        ) -> oxirush_aper::protocol::Ies<'_, T> {
            oxirush_aper::protocol::Source::__of_pdu(self).ies::<T>()
        }

        /// The first occurrence of the IE `T` in the message of this PDU.
        pub fn ie<T: oxirush_aper::protocol::Ie<Protocol = crate::Toy>>(
            &self,
        ) -> Option<Result<T::Value<'_>, Error>> {
            self.ies::<T>().next()
        }

        /// The items of the list `L` in the message of this PDU.
        pub fn items<L: oxirush_aper::protocol::List<Protocol = crate::Toy>>(
            &self,
        ) -> oxirush_aper::protocol::Items<'_, L> {
            oxirush_aper::protocol::Source::__of_pdu(self).items::<L>()
        }

        /// How the PDU deviates from what its object set assigns its message.
        pub fn check(&self) -> Result<Vec<crate::Deviation>, Error> {
            oxirush_aper::protocol::check(self)
        }

        /// How the container of the IEs of the PDU deviates from its object set.
        pub fn check_structure(&self) -> Result<Vec<crate::Deviation>, Error> {
            oxirush_aper::protocol::check_structure(self)
        }
    }

    /// The open type of a value.
    pub fn encode_open_type<T: Encode>(value: &T) -> Result<Octets<'static>, Error> {
        oxirush_aper::encode(value).map(Octets::owned)
    }

    pub use crate::registry::ToyPduKind;
}

/// The registry, as the generator writes it from the ASN.1.
#[rustfmt::skip]
pub mod registry {
oxirush_aper::__protocol! { protocol Toy "TOY", bindings toy, pdu TOYPDU, triggering initiating_message successful_outcome unsuccessful_outcome }
oxirush_aper::__procedures! {
    protocol Toy "TOY", bindings toy, pdu TOYPDU, kind ToyPduKind, hooks any();
    1 Setup reject { InitiatingMessage Initiating_Setup "SetupRequest" crate::toy::SetupRequest<'a> [1 reject mandatory, 2 ignore optional, 3 reject optional]; SuccessfulOutcome Successful_Setup "SetupResponse" crate::toy::SetupResponse<'a> [1 reject mandatory]; }
    2 Notification ignore { InitiatingMessage Initiating_Notification "Notification" crate::toy::Notification<'a> [1 ignore mandatory]; }
    3 Private ignore { InitiatingMessage Initiating_Private "PrivateMessage" crate::toy::PrivateMessage<'a>; }
}
oxirush_aper::__ies! {
    protocol Toy "TOY";
    1 "Identity" crate::toy::Identity => Identity;
    2 "Name" crate::toy::Name<'a> => Name;
    3 "Items" crate::toy::Items<'a> => Items;
    10 reject "Item" crate::toy::Item => Item;
}
oxirush_aper::__lists! {
    Items Item 1 16;
}
}

/// The functions of a type for the trees of `inspect`, which the toy protocol has none of.
#[cfg(feature = "inspect")]
pub mod inspect {
    use oxirush_aper::{Error, ErrorKind, inspect::Decoded, json::Value};

    #[allow(clippy::extra_unused_type_parameters)]
    pub(crate) fn __decode_typed<T>(_: &'static str, _: &[u8]) -> Result<Decoded, Error> {
        Err(Error::new(ErrorKind::Form, "no tree"))
    }

    #[allow(clippy::extra_unused_type_parameters)]
    pub(crate) fn __encode_typed<T>(_: &Value) -> Result<Vec<u8>, Error> {
        Err(Error::new(ErrorKind::Form, "no tree"))
    }
}

// The macros of a protocol crate call those of oxirush-aper through its re-export,
// `$crate::aper`, as a user of the crate may not depend on oxirush-aper; the toy calls them
// by the name of the crate, which a documentation test cannot re-export.

/// Build a PDU of the toy protocol, as `build_ngap!` builds one of NGAP.
#[macro_export]
macro_rules! build_toy {
    ($($arguments:tt)*) => {
        ::oxirush_aper::__build!($crate, toy, "build_toy!", $($arguments)*)
    };
}

/// Build one IE of the toy protocol, as `build_ngap_ie!` builds one of NGAP.
#[macro_export]
macro_rules! build_toy_ie {
    ($($arguments:tt)*) => {
        ::oxirush_aper::__build_ie!($crate, toy, "build_toy_ie!", $($arguments)*)
    };
}

/// Extract the IEs of a message or of a PDU of the toy protocol, as `extract_ngap_ies!`
/// does for NGAP.
#[macro_export]
macro_rules! extract_toy_ies {
    ($($arguments:tt)*) => {
        ::oxirush_aper::__extract_ies!($crate, toy, $($arguments)*)
    };
}

/// Change one IE of a message of the toy protocol, as `with_ngap_ie_mut!` does for NGAP.
#[macro_export]
macro_rules! with_toy_ie_mut {
    ($($arguments:tt)*) => {
        ::oxirush_aper::__with_ie_mut!($crate, toy, $($arguments)*)
    };
}
