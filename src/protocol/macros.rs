//! The macros of the protocol crates, written once: the registry of a protocol calls those
//! that write its types from its lists, and the public macros of a protocol crate, as
//! `build_ngap!`, call the others with the path of the crate, the module of its bindings and
//! their own name. None is called otherwise, and all are hidden from the documentation.

/// The procedures of a protocol, which its `src/registry.rs` lists from the ASN.1: the
/// code, the name and the criticality of each, then for each of its messages the
/// direction, the kind, the name that ASN.1 gives it, its type and the IEs of its object
/// set, each with its identifier, its criticality and its presence. A message that has no
/// container of protocol IEs, as a private message, has no object set.
///
/// The codes that the macros take by the names of the procedures, the type of the message
/// that they build, the kinds of a PDU, the messages whose protocol IEs a PDU gives and,
/// with the `inspect` feature, the messages of a view and of a tree, the IEs that each can
/// have and the criticality that ASN.1 assigns all come from these lines. The type of a
/// message is written with the lifetime `'a`, which the macro declares where it names the
/// type.
#[doc(hidden)]
#[macro_export]
// The registry of a protocol calls it in its crate, whose items `crate::` names.
#[allow(clippy::crate_in_macro_def)]
macro_rules! __procedures {
    (protocol $protocol:ident $protocol_name:literal, bindings $bindings:ident, pdu $pdu:ident,
        kind $kind:ident, hooks $hooks:meta;
    $($code:literal $procedure:ident $($criticality:ident)? {
        $(InitiatingMessage $initiating:ident $initiating_name:literal $initiating_type:ty
            $([$($initiating_ie:literal $initiating_criticality:ident
                $initiating_presence:ident),*])?;)?
        $(SuccessfulOutcome $successful:ident $successful_name:literal $successful_type:ty
            $([$($successful_ie:literal $successful_criticality:ident
                $successful_presence:ident),*])?;)?
        $(UnsuccessfulOutcome $unsuccessful:ident $unsuccessful_name:literal
            $unsuccessful_type:ty
            $([$($unsuccessful_ie:literal $unsuccessful_criticality:ident
                $unsuccessful_presence:ident),*])?;)?
    })*) => {
        /// The procedure codes, by the names of the procedures.
        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        pub mod procedures {
            $(pub const $procedure: u8 = $code;)*
        }

        /// The type of the message of each procedure, by its direction.
        #[doc(hidden)]
        #[allow(non_snake_case, non_camel_case_types)]
        pub mod messages {
            pub mod InitiatingMessage {
                $($(pub type $procedure<'a> = $initiating_type;)?)*
            }
            pub mod SuccessfulOutcome {
                $($(pub type $procedure<'a> = $successful_type;)?)*
            }
            pub mod UnsuccessfulOutcome {
                $($(pub type $procedure<'a> = $unsuccessful_type;)?)*
            }
        }

        /// The direction and the procedure of a PDU: each variant is one message.
        #[allow(non_camel_case_types)]
        #[derive(Clone, Debug, PartialEq, Eq)]
        #[non_exhaustive]
        pub enum $kind {
            $($(#[doc = concat!("`", $initiating_name, "`.")] $initiating,)?)*
            $($(#[doc = concat!("`", $successful_name, "`.")] $successful,)?)*
            $($(#[doc = concat!("`", $unsuccessful_name, "`.")] $unsuccessful,)?)*
            /// A procedure that the specification does not have in that direction, which
            /// is the one that `direction()` of the PDU gives.
            Other { direction: &'static str, procedure_code: u8 },
        }

        impl $kind {
            #[doc = concat!("Return this kind's ", $protocol_name, " procedure code.")]
            pub fn procedure_code(&self) -> u8 {
                match self {
                    $(
                        $(Self::$initiating => $code,)?
                        $(Self::$successful => $code,)?
                        $(Self::$unsuccessful => $code,)?
                    )*
                    Self::Other { procedure_code, .. } => *procedure_code,
                }
            }
        }

        // The type of each message is a message, and no other type is one.
        $($(
            $crate::__message!(crate::$protocol, $bindings, "InitiatingMessage", $code,
                $initiating_type $([$($initiating_ie),*])?);
        )?)*
        $($(
            $crate::__message!(crate::$protocol, $bindings, "SuccessfulOutcome", $code,
                $successful_type $([$($successful_ie),*])?);
        )?)*
        $($(
            $crate::__message!(crate::$protocol, $bindings, "UnsuccessfulOutcome", $code,
                $unsuccessful_type $([$($unsuccessful_ie),*])?);
        )?)*

        /// The name that ASN.1 gives the message of a direction and a procedure code.
        pub(crate) fn message_name(direction: &str, code: u8) -> Option<&'static str> {
            match (direction, code) {
                $($(("InitiatingMessage", $code) => Some($initiating_name),)?)*
                $($(("SuccessfulOutcome", $code) => Some($successful_name),)?)*
                $($(("UnsuccessfulOutcome", $code) => Some($unsuccessful_name),)?)*
                _ => None,
            }
        }

        impl crate::$bindings::$pdu<'_> {
            /// Return the ASN.1 procedure name.
            pub fn procedure_name(&self) -> &'static str {
                match self.procedure_code() {
                    $($code => stringify!($procedure),)*
                    _ => "Unknown",
                }
            }

            /// Return the canonical direction/procedure kind.
            pub fn kind(&self) -> $kind {
                let procedure_code = self.procedure_code();
                let kind = match self {
                    Self::initiatingMessage(_) => match procedure_code {
                        $($($code => Some($kind::$initiating),)?)*
                        _ => None,
                    },
                    Self::successfulOutcome(_) => match procedure_code {
                        $($($code => Some($kind::$successful),)?)*
                        _ => None,
                    },
                    Self::unsuccessfulOutcome(_) => match procedure_code {
                        $($($code => Some($kind::$unsuccessful),)?)*
                        _ => None,
                    },
                };
                let direction = self.direction();
                kind.unwrap_or($kind::Other { direction, procedure_code })
            }
        }

        /// For the fuzz targets and the tests of the oracle: the name in the bindings of
        /// the type of the message of a direction and a procedure code, when the
        /// protocol has that message.
        #[cfg($hooks)]
        pub(crate) fn message_type_name(direction: &str, code: u8) -> Option<&'static str> {
            match (direction, code) {
                $($(("InitiatingMessage", $code) => {
                    Some($crate::protocol::__type_name(stringify!($initiating_type)))
                })?)*
                $($(("SuccessfulOutcome", $code) => {
                    Some($crate::protocol::__type_name(stringify!($successful_type)))
                })?)*
                $($(("UnsuccessfulOutcome", $code) => {
                    Some($crate::protocol::__type_name(stringify!($unsuccessful_type)))
                })?)*
                _ => None,
            }
        }

        /// For the fuzz targets and the tests of the oracle: the message of a PDU as its
        /// type, read and written back, when the protocol has a message for the PDU: the
        /// octets that it encodes to, or why it does not decode.
        #[cfg($hooks)]
        pub(crate) fn message_read_and_written<'a>(
            pdu: &'a crate::$bindings::$pdu<'_>,
        ) -> Option<Result<Vec<u8>, $crate::Error>> {
            fn rewritten<'a, T: $crate::protocol::Message<'a, Protocol = crate::$protocol>>(
                pdu: &'a crate::$bindings::$pdu<'_>,
            ) -> Result<Vec<u8>, $crate::Error> {
                let message: T = pdu.decode_value()?;
                $crate::encode(&message)
            }
            match (pdu.direction(), pdu.procedure_code()) {
                $($(("InitiatingMessage", $code) => Some(rewritten::<$initiating_type>(pdu)),)?)*
                $($(("SuccessfulOutcome", $code) => Some(rewritten::<$successful_type>(pdu)),)?)*
                $($(("UnsuccessfulOutcome", $code) => {
                    Some(rewritten::<$unsuccessful_type>(pdu))
                })?)*
                _ => None,
            }
        }

        /// The criticality that ASN.1 assigns to the procedure of a code.
        pub(crate) const fn procedure_criticality(
            code: u8,
        ) -> Option<crate::$bindings::Criticality> {
            match code {
                $($($code => Some(crate::$bindings::Criticality::$criticality),)?)*
                _ => None,
            }
        }

        /// The object set of each message: the direction and the procedure code of its
        /// PDU, the name that ASN.1 gives it, and the IEs that its container of protocol
        /// IEs can have, in the order of the set.
        pub(crate) const OBJECT_SETS: &[$crate::protocol::ObjectSet<crate::$bindings::Criticality>] = &[
            $($($crate::protocol::ObjectSet {
                direction: "InitiatingMessage",
                procedure_code: $code,
                name: $initiating_name,
                ies: $crate::__object_set!($bindings $([$(
                    $initiating_ie $initiating_criticality $initiating_presence
                ),*])?),
            },)?)*
            $($($crate::protocol::ObjectSet {
                direction: "SuccessfulOutcome",
                procedure_code: $code,
                name: $successful_name,
                ies: $crate::__object_set!($bindings $([$(
                    $successful_ie $successful_criticality $successful_presence
                ),*])?),
            },)?)*
            $($($crate::protocol::ObjectSet {
                direction: "UnsuccessfulOutcome",
                procedure_code: $code,
                name: $unsuccessful_name,
                ies: $crate::__object_set!($bindings $([$(
                    $unsuccessful_ie $unsuccessful_criticality $unsuccessful_presence
                ),*])?),
            },)?)*
        ];

        /// Where each message is in the object sets, by the rank of its direction and its
        /// procedure code.
        pub(crate) const OBJECT_SET_INDEX: [[u16; 256]; 3] =
            $crate::protocol::__object_set_index(OBJECT_SETS);

        /// The messages: the direction, the procedure code, the name that ASN.1 gives it
        /// and the type of each, the IEs of its object set, each with its identifier and
        /// whether its presence is mandatory, and the path of its type.
        #[cfg(feature = "inspect")]
        #[allow(clippy::type_complexity)]
        pub(crate) const MESSAGES: &[(
            &str,
            u8,
            &str,
            fn() -> $crate::inspect::Typed,
            &[(u16, bool)],
            &str,
        )] = &[
            $($(("InitiatingMessage", $code, $initiating_name,
                || $crate::__typed!($initiating_type),
                &[$($(($initiating_ie, $crate::__presence!($initiating_presence))),*)?],
                stringify!($initiating_type)),)?)*
            $($(("SuccessfulOutcome", $code, $successful_name,
                || $crate::__typed!($successful_type),
                &[$($(($successful_ie, $crate::__presence!($successful_presence))),*)?],
                stringify!($successful_type)),)?)*
            $($(("UnsuccessfulOutcome", $code, $unsuccessful_name,
                || $crate::__typed!($unsuccessful_type),
                &[$($(($unsuccessful_ie, $crate::__presence!($unsuccessful_presence))),*)?],
                stringify!($unsuccessful_type)),)?)*
        ];

        /// The criticality that ASN.1 assigns to each procedure, by its code.
        #[cfg(feature = "inspect")]
        pub(crate) const PROCEDURE_CRITICALITIES: &[(u8, &str)] =
            &[$($(($code, stringify!($criticality)),)?)*];

        /// The criticality that ASN.1 assigns to the IEs of each message: the name that
        /// ASN.1 gives the message, then each IE of its object set with its identifier and
        /// its criticality.
        #[cfg(feature = "inspect")]
        pub(crate) const MESSAGE_IE_CRITICALITIES: &[(&str, &[(u16, &str)])] = &[
            $($(($initiating_name,
                &[$($(($initiating_ie, stringify!($initiating_criticality))),*)?]),)?)*
            $($(($successful_name,
                &[$($(($successful_ie, stringify!($successful_criticality))),*)?]),)?)*
            $($(($unsuccessful_name,
                &[$($(($unsuccessful_ie, stringify!($unsuccessful_criticality))),*)?]),)?)*
        ];
    };
}

/// The traits of one message of a protocol, of a direction and a procedure code, and its
/// readers of IEs: of a container of protocol IEs when its object set is listed, and of a
/// private message otherwise.
#[doc(hidden)]
#[macro_export]
// The registry of a protocol calls it in its crate, whose items `crate::` names.
#[allow(clippy::crate_in_macro_def)]
macro_rules! __message {
    ($protocol:path, $bindings:ident, $direction:literal, $code:literal, $type:ty) => {
        $crate::__message!(@traits $protocol, $bindings, $direction, $code, $type, false);
        impl<'a> $crate::protocol::Container<'a> for $type {
            const __OF_PROTOCOL_IES: bool = false;
            fn __assigned(_: u16) -> Option<$crate::Criticality> {
                None
            }
            fn __fields<'s>(&'s self) -> &'s [crate::$bindings::ProtocolIEField<'s>] {
                &[]
            }
        }
    };
    ($protocol:path, $bindings:ident, $direction:literal, $code:literal, $type:ty
        [$($ie:literal),*]) => {
        $crate::__message!(@traits $protocol, $bindings, $direction, $code, $type, true);
        impl<'a> $crate::protocol::Container<'a> for $type {
            const __OF_PROTOCOL_IES: bool = true;
            fn __assigned(id: u16) -> Option<$crate::Criticality> {
                $crate::protocol::__assigned::<$protocol>($direction, $code, id)
            }
            #[inline]
            fn __fields<'s>(&'s self) -> &'s [crate::$bindings::ProtocolIEField<'s>] {
                &self.protocol_ies.0
            }
        }
        $crate::__readers!($protocol, "message", $type);
    };
    (@traits $protocol:path, $bindings:ident, $direction:literal, $code:literal, $type:ty,
        $of_protocol_ies:literal) => {
        impl<'a> $crate::protocol::InProtocol for $type {
            type Protocol = $protocol;
        }
        impl<'a> $crate::protocol::Message<'a> for $type {
            const DIRECTION: &'static str = $direction;
            const PROCEDURE_CODE: u8 = $code;
            // The class of an elementary procedure has `ignore` by default.
            const CRITICALITY: crate::$bindings::Criticality =
                match crate::registry::procedure_criticality($code) {
                    Some(criticality) => criticality,
                    None => crate::$bindings::Criticality::ignore,
                };
            fn __decode_owned(octets: &[u8]) -> Result<Self, $crate::Error> {
                // The same type, of the octets that are read here.
                fn read<'a>(octets: &'a [u8]) -> Result<$type, $crate::Error> {
                    $crate::decode(octets)
                }
                read(octets).map(|message| message.into_owned())
            }
        }
    };
}

/// The readers of the IEs of a message or of a transfer of a protocol: `ies`, `ie` and
/// `items`, of what `what` names.
#[doc(hidden)]
#[macro_export]
// The registry of a protocol calls it in its crate, whose items `crate::` names.
#[allow(clippy::crate_in_macro_def)]
macro_rules! __readers {
    ($protocol:path, $what:literal, $type:ty) => {
        impl<'a> $type {
            #[doc = concat!(
                "Every occurrence of the IE `T` in this ", $what, ", in its order, each decoded \
                 as the type of `T`: an error of the kind `UndecodableIe` for one that does \
                 not decode. The IEs of other identifiers are passed over, and not decoded."
            )]
            #[inline]
            pub fn ies<T: $crate::protocol::Ie<Protocol = $protocol>>(
                &self,
            ) -> $crate::protocol::Ies<'_, T> {
                let fields = $crate::protocol::Container::__fields(self);
                let name = <Self as $crate::AsnType>::NAME;
                $crate::protocol::Source::__of_fields(name, fields).ies::<T>()
            }

            #[doc = concat!(
                "The first occurrence of the IE `T` in this ", $what, ", decoded as the type \
                 of `T`; `None` when it does not have it. `check()` of a PDU reports an IE \
                 that a message has twice, and [`ies`](Self::ies) gives each occurrence."
            )]
            #[inline]
            pub fn ie<T: $crate::protocol::Ie<Protocol = $protocol>>(
                &self,
            ) -> Option<Result<T::Value<'_>, $crate::Error>> {
                self.ies::<T>().next()
            }

            #[doc = concat!(
                "The items of the list `L` of this ", $what, ", a list of single containers, \
                 each decoded as the IE of its single container: the list is walked, and not \
                 built. Nothing when there is no such list, which [`ie`](Self::ie) tells."
            )]
            #[inline]
            pub fn items<L: $crate::protocol::List<Protocol = $protocol>>(
                &self,
            ) -> $crate::protocol::Items<'_, L> {
                let fields = $crate::protocol::Container::__fields(self);
                let name = <Self as $crate::AsnType>::NAME;
                $crate::protocol::items_of_fields::<L>(name, fields)
            }
        }
    };
}

