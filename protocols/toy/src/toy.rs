// Auto-generated from ASN.1 by the generator of oxirush-aper; do not edit.
#![allow(clippy::large_enum_variant)]

#[allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused,
    clippy::too_many_arguments,
    clippy::collapsible_if
)]
pub mod toy_common_data_types {
    use super::*;
    #[cfg(feature = "inspect")]
    use crate::inspect_registry::names;
    use alloc::borrow::Cow;
    use alloc::vec::Vec;
    pub use oxirush_aper::Criticality;
    pub use oxirush_aper::Presence;
    #[cfg(feature = "inspect")]
    use oxirush_aper::json::{self, Reading, Refused, Tree, Value};
    use oxirush_aper::{
        AsnType, BitString, Bits, Decode, Encode, Error, ObjectIdentifier, Octets, Reader,
        UNBOUNDED, Writer, in_field,
    };
    /// `PrivateIE-ID ::= CHOICE`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub enum PrivateIEID {
        local(u16),
        global(ObjectIdentifier),
    }
    impl From<u16> for PrivateIEID {
        fn from(value: u16) -> Self {
            Self::local(value)
        }
    }
    impl From<ObjectIdentifier> for PrivateIEID {
        fn from(value: ObjectIdentifier) -> Self {
            Self::global(value)
        }
    }
    impl AsnType for PrivateIEID {
        const NAME: &'static str = "PrivateIE-ID";
    }
    impl<'a> Decode<'a> for PrivateIEID {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(match reader.index(2, 0, false)? {
                0 => Self::local(reader.unsigned(0, 65535, false)? as u16),
                _ => Self::global(reader.object_identifier()?),
            })
        }
    }
    impl Encode for PrivateIEID {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            match self {
                Self::local(value) => {
                    writer.index(0, 2, false);
                    writer.unsigned(u64::from(*value), 0, 65535, false)?;
                }
                Self::global(value) => {
                    writer.index(1, 2, false);
                    writer.object_identifier(value);
                }
            }
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for PrivateIEID {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            match self {
                Self::local(value) => json::alternative("local", json::unsigned(u64::from(*value))),
                Self::global(value) => json::alternative("global", json::object_identifier(value)),
            }
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let (position, value) = reading.alternative(value, names::PrivateIEID)?;
            Ok(match position {
                0 => Self::local(reading.within("local", value, |value, reading| {
                    reading
                        .unsigned(value, 0, 65535)
                        .map(|number| number as u16)
                })?),
                _ => Self::global(reading.within("global", value, |value, reading| {
                    json::read_object_identifier(reading, value)
                })?),
            })
        }
    }
    /// `ProcedureCode ::= INTEGER (0..255)`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
    pub struct ProcedureCode(pub u8);
    impl From<ProcedureCode> for u8 {
        #[inline]
        fn from(value: ProcedureCode) -> Self {
            value.0
        }
    }
    impl From<u8> for ProcedureCode {
        fn from(value: u8) -> Self {
            Self(value)
        }
    }
    impl AsnType for ProcedureCode {
        const NAME: &'static str = "ProcedureCode";
    }
    impl<'a> Decode<'a> for ProcedureCode {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self(reader.unsigned(0, 255, false)? as u8))
        }
    }
    impl Encode for ProcedureCode {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.unsigned(u64::from(self.0), 0, 255, false)?;
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for ProcedureCode {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            json::unsigned(u64::from(self.0))
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            Ok(Self(
                reading.unsigned(value, 0, 255).map(|number| number as u8)?,
            ))
        }
    }
    /// `ProtocolExtensionID ::= INTEGER (0..65535)`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
    pub struct ProtocolExtensionID(pub u16);
    impl From<ProtocolExtensionID> for u16 {
        #[inline]
        fn from(value: ProtocolExtensionID) -> Self {
            value.0
        }
    }
    impl From<u16> for ProtocolExtensionID {
        fn from(value: u16) -> Self {
            Self(value)
        }
    }
    impl AsnType for ProtocolExtensionID {
        const NAME: &'static str = "ProtocolExtensionID";
    }
    impl<'a> Decode<'a> for ProtocolExtensionID {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self(reader.unsigned(0, 65535, false)? as u16))
        }
    }
    impl Encode for ProtocolExtensionID {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.unsigned(u64::from(self.0), 0, 65535, false)?;
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for ProtocolExtensionID {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            json::unsigned(u64::from(self.0))
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            Ok(Self(
                reading
                    .unsigned(value, 0, 65535)
                    .map(|number| number as u16)?,
            ))
        }
    }
    /// `ProtocolIE-ID ::= INTEGER (0..65535)`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
    pub struct ProtocolIEID(pub u16);
    impl From<ProtocolIEID> for u16 {
        #[inline]
        fn from(value: ProtocolIEID) -> Self {
            value.0
        }
    }
    impl From<u16> for ProtocolIEID {
        fn from(value: u16) -> Self {
            Self(value)
        }
    }
    impl AsnType for ProtocolIEID {
        const NAME: &'static str = "ProtocolIE-ID";
    }
    impl<'a> Decode<'a> for ProtocolIEID {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self(reader.unsigned(0, 65535, false)? as u16))
        }
    }
    impl Encode for ProtocolIEID {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.unsigned(u64::from(self.0), 0, 65535, false)?;
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for ProtocolIEID {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            json::unsigned(u64::from(self.0))
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            Ok(Self(
                reading
                    .unsigned(value, 0, 65535)
                    .map(|number| number as u16)?,
            ))
        }
    }
    /// `TriggeringMessage ::= ENUMERATED`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
    pub enum TriggeringMessage {
        initiating_message = 0,
        successful_outcome = 1,
        unsuccessful_outcome = 2,
    }
    impl AsnType for TriggeringMessage {
        const NAME: &'static str = "TriggeringMessage";
    }
    impl TriggeringMessage {
        /// The values in the order of their indexes in PER.
        pub(crate) const VALUES: [Self; 3] = [
            Self::initiating_message,
            Self::successful_outcome,
            Self::unsuccessful_outcome,
        ];
        /// The index of the value in PER.
        #[inline]
        fn index(&self) -> u64 {
            *self as u64
        }
    }
    impl<'a> Decode<'a> for TriggeringMessage {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self::VALUES[reader.index(3, 0, false)? as usize])
        }
    }
    impl Encode for TriggeringMessage {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.index(self.index(), 3, false);
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for TriggeringMessage {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            json::text(names::TriggeringMessage[self.index() as usize])
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            Ok(Self::VALUES[reading.named(value, names::TriggeringMessage)?])
        }
    }
}
#[allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused,
    clippy::too_many_arguments,
    clippy::collapsible_if
)]
pub mod toy_constants {
    use super::*;
    #[cfg(feature = "inspect")]
    use crate::inspect_registry::names;
    use alloc::borrow::Cow;
    use alloc::vec::Vec;
    #[cfg(feature = "inspect")]
    use oxirush_aper::json::{self, Reading, Refused, Tree, Value};
    use oxirush_aper::{
        AsnType, BitString, Bits, Decode, Encode, Error, ObjectIdentifier, Octets, Reader,
        UNBOUNDED, Writer, in_field,
    };
    pub const ID_ALGORITHMS: ProtocolIEID = ProtocolIEID(27);
    pub const ID_AREAS: ProtocolIEID = ProtocolIEID(26);
    pub const ID_CAUSE: ProtocolIEID = ProtocolIEID(15);
    pub const ID_CONTAINER: ProtocolIEID = ProtocolIEID(25);
    pub const ID_COUNTS: ProtocolIEID = ProtocolIEID(28);
    pub const ID_CRITICALITY_DIAGNOSTICS: ProtocolIEID = ProtocolIEID(16);
    pub const ID_ERROR_INDICATION: ProcedureCode = ProcedureCode(3);
    pub const ID_EXTENDED_CAUSE: ProtocolIEID = ProtocolIEID(41);
    pub const ID_EXTENSION_INDEX: ProtocolIEID = ProtocolIEID(40);
    pub const ID_FAILURE_TRANSFER: ProtocolIEID = ProtocolIEID(34);
    pub const ID_ITEM: ProtocolIEID = ProtocolIEID(23);
    pub const ID_ITEMS: ProtocolIEID = ProtocolIEID(22);
    pub const ID_NAS_PDU: ProtocolIEID = ProtocolIEID(30);
    pub const ID_NODE_ID: ProtocolIEID = ProtocolIEID(32);
    pub const ID_NODE_NAME: ProtocolIEID = ProtocolIEID(20);
    pub const ID_NOTIFICATION: ProcedureCode = ProcedureCode(5);
    pub const ID_PRIVATE_MESSAGE: ProcedureCode = ProcedureCode(4);
    pub const ID_RELEASE: ProcedureCode = ProcedureCode(2);
    pub const ID_SECURITY_CONTEXT: ProtocolIEID = ProtocolIEID(33);
    pub const ID_SECURITY_KEY: ProtocolIEID = ProtocolIEID(21);
    pub const ID_SETUP: ProcedureCode = ProcedureCode(1);
    pub const ID_SETUP_TRANSFER: ProtocolIEID = ProtocolIEID(35);
    pub const ID_TRANSFER_KIND: ProtocolIEID = ProtocolIEID(31);
    pub const ID_TRANSFER_LIST: ProtocolIEID = ProtocolIEID(24);
    pub const ID_TUNNEL: ProtocolIEID = ProtocolIEID(29);
    pub const ID_UE_CORE_ID: ProtocolIEID = ProtocolIEID(10);
    pub const ID_UE_IDS: ProtocolIEID = ProtocolIEID(12);
    pub const ID_UE_RAN_ID: ProtocolIEID = ProtocolIEID(11);
    pub const MAX_PRIVATE_IES: u64 = 65535;
    pub const MAX_PROTOCOL_EXTENSIONS: u64 = 65535;
    pub const MAX_PROTOCOL_IES: u64 = 65535;
    pub const MAXNOOF_AREAS: u64 = 16;
    pub const MAXNOOF_CONNECTIONS: u64 = 65536;
    pub const MAXNOOF_ERRORS: u64 = 256;
    pub const MAXNOOF_ITEMS: u64 = 16;
    pub const MAXNOOF_TRANSFERS: u64 = 8;
}
#[allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused,
    clippy::too_many_arguments,
    clippy::collapsible_if
)]
pub mod toy_containers {
    use super::*;
    #[cfg(feature = "inspect")]
    use crate::inspect_registry::names;
    use alloc::borrow::Cow;
    use alloc::vec::Vec;
    #[cfg(feature = "inspect")]
    use oxirush_aper::json::{self, Reading, Refused, Tree, Value};
    use oxirush_aper::{
        AsnType, BitString, Bits, Decode, Encode, Error, ObjectIdentifier, Octets, Reader,
        UNBOUNDED, Writer, in_field,
    };
    impl core::fmt::Debug for ProtocolIEField<'_> {
        fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            let holds_key = crate::registry::KEY_IES.contains(&self.id.0);
            let mut shown = formatter.debug_struct("ProtocolIEField");
            shown.field("id", &self.id);
            shown.field("criticality", &self.criticality);
            match holds_key {
                true => shown.field("value", &format_args!("{} octets", self.value.len())),
                false => shown.field("value", &self.value),
            };
            shown.finish()
        }
    }
    /// `ProtocolIE-Field ::= SEQUENCE`.
    #[doc = " `ProtocolIE-Field`: an IE of a protocol IE container, and a `ProtocolIE-SingleContainer`."]
    #[doc = " The object set of a container does not show in its type: `value` is an open type."]
    #[derive(Clone, PartialEq, Eq, Hash)]
    pub struct ProtocolIEField<'a> {
        pub id: ProtocolIEID,
        pub criticality: Criticality,
        pub value: Octets<'a>,
    }
    impl<'a> ProtocolIEField<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `ProtocolIEField<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
        #[inline]
        pub fn into_owned(self) -> ProtocolIEField<'static> {
            ProtocolIEField {
                id: self.id,
                criticality: self.criticality,
                value: self.value.into_owned(),
            }
        }
    }
    impl AsnType for ProtocolIEField<'_> {
        const NAME: &'static str = "ProtocolIE-Field";
    }
    impl<'a> Decode<'a> for ProtocolIEField<'a> {
        #[inline(always)]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let id = ProtocolIEID::decode(reader).map_err(in_field("id"))?;
            let criticality = Criticality::decode(reader).map_err(in_field("criticality"))?;
            let value = reader.open_type().map_err(in_field("value"))?;
            Ok(Self {
                id,
                criticality,
                value,
            })
        }
    }
    impl Encode for ProtocolIEField<'_> {
        #[inline(always)]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            self.id.encode(writer).map_err(in_field("id"))?;
            self.criticality
                .encode(writer)
                .map_err(in_field("criticality"))?;
            writer.open_type(&self.value);
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for ProtocolIEField<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            let mut members = json::Members::new();
            members.put("id", self.id.to_tree());
            members.put("criticality", self.criticality.to_tree());
            members.put("value", json::octets(&self.value));
            members.finish()
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let members = reading.members(value, &["id", "criticality", "value"])?;
            let id = reading.member(&members, "id", ProtocolIEID::from_tree)?;
            let criticality = reading.member(&members, "criticality", Criticality::from_tree)?;
            let value = reading.member(&members, "value", |value, reading| {
                json::read_octets(reading, value)
            })?;
            Ok(Self {
                id,
                criticality,
                value,
            })
        }
    }
    /// `ProtocolIE-Container ::= SEQUENCE (SIZE (0..65535)) OF ProtocolIEField`.
    #[doc = " `ProtocolIE-Container`: the IEs of a message."]
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct ProtocolIEContainer<'a>(pub Vec<ProtocolIEField<'a>>);
    impl<'a> From<ProtocolIEContainer<'a>> for Vec<ProtocolIEField<'a>> {
        #[inline]
        fn from(value: ProtocolIEContainer<'a>) -> Self {
            value.0
        }
    }
    impl<'a> ProtocolIEContainer<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `ProtocolIEContainer<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
        pub fn into_owned(self) -> ProtocolIEContainer<'static> {
            ProtocolIEContainer(self.0.into_iter().map(|item| item.into_owned()).collect())
        }
    }
    impl<'a> From<Vec<ProtocolIEField<'a>>> for ProtocolIEContainer<'a> {
        fn from(value: Vec<ProtocolIEField<'a>>) -> Self {
            Self(value)
        }
    }
    impl AsnType for ProtocolIEContainer<'_> {
        const NAME: &'static str = "ProtocolIE-Container";
    }
    impl<'a> Decode<'a> for ProtocolIEContainer<'a> {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self({
                let count = reader.size(0, 65535)?;
                let mut list = reader.list(count);
                for _ in 0..count {
                    list.push(ProtocolIEField::decode(reader)?);
                }
                list
            }))
        }
    }
    impl Encode for ProtocolIEContainer<'_> {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.size(self.0.len(), 0, 65535)?;
            for item in self.0.iter() {
                item.encode(writer)?;
            }
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for ProtocolIEContainer<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            Value::Array(self.0.iter().map(|item| item.to_tree()).collect())
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            Ok(Self(reading.list(value, |value, reading| {
                ProtocolIEField::from_tree(value, reading)
            })?))
        }
    }
    impl core::fmt::Debug for ProtocolExtensionField<'_> {
        fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            let holds_key = crate::registry::KEY_IES.contains(&self.id.0);
            let mut shown = formatter.debug_struct("ProtocolExtensionField");
            shown.field("id", &self.id);
            shown.field("criticality", &self.criticality);
            match holds_key {
                true => shown.field(
                    "extension_value",
                    &format_args!("{} octets", self.extension_value.len()),
                ),
                false => shown.field("extension_value", &self.extension_value),
            };
            shown.finish()
        }
    }
    /// `ProtocolExtensionField ::= SEQUENCE`.
    #[doc = " `ProtocolExtensionField`: an IE of a protocol extension container."]
    #[derive(Clone, PartialEq, Eq, Hash)]
    pub struct ProtocolExtensionField<'a> {
        pub id: ProtocolExtensionID,
        pub criticality: Criticality,
        pub extension_value: Octets<'a>,
    }
    impl<'a> ProtocolExtensionField<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `ProtocolExtensionField<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
        #[inline]
        pub fn into_owned(self) -> ProtocolExtensionField<'static> {
            ProtocolExtensionField {
                id: self.id,
                criticality: self.criticality,
                extension_value: self.extension_value.into_owned(),
            }
        }
    }
    impl AsnType for ProtocolExtensionField<'_> {
        const NAME: &'static str = "ProtocolExtensionField";
    }
    impl<'a> Decode<'a> for ProtocolExtensionField<'a> {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let id = ProtocolExtensionID::decode(reader).map_err(in_field("id"))?;
            let criticality = Criticality::decode(reader).map_err(in_field("criticality"))?;
            let extension_value = reader.open_type().map_err(in_field("extension_value"))?;
            Ok(Self {
                id,
                criticality,
                extension_value,
            })
        }
    }
    impl Encode for ProtocolExtensionField<'_> {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            self.id.encode(writer).map_err(in_field("id"))?;
            self.criticality
                .encode(writer)
                .map_err(in_field("criticality"))?;
            writer.open_type(&self.extension_value);
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for ProtocolExtensionField<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            let mut members = json::Members::new();
            members.put("id", self.id.to_tree());
            members.put("criticality", self.criticality.to_tree());
            members.put("extensionValue", json::octets(&self.extension_value));
            members.finish()
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let members = reading.members(value, &["id", "criticality", "extensionValue"])?;
            let id = reading.member(&members, "id", ProtocolExtensionID::from_tree)?;
            let criticality = reading.member(&members, "criticality", Criticality::from_tree)?;
            let extension_value =
                reading.member(&members, "extensionValue", |value, reading| {
                    json::read_octets(reading, value)
                })?;
            Ok(Self {
                id,
                criticality,
                extension_value,
            })
        }
    }
    /// `ProtocolExtensionContainer ::= SEQUENCE (SIZE (1..65535)) OF ProtocolExtensionField`.
    #[doc = " `ProtocolExtensionContainer`: the `iE-Extensions` of a type."]
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct ProtocolExtensionContainer<'a>(pub Vec<ProtocolExtensionField<'a>>);
    impl<'a> From<ProtocolExtensionContainer<'a>> for Vec<ProtocolExtensionField<'a>> {
        #[inline]
        fn from(value: ProtocolExtensionContainer<'a>) -> Self {
            value.0
        }
    }
    impl<'a> ProtocolExtensionContainer<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `ProtocolExtensionContainer<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
        pub fn into_owned(self) -> ProtocolExtensionContainer<'static> {
            ProtocolExtensionContainer(self.0.into_iter().map(|item| item.into_owned()).collect())
        }
    }
    impl<'a> From<Vec<ProtocolExtensionField<'a>>> for ProtocolExtensionContainer<'a> {
        fn from(value: Vec<ProtocolExtensionField<'a>>) -> Self {
            Self(value)
        }
    }
    impl AsnType for ProtocolExtensionContainer<'_> {
        const NAME: &'static str = "ProtocolExtensionContainer";
    }
    impl<'a> Decode<'a> for ProtocolExtensionContainer<'a> {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self({
                let count = reader.size(1, 65535)?;
                let mut list = reader.list(count);
                for _ in 0..count {
                    list.push(ProtocolExtensionField::decode(reader)?);
                }
                list
            }))
        }
    }
    impl Encode for ProtocolExtensionContainer<'_> {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.size(self.0.len(), 1, 65535)?;
            for item in self.0.iter() {
                item.encode(writer)?;
            }
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for ProtocolExtensionContainer<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            Value::Array(self.0.iter().map(|item| item.to_tree()).collect())
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            Ok(Self(reading.list(value, |value, reading| {
                ProtocolExtensionField::from_tree(value, reading)
            })?))
        }
    }
}
#[allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused,
    clippy::too_many_arguments,
    clippy::collapsible_if
)]
pub mod toy_ies {
    use super::*;
    #[cfg(feature = "inspect")]
    use crate::inspect_registry::names;
    use alloc::borrow::Cow;
    use alloc::vec::Vec;
    #[cfg(feature = "inspect")]
    use oxirush_aper::json::{self, Reading, Refused, Tree, Value};
    use oxirush_aper::{
        AsnType, BitString, Bits, Decode, Encode, Error, ObjectIdentifier, Octets, Reader,
        UNBOUNDED, Writer, in_field,
    };
    /// `Algorithms ::= BIT STRING (SIZE (16, ...))`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct Algorithms<'a>(pub BitString<'a>);
    impl<'a> From<Algorithms<'a>> for BitString<'a> {
        #[inline]
        fn from(value: Algorithms<'a>) -> Self {
            value.0
        }
    }
    impl<'a> Algorithms<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `Algorithms<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
        #[inline]
        pub fn into_owned(self) -> Algorithms<'static> {
            Algorithms(self.0.into_owned())
        }
    }
    impl<'a> From<BitString<'a>> for Algorithms<'a> {
        fn from(value: BitString<'a>) -> Self {
            Self(value)
        }
    }
    impl AsnType for Algorithms<'_> {
        const NAME: &'static str = "Algorithms";
    }
    impl<'a> Decode<'a> for Algorithms<'a> {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self(reader.bit_string(16, 16, true)?))
        }
    }
    impl Encode for Algorithms<'_> {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.bit_string(&self.0, 16, 16, true)?;
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for Algorithms<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            json::bit_string(&self.0, 16, 16, true)
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            Ok(Self(json::read_bit_string(reading, value, 16, 16, true)?))
        }
    }
    /// `AreaItem ::= SEQUENCE`, with an extension marker.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct AreaItem {
        pub tac: TAC,
        pub broadcast_plmns: BroadcastPLMNs,
        pub ie_extensions: Option<ProtocolExtensionContainer<'static>>,
    }
    impl AsnType for AreaItem {
        const NAME: &'static str = "AreaItem";
    }
    impl<'a> Decode<'a> for AreaItem {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let extended = reader.bit()?;
            let present = reader.bits(1)?;
            let tac = TAC::decode(reader).map_err(in_field("tac"))?;
            let broadcast_plmns =
                BroadcastPLMNs::decode(reader).map_err(in_field("broadcast_plmns"))?;
            let ie_extensions = if present & 1 != 0 {
                Some(
                    (ProtocolExtensionContainer::decode(reader)
                        .map_err(in_field("ie_extensions"))?)
                    .into_owned(),
                )
            } else {
                None
            };
            if extended {
                reader.skip_additions()?;
            }
            Ok(Self {
                tac,
                broadcast_plmns,
                ie_extensions,
            })
        }
    }
    impl Encode for AreaItem {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.bit(false);
            writer.bit(self.ie_extensions.is_some());
            self.tac.encode(writer).map_err(in_field("tac"))?;
            self.broadcast_plmns
                .encode(writer)
                .map_err(in_field("broadcast_plmns"))?;
            if let Some(value) = &self.ie_extensions {
                value.encode(writer).map_err(in_field("ie_extensions"))?;
            }
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for AreaItem {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            let mut members = json::Members::new();
            members.put("tAC", self.tac.to_tree());
            members.put("broadcastPLMNs", self.broadcast_plmns.to_tree());
            members.put_optional(
                "iE-Extensions",
                self.ie_extensions.as_ref().map(|value| value.to_tree()),
            );
            members.finish()
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let members = reading.members(value, names::AreaItem)?;
            let tac = reading.member(&members, "tAC", TAC::from_tree)?;
            let broadcast_plmns =
                reading.member(&members, "broadcastPLMNs", BroadcastPLMNs::from_tree)?;
            let ie_extensions = reading.optional(
                &members,
                "iE-Extensions",
                ProtocolExtensionContainer::from_tree,
            )?;
            Ok(Self {
                tac,
                broadcast_plmns,
                ie_extensions,
            })
        }
    }
    /// `Areas ::= SEQUENCE (SIZE (1..16)) OF AreaItem`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct Areas(pub Vec<AreaItem>);
    impl From<Areas> for Vec<AreaItem> {
        #[inline]
        fn from(value: Areas) -> Self {
            value.0
        }
    }
    impl From<Vec<AreaItem>> for Areas {
        fn from(value: Vec<AreaItem>) -> Self {
            Self(value)
        }
    }
    impl AsnType for Areas {
        const NAME: &'static str = "Areas";
    }
    impl<'a> Decode<'a> for Areas {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self({
                let count = reader.size(1, 16)?;
                let mut list = reader.list(count);
                for _ in 0..count {
                    list.push(AreaItem::decode(reader)?);
                }
                list
            }))
        }
    }
    impl Encode for Areas {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.size(self.0.len(), 1, 16)?;
            for item in self.0.iter() {
                item.encode(writer)?;
            }
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for Areas {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            Value::Array(self.0.iter().map(|item| item.to_tree()).collect())
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            Ok(Self(reading.list(value, |value, reading| {
                AreaItem::from_tree(value, reading)
            })?))
        }
    }
    /// `BroadcastPLMNs ::= SEQUENCE (SIZE (1..6)) OF PLMNIdentity`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct BroadcastPLMNs(pub Vec<PLMNIdentity>);
    impl From<BroadcastPLMNs> for Vec<PLMNIdentity> {
        #[inline]
        fn from(value: BroadcastPLMNs) -> Self {
            value.0
        }
    }
    impl From<Vec<PLMNIdentity>> for BroadcastPLMNs {
        fn from(value: Vec<PLMNIdentity>) -> Self {
            Self(value)
        }
    }
    impl AsnType for BroadcastPLMNs {
        const NAME: &'static str = "BroadcastPLMNs";
    }
    impl<'a> Decode<'a> for BroadcastPLMNs {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self({
                let count = reader.size(1, 6)?;
                let mut list = reader.list(count);
                for _ in 0..count {
                    list.push(PLMNIdentity::decode(reader)?);
                }
                list
            }))
        }
    }
    impl Encode for BroadcastPLMNs {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.size(self.0.len(), 1, 6)?;
            for item in self.0.iter() {
                item.encode(writer)?;
            }
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for BroadcastPLMNs {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            Value::Array(self.0.iter().map(|item| item.to_tree()).collect())
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            Ok(Self(reading.list(value, |value, reading| {
                PLMNIdentity::from_tree(value, reading)
            })?))
        }
    }
    /// `Cause ::= CHOICE`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub enum Cause {
        radioNetwork(CauseRadioNetwork),
        nas(CauseNas),
        protocol(CauseProtocol),
        misc(CauseMisc),
        choice_Extensions(ProtocolIEField<'static>),
    }
    impl From<CauseRadioNetwork> for Cause {
        fn from(value: CauseRadioNetwork) -> Self {
            Self::radioNetwork(value)
        }
    }
    impl From<CauseNas> for Cause {
        fn from(value: CauseNas) -> Self {
            Self::nas(value)
        }
    }
    impl From<CauseProtocol> for Cause {
        fn from(value: CauseProtocol) -> Self {
            Self::protocol(value)
        }
    }
    impl From<CauseMisc> for Cause {
        fn from(value: CauseMisc) -> Self {
            Self::misc(value)
        }
    }
    impl From<ProtocolIEField<'static>> for Cause {
        fn from(value: ProtocolIEField<'static>) -> Self {
            Self::choice_Extensions(value)
        }
    }
    impl AsnType for Cause {
        const NAME: &'static str = "Cause";
    }
    impl<'a> Decode<'a> for Cause {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(match reader.index(5, 0, false)? {
                0 => Self::radioNetwork(CauseRadioNetwork::decode(reader)?),
                1 => Self::nas(CauseNas::decode(reader)?),
                2 => Self::protocol(CauseProtocol::decode(reader)?),
                3 => Self::misc(CauseMisc::decode(reader)?),
                _ => Self::choice_Extensions((ProtocolIEField::decode(reader)?).into_owned()),
            })
        }
    }
    impl Encode for Cause {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            match self {
                Self::radioNetwork(value) => {
                    writer.index(0, 5, false);
                    value.encode(writer)?;
                }
                Self::nas(value) => {
                    writer.index(1, 5, false);
                    value.encode(writer)?;
                }
                Self::protocol(value) => {
                    writer.index(2, 5, false);
                    value.encode(writer)?;
                }
                Self::misc(value) => {
                    writer.index(3, 5, false);
                    value.encode(writer)?;
                }
                Self::choice_Extensions(value) => {
                    writer.index(4, 5, false);
                    value.encode(writer)?;
                }
            }
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for Cause {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            match self {
                Self::radioNetwork(value) => json::alternative("radioNetwork", value.to_tree()),
                Self::nas(value) => json::alternative("nas", value.to_tree()),
                Self::protocol(value) => json::alternative("protocol", value.to_tree()),
                Self::misc(value) => json::alternative("misc", value.to_tree()),
                Self::choice_Extensions(value) => {
                    json::alternative("choice-Extensions", value.to_tree())
                }
            }
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let (position, value) = reading.alternative(value, names::Cause)?;
            Ok(match position {
                0 => Self::radioNetwork(reading.within(
                    "radioNetwork",
                    value,
                    CauseRadioNetwork::from_tree,
                )?),
                1 => Self::nas(reading.within("nas", value, CauseNas::from_tree)?),
                2 => Self::protocol(reading.within("protocol", value, CauseProtocol::from_tree)?),
                3 => Self::misc(reading.within("misc", value, CauseMisc::from_tree)?),
                _ => Self::choice_Extensions(reading.within(
                    "choice-Extensions",
                    value,
                    ProtocolIEField::from_tree,
                )?),
            })
        }
    }
    /// `CauseMisc ::= ENUMERATED`, with an extension marker.
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
    #[non_exhaustive]
    pub enum CauseMisc {
        control_processing_overload = 0,
        unspecified = 1,
    }
    impl AsnType for CauseMisc {
        const NAME: &'static str = "CauseMisc";
    }
    impl CauseMisc {
        /// The values in the order of their indexes in PER.
        pub(crate) const VALUES: [Self; 2] = [Self::control_processing_overload, Self::unspecified];
        /// The index of the value in PER.
        #[inline]
        fn index(&self) -> u64 {
            *self as u64
        }
    }
    impl<'a> Decode<'a> for CauseMisc {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self::VALUES[reader.index(2, 0, true)? as usize])
        }
    }
    impl Encode for CauseMisc {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.index(self.index(), 2, true);
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for CauseMisc {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            json::text(names::CauseMisc[self.index() as usize])
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            Ok(Self::VALUES[reading.named(value, names::CauseMisc)?])
        }
    }
    /// `CauseNas ::= ENUMERATED`, with an extension marker.
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
    #[non_exhaustive]
    pub enum CauseNas {
        normal_release = 0,
        deregister = 1,
        unspecified = 2,
    }
    impl AsnType for CauseNas {
        const NAME: &'static str = "CauseNas";
    }
    impl CauseNas {
        /// The values in the order of their indexes in PER.
        pub(crate) const VALUES: [Self; 3] =
            [Self::normal_release, Self::deregister, Self::unspecified];
        /// The index of the value in PER.
        #[inline]
        fn index(&self) -> u64 {
            *self as u64
        }
    }
    impl<'a> Decode<'a> for CauseNas {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self::VALUES[reader.index(3, 0, true)? as usize])
        }
    }
    impl Encode for CauseNas {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.index(self.index(), 3, true);
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for CauseNas {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            json::text(names::CauseNas[self.index() as usize])
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            Ok(Self::VALUES[reading.named(value, names::CauseNas)?])
        }
    }
    /// `CauseProtocol ::= ENUMERATED`, with an extension marker.
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
    #[non_exhaustive]
    pub enum CauseProtocol {
        transfer_syntax_error = 0,
        abstract_syntax_error_reject = 1,
        abstract_syntax_error_ignore_and_notify = 2,
        message_not_compatible_with_receiver_state = 3,
        semantic_error = 4,
        abstract_syntax_error_falsely_constructed_message = 5,
        unspecified = 6,
    }
    impl AsnType for CauseProtocol {
        const NAME: &'static str = "CauseProtocol";
    }
    impl CauseProtocol {
        /// The values in the order of their indexes in PER.
        pub(crate) const VALUES: [Self; 7] = [
            Self::transfer_syntax_error,
            Self::abstract_syntax_error_reject,
            Self::abstract_syntax_error_ignore_and_notify,
            Self::message_not_compatible_with_receiver_state,
            Self::semantic_error,
            Self::abstract_syntax_error_falsely_constructed_message,
            Self::unspecified,
        ];
        /// The index of the value in PER.
        #[inline]
        fn index(&self) -> u64 {
            *self as u64
        }
    }
    impl<'a> Decode<'a> for CauseProtocol {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self::VALUES[reader.index(7, 0, true)? as usize])
        }
    }
    impl Encode for CauseProtocol {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.index(self.index(), 7, true);
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for CauseProtocol {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            json::text(names::CauseProtocol[self.index() as usize])
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            Ok(Self::VALUES[reading.named(value, names::CauseProtocol)?])
        }
    }
    /// `CauseRadioNetwork ::= ENUMERATED`, with an extension marker.
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
    #[non_exhaustive]
    pub enum CauseRadioNetwork {
        unspecified = 0,
        user_inactivity = 1,
        release_due_to_ran_generated_reason = 2,
    }
    impl AsnType for CauseRadioNetwork {
        const NAME: &'static str = "CauseRadioNetwork";
    }
    impl CauseRadioNetwork {
        /// The values in the order of their indexes in PER.
        pub(crate) const VALUES: [Self; 3] = [
            Self::unspecified,
            Self::user_inactivity,
            Self::release_due_to_ran_generated_reason,
        ];
        /// The index of the value in PER.
        #[inline]
        fn index(&self) -> u64 {
            *self as u64
        }
    }
    impl<'a> Decode<'a> for CauseRadioNetwork {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self::VALUES[reader.index(3, 0, true)? as usize])
        }
    }
    impl Encode for CauseRadioNetwork {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.index(self.index(), 3, true);
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for CauseRadioNetwork {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            json::text(names::CauseRadioNetwork[self.index() as usize])
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            Ok(Self::VALUES[reading.named(value, names::CauseRadioNetwork)?])
        }
    }
    /// `ConnectionItem ::= SEQUENCE`, with an extension marker.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct ConnectionItem {
        pub ue_core_id: Option<UECoreID>,
        pub ue_ran_id: Option<UERANID>,
        pub ie_extensions: Option<ProtocolExtensionContainer<'static>>,
    }
    impl AsnType for ConnectionItem {
        const NAME: &'static str = "ConnectionItem";
    }
    impl<'a> Decode<'a> for ConnectionItem {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let extended = reader.bit()?;
            let present = reader.bits(3)?;
            let ue_core_id = if present & 4 != 0 {
                Some(UECoreID::decode(reader).map_err(in_field("ue_core_id"))?)
            } else {
                None
            };
            let ue_ran_id = if present & 2 != 0 {
                Some(UERANID::decode(reader).map_err(in_field("ue_ran_id"))?)
            } else {
                None
            };
            let ie_extensions = if present & 1 != 0 {
                Some(
                    (ProtocolExtensionContainer::decode(reader)
                        .map_err(in_field("ie_extensions"))?)
                    .into_owned(),
                )
            } else {
                None
            };
            if extended {
                reader.skip_additions()?;
            }
            Ok(Self {
                ue_core_id,
                ue_ran_id,
                ie_extensions,
            })
        }
    }
    impl Encode for ConnectionItem {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.bit(false);
            writer.bit(self.ue_core_id.is_some());
            writer.bit(self.ue_ran_id.is_some());
            writer.bit(self.ie_extensions.is_some());
            if let Some(value) = &self.ue_core_id {
                value.encode(writer).map_err(in_field("ue_core_id"))?;
            }
            if let Some(value) = &self.ue_ran_id {
                value.encode(writer).map_err(in_field("ue_ran_id"))?;
            }
            if let Some(value) = &self.ie_extensions {
                value.encode(writer).map_err(in_field("ie_extensions"))?;
            }
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for ConnectionItem {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            let mut members = json::Members::new();
            members.put_optional(
                "uE-Core-ID",
                self.ue_core_id.as_ref().map(|value| value.to_tree()),
            );
            members.put_optional(
                "uE-RAN-ID",
                self.ue_ran_id.as_ref().map(|value| value.to_tree()),
            );
            members.put_optional(
                "iE-Extensions",
                self.ie_extensions.as_ref().map(|value| value.to_tree()),
            );
            members.finish()
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let members = reading.members(value, names::ConnectionItem)?;
            let ue_core_id = reading.optional(&members, "uE-Core-ID", UECoreID::from_tree)?;
            let ue_ran_id = reading.optional(&members, "uE-RAN-ID", UERANID::from_tree)?;
            let ie_extensions = reading.optional(
                &members,
                "iE-Extensions",
                ProtocolExtensionContainer::from_tree,
            )?;
            Ok(Self {
                ue_core_id,
                ue_ran_id,
                ie_extensions,
            })
        }
    }
    /// `Connections ::= SEQUENCE (SIZE (1..65536)) OF ConnectionItem`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct Connections(pub Vec<ConnectionItem>);
    impl From<Connections> for Vec<ConnectionItem> {
        #[inline]
        fn from(value: Connections) -> Self {
            value.0
        }
    }
    impl From<Vec<ConnectionItem>> for Connections {
        fn from(value: Vec<ConnectionItem>) -> Self {
            Self(value)
        }
    }
    impl AsnType for Connections {
        const NAME: &'static str = "Connections";
    }
    impl<'a> Decode<'a> for Connections {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self({
                let mut list = Vec::new();
                let (mut count, mut more) = reader.length()?;
                loop {
                    let size = list.len().saturating_add(count);
                    if !more || size > 65536 {
                        Reader::check_size(size, 1, 65536)?;
                    }
                    for _ in 0..count {
                        list.push(ConnectionItem::decode(reader)?);
                    }
                    if !more {
                        break;
                    }
                    (count, more) = reader.length_after(count)?;
                }
                list
            }))
        }
    }
    impl Encode for Connections {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            Writer::check_size(self.0.len(), 1, 65536)?;
            let mut left = &self.0[..];
            loop {
                let (count, more) = writer.length(left.len());
                for item in &left[..count] {
                    item.encode(writer)?;
                }
                left = &left[count..];
                if !more {
                    break;
                }
            }
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for Connections {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            Value::Array(self.0.iter().map(|item| item.to_tree()).collect())
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            Ok(Self(reading.list(value, |value, reading| {
                ConnectionItem::from_tree(value, reading)
            })?))
        }
    }
    /// `Container ::= OCTET STRING`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct Container<'a>(pub Octets<'a>);
    impl<'a> From<Container<'a>> for Octets<'a> {
        #[inline]
        fn from(value: Container<'a>) -> Self {
            value.0
        }
    }
    impl<'a> Container<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `Container<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
        #[inline]
        pub fn into_owned(self) -> Container<'static> {
            Container(self.0.into_owned())
        }
    }
    impl<'a> From<Octets<'a>> for Container<'a> {
        fn from(value: Octets<'a>) -> Self {
            Self(value)
        }
    }
    impl<'a> From<Vec<u8>> for Container<'a> {
        fn from(value: Vec<u8>) -> Self {
            Self(Octets::from(value))
        }
    }
    impl<'a> From<&'a [u8]> for Container<'a> {
        fn from(value: &'a [u8]) -> Self {
            Self(Octets::from(value))
        }
    }
    impl AsnType for Container<'_> {
        const NAME: &'static str = "Container";
    }
    impl<'a> Decode<'a> for Container<'a> {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self(reader.octet_string(0, UNBOUNDED, false)?))
        }
    }
    impl Encode for Container<'_> {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.octet_string(&self.0, 0, UNBOUNDED, false)?;
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for Container<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            json::octets(&self.0)
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            Ok(Self(json::read_octets(reading, value)?))
        }
    }
    /// `ContainerBlob ::= SEQUENCE`, with an extension marker.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct ContainerBlob<'a> {
        pub tac: TAC,
        pub node_name: Option<NodeName<'a>>,
        pub message: Option<Octets<'a>>,
        pub kinds: Option<Kinds>,
        pub mixed: Option<Mixed>,
        pub ie_extensions: Option<ProtocolExtensionContainer<'static>>,
    }
    impl<'a> ContainerBlob<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `ContainerBlob<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
        pub fn into_owned(self) -> ContainerBlob<'static> {
            ContainerBlob {
                tac: self.tac,
                node_name: self.node_name.map(|value| value.into_owned()),
                message: self.message.map(|value| value.into_owned()),
                kinds: self.kinds,
                mixed: self.mixed,
                ie_extensions: self.ie_extensions,
            }
        }
    }
    impl AsnType for ContainerBlob<'_> {
        const NAME: &'static str = "ContainerBlob";
    }
    impl<'a> Decode<'a> for ContainerBlob<'a> {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let extended = reader.bit()?;
            let present = reader.bits(5)?;
            let tac = TAC::decode(reader).map_err(in_field("tac"))?;
            let node_name = if present & 16 != 0 {
                Some(NodeName::decode(reader).map_err(in_field("node_name"))?)
            } else {
                None
            };
            let message = if present & 8 != 0 {
                Some(
                    reader
                        .octet_string(1, 9600, false)
                        .map_err(in_field("message"))?,
                )
            } else {
                None
            };
            let kinds = if present & 4 != 0 {
                Some(Kinds::decode(reader).map_err(in_field("kinds"))?)
            } else {
                None
            };
            let mixed = if present & 2 != 0 {
                Some(Mixed::decode(reader).map_err(in_field("mixed"))?)
            } else {
                None
            };
            let ie_extensions = if present & 1 != 0 {
                Some(
                    (ProtocolExtensionContainer::decode(reader)
                        .map_err(in_field("ie_extensions"))?)
                    .into_owned(),
                )
            } else {
                None
            };
            if extended {
                reader.skip_additions()?;
            }
            Ok(Self {
                tac,
                node_name,
                message,
                kinds,
                mixed,
                ie_extensions,
            })
        }
    }
    impl Encode for ContainerBlob<'_> {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.bit(false);
            writer.bit(self.node_name.is_some());
            writer.bit(self.message.is_some());
            writer.bit(self.kinds.is_some());
            writer.bit(self.mixed.is_some());
            writer.bit(self.ie_extensions.is_some());
            self.tac.encode(writer).map_err(in_field("tac"))?;
            if let Some(value) = &self.node_name {
                value.encode(writer).map_err(in_field("node_name"))?;
            }
            if let Some(value) = &self.message {
                writer
                    .octet_string(value, 1, 9600, false)
                    .map_err(in_field("message"))?;
            }
            if let Some(value) = &self.kinds {
                value.encode(writer).map_err(in_field("kinds"))?;
            }
            if let Some(value) = &self.mixed {
                value.encode(writer).map_err(in_field("mixed"))?;
            }
            if let Some(value) = &self.ie_extensions {
                value.encode(writer).map_err(in_field("ie_extensions"))?;
            }
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for ContainerBlob<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            let mut members = json::Members::new();
            members.put("tAC", self.tac.to_tree());
            members.put_optional(
                "nodeName",
                self.node_name.as_ref().map(|value| value.to_tree()),
            );
            members.put_optional(
                "message",
                self.message.as_ref().map(|value| json::octets(value)),
            );
            members.put_optional("kinds", self.kinds.as_ref().map(|value| value.to_tree()));
            members.put_optional("mixed", self.mixed.as_ref().map(|value| value.to_tree()));
            members.put_optional(
                "iE-Extensions",
                self.ie_extensions.as_ref().map(|value| value.to_tree()),
            );
            members.finish()
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let members = reading.members(value, names::ContainerBlob)?;
            let tac = reading.member(&members, "tAC", TAC::from_tree)?;
            let node_name = reading.optional(&members, "nodeName", NodeName::from_tree)?;
            let message = reading.optional(&members, "message", |value, reading| {
                json::read_octets(reading, value)
            })?;
            let kinds = reading.optional(&members, "kinds", Kinds::from_tree)?;
            let mixed = reading.optional(&members, "mixed", Mixed::from_tree)?;
            let ie_extensions = reading.optional(
                &members,
                "iE-Extensions",
                ProtocolExtensionContainer::from_tree,
            )?;
            Ok(Self {
                tac,
                node_name,
                message,
                kinds,
                mixed,
                ie_extensions,
            })
        }
    }
    /// `Counts ::= SEQUENCE`, with an extension marker.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct Counts {
        pub count_ul: u64,
        pub counts: Option<u8>,
        pub count_dl: u64,
        pub ie_extensions: Option<ProtocolExtensionContainer<'static>>,
    }
    impl AsnType for Counts {
        const NAME: &'static str = "Counts";
    }
    impl<'a> Decode<'a> for Counts {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let extended = reader.bit()?;
            let present = reader.bits(2)?;
            let count_ul = reader
                .unsigned(0, 18446744073709551615, false)
                .map_err(in_field("count_ul"))?;
            let counts = if present & 2 != 0 {
                Some(reader.unsigned(0, 255, false).map_err(in_field("counts"))? as u8)
            } else {
                None
            };
            let count_dl = reader
                .unsigned(0, 4000000000000, true)
                .map_err(in_field("count_dl"))?;
            let ie_extensions = if present & 1 != 0 {
                Some(
                    (ProtocolExtensionContainer::decode(reader)
                        .map_err(in_field("ie_extensions"))?)
                    .into_owned(),
                )
            } else {
                None
            };
            if extended {
                reader.skip_additions()?;
            }
            Ok(Self {
                count_ul,
                counts,
                count_dl,
                ie_extensions,
            })
        }
    }
    impl Encode for Counts {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.bit(false);
            writer.bit(self.counts.is_some());
            writer.bit(self.ie_extensions.is_some());
            writer
                .unsigned(self.count_ul, 0, 18446744073709551615, false)
                .map_err(in_field("count_ul"))?;
            if let Some(value) = &self.counts {
                writer
                    .unsigned(u64::from(*value), 0, 255, false)
                    .map_err(in_field("counts"))?;
            }
            writer
                .unsigned(self.count_dl, 0, 4000000000000, true)
                .map_err(in_field("count_dl"))?;
            if let Some(value) = &self.ie_extensions {
                value.encode(writer).map_err(in_field("ie_extensions"))?;
            }
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for Counts {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            let mut members = json::Members::new();
            members.put("countUL", json::unsigned(self.count_ul));
            members.put_optional(
                "counts",
                self.counts
                    .as_ref()
                    .map(|value| json::unsigned(u64::from(*value))),
            );
            members.put("countDL", json::unsigned(self.count_dl));
            members.put_optional(
                "iE-Extensions",
                self.ie_extensions.as_ref().map(|value| value.to_tree()),
            );
            members.finish()
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let members = reading.members(value, names::Counts)?;
            let count_ul = reading.member(&members, "countUL", |value, reading| {
                reading.unsigned(value, 0, 18446744073709551615)
            })?;
            let counts = reading.optional(&members, "counts", |value, reading| {
                reading.unsigned(value, 0, 255).map(|number| number as u8)
            })?;
            let count_dl = reading.member(&members, "countDL", |value, reading| {
                reading.unsigned(value, u64::MIN, u64::MAX)
            })?;
            let ie_extensions = reading.optional(
                &members,
                "iE-Extensions",
                ProtocolExtensionContainer::from_tree,
            )?;
            Ok(Self {
                count_ul,
                counts,
                count_dl,
                ie_extensions,
            })
        }
    }
    /// `CriticalityDiagnostics ::= SEQUENCE`, with an extension marker.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct CriticalityDiagnostics {
        pub procedure_code: Option<ProcedureCode>,
        pub triggering_message: Option<TriggeringMessage>,
        pub procedure_criticality: Option<Criticality>,
        pub ies_criticality_diagnostics: Option<CriticalityDiagnosticsIEList>,
        pub ie_extensions: Option<ProtocolExtensionContainer<'static>>,
    }
    impl AsnType for CriticalityDiagnostics {
        const NAME: &'static str = "CriticalityDiagnostics";
    }
    impl<'a> Decode<'a> for CriticalityDiagnostics {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let extended = reader.bit()?;
            let present = reader.bits(5)?;
            let procedure_code = if present & 16 != 0 {
                Some(ProcedureCode::decode(reader).map_err(in_field("procedure_code"))?)
            } else {
                None
            };
            let triggering_message = if present & 8 != 0 {
                Some(TriggeringMessage::decode(reader).map_err(in_field("triggering_message"))?)
            } else {
                None
            };
            let procedure_criticality = if present & 4 != 0 {
                Some(Criticality::decode(reader).map_err(in_field("procedure_criticality"))?)
            } else {
                None
            };
            let ies_criticality_diagnostics = if present & 2 != 0 {
                Some(
                    CriticalityDiagnosticsIEList::decode(reader)
                        .map_err(in_field("ies_criticality_diagnostics"))?,
                )
            } else {
                None
            };
            let ie_extensions = if present & 1 != 0 {
                Some(
                    (ProtocolExtensionContainer::decode(reader)
                        .map_err(in_field("ie_extensions"))?)
                    .into_owned(),
                )
            } else {
                None
            };
            if extended {
                reader.skip_additions()?;
            }
            Ok(Self {
                procedure_code,
                triggering_message,
                procedure_criticality,
                ies_criticality_diagnostics,
                ie_extensions,
            })
        }
    }
    impl Encode for CriticalityDiagnostics {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.bit(false);
            writer.bit(self.procedure_code.is_some());
            writer.bit(self.triggering_message.is_some());
            writer.bit(self.procedure_criticality.is_some());
            writer.bit(self.ies_criticality_diagnostics.is_some());
            writer.bit(self.ie_extensions.is_some());
            if let Some(value) = &self.procedure_code {
                value.encode(writer).map_err(in_field("procedure_code"))?;
            }
            if let Some(value) = &self.triggering_message {
                value
                    .encode(writer)
                    .map_err(in_field("triggering_message"))?;
            }
            if let Some(value) = &self.procedure_criticality {
                value
                    .encode(writer)
                    .map_err(in_field("procedure_criticality"))?;
            }
            if let Some(value) = &self.ies_criticality_diagnostics {
                value
                    .encode(writer)
                    .map_err(in_field("ies_criticality_diagnostics"))?;
            }
            if let Some(value) = &self.ie_extensions {
                value.encode(writer).map_err(in_field("ie_extensions"))?;
            }
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for CriticalityDiagnostics {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            let mut members = json::Members::new();
            members.put_optional(
                "procedureCode",
                self.procedure_code.as_ref().map(|value| value.to_tree()),
            );
            members.put_optional(
                "triggeringMessage",
                self.triggering_message
                    .as_ref()
                    .map(|value| value.to_tree()),
            );
            members.put_optional(
                "procedureCriticality",
                self.procedure_criticality
                    .as_ref()
                    .map(|value| value.to_tree()),
            );
            members.put_optional(
                "iEsCriticalityDiagnostics",
                self.ies_criticality_diagnostics
                    .as_ref()
                    .map(|value| value.to_tree()),
            );
            members.put_optional(
                "iE-Extensions",
                self.ie_extensions.as_ref().map(|value| value.to_tree()),
            );
            members.finish()
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let members = reading.members(value, names::CriticalityDiagnostics)?;
            let procedure_code =
                reading.optional(&members, "procedureCode", ProcedureCode::from_tree)?;
            let triggering_message =
                reading.optional(&members, "triggeringMessage", TriggeringMessage::from_tree)?;
            let procedure_criticality =
                reading.optional(&members, "procedureCriticality", Criticality::from_tree)?;
            let ies_criticality_diagnostics = reading.optional(
                &members,
                "iEsCriticalityDiagnostics",
                CriticalityDiagnosticsIEList::from_tree,
            )?;
            let ie_extensions = reading.optional(
                &members,
                "iE-Extensions",
                ProtocolExtensionContainer::from_tree,
            )?;
            Ok(Self {
                procedure_code,
                triggering_message,
                procedure_criticality,
                ies_criticality_diagnostics,
                ie_extensions,
            })
        }
    }
    /// `CriticalityDiagnostics-IE-Item ::= SEQUENCE`, with an extension marker.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct CriticalityDiagnosticsIEItem {
        pub ie_criticality: Criticality,
        pub ie_id: ProtocolIEID,
        pub type_of_error: TypeOfError,
        pub ie_extensions: Option<ProtocolExtensionContainer<'static>>,
    }
    impl AsnType for CriticalityDiagnosticsIEItem {
        const NAME: &'static str = "CriticalityDiagnostics-IE-Item";
    }
    impl<'a> Decode<'a> for CriticalityDiagnosticsIEItem {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let extended = reader.bit()?;
            let present = reader.bits(1)?;
            let ie_criticality = Criticality::decode(reader).map_err(in_field("ie_criticality"))?;
            let ie_id = ProtocolIEID::decode(reader).map_err(in_field("ie_id"))?;
            let type_of_error = TypeOfError::decode(reader).map_err(in_field("type_of_error"))?;
            let ie_extensions = if present & 1 != 0 {
                Some(
                    (ProtocolExtensionContainer::decode(reader)
                        .map_err(in_field("ie_extensions"))?)
                    .into_owned(),
                )
            } else {
                None
            };
            if extended {
                reader.skip_additions()?;
            }
            Ok(Self {
                ie_criticality,
                ie_id,
                type_of_error,
                ie_extensions,
            })
        }
    }
    impl Encode for CriticalityDiagnosticsIEItem {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.bit(false);
            writer.bit(self.ie_extensions.is_some());
            self.ie_criticality
                .encode(writer)
                .map_err(in_field("ie_criticality"))?;
            self.ie_id.encode(writer).map_err(in_field("ie_id"))?;
            self.type_of_error
                .encode(writer)
                .map_err(in_field("type_of_error"))?;
            if let Some(value) = &self.ie_extensions {
                value.encode(writer).map_err(in_field("ie_extensions"))?;
            }
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for CriticalityDiagnosticsIEItem {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            let mut members = json::Members::new();
            members.put("iECriticality", self.ie_criticality.to_tree());
            members.put("iE-ID", self.ie_id.to_tree());
            members.put("typeOfError", self.type_of_error.to_tree());
            members.put_optional(
                "iE-Extensions",
                self.ie_extensions.as_ref().map(|value| value.to_tree()),
            );
            members.finish()
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let members = reading.members(value, names::CriticalityDiagnosticsIEItem)?;
            let ie_criticality =
                reading.member(&members, "iECriticality", Criticality::from_tree)?;
            let ie_id = reading.member(&members, "iE-ID", ProtocolIEID::from_tree)?;
            let type_of_error = reading.member(&members, "typeOfError", TypeOfError::from_tree)?;
            let ie_extensions = reading.optional(
                &members,
                "iE-Extensions",
                ProtocolExtensionContainer::from_tree,
            )?;
            Ok(Self {
                ie_criticality,
                ie_id,
                type_of_error,
                ie_extensions,
            })
        }
    }
    /// `CriticalityDiagnostics-IE-List ::= SEQUENCE (SIZE (1..256)) OF CriticalityDiagnosticsIEItem`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct CriticalityDiagnosticsIEList(pub Vec<CriticalityDiagnosticsIEItem>);
    impl From<CriticalityDiagnosticsIEList> for Vec<CriticalityDiagnosticsIEItem> {
        #[inline]
        fn from(value: CriticalityDiagnosticsIEList) -> Self {
            value.0
        }
    }
    impl From<Vec<CriticalityDiagnosticsIEItem>> for CriticalityDiagnosticsIEList {
        fn from(value: Vec<CriticalityDiagnosticsIEItem>) -> Self {
            Self(value)
        }
    }
    impl AsnType for CriticalityDiagnosticsIEList {
        const NAME: &'static str = "CriticalityDiagnostics-IE-List";
    }
    impl<'a> Decode<'a> for CriticalityDiagnosticsIEList {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self({
                let count = reader.size(1, 256)?;
                let mut list = reader.list(count);
                for _ in 0..count {
                    list.push(CriticalityDiagnosticsIEItem::decode(reader)?);
                }
                list
            }))
        }
    }
    impl Encode for CriticalityDiagnosticsIEList {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.size(self.0.len(), 1, 256)?;
            for item in self.0.iter() {
                item.encode(writer)?;
            }
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for CriticalityDiagnosticsIEList {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            Value::Array(self.0.iter().map(|item| item.to_tree()).collect())
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            Ok(Self(reading.list(value, |value, reading| {
                CriticalityDiagnosticsIEItem::from_tree(value, reading)
            })?))
        }
    }
    /// `ExtendedCause ::= ENUMERATED`, with an extension marker.
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
    #[non_exhaustive]
    pub enum ExtendedCause {
        congestion = 0,
        overload = 1,
    }
    impl AsnType for ExtendedCause {
        const NAME: &'static str = "ExtendedCause";
    }
    impl ExtendedCause {
        /// The values in the order of their indexes in PER.
        pub(crate) const VALUES: [Self; 2] = [Self::congestion, Self::overload];
        /// The index of the value in PER.
        #[inline]
        fn index(&self) -> u64 {
            *self as u64
        }
    }
    impl<'a> Decode<'a> for ExtendedCause {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self::VALUES[reader.index(2, 0, true)? as usize])
        }
    }
    impl Encode for ExtendedCause {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.index(self.index(), 2, true);
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for ExtendedCause {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            json::text(names::ExtendedCause[self.index() as usize])
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            Ok(Self::VALUES[reading.named(value, names::ExtendedCause)?])
        }
    }
    /// `FailureTransfer ::= SEQUENCE`, with an extension marker.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct FailureTransfer {
        pub cause: Cause,
        pub ie_extensions: Option<ProtocolExtensionContainer<'static>>,
    }
    impl AsnType for FailureTransfer {
        const NAME: &'static str = "FailureTransfer";
    }
    impl<'a> Decode<'a> for FailureTransfer {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let extended = reader.bit()?;
            let present = reader.bits(1)?;
            let cause = Cause::decode(reader).map_err(in_field("cause"))?;
            let ie_extensions = if present & 1 != 0 {
                Some(
                    (ProtocolExtensionContainer::decode(reader)
                        .map_err(in_field("ie_extensions"))?)
                    .into_owned(),
                )
            } else {
                None
            };
            if extended {
                reader.skip_additions()?;
            }
            Ok(Self {
                cause,
                ie_extensions,
            })
        }
    }
    impl Encode for FailureTransfer {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.bit(false);
            writer.bit(self.ie_extensions.is_some());
            self.cause.encode(writer).map_err(in_field("cause"))?;
            if let Some(value) = &self.ie_extensions {
                value.encode(writer).map_err(in_field("ie_extensions"))?;
            }
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for FailureTransfer {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            let mut members = json::Members::new();
            members.put("cause", self.cause.to_tree());
            members.put_optional(
                "iE-Extensions",
                self.ie_extensions.as_ref().map(|value| value.to_tree()),
            );
            members.finish()
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let members = reading.members(value, names::FailureTransfer)?;
            let cause = reading.member(&members, "cause", Cause::from_tree)?;
            let ie_extensions = reading.optional(
                &members,
                "iE-Extensions",
                ProtocolExtensionContainer::from_tree,
            )?;
            Ok(Self {
                cause,
                ie_extensions,
            })
        }
    }
    /// The octets, in hexadecimal.
    impl core::fmt::Display for GTPTEID {
        fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            self.0
                .iter()
                .try_for_each(|octet| write!(formatter, "{octet:02x}"))
        }
    }
    impl core::fmt::Debug for GTPTEID {
        fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(formatter, "GTPTEID({self})")
        }
    }
    /// `GTP-TEID ::= OCTET STRING (SIZE (4))`.
    #[derive(Clone, PartialEq, Eq, Hash, Copy)]
    pub struct GTPTEID(pub [u8; 4]);
    impl From<GTPTEID> for [u8; 4] {
        #[inline]
        fn from(value: GTPTEID) -> Self {
            value.0
        }
    }
    impl From<[u8; 4]> for GTPTEID {
        fn from(value: [u8; 4]) -> Self {
            Self(value)
        }
    }
    impl AsnType for GTPTEID {
        const NAME: &'static str = "GTP-TEID";
    }
    impl<'a> Decode<'a> for GTPTEID {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self(reader.octet_array::<4>()?))
        }
    }
    impl Encode for GTPTEID {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.octet_array(&self.0);
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for GTPTEID {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            json::readable(json::Form::Number(32), json::octets(&self.0))
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let value = &json::read_readable(reading, json::Form::Number(32), value)?;
            Ok(Self(reading.octet_array::<4>(value)?))
        }
    }
    /// `GTPTunnel ::= SEQUENCE`, with an extension marker.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct GTPTunnel<'a> {
        pub transport_layer_address: TransportLayerAddress<'a>,
        pub gtp_teid: GTPTEID,
        pub ie_extensions: Option<ProtocolExtensionContainer<'static>>,
    }
    impl<'a> GTPTunnel<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `GTPTunnel<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
        pub fn into_owned(self) -> GTPTunnel<'static> {
            GTPTunnel {
                transport_layer_address: self.transport_layer_address.into_owned(),
                gtp_teid: self.gtp_teid,
                ie_extensions: self.ie_extensions,
            }
        }
    }
    impl AsnType for GTPTunnel<'_> {
        const NAME: &'static str = "GTPTunnel";
    }
    impl<'a> Decode<'a> for GTPTunnel<'a> {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let extended = reader.bit()?;
            let present = reader.bits(1)?;
            let transport_layer_address = TransportLayerAddress::decode(reader)
                .map_err(in_field("transport_layer_address"))?;
            let gtp_teid = GTPTEID::decode(reader).map_err(in_field("gtp_teid"))?;
            let ie_extensions = if present & 1 != 0 {
                Some(
                    (ProtocolExtensionContainer::decode(reader)
                        .map_err(in_field("ie_extensions"))?)
                    .into_owned(),
                )
            } else {
                None
            };
            if extended {
                reader.skip_additions()?;
            }
            Ok(Self {
                transport_layer_address,
                gtp_teid,
                ie_extensions,
            })
        }
    }
    impl Encode for GTPTunnel<'_> {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.bit(false);
            writer.bit(self.ie_extensions.is_some());
            self.transport_layer_address
                .encode(writer)
                .map_err(in_field("transport_layer_address"))?;
            self.gtp_teid.encode(writer).map_err(in_field("gtp_teid"))?;
            if let Some(value) = &self.ie_extensions {
                value.encode(writer).map_err(in_field("ie_extensions"))?;
            }
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for GTPTunnel<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            let mut members = json::Members::new();
            members.put(
                "transportLayerAddress",
                self.transport_layer_address.to_tree(),
            );
            members.put("gTP-TEID", self.gtp_teid.to_tree());
            members.put_optional(
                "iE-Extensions",
                self.ie_extensions.as_ref().map(|value| value.to_tree()),
            );
            members.finish()
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let members = reading.members(value, names::GTPTunnel)?;
            let transport_layer_address = reading.member(
                &members,
                "transportLayerAddress",
                TransportLayerAddress::from_tree,
            )?;
            let gtp_teid = reading.member(&members, "gTP-TEID", GTPTEID::from_tree)?;
            let ie_extensions = reading.optional(
                &members,
                "iE-Extensions",
                ProtocolExtensionContainer::from_tree,
            )?;
            Ok(Self {
                transport_layer_address,
                gtp_teid,
                ie_extensions,
            })
        }
    }
    /// `IndexOfItem ::= INTEGER (1..8, ...)`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
    pub struct IndexOfItem(pub u64);
    impl From<IndexOfItem> for u64 {
        #[inline]
        fn from(value: IndexOfItem) -> Self {
            value.0
        }
    }
    impl From<u64> for IndexOfItem {
        fn from(value: u64) -> Self {
            Self(value)
        }
    }
    impl AsnType for IndexOfItem {
        const NAME: &'static str = "IndexOfItem";
    }
    impl<'a> Decode<'a> for IndexOfItem {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self(reader.unsigned(1, 8, true)?))
        }
    }
    impl Encode for IndexOfItem {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.unsigned(self.0, 1, 8, true)?;
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for IndexOfItem {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            json::unsigned(self.0)
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            Ok(Self(reading.unsigned(value, u64::MIN, u64::MAX)?))
        }
    }
    /// `IndexOfTransfer ::= INTEGER (1..4, ...)`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
    pub struct IndexOfTransfer(pub u64);
    impl From<IndexOfTransfer> for u64 {
        #[inline]
        fn from(value: IndexOfTransfer) -> Self {
            value.0
        }
    }
    impl From<u64> for IndexOfTransfer {
        fn from(value: u64) -> Self {
            Self(value)
        }
    }
    impl AsnType for IndexOfTransfer {
        const NAME: &'static str = "IndexOfTransfer";
    }
    impl<'a> Decode<'a> for IndexOfTransfer {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self(reader.unsigned(1, 4, true)?))
        }
    }
    impl Encode for IndexOfTransfer {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.unsigned(self.0, 1, 4, true)?;
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for IndexOfTransfer {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            json::unsigned(self.0)
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            Ok(Self(reading.unsigned(value, u64::MIN, u64::MAX)?))
        }
    }
    /// `Item ::= SEQUENCE`, with an extension marker.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct Item {
        pub item_id: u8,
        pub cause: Option<Cause>,
        pub ie_extensions: Option<ProtocolExtensionContainer<'static>>,
    }
    impl AsnType for Item {
        const NAME: &'static str = "Item";
    }
    impl<'a> Decode<'a> for Item {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let extended = reader.bit()?;
            let present = reader.bits(2)?;
            let item_id = reader
                .unsigned(0, 255, false)
                .map_err(in_field("item_id"))? as u8;
            let cause = if present & 2 != 0 {
                Some(Cause::decode(reader).map_err(in_field("cause"))?)
            } else {
                None
            };
            let ie_extensions = if present & 1 != 0 {
                Some(
                    (ProtocolExtensionContainer::decode(reader)
                        .map_err(in_field("ie_extensions"))?)
                    .into_owned(),
                )
            } else {
                None
            };
            if extended {
                reader.skip_additions()?;
            }
            Ok(Self {
                item_id,
                cause,
                ie_extensions,
            })
        }
    }
    impl Encode for Item {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.bit(false);
            writer.bit(self.cause.is_some());
            writer.bit(self.ie_extensions.is_some());
            writer
                .unsigned(u64::from(self.item_id), 0, 255, false)
                .map_err(in_field("item_id"))?;
            if let Some(value) = &self.cause {
                value.encode(writer).map_err(in_field("cause"))?;
            }
            if let Some(value) = &self.ie_extensions {
                value.encode(writer).map_err(in_field("ie_extensions"))?;
            }
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for Item {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            let mut members = json::Members::new();
            members.put("item-ID", json::unsigned(u64::from(self.item_id)));
            members.put_optional("cause", self.cause.as_ref().map(|value| value.to_tree()));
            members.put_optional(
                "iE-Extensions",
                self.ie_extensions.as_ref().map(|value| value.to_tree()),
            );
            members.finish()
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let members = reading.members(value, names::Item)?;
            let item_id = reading.member(&members, "item-ID", |value, reading| {
                reading.unsigned(value, 0, 255).map(|number| number as u8)
            })?;
            let cause = reading.optional(&members, "cause", Cause::from_tree)?;
            let ie_extensions = reading.optional(
                &members,
                "iE-Extensions",
                ProtocolExtensionContainer::from_tree,
            )?;
            Ok(Self {
                item_id,
                cause,
                ie_extensions,
            })
        }
    }
    /// `Items ::= SEQUENCE (SIZE (1..16)) OF ProtocolIEField`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct Items<'a>(pub Vec<ProtocolIEField<'a>>);
    impl<'a> From<Items<'a>> for Vec<ProtocolIEField<'a>> {
        #[inline]
        fn from(value: Items<'a>) -> Self {
            value.0
        }
    }
    impl<'a> Items<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `Items<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
        pub fn into_owned(self) -> Items<'static> {
            Items(self.0.into_iter().map(|item| item.into_owned()).collect())
        }
    }
    impl<'a> From<Vec<ProtocolIEField<'a>>> for Items<'a> {
        fn from(value: Vec<ProtocolIEField<'a>>) -> Self {
            Self(value)
        }
    }
    impl AsnType for Items<'_> {
        const NAME: &'static str = "Items";
    }
    impl<'a> Decode<'a> for Items<'a> {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self({
                let count = reader.size(1, 16)?;
                let mut list = reader.list(count);
                for _ in 0..count {
                    list.push(ProtocolIEField::decode(reader)?);
                }
                list
            }))
        }
    }
    impl Encode for Items<'_> {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.size(self.0.len(), 1, 16)?;
            for item in self.0.iter() {
                item.encode(writer)?;
            }
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for Items<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            Value::Array(self.0.iter().map(|item| item.to_tree()).collect())
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            Ok(Self(reading.list(value, |value, reading| {
                ProtocolIEField::from_tree(value, reading)
            })?))
        }
    }
    /// `KindOfTransfer ::= TransferKind`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
    pub struct KindOfTransfer(pub TransferKind);
    impl From<KindOfTransfer> for TransferKind {
        #[inline]
        fn from(value: KindOfTransfer) -> Self {
            value.0
        }
    }
    impl From<TransferKind> for KindOfTransfer {
        fn from(value: TransferKind) -> Self {
            Self(value)
        }
    }
    impl AsnType for KindOfTransfer {
        const NAME: &'static str = "KindOfTransfer";
    }
    impl<'a> Decode<'a> for KindOfTransfer {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self(TransferKind::decode(reader)?))
        }
    }
    impl Encode for KindOfTransfer {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            self.0.encode(writer)?;
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for KindOfTransfer {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            self.0.to_tree()
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            Ok(Self(TransferKind::from_tree(value, reading)?))
        }
    }
    /// `Kinds ::= SEQUENCE`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct Kinds {
        pub first: TransferKind,
        pub second: KindOfTransfer,
    }
    impl AsnType for Kinds {
        const NAME: &'static str = "Kinds";
    }
    impl<'a> Decode<'a> for Kinds {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let first = TransferKind::decode(reader).map_err(in_field("first"))?;
            let second = KindOfTransfer::decode(reader).map_err(in_field("second"))?;
            Ok(Self { first, second })
        }
    }
    impl Encode for Kinds {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            self.first.encode(writer).map_err(in_field("first"))?;
            self.second.encode(writer).map_err(in_field("second"))?;
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for Kinds {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            let mut members = json::Members::new();
            members.put("first", self.first.to_tree());
            members.put("second", self.second.to_tree());
            members.finish()
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let members = reading.members(value, names::Kinds)?;
            let first = reading.member(&members, "first", TransferKind::from_tree)?;
            let second = reading.member(&members, "second", KindOfTransfer::from_tree)?;
            Ok(Self { first, second })
        }
    }
    /// `Mixed ::= SEQUENCE`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct Mixed {
        pub kind: TransferKind,
        pub cause: CauseNas,
    }
    impl AsnType for Mixed {
        const NAME: &'static str = "Mixed";
    }
    impl<'a> Decode<'a> for Mixed {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let kind = TransferKind::decode(reader).map_err(in_field("kind"))?;
            let cause = CauseNas::decode(reader).map_err(in_field("cause"))?;
            Ok(Self { kind, cause })
        }
    }
    impl Encode for Mixed {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            self.kind.encode(writer).map_err(in_field("kind"))?;
            self.cause.encode(writer).map_err(in_field("cause"))?;
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for Mixed {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            let mut members = json::Members::new();
            members.put("kind", self.kind.to_tree());
            members.put("cause", self.cause.to_tree());
            members.finish()
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let members = reading.members(value, names::Mixed)?;
            let kind = reading.member(&members, "kind", TransferKind::from_tree)?;
            let cause = reading.member(&members, "cause", CauseNas::from_tree)?;
            Ok(Self { kind, cause })
        }
    }
    /// `NAS-PDU ::= OCTET STRING`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct NASPDU<'a>(pub Octets<'a>);
    impl<'a> From<NASPDU<'a>> for Octets<'a> {
        #[inline]
        fn from(value: NASPDU<'a>) -> Self {
            value.0
        }
    }
    impl<'a> NASPDU<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `NASPDU<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
        #[inline]
        pub fn into_owned(self) -> NASPDU<'static> {
            NASPDU(self.0.into_owned())
        }
    }
    impl<'a> From<Octets<'a>> for NASPDU<'a> {
        fn from(value: Octets<'a>) -> Self {
            Self(value)
        }
    }
    impl<'a> From<Vec<u8>> for NASPDU<'a> {
        fn from(value: Vec<u8>) -> Self {
            Self(Octets::from(value))
        }
    }
    impl<'a> From<&'a [u8]> for NASPDU<'a> {
        fn from(value: &'a [u8]) -> Self {
            Self(Octets::from(value))
        }
    }
    impl AsnType for NASPDU<'_> {
        const NAME: &'static str = "NAS-PDU";
    }
    impl<'a> Decode<'a> for NASPDU<'a> {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self(reader.octet_string(0, UNBOUNDED, false)?))
        }
    }
    impl Encode for NASPDU<'_> {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.octet_string(&self.0, 0, UNBOUNDED, false)?;
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for NASPDU<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            json::octets(&self.0)
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            Ok(Self(json::read_octets(reading, value)?))
        }
    }
    /// `NodeID ::= CHOICE`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub enum NodeID {
        macro_NodeID(Bits),
        home_NodeID(Bits),
        unassigned(()),
        choice_Extensions(ProtocolIEField<'static>),
    }
    impl From<()> for NodeID {
        fn from(value: ()) -> Self {
            Self::unassigned(value)
        }
    }
    impl From<ProtocolIEField<'static>> for NodeID {
        fn from(value: ProtocolIEField<'static>) -> Self {
            Self::choice_Extensions(value)
        }
    }
    impl AsnType for NodeID {
        const NAME: &'static str = "NodeID";
    }
    impl<'a> Decode<'a> for NodeID {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(match reader.index(4, 0, false)? {
                0 => Self::macro_NodeID(reader.bits_sized(20, 20)?),
                1 => Self::home_NodeID(reader.bits_sized(28, 28)?),
                2 => Self::unassigned(()),
                _ => Self::choice_Extensions((ProtocolIEField::decode(reader)?).into_owned()),
            })
        }
    }
    impl Encode for NodeID {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            match self {
                Self::macro_NodeID(value) => {
                    writer.index(0, 4, false);
                    writer.bits_sized(*value, 20, 20)?;
                }
                Self::home_NodeID(value) => {
                    writer.index(1, 4, false);
                    writer.bits_sized(*value, 28, 28)?;
                }
                Self::unassigned(value) => {
                    writer.index(2, 4, false);
                }
                Self::choice_Extensions(value) => {
                    writer.index(3, 4, false);
                    value.encode(writer)?;
                }
            }
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for NodeID {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            match self {
                Self::macro_NodeID(value) => {
                    json::alternative("macro-NodeID", json::bits(*value, 20, 20))
                }
                Self::home_NodeID(value) => {
                    json::alternative("home-NodeID", json::bits(*value, 28, 28))
                }
                Self::unassigned(value) => json::alternative("unassigned", Value::Null),
                Self::choice_Extensions(value) => {
                    json::alternative("choice-Extensions", value.to_tree())
                }
            }
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let (position, value) = reading.alternative(value, names::NodeID)?;
            Ok(match position {
                0 => Self::macro_NodeID(reading.within(
                    "macro-NodeID",
                    value,
                    |value, reading| json::read_bits(reading, value, 20, 20),
                )?),
                1 => {
                    Self::home_NodeID(reading.within("home-NodeID", value, |value, reading| {
                        json::read_bits(reading, value, 28, 28)
                    })?)
                }
                2 => Self::unassigned(
                    reading.within("unassigned", value, |value, reading| reading.null(value))?,
                ),
                _ => Self::choice_Extensions(reading.within(
                    "choice-Extensions",
                    value,
                    ProtocolIEField::from_tree,
                )?),
            })
        }
    }
    /// `NodeName ::= PrintableString (SIZE (1..150, ...))`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct NodeName<'a>(pub Cow<'a, str>);
    impl<'a> From<NodeName<'a>> for Cow<'a, str> {
        #[inline]
        fn from(value: NodeName<'a>) -> Self {
            value.0
        }
    }
    impl<'a> NodeName<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `NodeName<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
        #[inline]
        pub fn into_owned(self) -> NodeName<'static> {
            NodeName(Cow::Owned(self.0.into_owned()))
        }
    }
    impl<'a> From<Cow<'a, str>> for NodeName<'a> {
        fn from(value: Cow<'a, str>) -> Self {
            Self(value)
        }
    }
    impl<'a> From<&'a str> for NodeName<'a> {
        fn from(value: &'a str) -> Self {
            Self(Cow::Borrowed(value))
        }
    }
    impl<'a> From<alloc::string::String> for NodeName<'a> {
        fn from(value: alloc::string::String) -> Self {
            Self(Cow::Owned(value))
        }
    }
    impl AsnType for NodeName<'_> {
        const NAME: &'static str = "NodeName";
    }
    impl<'a> Decode<'a> for NodeName<'a> {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self(reader.printable_string(1, 150, true)?))
        }
    }
    impl Encode for NodeName<'_> {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.printable_string(&self.0, 1, 150, true)?;
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for NodeName<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            json::text(&self.0)
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            Ok(Self(json::read_text(
                reading,
                value,
                json::Characters::Printable,
            )?))
        }
    }
    /// The MCC and the MNC, as `208-93`. Octets that are no PLMN identity print in hexadecimal.
    impl core::fmt::Display for PLMNIdentity {
        fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match oxirush_utils::identity::Plmn::from_ap_octets(self.0) {
                Ok(plmn) => write!(formatter, "{plmn}"),
                Err(_) => self
                    .0
                    .iter()
                    .try_for_each(|octet| write!(formatter, "{octet:02x}")),
            }
        }
    }
    /// The PLMN identity of a PLMN, in the order of the application protocols.
    impl From<oxirush_utils::identity::Plmn> for PLMNIdentity {
        fn from(plmn: oxirush_utils::identity::Plmn) -> Self {
            Self(plmn.to_ap_octets())
        }
    }
    /// The PLMN of a PLMN identity: an error for octets that are none.
    impl TryFrom<PLMNIdentity> for oxirush_utils::identity::Plmn {
        type Error = oxirush_utils::identity::Error;
        fn try_from(identity: PLMNIdentity) -> Result<Self, Self::Error> {
            Self::from_ap_octets(identity.0)
        }
    }
    impl core::fmt::Debug for PLMNIdentity {
        fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(formatter, "PLMNIdentity({self})")
        }
    }
    /// `PLMNIdentity ::= OCTET STRING (SIZE (3))`.
    #[derive(Clone, PartialEq, Eq, Hash, Copy)]
    pub struct PLMNIdentity(pub [u8; 3]);
    impl From<PLMNIdentity> for [u8; 3] {
        #[inline]
        fn from(value: PLMNIdentity) -> Self {
            value.0
        }
    }
    impl From<[u8; 3]> for PLMNIdentity {
        fn from(value: [u8; 3]) -> Self {
            Self(value)
        }
    }
    impl AsnType for PLMNIdentity {
        const NAME: &'static str = "PLMNIdentity";
    }
    impl<'a> Decode<'a> for PLMNIdentity {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self(reader.octet_array::<3>()?))
        }
    }
    impl Encode for PLMNIdentity {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.octet_array(&self.0);
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for PLMNIdentity {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            json::readable(json::Form::Plmn, json::octets(&self.0))
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let value = &json::read_readable(reading, json::Form::Plmn, value)?;
            Ok(Self(reading.octet_array::<3>(value)?))
        }
    }
    /// `SecurityContext ::= SEQUENCE`, with an extension marker.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct SecurityContext<'a> {
        pub next_hop_chaining_count: u8,
        pub next_hop_nh: SecurityKey<'a>,
        pub ie_extensions: Option<ProtocolExtensionContainer<'static>>,
    }
    impl<'a> SecurityContext<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `SecurityContext<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
        pub fn into_owned(self) -> SecurityContext<'static> {
            SecurityContext {
                next_hop_chaining_count: self.next_hop_chaining_count,
                next_hop_nh: self.next_hop_nh.into_owned(),
                ie_extensions: self.ie_extensions,
            }
        }
    }
    impl AsnType for SecurityContext<'_> {
        const NAME: &'static str = "SecurityContext";
    }
    impl<'a> Decode<'a> for SecurityContext<'a> {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let extended = reader.bit()?;
            let present = reader.bits(1)?;
            let next_hop_chaining_count = reader
                .unsigned(0, 7, false)
                .map_err(in_field("next_hop_chaining_count"))?
                as u8;
            let next_hop_nh = SecurityKey::decode(reader).map_err(in_field("next_hop_nh"))?;
            let ie_extensions = if present & 1 != 0 {
                Some(
                    (ProtocolExtensionContainer::decode(reader)
                        .map_err(in_field("ie_extensions"))?)
                    .into_owned(),
                )
            } else {
                None
            };
            if extended {
                reader.skip_additions()?;
            }
            Ok(Self {
                next_hop_chaining_count,
                next_hop_nh,
                ie_extensions,
            })
        }
    }
    impl Encode for SecurityContext<'_> {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.bit(false);
            writer.bit(self.ie_extensions.is_some());
            writer
                .unsigned(u64::from(self.next_hop_chaining_count), 0, 7, false)
                .map_err(in_field("next_hop_chaining_count"))?;
            self.next_hop_nh
                .encode(writer)
                .map_err(in_field("next_hop_nh"))?;
            if let Some(value) = &self.ie_extensions {
                value.encode(writer).map_err(in_field("ie_extensions"))?;
            }
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for SecurityContext<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            let mut members = json::Members::new();
            members.put(
                "nextHopChainingCount",
                json::unsigned(u64::from(self.next_hop_chaining_count)),
            );
            members.put("nextHopNH", self.next_hop_nh.to_tree());
            members.put_optional(
                "iE-Extensions",
                self.ie_extensions.as_ref().map(|value| value.to_tree()),
            );
            members.finish()
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let members = reading.members(value, names::SecurityContext)?;
            let next_hop_chaining_count =
                reading.member(&members, "nextHopChainingCount", |value, reading| {
                    reading.unsigned(value, 0, 7).map(|number| number as u8)
                })?;
            let next_hop_nh = reading.member(&members, "nextHopNH", SecurityKey::from_tree)?;
            let ie_extensions = reading.optional(
                &members,
                "iE-Extensions",
                ProtocolExtensionContainer::from_tree,
            )?;
            Ok(Self {
                next_hop_chaining_count,
                next_hop_nh,
                ie_extensions,
            })
        }
    }
    impl core::fmt::Debug for SecurityKey<'_> {
        fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(formatter, "SecurityKey({} bits)", self.0.len())
        }
    }
    /// `SecurityKey ::= BIT STRING (SIZE (256))`.
    #[derive(Clone, PartialEq, Eq, Hash)]
    pub struct SecurityKey<'a>(pub BitString<'a>);
    impl<'a> From<SecurityKey<'a>> for BitString<'a> {
        #[inline]
        fn from(value: SecurityKey<'a>) -> Self {
            value.0
        }
    }
    impl<'a> SecurityKey<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `SecurityKey<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
        #[inline]
        pub fn into_owned(self) -> SecurityKey<'static> {
            SecurityKey(self.0.into_owned())
        }
    }
    impl<'a> From<BitString<'a>> for SecurityKey<'a> {
        fn from(value: BitString<'a>) -> Self {
            Self(value)
        }
    }
    impl<'a> From<[u8; 32]> for SecurityKey<'a> {
        fn from(value: [u8; 32]) -> Self {
            Self(BitString::from(value))
        }
    }
    impl AsnType for SecurityKey<'_> {
        const NAME: &'static str = "SecurityKey";
    }
    impl<'a> Decode<'a> for SecurityKey<'a> {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self(reader.bit_string(256, 256, false)?))
        }
    }
    impl Encode for SecurityKey<'_> {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.bit_string(&self.0, 256, 256, false)?;
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for SecurityKey<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            json::bit_string(&self.0, 256, 256, false)
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            Ok(Self(reading.secret(|reading| {
                json::read_bit_string(reading, value, 256, 256, false)
            })?))
        }
    }
    /// `SetupTransfer ::= SEQUENCE`, with an extension marker.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct SetupTransfer<'a> {
        pub protocol_ies: ProtocolIEContainer<'a>,
    }
    impl<'a> SetupTransfer<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `SetupTransfer<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
        pub fn into_owned(self) -> SetupTransfer<'static> {
            SetupTransfer {
                protocol_ies: self.protocol_ies.into_owned(),
            }
        }
    }
    impl AsnType for SetupTransfer<'_> {
        const NAME: &'static str = "SetupTransfer";
    }
    impl<'a> Decode<'a> for SetupTransfer<'a> {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let extended = reader.bit()?;
            let protocol_ies =
                ProtocolIEContainer::decode(reader).map_err(in_field("protocol_ies"))?;
            if extended {
                reader.skip_additions()?;
            }
            Ok(Self { protocol_ies })
        }
    }
    impl Encode for SetupTransfer<'_> {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.bit(false);
            self.protocol_ies
                .encode(writer)
                .map_err(in_field("protocol_ies"))?;
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for SetupTransfer<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            let mut members = json::Members::new();
            members.put("protocolIEs", self.protocol_ies.to_tree());
            members.finish()
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let members = reading.members(value, names::SetupTransfer)?;
            let protocol_ies =
                reading.member(&members, "protocolIEs", ProtocolIEContainer::from_tree)?;
            Ok(Self { protocol_ies })
        }
    }
    /// The octets, in hexadecimal.
    impl core::fmt::Display for TAC {
        fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            self.0
                .iter()
                .try_for_each(|octet| write!(formatter, "{octet:02x}"))
        }
    }
    impl core::fmt::Debug for TAC {
        fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(formatter, "TAC({self})")
        }
    }
    /// `TAC ::= OCTET STRING (SIZE (3))`.
    #[derive(Clone, PartialEq, Eq, Hash, Copy)]
    pub struct TAC(pub [u8; 3]);
    impl From<TAC> for [u8; 3] {
        #[inline]
        fn from(value: TAC) -> Self {
            value.0
        }
    }
    impl From<[u8; 3]> for TAC {
        fn from(value: [u8; 3]) -> Self {
            Self(value)
        }
    }
    impl AsnType for TAC {
        const NAME: &'static str = "TAC";
    }
    impl<'a> Decode<'a> for TAC {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self(reader.octet_array::<3>()?))
        }
    }
    impl Encode for TAC {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.octet_array(&self.0);
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for TAC {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            json::readable(json::Form::Number(24), json::octets(&self.0))
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let value = &json::read_readable(reading, json::Form::Number(24), value)?;
            Ok(Self(reading.octet_array::<3>(value)?))
        }
    }
    /// `TransferItem ::= SEQUENCE`, with an extension marker.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct TransferItem<'a> {
        pub transfer_id: u8,
        pub setup_transfer: Octets<'a>,
        pub failure_transfer: Option<Octets<'a>>,
        pub ie_extensions: Option<ProtocolExtensionContainer<'static>>,
    }
    impl<'a> TransferItem<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `TransferItem<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
        pub fn into_owned(self) -> TransferItem<'static> {
            TransferItem {
                transfer_id: self.transfer_id,
                setup_transfer: self.setup_transfer.into_owned(),
                failure_transfer: self.failure_transfer.map(|value| value.into_owned()),
                ie_extensions: self.ie_extensions,
            }
        }
    }
    impl AsnType for TransferItem<'_> {
        const NAME: &'static str = "TransferItem";
    }
    impl<'a> Decode<'a> for TransferItem<'a> {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let extended = reader.bit()?;
            let present = reader.bits(2)?;
            let transfer_id = reader
                .unsigned(0, 255, false)
                .map_err(in_field("transfer_id"))? as u8;
            let setup_transfer = reader
                .octet_string(0, UNBOUNDED, false)
                .map_err(in_field("setup_transfer"))?;
            let failure_transfer = if present & 2 != 0 {
                Some(
                    reader
                        .octet_string(0, UNBOUNDED, false)
                        .map_err(in_field("failure_transfer"))?,
                )
            } else {
                None
            };
            let ie_extensions = if present & 1 != 0 {
                Some(
                    (ProtocolExtensionContainer::decode(reader)
                        .map_err(in_field("ie_extensions"))?)
                    .into_owned(),
                )
            } else {
                None
            };
            if extended {
                reader.skip_additions()?;
            }
            Ok(Self {
                transfer_id,
                setup_transfer,
                failure_transfer,
                ie_extensions,
            })
        }
    }
    impl Encode for TransferItem<'_> {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.bit(false);
            writer.bit(self.failure_transfer.is_some());
            writer.bit(self.ie_extensions.is_some());
            writer
                .unsigned(u64::from(self.transfer_id), 0, 255, false)
                .map_err(in_field("transfer_id"))?;
            writer
                .octet_string(&self.setup_transfer, 0, UNBOUNDED, false)
                .map_err(in_field("setup_transfer"))?;
            if let Some(value) = &self.failure_transfer {
                writer
                    .octet_string(value, 0, UNBOUNDED, false)
                    .map_err(in_field("failure_transfer"))?;
            }
            if let Some(value) = &self.ie_extensions {
                value.encode(writer).map_err(in_field("ie_extensions"))?;
            }
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for TransferItem<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            let mut members = json::Members::new();
            members.put("transfer-ID", json::unsigned(u64::from(self.transfer_id)));
            members.put("setupTransfer", json::octets(&self.setup_transfer));
            members.put_optional(
                "failureTransfer",
                self.failure_transfer
                    .as_ref()
                    .map(|value| json::octets(value)),
            );
            members.put_optional(
                "iE-Extensions",
                self.ie_extensions.as_ref().map(|value| value.to_tree()),
            );
            members.finish()
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let members = reading.members(value, names::TransferItem)?;
            let transfer_id = reading.member(&members, "transfer-ID", |value, reading| {
                reading.unsigned(value, 0, 255).map(|number| number as u8)
            })?;
            let setup_transfer = reading.member(&members, "setupTransfer", |value, reading| {
                json::read_octets(reading, value)
            })?;
            let failure_transfer =
                reading.optional(&members, "failureTransfer", |value, reading| {
                    json::read_octets(reading, value)
                })?;
            let ie_extensions = reading.optional(
                &members,
                "iE-Extensions",
                ProtocolExtensionContainer::from_tree,
            )?;
            Ok(Self {
                transfer_id,
                setup_transfer,
                failure_transfer,
                ie_extensions,
            })
        }
    }
    /// `TransferKind ::= ENUMERATED`, with an extension marker.
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
    #[non_exhaustive]
    pub enum TransferKind {
        ipv6 = 10,
        ipv4 = 5,
        ipv4v6 = 7,
    }
    impl AsnType for TransferKind {
        const NAME: &'static str = "TransferKind";
    }
    impl TransferKind {
        /// The values in the order of their indexes in PER.
        pub(crate) const VALUES: [Self; 3] = [Self::ipv4, Self::ipv6, Self::ipv4v6];
        /// The index of the value in PER.
        #[inline]
        fn index(&self) -> u64 {
            match self {
                Self::ipv4 => 0,
                Self::ipv6 => 1,
                Self::ipv4v6 => 2,
            }
        }
    }
    impl<'a> Decode<'a> for TransferKind {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self::VALUES[reader.index(2, 1, true)? as usize])
        }
    }
    impl Encode for TransferKind {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.index(self.index(), 2, true);
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for TransferKind {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            json::text(names::TransferKind[self.index() as usize])
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            Ok(Self::VALUES[reading.named(value, names::TransferKind)?])
        }
    }
    /// `TransferList ::= SEQUENCE (SIZE (1..8)) OF TransferItem`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct TransferList<'a>(pub Vec<TransferItem<'a>>);
    impl<'a> From<TransferList<'a>> for Vec<TransferItem<'a>> {
        #[inline]
        fn from(value: TransferList<'a>) -> Self {
            value.0
        }
    }
    impl<'a> TransferList<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `TransferList<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
        pub fn into_owned(self) -> TransferList<'static> {
            TransferList(self.0.into_iter().map(|item| item.into_owned()).collect())
        }
    }
    impl<'a> From<Vec<TransferItem<'a>>> for TransferList<'a> {
        fn from(value: Vec<TransferItem<'a>>) -> Self {
            Self(value)
        }
    }
    impl AsnType for TransferList<'_> {
        const NAME: &'static str = "TransferList";
    }
    impl<'a> Decode<'a> for TransferList<'a> {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self({
                let count = reader.size(1, 8)?;
                let mut list = reader.list(count);
                for _ in 0..count {
                    list.push(TransferItem::decode(reader)?);
                }
                list
            }))
        }
    }
    impl Encode for TransferList<'_> {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.size(self.0.len(), 1, 8)?;
            for item in self.0.iter() {
                item.encode(writer)?;
            }
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for TransferList<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            Value::Array(self.0.iter().map(|item| item.to_tree()).collect())
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            Ok(Self(reading.list(value, |value, reading| {
                TransferItem::from_tree(value, reading)
            })?))
        }
    }
    /// `TransportLayerAddress ::= BIT STRING (SIZE (1..160, ...))`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct TransportLayerAddress<'a>(pub BitString<'a>);
    impl<'a> From<TransportLayerAddress<'a>> for BitString<'a> {
        #[inline]
        fn from(value: TransportLayerAddress<'a>) -> Self {
            value.0
        }
    }
    impl<'a> TransportLayerAddress<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `TransportLayerAddress<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
        #[inline]
        pub fn into_owned(self) -> TransportLayerAddress<'static> {
            TransportLayerAddress(self.0.into_owned())
        }
    }
    impl<'a> From<BitString<'a>> for TransportLayerAddress<'a> {
        fn from(value: BitString<'a>) -> Self {
            Self(value)
        }
    }
    impl AsnType for TransportLayerAddress<'_> {
        const NAME: &'static str = "TransportLayerAddress";
    }
    impl<'a> Decode<'a> for TransportLayerAddress<'a> {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self(reader.bit_string(1, 160, true)?))
        }
    }
    impl Encode for TransportLayerAddress<'_> {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.bit_string(&self.0, 1, 160, true)?;
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for TransportLayerAddress<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            json::readable(json::Form::Address, json::bit_string(&self.0, 1, 160, true))
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let value = &json::read_readable(reading, json::Form::Address, value)?;
            Ok(Self(json::read_bit_string(reading, value, 1, 160, true)?))
        }
    }
    /// `Tunnel ::= CHOICE`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub enum Tunnel<'a> {
        gTPTunnel(GTPTunnel<'a>),
        unassigned(()),
        choice_Extensions(ProtocolIEField<'static>),
    }
    impl<'a> Tunnel<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `Tunnel<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
        pub fn into_owned(self) -> Tunnel<'static> {
            match self {
                Self::gTPTunnel(value) => Tunnel::gTPTunnel(value.into_owned()),
                Self::unassigned(value) => Tunnel::unassigned(value),
                Self::choice_Extensions(value) => Tunnel::choice_Extensions(value),
            }
        }
    }
    impl<'a> From<GTPTunnel<'a>> for Tunnel<'a> {
        fn from(value: GTPTunnel<'a>) -> Self {
            Self::gTPTunnel(value)
        }
    }
    impl<'a> From<()> for Tunnel<'a> {
        fn from(value: ()) -> Self {
            Self::unassigned(value)
        }
    }
    impl<'a> From<ProtocolIEField<'static>> for Tunnel<'a> {
        fn from(value: ProtocolIEField<'static>) -> Self {
            Self::choice_Extensions(value)
        }
    }
    impl AsnType for Tunnel<'_> {
        const NAME: &'static str = "Tunnel";
    }
    impl<'a> Decode<'a> for Tunnel<'a> {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(match reader.index(3, 0, false)? {
                0 => Self::gTPTunnel(GTPTunnel::decode(reader)?),
                1 => Self::unassigned(()),
                _ => Self::choice_Extensions((ProtocolIEField::decode(reader)?).into_owned()),
            })
        }
    }
    impl Encode for Tunnel<'_> {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            match self {
                Self::gTPTunnel(value) => {
                    writer.index(0, 3, false);
                    value.encode(writer)?;
                }
                Self::unassigned(value) => {
                    writer.index(1, 3, false);
                }
                Self::choice_Extensions(value) => {
                    writer.index(2, 3, false);
                    value.encode(writer)?;
                }
            }
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for Tunnel<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            match self {
                Self::gTPTunnel(value) => json::alternative("gTPTunnel", value.to_tree()),
                Self::unassigned(value) => json::alternative("unassigned", Value::Null),
                Self::choice_Extensions(value) => {
                    json::alternative("choice-Extensions", value.to_tree())
                }
            }
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let (position, value) = reading.alternative(value, names::Tunnel)?;
            Ok(match position {
                0 => Self::gTPTunnel(reading.within("gTPTunnel", value, GTPTunnel::from_tree)?),
                1 => Self::unassigned(
                    reading.within("unassigned", value, |value, reading| reading.null(value))?,
                ),
                _ => Self::choice_Extensions(reading.within(
                    "choice-Extensions",
                    value,
                    ProtocolIEField::from_tree,
                )?),
            })
        }
    }
    /// `TypeOfError ::= ENUMERATED`, with an extension marker.
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
    #[non_exhaustive]
    pub enum TypeOfError {
        not_understood = 0,
        missing = 1,
    }
    impl AsnType for TypeOfError {
        const NAME: &'static str = "TypeOfError";
    }
    impl TypeOfError {
        /// The values in the order of their indexes in PER.
        pub(crate) const VALUES: [Self; 2] = [Self::not_understood, Self::missing];
        /// The index of the value in PER.
        #[inline]
        fn index(&self) -> u64 {
            *self as u64
        }
    }
    impl<'a> Decode<'a> for TypeOfError {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self::VALUES[reader.index(2, 0, true)? as usize])
        }
    }
    impl Encode for TypeOfError {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.index(self.index(), 2, true);
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for TypeOfError {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            json::text(names::TypeOfError[self.index() as usize])
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            Ok(Self::VALUES[reading.named(value, names::TypeOfError)?])
        }
    }
    /// `UE-Core-ID ::= INTEGER (0..1099511627775)`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
    pub struct UECoreID(pub u64);
    impl From<UECoreID> for u64 {
        #[inline]
        fn from(value: UECoreID) -> Self {
            value.0
        }
    }
    impl From<u64> for UECoreID {
        fn from(value: u64) -> Self {
            Self(value)
        }
    }
    impl AsnType for UECoreID {
        const NAME: &'static str = "UE-Core-ID";
    }
    impl<'a> Decode<'a> for UECoreID {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self(reader.unsigned(0, 1099511627775, false)?))
        }
    }
    impl Encode for UECoreID {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.unsigned(self.0, 0, 1099511627775, false)?;
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for UECoreID {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            json::unsigned(self.0)
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            Ok(Self(reading.unsigned(value, 0, 1099511627775)?))
        }
    }
    /// `UE-ID-pair ::= SEQUENCE`, with an extension marker.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct UEIDPair {
        pub ue_core_id: UECoreID,
        pub ue_ran_id: UERANID,
        pub ie_extensions: Option<ProtocolExtensionContainer<'static>>,
    }
    impl AsnType for UEIDPair {
        const NAME: &'static str = "UE-ID-pair";
    }
    impl<'a> Decode<'a> for UEIDPair {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let extended = reader.bit()?;
            let present = reader.bits(1)?;
            let ue_core_id = UECoreID::decode(reader).map_err(in_field("ue_core_id"))?;
            let ue_ran_id = UERANID::decode(reader).map_err(in_field("ue_ran_id"))?;
            let ie_extensions = if present & 1 != 0 {
                Some(
                    (ProtocolExtensionContainer::decode(reader)
                        .map_err(in_field("ie_extensions"))?)
                    .into_owned(),
                )
            } else {
                None
            };
            if extended {
                reader.skip_additions()?;
            }
            Ok(Self {
                ue_core_id,
                ue_ran_id,
                ie_extensions,
            })
        }
    }
    impl Encode for UEIDPair {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.bit(false);
            writer.bit(self.ie_extensions.is_some());
            self.ue_core_id
                .encode(writer)
                .map_err(in_field("ue_core_id"))?;
            self.ue_ran_id
                .encode(writer)
                .map_err(in_field("ue_ran_id"))?;
            if let Some(value) = &self.ie_extensions {
                value.encode(writer).map_err(in_field("ie_extensions"))?;
            }
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for UEIDPair {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            let mut members = json::Members::new();
            members.put("uE-Core-ID", self.ue_core_id.to_tree());
            members.put("uE-RAN-ID", self.ue_ran_id.to_tree());
            members.put_optional(
                "iE-Extensions",
                self.ie_extensions.as_ref().map(|value| value.to_tree()),
            );
            members.finish()
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let members = reading.members(value, names::UEIDPair)?;
            let ue_core_id = reading.member(&members, "uE-Core-ID", UECoreID::from_tree)?;
            let ue_ran_id = reading.member(&members, "uE-RAN-ID", UERANID::from_tree)?;
            let ie_extensions = reading.optional(
                &members,
                "iE-Extensions",
                ProtocolExtensionContainer::from_tree,
            )?;
            Ok(Self {
                ue_core_id,
                ue_ran_id,
                ie_extensions,
            })
        }
    }
    /// `UE-IDs ::= CHOICE`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub enum UEIDs {
        uE_ID_pair(UEIDPair),
        uE_Core_ID(UECoreID),
        choice_Extensions(ProtocolIEField<'static>),
    }
    impl From<UEIDPair> for UEIDs {
        fn from(value: UEIDPair) -> Self {
            Self::uE_ID_pair(value)
        }
    }
    impl From<UECoreID> for UEIDs {
        fn from(value: UECoreID) -> Self {
            Self::uE_Core_ID(value)
        }
    }
    impl From<ProtocolIEField<'static>> for UEIDs {
        fn from(value: ProtocolIEField<'static>) -> Self {
            Self::choice_Extensions(value)
        }
    }
    impl AsnType for UEIDs {
        const NAME: &'static str = "UE-IDs";
    }
    impl<'a> Decode<'a> for UEIDs {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(match reader.index(3, 0, false)? {
                0 => Self::uE_ID_pair(UEIDPair::decode(reader)?),
                1 => Self::uE_Core_ID(UECoreID::decode(reader)?),
                _ => Self::choice_Extensions((ProtocolIEField::decode(reader)?).into_owned()),
            })
        }
    }
    impl Encode for UEIDs {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            match self {
                Self::uE_ID_pair(value) => {
                    writer.index(0, 3, false);
                    value.encode(writer)?;
                }
                Self::uE_Core_ID(value) => {
                    writer.index(1, 3, false);
                    value.encode(writer)?;
                }
                Self::choice_Extensions(value) => {
                    writer.index(2, 3, false);
                    value.encode(writer)?;
                }
            }
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for UEIDs {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            match self {
                Self::uE_ID_pair(value) => json::alternative("uE-ID-pair", value.to_tree()),
                Self::uE_Core_ID(value) => json::alternative("uE-Core-ID", value.to_tree()),
                Self::choice_Extensions(value) => {
                    json::alternative("choice-Extensions", value.to_tree())
                }
            }
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let (position, value) = reading.alternative(value, names::UEIDs)?;
            Ok(match position {
                0 => Self::uE_ID_pair(reading.within("uE-ID-pair", value, UEIDPair::from_tree)?),
                1 => Self::uE_Core_ID(reading.within("uE-Core-ID", value, UECoreID::from_tree)?),
                _ => Self::choice_Extensions(reading.within(
                    "choice-Extensions",
                    value,
                    ProtocolIEField::from_tree,
                )?),
            })
        }
    }
    /// `UE-RAN-ID ::= INTEGER (0..4294967295)`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
    pub struct UERANID(pub u32);
    impl From<UERANID> for u32 {
        #[inline]
        fn from(value: UERANID) -> Self {
            value.0
        }
    }
    impl From<u32> for UERANID {
        fn from(value: u32) -> Self {
            Self(value)
        }
    }
    impl AsnType for UERANID {
        const NAME: &'static str = "UE-RAN-ID";
    }
    impl<'a> Decode<'a> for UERANID {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self(reader.unsigned(0, 4294967295, false)? as u32))
        }
    }
    impl Encode for UERANID {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.unsigned(u64::from(self.0), 0, 4294967295, false)?;
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for UERANID {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            json::unsigned(u64::from(self.0))
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            Ok(Self(
                reading
                    .unsigned(value, 0, 4294967295)
                    .map(|number| number as u32)?,
            ))
        }
    }
}
#[allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused,
    clippy::too_many_arguments,
    clippy::collapsible_if
)]
pub mod toy_pdu_contents {
    use super::*;
    #[cfg(feature = "inspect")]
    use crate::inspect_registry::names;
    use alloc::borrow::Cow;
    use alloc::vec::Vec;
    #[cfg(feature = "inspect")]
    use oxirush_aper::json::{self, Reading, Refused, Tree, Value};
    use oxirush_aper::{
        AsnType, BitString, Bits, Decode, Encode, Error, ObjectIdentifier, Octets, Reader,
        UNBOUNDED, Writer, in_field,
    };
    /// `ErrorIndication ::= SEQUENCE`, with an extension marker.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct ErrorIndication<'a> {
        pub protocol_ies: ProtocolIEContainer<'a>,
    }
    impl<'a> ErrorIndication<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `ErrorIndication<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
        pub fn into_owned(self) -> ErrorIndication<'static> {
            ErrorIndication {
                protocol_ies: self.protocol_ies.into_owned(),
            }
        }
    }
    impl AsnType for ErrorIndication<'_> {
        const NAME: &'static str = "ErrorIndication";
    }
    impl<'a> Decode<'a> for ErrorIndication<'a> {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let extended = reader.bit()?;
            let protocol_ies =
                ProtocolIEContainer::decode(reader).map_err(in_field("protocol_ies"))?;
            if extended {
                reader.skip_additions()?;
            }
            Ok(Self { protocol_ies })
        }
    }
    impl Encode for ErrorIndication<'_> {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.bit(false);
            self.protocol_ies
                .encode(writer)
                .map_err(in_field("protocol_ies"))?;
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for ErrorIndication<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            let mut members = json::Members::new();
            members.put("protocolIEs", self.protocol_ies.to_tree());
            members.finish()
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let members = reading.members(value, names::ErrorIndication)?;
            let protocol_ies =
                reading.member(&members, "protocolIEs", ProtocolIEContainer::from_tree)?;
            Ok(Self { protocol_ies })
        }
    }
    /// `Notification ::= SEQUENCE`, with an extension marker.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct Notification<'a> {
        pub protocol_ies: ProtocolIEContainer<'a>,
    }
    impl<'a> Notification<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `Notification<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
        pub fn into_owned(self) -> Notification<'static> {
            Notification {
                protocol_ies: self.protocol_ies.into_owned(),
            }
        }
    }
    impl AsnType for Notification<'_> {
        const NAME: &'static str = "Notification";
    }
    impl<'a> Decode<'a> for Notification<'a> {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let extended = reader.bit()?;
            let protocol_ies =
                ProtocolIEContainer::decode(reader).map_err(in_field("protocol_ies"))?;
            if extended {
                reader.skip_additions()?;
            }
            Ok(Self { protocol_ies })
        }
    }
    impl Encode for Notification<'_> {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.bit(false);
            self.protocol_ies
                .encode(writer)
                .map_err(in_field("protocol_ies"))?;
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for Notification<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            let mut members = json::Members::new();
            members.put("protocolIEs", self.protocol_ies.to_tree());
            members.finish()
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let members = reading.members(value, names::Notification)?;
            let protocol_ies =
                reading.member(&members, "protocolIEs", ProtocolIEContainer::from_tree)?;
            Ok(Self { protocol_ies })
        }
    }
    /// `SEQUENCE ::= SEQUENCE`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct AnonymousPrivateMessagePrivateIEs<'a> {
        pub id: PrivateIEID,
        pub criticality: Criticality,
        pub value: Octets<'a>,
    }
    impl<'a> AnonymousPrivateMessagePrivateIEs<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `AnonymousPrivateMessagePrivateIEs<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
        pub fn into_owned(self) -> AnonymousPrivateMessagePrivateIEs<'static> {
            AnonymousPrivateMessagePrivateIEs {
                id: self.id,
                criticality: self.criticality,
                value: self.value.into_owned(),
            }
        }
    }
    impl AsnType for AnonymousPrivateMessagePrivateIEs<'_> {
        const NAME: &'static str = "SEQUENCE";
    }
    impl<'a> Decode<'a> for AnonymousPrivateMessagePrivateIEs<'a> {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let id = PrivateIEID::decode(reader).map_err(in_field("id"))?;
            let criticality = Criticality::decode(reader).map_err(in_field("criticality"))?;
            let value = reader.open_type().map_err(in_field("value"))?;
            Ok(Self {
                id,
                criticality,
                value,
            })
        }
    }
    impl Encode for AnonymousPrivateMessagePrivateIEs<'_> {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            self.id.encode(writer).map_err(in_field("id"))?;
            self.criticality
                .encode(writer)
                .map_err(in_field("criticality"))?;
            writer.open_type(&self.value);
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for AnonymousPrivateMessagePrivateIEs<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            let mut members = json::Members::new();
            members.put("id", self.id.to_tree());
            members.put("criticality", self.criticality.to_tree());
            members.put("value", json::octets(&self.value));
            members.finish()
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let members = reading.members(value, names::AnonymousPrivateMessagePrivateIEs)?;
            let id = reading.member(&members, "id", PrivateIEID::from_tree)?;
            let criticality = reading.member(&members, "criticality", Criticality::from_tree)?;
            let value = reading.member(&members, "value", |value, reading| {
                json::read_octets(reading, value)
            })?;
            Ok(Self {
                id,
                criticality,
                value,
            })
        }
    }
    /// `PrivateMessagePrivateIEs ::= SEQUENCE (SIZE (1..65535)) OF AnonymousPrivateMessagePrivateIEs`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct PrivateMessagePrivateIEs<'a>(pub Vec<AnonymousPrivateMessagePrivateIEs<'a>>);
    impl<'a> From<PrivateMessagePrivateIEs<'a>> for Vec<AnonymousPrivateMessagePrivateIEs<'a>> {
        #[inline]
        fn from(value: PrivateMessagePrivateIEs<'a>) -> Self {
            value.0
        }
    }
    impl<'a> PrivateMessagePrivateIEs<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `PrivateMessagePrivateIEs<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
        pub fn into_owned(self) -> PrivateMessagePrivateIEs<'static> {
            PrivateMessagePrivateIEs(self.0.into_iter().map(|item| item.into_owned()).collect())
        }
    }
    impl<'a> From<Vec<AnonymousPrivateMessagePrivateIEs<'a>>> for PrivateMessagePrivateIEs<'a> {
        fn from(value: Vec<AnonymousPrivateMessagePrivateIEs<'a>>) -> Self {
            Self(value)
        }
    }
    impl AsnType for PrivateMessagePrivateIEs<'_> {
        const NAME: &'static str = "PrivateMessagePrivateIEs";
    }
    impl<'a> Decode<'a> for PrivateMessagePrivateIEs<'a> {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self({
                let count = reader.size(1, 65535)?;
                let mut list = reader.list(count);
                for _ in 0..count {
                    list.push(AnonymousPrivateMessagePrivateIEs::decode(reader)?);
                }
                list
            }))
        }
    }
    impl Encode for PrivateMessagePrivateIEs<'_> {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.size(self.0.len(), 1, 65535)?;
            for item in self.0.iter() {
                item.encode(writer)?;
            }
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for PrivateMessagePrivateIEs<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            Value::Array(self.0.iter().map(|item| item.to_tree()).collect())
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            Ok(Self(reading.list(value, |value, reading| {
                AnonymousPrivateMessagePrivateIEs::from_tree(value, reading)
            })?))
        }
    }
    /// `PrivateMessage ::= SEQUENCE`, with an extension marker.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct PrivateMessage<'a> {
        pub private_ies: PrivateMessagePrivateIEs<'a>,
    }
    impl<'a> PrivateMessage<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `PrivateMessage<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
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
            let private_ies =
                PrivateMessagePrivateIEs::decode(reader).map_err(in_field("private_ies"))?;
            if extended {
                reader.skip_additions()?;
            }
            Ok(Self { private_ies })
        }
    }
    impl Encode for PrivateMessage<'_> {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.bit(false);
            self.private_ies
                .encode(writer)
                .map_err(in_field("private_ies"))?;
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for PrivateMessage<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            let mut members = json::Members::new();
            members.put("privateIEs", self.private_ies.to_tree());
            members.finish()
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let members = reading.members(value, names::PrivateMessage)?;
            let private_ies =
                reading.member(&members, "privateIEs", PrivateMessagePrivateIEs::from_tree)?;
            Ok(Self { private_ies })
        }
    }
    /// `ReleaseRequest ::= SEQUENCE`, with an extension marker.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct ReleaseRequest<'a> {
        pub protocol_ies: ProtocolIEContainer<'a>,
    }
    impl<'a> ReleaseRequest<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `ReleaseRequest<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
        pub fn into_owned(self) -> ReleaseRequest<'static> {
            ReleaseRequest {
                protocol_ies: self.protocol_ies.into_owned(),
            }
        }
    }
    impl AsnType for ReleaseRequest<'_> {
        const NAME: &'static str = "ReleaseRequest";
    }
    impl<'a> Decode<'a> for ReleaseRequest<'a> {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let extended = reader.bit()?;
            let protocol_ies =
                ProtocolIEContainer::decode(reader).map_err(in_field("protocol_ies"))?;
            if extended {
                reader.skip_additions()?;
            }
            Ok(Self { protocol_ies })
        }
    }
    impl Encode for ReleaseRequest<'_> {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.bit(false);
            self.protocol_ies
                .encode(writer)
                .map_err(in_field("protocol_ies"))?;
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for ReleaseRequest<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            let mut members = json::Members::new();
            members.put("protocolIEs", self.protocol_ies.to_tree());
            members.finish()
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let members = reading.members(value, names::ReleaseRequest)?;
            let protocol_ies =
                reading.member(&members, "protocolIEs", ProtocolIEContainer::from_tree)?;
            Ok(Self { protocol_ies })
        }
    }
    /// `SetupFailure ::= SEQUENCE`, with an extension marker.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct SetupFailure<'a> {
        pub protocol_ies: ProtocolIEContainer<'a>,
    }
    impl<'a> SetupFailure<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `SetupFailure<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
        pub fn into_owned(self) -> SetupFailure<'static> {
            SetupFailure {
                protocol_ies: self.protocol_ies.into_owned(),
            }
        }
    }
    impl AsnType for SetupFailure<'_> {
        const NAME: &'static str = "SetupFailure";
    }
    impl<'a> Decode<'a> for SetupFailure<'a> {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let extended = reader.bit()?;
            let protocol_ies =
                ProtocolIEContainer::decode(reader).map_err(in_field("protocol_ies"))?;
            if extended {
                reader.skip_additions()?;
            }
            Ok(Self { protocol_ies })
        }
    }
    impl Encode for SetupFailure<'_> {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.bit(false);
            self.protocol_ies
                .encode(writer)
                .map_err(in_field("protocol_ies"))?;
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for SetupFailure<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            let mut members = json::Members::new();
            members.put("protocolIEs", self.protocol_ies.to_tree());
            members.finish()
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let members = reading.members(value, names::SetupFailure)?;
            let protocol_ies =
                reading.member(&members, "protocolIEs", ProtocolIEContainer::from_tree)?;
            Ok(Self { protocol_ies })
        }
    }
    /// `SetupRequest ::= SEQUENCE`, with an extension marker.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct SetupRequest<'a> {
        pub protocol_ies: ProtocolIEContainer<'a>,
    }
    impl<'a> SetupRequest<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `SetupRequest<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
        pub fn into_owned(self) -> SetupRequest<'static> {
            SetupRequest {
                protocol_ies: self.protocol_ies.into_owned(),
            }
        }
    }
    impl AsnType for SetupRequest<'_> {
        const NAME: &'static str = "SetupRequest";
    }
    impl<'a> Decode<'a> for SetupRequest<'a> {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let extended = reader.bit()?;
            let protocol_ies =
                ProtocolIEContainer::decode(reader).map_err(in_field("protocol_ies"))?;
            if extended {
                reader.skip_additions()?;
            }
            Ok(Self { protocol_ies })
        }
    }
    impl Encode for SetupRequest<'_> {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.bit(false);
            self.protocol_ies
                .encode(writer)
                .map_err(in_field("protocol_ies"))?;
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for SetupRequest<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            let mut members = json::Members::new();
            members.put("protocolIEs", self.protocol_ies.to_tree());
            members.finish()
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let members = reading.members(value, names::SetupRequest)?;
            let protocol_ies =
                reading.member(&members, "protocolIEs", ProtocolIEContainer::from_tree)?;
            Ok(Self { protocol_ies })
        }
    }
    /// `SetupResponse ::= SEQUENCE`, with an extension marker.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct SetupResponse<'a> {
        pub protocol_ies: ProtocolIEContainer<'a>,
    }
    impl<'a> SetupResponse<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `SetupResponse<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
        pub fn into_owned(self) -> SetupResponse<'static> {
            SetupResponse {
                protocol_ies: self.protocol_ies.into_owned(),
            }
        }
    }
    impl AsnType for SetupResponse<'_> {
        const NAME: &'static str = "SetupResponse";
    }
    impl<'a> Decode<'a> for SetupResponse<'a> {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let extended = reader.bit()?;
            let protocol_ies =
                ProtocolIEContainer::decode(reader).map_err(in_field("protocol_ies"))?;
            if extended {
                reader.skip_additions()?;
            }
            Ok(Self { protocol_ies })
        }
    }
    impl Encode for SetupResponse<'_> {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.bit(false);
            self.protocol_ies
                .encode(writer)
                .map_err(in_field("protocol_ies"))?;
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for SetupResponse<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            let mut members = json::Members::new();
            members.put("protocolIEs", self.protocol_ies.to_tree());
            members.finish()
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let members = reading.members(value, names::SetupResponse)?;
            let protocol_ies =
                reading.member(&members, "protocolIEs", ProtocolIEContainer::from_tree)?;
            Ok(Self { protocol_ies })
        }
    }
}
#[allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused,
    clippy::too_many_arguments,
    clippy::collapsible_if
)]
pub mod toy_pdu_descriptions {
    use super::*;
    #[cfg(feature = "inspect")]
    use crate::inspect_registry::names;
    use alloc::borrow::Cow;
    use alloc::vec::Vec;
    #[cfg(feature = "inspect")]
    use oxirush_aper::json::{self, Reading, Refused, Tree, Value};
    use oxirush_aper::{
        AsnType, BitString, Bits, Decode, Encode, Error, ObjectIdentifier, Octets, Reader,
        UNBOUNDED, Writer, in_field,
    };
    impl core::fmt::Debug for InitiatingMessage<'_> {
        fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            let holds_key = oxirush_aper::protocol::holds_key::<crate::Toy>(
                oxirush_aper::Direction::Initiating,
                self.procedure_code.0,
            );
            let mut shown = formatter.debug_struct("InitiatingMessage");
            shown.field("procedure_code", &self.procedure_code);
            shown.field("criticality", &self.criticality);
            match holds_key {
                true => shown.field("value", &format_args!("{} octets", self.value.len())),
                false => shown.field("value", &self.value),
            };
            shown.finish()
        }
    }
    /// `InitiatingMessage ::= SEQUENCE`.
    #[derive(Clone, PartialEq, Eq, Hash)]
    pub struct InitiatingMessage<'a> {
        pub procedure_code: ProcedureCode,
        pub criticality: Criticality,
        pub value: Octets<'a>,
    }
    impl<'a> InitiatingMessage<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `InitiatingMessage<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
        #[inline]
        pub fn into_owned(self) -> InitiatingMessage<'static> {
            InitiatingMessage {
                procedure_code: self.procedure_code,
                criticality: self.criticality,
                value: self.value.into_owned(),
            }
        }
    }
    impl AsnType for InitiatingMessage<'_> {
        const NAME: &'static str = "InitiatingMessage";
    }
    impl<'a> Decode<'a> for InitiatingMessage<'a> {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let procedure_code =
                ProcedureCode::decode(reader).map_err(in_field("procedure_code"))?;
            let criticality = Criticality::decode(reader).map_err(in_field("criticality"))?;
            let value = reader.open_type().map_err(in_field("value"))?;
            Ok(Self {
                procedure_code,
                criticality,
                value,
            })
        }
    }
    impl Encode for InitiatingMessage<'_> {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            self.procedure_code
                .encode(writer)
                .map_err(in_field("procedure_code"))?;
            self.criticality
                .encode(writer)
                .map_err(in_field("criticality"))?;
            writer.open_type(&self.value);
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for InitiatingMessage<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            let mut members = json::Members::new();
            members.put("procedureCode", self.procedure_code.to_tree());
            members.put("criticality", self.criticality.to_tree());
            members.put("value", json::octets(&self.value));
            members.finish()
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let members = reading.members(value, names::InitiatingMessage)?;
            let procedure_code =
                reading.member(&members, "procedureCode", ProcedureCode::from_tree)?;
            let criticality = reading.member(&members, "criticality", Criticality::from_tree)?;
            let value = reading.member(&members, "value", |value, reading| {
                json::read_octets(reading, value)
            })?;
            Ok(Self {
                procedure_code,
                criticality,
                value,
            })
        }
    }
    impl core::fmt::Debug for SuccessfulOutcome<'_> {
        fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            let holds_key = oxirush_aper::protocol::holds_key::<crate::Toy>(
                oxirush_aper::Direction::Successful,
                self.procedure_code.0,
            );
            let mut shown = formatter.debug_struct("SuccessfulOutcome");
            shown.field("procedure_code", &self.procedure_code);
            shown.field("criticality", &self.criticality);
            match holds_key {
                true => shown.field("value", &format_args!("{} octets", self.value.len())),
                false => shown.field("value", &self.value),
            };
            shown.finish()
        }
    }
    /// `SuccessfulOutcome ::= SEQUENCE`.
    #[derive(Clone, PartialEq, Eq, Hash)]
    pub struct SuccessfulOutcome<'a> {
        pub procedure_code: ProcedureCode,
        pub criticality: Criticality,
        pub value: Octets<'a>,
    }
    impl<'a> SuccessfulOutcome<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `SuccessfulOutcome<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
        #[inline]
        pub fn into_owned(self) -> SuccessfulOutcome<'static> {
            SuccessfulOutcome {
                procedure_code: self.procedure_code,
                criticality: self.criticality,
                value: self.value.into_owned(),
            }
        }
    }
    impl AsnType for SuccessfulOutcome<'_> {
        const NAME: &'static str = "SuccessfulOutcome";
    }
    impl<'a> Decode<'a> for SuccessfulOutcome<'a> {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let procedure_code =
                ProcedureCode::decode(reader).map_err(in_field("procedure_code"))?;
            let criticality = Criticality::decode(reader).map_err(in_field("criticality"))?;
            let value = reader.open_type().map_err(in_field("value"))?;
            Ok(Self {
                procedure_code,
                criticality,
                value,
            })
        }
    }
    impl Encode for SuccessfulOutcome<'_> {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            self.procedure_code
                .encode(writer)
                .map_err(in_field("procedure_code"))?;
            self.criticality
                .encode(writer)
                .map_err(in_field("criticality"))?;
            writer.open_type(&self.value);
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for SuccessfulOutcome<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            let mut members = json::Members::new();
            members.put("procedureCode", self.procedure_code.to_tree());
            members.put("criticality", self.criticality.to_tree());
            members.put("value", json::octets(&self.value));
            members.finish()
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let members = reading.members(value, names::SuccessfulOutcome)?;
            let procedure_code =
                reading.member(&members, "procedureCode", ProcedureCode::from_tree)?;
            let criticality = reading.member(&members, "criticality", Criticality::from_tree)?;
            let value = reading.member(&members, "value", |value, reading| {
                json::read_octets(reading, value)
            })?;
            Ok(Self {
                procedure_code,
                criticality,
                value,
            })
        }
    }
    /// `TOY-PDU ::= CHOICE`, with an extension marker.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    #[non_exhaustive]
    pub enum TOYPDU<'a> {
        initiatingMessage(InitiatingMessage<'a>),
        successfulOutcome(SuccessfulOutcome<'a>),
        unsuccessfulOutcome(UnsuccessfulOutcome<'a>),
    }
    impl<'a> TOYPDU<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `TOYPDU<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
        #[inline]
        pub fn into_owned(self) -> TOYPDU<'static> {
            match self {
                Self::initiatingMessage(value) => TOYPDU::initiatingMessage(value.into_owned()),
                Self::successfulOutcome(value) => TOYPDU::successfulOutcome(value.into_owned()),
                Self::unsuccessfulOutcome(value) => TOYPDU::unsuccessfulOutcome(value.into_owned()),
            }
        }
    }
    impl<'a> From<InitiatingMessage<'a>> for TOYPDU<'a> {
        fn from(value: InitiatingMessage<'a>) -> Self {
            Self::initiatingMessage(value)
        }
    }
    impl<'a> From<SuccessfulOutcome<'a>> for TOYPDU<'a> {
        fn from(value: SuccessfulOutcome<'a>) -> Self {
            Self::successfulOutcome(value)
        }
    }
    impl<'a> From<UnsuccessfulOutcome<'a>> for TOYPDU<'a> {
        fn from(value: UnsuccessfulOutcome<'a>) -> Self {
            Self::unsuccessfulOutcome(value)
        }
    }
    impl AsnType for TOYPDU<'_> {
        const NAME: &'static str = "TOY-PDU";
    }
    impl<'a> Decode<'a> for TOYPDU<'a> {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(match reader.index(3, 0, true)? {
                0 => Self::initiatingMessage(InitiatingMessage::decode(reader)?),
                1 => Self::successfulOutcome(SuccessfulOutcome::decode(reader)?),
                _ => Self::unsuccessfulOutcome(UnsuccessfulOutcome::decode(reader)?),
            })
        }
    }
    impl Encode for TOYPDU<'_> {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            match self {
                Self::initiatingMessage(value) => {
                    writer.index(0, 3, true);
                    value.encode(writer)?;
                }
                Self::successfulOutcome(value) => {
                    writer.index(1, 3, true);
                    value.encode(writer)?;
                }
                Self::unsuccessfulOutcome(value) => {
                    writer.index(2, 3, true);
                    value.encode(writer)?;
                }
            }
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for TOYPDU<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            match self {
                Self::initiatingMessage(value) => {
                    json::alternative("initiatingMessage", value.to_tree())
                }
                Self::successfulOutcome(value) => {
                    json::alternative("successfulOutcome", value.to_tree())
                }
                Self::unsuccessfulOutcome(value) => {
                    json::alternative("unsuccessfulOutcome", value.to_tree())
                }
            }
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let (position, value) = reading.alternative(value, names::TOYPDU)?;
            Ok(match position {
                0 => Self::initiatingMessage(reading.within(
                    "initiatingMessage",
                    value,
                    InitiatingMessage::from_tree,
                )?),
                1 => Self::successfulOutcome(reading.within(
                    "successfulOutcome",
                    value,
                    SuccessfulOutcome::from_tree,
                )?),
                _ => Self::unsuccessfulOutcome(reading.within(
                    "unsuccessfulOutcome",
                    value,
                    UnsuccessfulOutcome::from_tree,
                )?),
            })
        }
    }
    impl core::fmt::Debug for UnsuccessfulOutcome<'_> {
        fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            let holds_key = oxirush_aper::protocol::holds_key::<crate::Toy>(
                oxirush_aper::Direction::Unsuccessful,
                self.procedure_code.0,
            );
            let mut shown = formatter.debug_struct("UnsuccessfulOutcome");
            shown.field("procedure_code", &self.procedure_code);
            shown.field("criticality", &self.criticality);
            match holds_key {
                true => shown.field("value", &format_args!("{} octets", self.value.len())),
                false => shown.field("value", &self.value),
            };
            shown.finish()
        }
    }
    /// `UnsuccessfulOutcome ::= SEQUENCE`.
    #[derive(Clone, PartialEq, Eq, Hash)]
    pub struct UnsuccessfulOutcome<'a> {
        pub procedure_code: ProcedureCode,
        pub criticality: Criticality,
        pub value: Octets<'a>,
    }
    impl<'a> UnsuccessfulOutcome<'a> {
        /// The same value, which borrows nothing and is kept as long as it is needed: a `UnsuccessfulOutcome<'static>`, which is the type to write in a struct. What the value borrowed from the octets that it was decoded from is copied.
        #[doc(alias("to_owned", "owned", "keep"))]
        #[inline]
        pub fn into_owned(self) -> UnsuccessfulOutcome<'static> {
            UnsuccessfulOutcome {
                procedure_code: self.procedure_code,
                criticality: self.criticality,
                value: self.value.into_owned(),
            }
        }
    }
    impl AsnType for UnsuccessfulOutcome<'_> {
        const NAME: &'static str = "UnsuccessfulOutcome";
    }
    impl<'a> Decode<'a> for UnsuccessfulOutcome<'a> {
        #[inline]
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let procedure_code =
                ProcedureCode::decode(reader).map_err(in_field("procedure_code"))?;
            let criticality = Criticality::decode(reader).map_err(in_field("criticality"))?;
            let value = reader.open_type().map_err(in_field("value"))?;
            Ok(Self {
                procedure_code,
                criticality,
                value,
            })
        }
    }
    impl Encode for UnsuccessfulOutcome<'_> {
        #[inline]
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            self.procedure_code
                .encode(writer)
                .map_err(in_field("procedure_code"))?;
            self.criticality
                .encode(writer)
                .map_err(in_field("criticality"))?;
            writer.open_type(&self.value);
            Ok(())
        }
    }
    #[cfg(feature = "inspect")]
    impl Tree for UnsuccessfulOutcome<'_> {
        const NAME: &'static str = <Self as AsnType>::NAME;
        fn to_tree(&self) -> Value {
            let mut members = json::Members::new();
            members.put("procedureCode", self.procedure_code.to_tree());
            members.put("criticality", self.criticality.to_tree());
            members.put("value", json::octets(&self.value));
            members.finish()
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let members = reading.members(value, names::UnsuccessfulOutcome)?;
            let procedure_code =
                reading.member(&members, "procedureCode", ProcedureCode::from_tree)?;
            let criticality = reading.member(&members, "criticality", Criticality::from_tree)?;
            let value = reading.member(&members, "value", |value, reading| {
                json::read_octets(reading, value)
            })?;
            Ok(Self {
                procedure_code,
                criticality,
                value,
            })
        }
    }
}

// The names of the modules at the root.
pub use crate::registry::ToyPduKind;
pub use toy_common_data_types::*;
pub use toy_constants::*;
pub use toy_containers::*;
pub use toy_ies::*;
pub use toy_pdu_contents::*;
pub use toy_pdu_descriptions::*;