/// A transfer of a protocol whose octets hold a container of protocol IEs, which the macros
/// read and build IEs for as they do for a message, and which has the readers of a message.
#[doc(hidden)]
#[macro_export]
// The registry of a protocol calls it in its crate, whose items `crate::` names.
#[allow(clippy::crate_in_macro_def)]
macro_rules! __transfer {
    ($protocol:path, $bindings:ident, $($type:ident)*) => {
        $(
            impl $crate::protocol::InProtocol for crate::$bindings::$type<'_> {
                type Protocol = $protocol;
            }
            impl<'a> $crate::protocol::Container<'a> for crate::$bindings::$type<'a> {
                const __OF_PROTOCOL_IES: bool = true;
                fn __assigned(_: u16) -> Option<$crate::Criticality> {
                    None
                }
                #[inline]
                fn __fields<'s>(&'s self) -> &'s [crate::$bindings::ProtocolIEField<'s>] {
                    &self.protocol_ies.0
                }
            }
            $crate::__readers!($protocol, "transfer", crate::$bindings::$type<'a>);
        )*
    };
}

/// The IEs of an object set, each as its identifier, the criticality that the set
/// assigns it and whether its presence is mandatory: none for a message that has no
/// container of protocol IEs.
#[doc(hidden)]
#[macro_export]
// The registry of a protocol calls it in its crate, whose items `crate::` names.
#[allow(clippy::crate_in_macro_def)]
macro_rules! __object_set {
    ($bindings:ident) => {
        None
    };
    ($bindings:ident [$($id:literal $criticality:ident $presence:ident),*]) => {
        Some(&[$((
            $id,
            crate::$bindings::Criticality::$criticality,
            $crate::__presence!($presence),
        )),*])
    };
}

/// Whether an IE of an object set is always there: its `PRESENCE`, as ASN.1 words it.
#[doc(hidden)]
#[macro_export]
macro_rules! __presence {
    (mandatory) => {
        true
    };
    (optional) => {
        false
    };
    (conditional) => {
        false
    };
}

/// The IEs of a protocol, which its `src/registry.rs` lists from the ASN.1: the identifier,
/// the name that ASN.1 gives it and the type of each, the type that its octets contain,
/// then the names that the macros take it by: its own, then those of its type. An IE that
/// a value holds alone, in a single container, has its criticality after its identifier
/// when its object sets give it one. An IE whose value is a list of single containers has
/// the identifier of its item and the size of the list in parentheses after its type. An IE
/// that holds a key is marked `@ key`. An IE of an extension container that has the type
/// which the type that it extends gives it has its types in brackets after `where`, each
/// with the types that it extends so.
///
/// The marker type of each IE, by the names that the macros take it by, and, with the
/// `inspect` feature, the names and the types of a view and of a tree and the criticality
/// of an IE in a single container all come from these lines. A type that borrows is
/// written with the lifetime `'a`, which the macro declares where it names the type.
#[doc(hidden)]
#[macro_export]
// The registry of a protocol calls it in its crate, whose items `crate::` names.
#[allow(clippy::crate_in_macro_def)]
macro_rules! __ies {
    (protocol $protocol:ident $protocol_name:literal;
    $($id:literal $($item:ident)? $name:literal $(@ $key:ident)?
        $($ie:ty $(, $contents:ty)? $(=> $own:ident $($alias:ident)*)?)?
        $(where [$($extending:ty: $($extended:ident)+),+])?;
    )*) => {
        /// The IEs of the protocol, one marker type each, by the names that the macros
        /// take: its own, after the identifier that ASN.1 gives it, and the name of its
        /// type for the one IE of that type.
        #[allow(non_camel_case_types)]
        pub mod ie {
            $($($(
                #[doc = concat!("The IE ", $name, ", of identifier ", stringify!($id), ".")]
                #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
                pub struct $own;

                impl $crate::protocol::Ie for $own {
                    type Protocol = crate::$protocol;
                    const ID: u16 = $id;
                    const NAME: &'static str = $name;
                    type Value<'a> = $ie;

                    #[inline]
                    fn __owned<'a, 'b>(octets: &'a [u8]) -> Result<Self::Value<'b>, $crate::Error> {
                        // A type that borrows nothing is its own owned value.
                        #[allow(unused_imports)]
                        use $crate::protocol::__IntoOwned as _;
                        $crate::decode::<$ie>(octets).map(|value| value.into_owned())
                    }
                }
                $(pub use $own as $alias;)*
            )?)?)*
        }

        /// The identifier of each IE with the name that ASN.1 gives it after `id-`.
        pub(crate) const IE_NAMES: &[(u16, &str)] = &[$(($id, $name),)*];

        /// Whether `octets`, the value of the IE `id`, decode as the type that the list
        /// gives the IE: the error of the codec when they do not, and `None` for an
        /// identifier that has no one type. A value that decodes may be outside the
        /// root of an extensible constraint of its type: it then has the error of an
        /// encoding that refuses it. And it has what of its encoding is in another
        /// form than the one that X.691 gives it.
        #[allow(clippy::type_complexity)]
        pub(crate) fn decodes<'a>(
            id: u16,
            octets: &'a [u8],
        ) -> Option<Result<(Option<$crate::Error>, $crate::Faults), $crate::Error>> {
            match id {
                $($(
                    $id => {
                        let value: Result<($ie, _), $crate::Error> =
                            $crate::decode_with_faults(octets);
                        // A reader that notes reads what one that does not reads.
                        debug_assert_eq!(
                            value.as_ref().map(|(value, _)| value),
                            $crate::decode::<$ie>(octets).as_ref()
                        );
                        Some(value.map(|(value, faults)| {
                            ($crate::encode_strict(&value).err(), faults)
                        }))
                    }
                )?)*
                _ => None,
            }
        }

        /// The identifiers of the IEs whose type holds a key, which the list marks:
        /// `Debug` shows the length of the octets of such an IE, and not the octets.
        pub(crate) const KEY_IES: &[u16] = &[$($({ let _ = stringify!($key); $id },)?)*];

        /// The criticality that ASN.1 assigns to each IE that a value holds alone, in a
        /// single container, when its object sets give it one.
        #[cfg(feature = "inspect")]
        pub(crate) const ITEM_CRITICALITIES: &[(u16, &str)] =
            &[$($(($id, stringify!($item)),)?)*];

        /// The path of the type of each IE that has one type, and that of the type that
        /// its octets contain.
        #[cfg(feature = "inspect")]
        pub(crate) const IE_TYPES: &[(u16, &str, &str)] = &[
            $($(($id, stringify!($ie), concat!($(stringify!($contents))?)),)?)*
        ];

        /// The IEs of an extension container that have the type which the type that
        /// they extend gives them: the identifier, the name of a type that is extended
        /// and the path of the type that the IE has there.
        #[cfg(feature = "inspect")]
        pub(crate) const EXTENSION_TYPES: &[(u16, &str, &str)] = &[
            $($($($(($id, stringify!($extended), stringify!($extending)),)+)+)?)*
        ];

        /// The functions of the type of the IE `id` where it extends the type that the
        /// bindings name `extended`; `None` for an IE that has its type whatever it
        /// extends.
        #[cfg(feature = "inspect")]
        #[allow(clippy::match_single_binding)]
        pub(crate) fn extension(id: u16, extended: &str) -> Option<$crate::inspect::Typed> {
            match (id, extended) {
                $($($($(($id, stringify!($extended)) => {
                    Some($crate::__typed!($extending))
                })+)+)?)*
                _ => None,
            }
        }

        #[cfg(feature = "inspect")]
        pub(crate) fn ie(id: u16) -> Result<$crate::inspect::Typed, $crate::Error> {
            match id {
                $($($id => Ok($crate::__typed!($ie)),)?)*
                _ => {
                    // An IE that has a type for each type that it extends has none
                    // elsewhere.
                    let extensions = EXTENSION_TYPES.iter().filter(|(known, ..)| *known == id);
                    let extended: Vec<_> = extensions.map(|(_, extended, _)| *extended).collect();
                    Err($crate::Error::unknown_ie(match extended.is_empty() {
                        true => format!("{} IE {id} is unknown", $protocol_name),
                        false => format!(
                            "{} IE {id} has a type where it extends {}, and none here",
                            $protocol_name,
                            extended.join(", ")
                        ),
                    }))
                }
            }
        }

        #[cfg(feature = "inspect")]
        #[allow(clippy::match_single_binding)]
        pub(crate) fn ie_contents(id: u16) -> Option<$crate::inspect::Typed> {
            match id {
                $($($($id => Some($crate::__typed!($contents)),)?)?)*
                _ => None,
            }
        }
    };
}

/// The lists of single containers of a protocol, which its `src/registry.rs` lists: the
/// marker type of the list IE, the marker type of the IE of its items, and the size of the
/// list.
#[doc(hidden)]
#[macro_export]
// The registry of a protocol calls it in its crate, whose items `crate::` names.
#[allow(clippy::crate_in_macro_def)]
macro_rules! __lists {
    ($($list:ident $item:ident $lower:literal $upper:literal;)*) => {
        $(
            impl $crate::protocol::List for crate::registry::ie::$list {
                type Item = crate::registry::ie::$item;
                const __SIZE: (usize, usize) = ($lower, $upper);
            }
        )*
    };
}

/// The marker type of a protocol and what the readers of `oxirush_aper::protocol` take of
/// its bindings: its PDU, the IE of a container, its criticality, and the types that its
/// deviations are told with. `triggering` names the alternatives of the `TriggeringMessage`
/// of the protocol, in the order of the directions.
#[doc(hidden)]
#[macro_export]
// The registry of a protocol calls it in its crate, whose items `crate::` names.
#[allow(clippy::crate_in_macro_def)]
macro_rules! __protocol {
    (protocol $protocol:ident $protocol_name:literal, bindings $bindings:ident, pdu $pdu:ident,
        triggering $initiating:ident $successful:ident $unsuccessful:ident) => {
        #[doc = concat!(
            $protocol_name, ", as the types that its deviations are told with: a [`Deviation`] \
             has the criticality of the protocol, and its answer the `TypeOfError`, the \
             `CauseProtocol`, the `CriticalityDiagnostics-IE-Item` and the \
             `CriticalityDiagnostics` of the protocol. The traits of \
             `oxirush_aper::protocol` name the protocol by it."
        )]
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $protocol;

        #[doc = concat!(
            "One way in which a PDU is not as the specification has a sender construct it: \
             what `check()` of a PDU reports, in the types of ", $protocol_name, ". See \
             [`oxirush_aper::check::Deviation`], which the protocols share, for its variants \
             and what each says."
        )]
        pub type Deviation = $crate::check::Deviation<$protocol>;

        impl $crate::protocol::Protocol for $protocol {
            const NAME: &'static str = $protocol_name;
            type Pdu<'a> = crate::$bindings::$pdu<'a>;
            type Field<'a> = crate::$bindings::ProtocolIEField<'a>;
            const __OBJECT_SETS: &'static [$crate::protocol::ObjectSet<crate::$bindings::Criticality>] =
                crate::registry::OBJECT_SETS;
            const __OBJECT_SET_INDEX: &'static [[u16; 256]; 3] = &crate::registry::OBJECT_SET_INDEX;
            const __IE_NAMES: &'static [(u16, &'static str)] = crate::registry::IE_NAMES;
            const __KEY_IES: &'static [u16] = crate::registry::KEY_IES;
            #[inline]
            fn __decodes(
                id: u16,
                octets: &[u8],
            ) -> Option<Result<(Option<$crate::Error>, $crate::Faults), $crate::Error>> {
                crate::registry::decodes(id, octets)
            }
            #[inline]
            fn __procedure_criticality(code: u8) -> Option<crate::$bindings::Criticality> {
                crate::registry::procedure_criticality(code)
            }
        }

        impl $crate::check::Diagnostics for $protocol {
            type Criticality = crate::$bindings::Criticality;
            type TypeOfError = crate::$bindings::TypeOfError;
            type Cause = crate::$bindings::CauseProtocol;
            type Item = crate::$bindings::CriticalityDiagnosticsIEItem;
            type CriticalityDiagnostics = crate::$bindings::CriticalityDiagnostics;

            fn name(id: u16) -> Option<&'static str> {
                $crate::protocol::name::<$protocol>(id)
            }

            fn type_of_error(missing: bool) -> crate::$bindings::TypeOfError {
                match missing {
                    true => crate::$bindings::TypeOfError::missing,
                    false => crate::$bindings::TypeOfError::not_understood,
                }
            }

            fn cause(cause: $crate::check::ProtocolCause) -> crate::$bindings::CauseProtocol {
                use crate::$bindings::CauseProtocol;
                use $crate::check::ProtocolCause;
                match cause {
                    ProtocolCause::TransferSyntaxError => CauseProtocol::transfer_syntax_error,
                    ProtocolCause::AbstractSyntaxErrorReject => {
                        CauseProtocol::abstract_syntax_error_reject
                    }
                    ProtocolCause::AbstractSyntaxErrorIgnoreAndNotify => {
                        CauseProtocol::abstract_syntax_error_ignore_and_notify
                    }
                    ProtocolCause::AbstractSyntaxErrorFalselyConstructedMessage => {
                        CauseProtocol::abstract_syntax_error_falsely_constructed_message
                    }
                    // A cause that a later version of oxirush-aper gives.
                    _ => CauseProtocol::unspecified,
                }
            }

            fn item(
                criticality: crate::$bindings::Criticality,
                id: u16,
                type_of_error: crate::$bindings::TypeOfError,
            ) -> crate::$bindings::CriticalityDiagnosticsIEItem {
                crate::$bindings::CriticalityDiagnosticsIEItem::new(
                    criticality,
                    crate::$bindings::ProtocolIEID(id),
                    type_of_error,
                    None,
                )
            }

            fn criticality_diagnostics(
                triggering: Option<(u8, &str)>,
                criticality: crate::$bindings::Criticality,
                items: Option<Vec<crate::$bindings::CriticalityDiagnosticsIEItem>>,
            ) -> crate::$bindings::CriticalityDiagnostics {
                use crate::$bindings::TriggeringMessage;
                let message = |direction: &str| match direction {
                    "SuccessfulOutcome" => TriggeringMessage::$successful,
                    "UnsuccessfulOutcome" => TriggeringMessage::$unsuccessful,
                    _ => TriggeringMessage::$initiating,
                };
                crate::$bindings::CriticalityDiagnostics::new(
                    triggering.map(|(code, _)| crate::$bindings::ProcedureCode(code)),
                    triggering.map(|(_, direction)| message(direction)),
                    Some(criticality),
                    items.map(crate::$bindings::CriticalityDiagnosticsIEList),
                    None,
                )
            }
        }

        /// The criticality of the bindings is that of oxirush-aper, which an error has.
        impl From<crate::$bindings::Criticality> for $crate::Criticality {
            fn from(criticality: crate::$bindings::Criticality) -> Self {
                match criticality {
                    crate::$bindings::Criticality::reject => Self::Reject,
                    crate::$bindings::Criticality::ignore => Self::Ignore,
                    crate::$bindings::Criticality::notify => Self::Notify,
                }
            }
        }

        impl From<$crate::Criticality> for crate::$bindings::Criticality {
            fn from(criticality: $crate::Criticality) -> Self {
                match criticality {
                    $crate::Criticality::Reject => Self::reject,
                    $crate::Criticality::Ignore => Self::ignore,
                    $crate::Criticality::Notify => Self::notify,
                }
            }
        }

        impl $crate::protocol::InProtocol for crate::$bindings::ProtocolIEField<'_> {
            type Protocol = $protocol;
        }

        impl<'a> $crate::protocol::Field<'a> for crate::$bindings::ProtocolIEField<'a> {
            #[inline]
            fn id(&self) -> u16 {
                self.id.0
            }
            #[inline]
            fn criticality(&self) -> crate::$bindings::Criticality {
                self.criticality
            }
            #[inline]
            fn value(&self) -> &$crate::Octets<'a> {
                &self.value
            }
            #[inline]
            fn __new(
                id: u16,
                criticality: crate::$bindings::Criticality,
                value: $crate::Octets<'a>,
            ) -> Self {
                Self {
                    id: crate::$bindings::ProtocolIEID(id),
                    criticality,
                    value,
                }
            }
        }

        impl $crate::protocol::InProtocol for crate::$bindings::$pdu<'_> {
            type Protocol = $protocol;
        }

        impl<'a> $crate::protocol::Pdu<'a> for crate::$bindings::$pdu<'a> {
            #[inline]
            fn direction(&self) -> &'static str {
                Self::direction(self)
            }
            #[inline]
            fn procedure_code(&self) -> u8 {
                Self::procedure_code(self)
            }
            #[inline]
            fn criticality(&self) -> crate::$bindings::Criticality {
                match self {
                    Self::initiatingMessage(message) => message.criticality,
                    Self::successfulOutcome(message) => message.criticality,
                    Self::unsuccessfulOutcome(message) => message.criticality,
                }
            }
            #[inline]
            fn value(&self) -> &$crate::Octets<'a> {
                match self {
                    Self::initiatingMessage(message) => &message.value,
                    Self::successfulOutcome(message) => &message.value,
                    Self::unsuccessfulOutcome(message) => &message.value,
                }
            }
            fn __set_value(&mut self, value: $crate::Octets<'a>) {
                match self {
                    Self::initiatingMessage(message) => message.value = value,
                    Self::successfulOutcome(message) => message.value = value,
                    Self::unsuccessfulOutcome(message) => message.value = value,
                }
            }
            fn __of(
                direction: &str,
                procedure_code: u8,
                criticality: crate::$bindings::Criticality,
                value: $crate::Octets<'a>,
            ) -> Option<Self> {
                use crate::$bindings::{
                    InitiatingMessage, ProcedureCode, SuccessfulOutcome, UnsuccessfulOutcome,
                };
                let procedure_code = ProcedureCode(procedure_code);
                Some(match direction {
                    "InitiatingMessage" => Self::initiatingMessage(InitiatingMessage {
                        procedure_code,
                        criticality,
                        value,
                    }),
                    "SuccessfulOutcome" => Self::successfulOutcome(SuccessfulOutcome {
                        procedure_code,
                        criticality,
                        value,
                    }),
                    "UnsuccessfulOutcome" => Self::unsuccessfulOutcome(UnsuccessfulOutcome {
                        procedure_code,
                        criticality,
                        value,
                    }),
                    _ => return None,
                })
            }
        }

        // A PDU is what the macro that extracts IEs reads them from, as the message that
        // it holds.
        impl<'p, 'a, M: $crate::protocol::Message<'a, Protocol = $protocol>>
            $crate::protocol::__Holds<M> for crate::$bindings::$pdu<'p>
        {
            #[inline]
            fn __message(&self) -> Result<&'static str, $crate::Error> {
                $crate::protocol::__pdu_holds::<Self, M>(self)
            }
        }

        impl<'s, 'p: 's> $crate::protocol::__Source<'s> for crate::$bindings::$pdu<'p> {
            type Protocol = $protocol;
            type Walk = $crate::protocol::Source<'s, $protocol>;

            #[inline]
            fn __source<N>(
                &'s self,
            ) -> Result<$crate::protocol::Source<'s, $protocol>, $crate::Error>
            where
                Self: $crate::protocol::__Holds<N>,
            {
                let message = <Self as $crate::protocol::__Holds<N>>::__message(self)?;
                let octets = <Self as $crate::protocol::Pdu<'p>>::value(self);
                Ok($crate::protocol::Source::__of_octets(message, octets.as_slice()))
            }
        }

        /// The tables of the registry, which `check()` of a PDU and the readers read: the
        /// object set of each message, and the names of the IEs.
        #[cfg(test)]
        mod tables {
            use crate::registry::{IE_NAMES, OBJECT_SETS};
            use $crate::protocol::{name, object_set};

            #[test]
            fn the_ies_are_listed_in_the_order_of_their_identifiers() {
                // The name of an IE is found by a binary search of its identifier.
                assert!(IE_NAMES.windows(2).all(|pair| pair[0].0 < pair[1].0));
                for (id, known) in IE_NAMES {
                    assert_eq!(name::<super::$protocol>(*id), Some(*known));
                }
                assert_eq!(name::<super::$protocol>(u16::MAX), None);
            }

            #[test]
            fn each_message_but_the_private_one_has_an_object_set() {
                let without = OBJECT_SETS.iter().filter(|set| set.ies.is_none());
                assert!(without.map(|set| set.name).eq(["PrivateMessage"]));
                // A message is found by its direction and its procedure code.
                for message in OBJECT_SETS {
                    let found = object_set::<super::$protocol>(message.direction, message.procedure_code);
                    assert_eq!(found.map(|found| found.name), Some(message.name));
                }
                assert!(object_set::<super::$protocol>("InitiatingMessage", 255).is_none());
            }

            #[test]
            fn every_ie_of_an_object_set_has_a_name_and_is_there_once() {
                for message in OBJECT_SETS {
                    for (index, (id, ..)) in message.ies.into_iter().flatten().enumerate() {
                        assert!(name::<super::$protocol>(*id).is_some(), "IE {id} of {}", message.name);
                        let before = &message.ies.unwrap_or_default()[..index];
                        let again = before.iter().any(|(known, ..)| known == id);
                        assert!(!again, "IE {id} is twice in the set of {}", message.name);
                    }
                }
            }
        }
    };
}

/// `extract_ngap_ies!` and `extract_s1ap_ies!`: the IEs that the fields ask for, from a
/// message, a transfer or a PDU, in one pass over its IEs, each decoded once.
#[doc(hidden)]
#[macro_export]
macro_rules! __extract_ies {
    ($k:tt, $bindings:ident, $msg_var:expr, $msg:ident,
        $($kind:ident $name:ident : $ty:ty = $ie_name:ident ($bind:ident) $(=> $expr:expr)?),+
        $(,)?
    ) => {
        // The name is the type of what the IEs are read from, which has protocol IEs.
        $crate::protocol::__message_of_protocol_ies::<$k::$bindings::$msg<'_>>();
        // The first occurrence of each IE, in one pass over the container, which decodes
        // none.
        $(
            #[allow(unused_mut)]
            let mut $name = ::core::option::Option::None;
        )+
        #[allow(unused_mut)]
        let mut _failed: ::core::option::Option<$crate::Error> = ::core::option::Option::None;
        {
            #[allow(unused_imports)]
            use $crate::protocol::{__Each as _, __Source as _};
            match (&$msg_var).__source::<$k::$bindings::$msg<'_>>() {
                ::core::result::Result::Err(_error) => _failed = ::core::option::Option::Some(_error),
                ::core::result::Result::Ok(_source) => {
                    let _read = _source.__each(|_entry| {
                        $(
                            if _entry.id() == <$k::ie::$ie_name as $crate::protocol::Ie>::ID
                                && $name.is_none()
                            {
                                $name = ::core::option::Option::Some(_entry.clone());
                            }
                        )+
                    });
                    if let ::core::result::Result::Err(_error) = _read {
                        // The IEs cannot be read: no field has an IE.
                        $( $name = ::core::option::Option::None; )+
                        _failed = ::core::option::Option::Some(_error);
                    }
                }
            }
        }
        // The IEs that could not be read are the error of the first `req` or `opt` field.
        $( $crate::__extract_ie!(@failed $kind _failed); )+
        #[allow(unused_imports)]
        use $crate::protocol::__Item as _;
        // Each field, of the IE that was found, decoded, in the order of the fields: the
        // first `req` or `opt` field that has no value returns why.
        $(
            $crate::__extract_ie!(@take $kind $name: $ty = $ie_name($bind) $(=> $expr)?,
                $k, $bindings, $msg);
        )+
    };
}

/// The arms of the macro that extracts IEs that its expansion calls: the value of a field,
/// the error of the IEs that could not be read, and what a field is after the pass.
#[doc(hidden)]
#[macro_export]
macro_rules! __extract_ie {
    // The value of the IE, or the one value that its type is a name for: what the type of
    // the field says.
    (@val $bind:ident) => {
        $crate::protocol::__Field::__field($bind)
    };
    (@val $_bind:ident, $expr:expr) => {
        $expr
    };
    // The IEs could not be read: a `req` or an `opt` field returns why, and a `lax` field
    // is `None`.
    (@failed req $failed:ident) => {
        if let ::core::option::Option::Some(_error) = $failed.take() {
            return ::core::result::Result::Err(::core::convert::From::from(_error));
        }
    };
    (@failed opt $failed:ident) => {
        if let ::core::option::Option::Some(_error) = $failed.take() {
            return ::core::result::Result::Err(::core::convert::From::from(_error));
        }
    };
    (@failed $kind:ident $failed:ident) => {};
    // `$bind` is out of scope where `$name` is assigned, as both may have the same name.
    (@take req $name:ident: $ty:ty = $ie_name:ident($bind:ident) $(=> $expr:expr)?,
        $k:tt, $bindings:ident, $msg:ident) => {
        let $name: $ty = match &$name {
            ::core::option::Option::Some(_entry) => match _entry.decode::<$k::ie::$ie_name>(
                <$k::$bindings::$msg<'_> as $crate::AsnType>::NAME,
            ) {
                ::core::result::Result::Ok($bind) => $crate::__extract_ie!(@val $bind $(, $expr)?),
                // The error of the enclosing function, converted as `?` converts one.
                ::core::result::Result::Err(_error) => {
                    return ::core::result::Result::Err(::core::convert::From::from(_error));
                }
            },
            ::core::option::Option::None => {
                return ::core::result::Result::Err(::core::convert::From::from(
                    $crate::protocol::__missing_ie::<$k::$bindings::$msg<'_>>(
                        <$k::ie::$ie_name as $crate::protocol::Ie>::ID,
                        stringify!($name),
                    ),
                ));
            }
        };
    };
    (@take opt $name:ident: $ty:ty = $ie_name:ident($bind:ident) $(=> $expr:expr)?,
        $k:tt, $bindings:ident, $msg:ident) => {
        let $name: ::core::option::Option<$ty> = match &$name {
            ::core::option::Option::Some(_entry) => match _entry.decode::<$k::ie::$ie_name>(
                <$k::$bindings::$msg<'_> as $crate::AsnType>::NAME,
            ) {
                ::core::result::Result::Ok($bind) => {
                    ::core::option::Option::Some($crate::__extract_ie!(@val $bind $(, $expr)?))
                }
                ::core::result::Result::Err(_error) => {
                    return ::core::result::Result::Err(::core::convert::From::from(_error));
                }
            },
            ::core::option::Option::None => ::core::option::Option::None,
        };
    };
    // The IE if it decodes: nothing is returned.
    (@take lax $name:ident: $ty:ty = $ie_name:ident($bind:ident) $(=> $expr:expr)?,
        $k:tt, $bindings:ident, $msg:ident) => {
        let $name: ::core::option::Option<$ty> = match &$name {
            ::core::option::Option::Some(_entry) => match _entry.decode::<$k::ie::$ie_name>(
                <$k::$bindings::$msg<'_> as $crate::AsnType>::NAME,
            ) {
                ::core::result::Result::Ok($bind) => {
                    ::core::option::Option::Some($crate::__extract_ie!(@val $bind $(, $expr)?))
                }
                ::core::result::Result::Err(_) => ::core::option::Option::None,
            },
            ::core::option::Option::None => ::core::option::Option::None,
        };
    };
    (@take $kind:ident $name:ident: $ty:ty = $ie_name:ident($bind:ident) $(=> $expr:expr)?,
        $k:tt, $bindings:ident, $msg:ident) => {
        compile_error!(concat!(
            "`",
            stringify!($kind),
            "` is no kind of field: write req, opt or lax before the name of the field ",
            stringify!($name),
        ));
    };
}

/// `with_ngap_ie_mut!` and `with_s1ap_ie_mut!`: the first occurrence of an IE of a message
/// or of a transfer, decoded, changed by an expression, and written back.
#[doc(hidden)]
#[macro_export]
macro_rules! __with_ie_mut {
    ($k:tt, $bindings:ident, $msg_var:expr, $msg:ident, $ie_name:ident($bind:ident) = $value:expr
        $(,)?) => {{
        $crate::__with_ie_mut!($k, $bindings, $msg_var, $msg, $ie_name($bind) => {
            $bind.0 = $value;
            true
        })
    }};
    ($k:tt, $bindings:ident, $msg_var:expr, $msg:ident, $ie_name:ident($bind:ident) => $expr:expr
        $(,)?) => {{
        // The name is the type of what the IE is in.
        let _: &$k::$bindings::$msg<'_> = &$msg_var;
        // The IE that the macro that extracts IEs reads: the first one. The value is changed
        // and written to octets of its own before the message is, which it borrowed its
        // octets from.
        let _written = match $crate::protocol::__first::<$k::ie::$ie_name, $k::$bindings::$msg<'_>>(
            &$msg_var,
        ) {
            ::core::result::Result::Err(_error) => ::core::result::Result::Err(_error),
            ::core::result::Result::Ok(::core::option::Option::None) => {
                ::core::result::Result::Ok(::core::option::Option::None)
            }
            ::core::result::Result::Ok(::core::option::Option::Some((_index, mut $bind))) => {
                let _changed: bool = $expr;
                match $k::$bindings::encode_open_type(&$bind) {
                    ::core::result::Result::Ok(_value) => ::core::result::Result::Ok(
                        ::core::option::Option::Some((_index, _changed, _value)),
                    ),
                    ::core::result::Result::Err(_error) => ::core::result::Result::Err(_error),
                }
            }
        };
        match _written {
            ::core::result::Result::Err(_error) => ::core::result::Result::Err(_error),
            ::core::result::Result::Ok(::core::option::Option::None) => {
                ::core::result::Result::Ok(false)
            }
            ::core::result::Result::Ok(::core::option::Option::Some((
                _index,
                _changed,
                _value,
            ))) => {
                // The IE is where it was read, unless the message is another one each
                // time that its expression is read.
                match $msg_var.protocol_ies.0.get_mut(_index) {
                    ::core::option::Option::Some(_ie) => {
                        _ie.value = _value;
                        ::core::result::Result::Ok(_changed)
                    }
                    ::core::option::Option::None => ::core::result::Result::Ok(false),
                }
            }
        }
    }};
}

/// `build_ngap!` and `build_s1ap!`: a PDU of a direction, a procedure, its criticality or
/// the one of its procedure, a message, and its IEs.
#[doc(hidden)]
#[macro_export]
macro_rules! __build {
    ($k:tt, $bindings:ident, $name:literal, $direction:ident, $proc:ident, REJECT, $msg:ident
        $(, $($ies:tt)*)?) => {
        $crate::__build_pdu!($k, $bindings, $direction, $proc,
            $k::$bindings::Criticality::reject, $msg, $($($ies)*)?)
    };
    ($k:tt, $bindings:ident, $name:literal, $direction:ident, $proc:ident, IGNORE, $msg:ident
        $(, $($ies:tt)*)?) => {
        $crate::__build_pdu!($k, $bindings, $direction, $proc,
            $k::$bindings::Criticality::ignore, $msg, $($($ies)*)?)
    };
    ($k:tt, $bindings:ident, $name:literal, $direction:ident, $proc:ident, NOTIFY, $msg:ident
        $(, $($ies:tt)*)?) => {
        $crate::__build_pdu!($k, $bindings, $direction, $proc,
            $k::$bindings::Criticality::notify, $msg, $($($ies)*)?)
    };
    // The criticality of the PDU is that of its procedure.
    ($k:tt, $bindings:ident, $name:literal, $direction:ident, $proc:ident, $msg:ident
        $(, $($ies:tt)*)?) => {
        $crate::__build_pdu!($k, $bindings, $direction, $proc,
            <$k::$bindings::$msg<'static> as $crate::protocol::Message<'static>>::CRITICALITY,
            $msg, $($($ies)*)?)
    };
    // What is not written so, which says what is wrong.
    ($k:tt, $bindings:ident, $name:literal, $($other:tt)*) => {
        compile_error!(concat!(
            $name,
            " takes the direction, the procedure, the criticality of the PDU, the message, \
             and each IE as `CRITICALITY Name(value)`"
        ))
    };
}

/// What the macro that builds a PDU does once the criticality of the PDU is an expression.
#[doc(hidden)]
#[macro_export]
macro_rules! __build_pdu {
    ($k:tt, $bindings:ident, $direction:ident, $proc:ident, $outer:expr, $msg:ident,
        $($ies:tt)*) => {{
        // The message is the one that the procedure has in that direction, and a container
        // of protocol IEs.
        $crate::protocol::__message_of(
            |message: $k::$bindings::$msg<'static>|
                -> $k::registry::messages::$direction::$proc<'static> { message },
        );
        $crate::protocol::__message_of_protocol_ies::<$k::$bindings::$msg<'static>>();
        #[allow(unused_mut)]
        let mut _message = $crate::protocol::__Message::<
            <$k::$bindings::$msg<'static> as $crate::protocol::InProtocol>::Protocol,
        >::new();
        $crate::__build_ies!($k, $bindings, _message, $msg, $($ies)*);
        _message.pdu::<$k::$bindings::$msg<'static>>($outer)
    }};
}

/// The IEs of a call of the macro that builds a PDU, one after the other, into the message
/// that is written: each with its criticality, or with the one that the object set assigns
/// it.
#[doc(hidden)]
#[macro_export]
macro_rules! __build_ies {
    ($k:tt, $bindings:ident, $message:ident, $msg:ident $(,)?) => {};
    ($k:tt, $bindings:ident, $message:ident, $msg:ident,
        REJECT $ie_name:ident ($($ie_value:tt)+) $(, $($rest:tt)*)?) => {
        $message.ie(
            <$k::ie::$ie_name as $crate::protocol::Ie>::ID,
            $k::$bindings::Criticality::reject,
            &<$k::ie::$ie_name as $crate::protocol::Ie>::value(($($ie_value)+)),
        );
        $crate::__build_ies!($k, $bindings, $message, $msg, $($($rest)*)?);
    };
    ($k:tt, $bindings:ident, $message:ident, $msg:ident,
        IGNORE $ie_name:ident ($($ie_value:tt)+) $(, $($rest:tt)*)?) => {
        $message.ie(
            <$k::ie::$ie_name as $crate::protocol::Ie>::ID,
            $k::$bindings::Criticality::ignore,
            &<$k::ie::$ie_name as $crate::protocol::Ie>::value(($($ie_value)+)),
        );
        $crate::__build_ies!($k, $bindings, $message, $msg, $($($rest)*)?);
    };
    ($k:tt, $bindings:ident, $message:ident, $msg:ident,
        NOTIFY $ie_name:ident ($($ie_value:tt)+) $(, $($rest:tt)*)?) => {
        $message.ie(
            <$k::ie::$ie_name as $crate::protocol::Ie>::ID,
            $k::$bindings::Criticality::notify,
            &<$k::ie::$ie_name as $crate::protocol::Ie>::value(($($ie_value)+)),
        );
        $crate::__build_ies!($k, $bindings, $message, $msg, $($($rest)*)?);
    };
    ($k:tt, $bindings:ident, $message:ident, $msg:ident,
        $ie_name:ident ($($ie_value:tt)+) $(, $($rest:tt)*)?) => {
        $message.ie_assigned::<$k::$bindings::$msg<'static>, _>(
            <$k::ie::$ie_name as $crate::protocol::Ie>::ID,
            stringify!($ie_name),
            &<$k::ie::$ie_name as $crate::protocol::Ie>::value(($($ie_value)+)),
        );
        $crate::__build_ies!($k, $bindings, $message, $msg, $($($rest)*)?);
    };
    ($k:tt, $bindings:ident, $message:ident, $msg:ident, $($other:tt)*) => {
        $crate::__ie_syntax!($($other)*)
    };
}

/// Why the IEs of a call of the macro that builds a PDU are not written as they are read:
/// the one that is not, by its name.
#[doc(hidden)]
#[macro_export]
macro_rules! __ie_syntax {
    // An IE that is written so: the fault is in one that follows.
    ($criticality:ident $name:ident ($($value:tt)*) $(, $($rest:tt)*)?) => {
        $crate::__ie_syntax!($($($rest)*)?)
    };
    ($name:ident ($($value:tt)*) $(, $($rest:tt)*)?) => {
        $crate::__ie_syntax!($($($rest)*)?)
    };
    ($($other:tt)*) => {
        compile_error!(
            "each IE is written `CRITICALITY Name(value)`, with REJECT, IGNORE or NOTIFY \
             as its criticality, or `Name(value)` for the one that its message assigns it, \
             and a comma after it"
        )
    };
}

/// The criticality that the macros take as `REJECT`, `IGNORE` or `NOTIFY`.
#[doc(hidden)]
#[macro_export]
macro_rules! __criticality {
    ($k:tt, $bindings:ident, REJECT) => {
        $k::$bindings::Criticality::reject
    };
    ($k:tt, $bindings:ident, IGNORE) => {
        $k::$bindings::Criticality::ignore
    };
    ($k:tt, $bindings:ident, NOTIFY) => {
        $k::$bindings::Criticality::notify
    };
    ($k:tt, $bindings:ident, $other:tt) => {
        compile_error!(concat!(
            "`",
            stringify!($other),
            "` is no criticality: write REJECT, IGNORE or NOTIFY",
        ))
    };
}

/// `build_ngap_ie!` and `build_s1ap_ie!`: one IE, of a message or of a transfer whose name
/// comes first, or in no message.
#[doc(hidden)]
#[macro_export]
macro_rules! __build_ie {
    ($k:tt, $bindings:ident, $name:literal, $msg:ident,
        $criticality:ident $ie_name:ident ($($value:tt)+)) => {{
        // The name is the type of a message that has protocol IEs.
        $crate::protocol::__message_of_protocol_ies::<$k::$bindings::$msg<'_>>();
        $crate::__build_ie!($k, $bindings, $name, $criticality $ie_name ($($value)+))
    }};
    // The criticality that the object set of the message assigns to the IE.
    ($k:tt, $bindings:ident, $name:literal, $msg:ident, $ie_name:ident ($($value:tt)+)) => {{
        $crate::protocol::__message_of_protocol_ies::<$k::$bindings::$msg<'_>>();
        $crate::protocol::__assigned_ie::<$k::$bindings::$msg<'_>, _>(
            <$k::ie::$ie_name as $crate::protocol::Ie>::ID,
            stringify!($ie_name),
            &<$k::ie::$ie_name as $crate::protocol::Ie>::value(($($value)+)),
        )
    }};
    // An IE that is in no message: an item of a list of single containers.
    ($k:tt, $bindings:ident, $name:literal, $criticality:ident $ie_name:ident ($($value:tt)+)) => {
        $crate::protocol::__ie::<<$k::ie::$ie_name as $crate::protocol::Ie>::Protocol, _>(
            <$k::ie::$ie_name as $crate::protocol::Ie>::ID,
            $crate::__criticality!($k, $bindings, $criticality),
            &<$k::ie::$ie_name as $crate::protocol::Ie>::value(($($value)+)),
        )
    };
    // What is not written so, which says what is wrong.
    ($k:tt, $bindings:ident, $name:literal, $ie_name:ident ($($value:tt)+)) => {
        compile_error!(concat!(
            "the criticality of the IE ",
            stringify!($ie_name),
            " is missing: an IE that goes in no message has it written, as REJECT, IGNORE \
             or NOTIFY before its name",
        ))
    };
    ($k:tt, $bindings:ident, $name:literal, $($other:tt)*) => {
        compile_error!(concat!(
            $name,
            " takes the message, then the IE as `CRITICALITY Name(value)`, or the IE alone \
             for one that is in no message"
        ))
    };
}
