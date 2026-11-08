//! The macros of the inspection of the protocol crates, written once: the tables of their
//! trees, and the functions that read them.

/// The shape that the list of the types of a protocol writes.
#[doc(hidden)]
#[macro_export]
macro_rules! __shape {
    (_) => {
        $crate::inspect::Shape::Plain
    };
    (null) => {
        $crate::inspect::Shape::Null
    };
    (bits) => {
        $crate::inspect::Shape::Bits
    };
    (ies) => {
        $crate::inspect::Shape::Ies(None)
    };
    (ie) => {
        $crate::inspect::Shape::Ie(None)
    };
    (extensions) => {
        $crate::inspect::Shape::Extensions("")
    };
    (extension) => {
        $crate::inspect::Shape::Extension(None, "")
    };
    ([$of:tt]) => {
        $crate::inspect::Shape::List(&$crate::__shape!($of))
    };
    (()) => {
        $crate::inspect::Shape::Transfer("")
    };
    (($contained:ident)) => {
        $crate::inspect::Shape::Transfer(stringify!($contained))
    };
    ($name:ident) => {
        $crate::inspect::Shape::Named(stringify!($name))
    };
}

/// Whether the list of the types writes a member as one that may be absent.
#[doc(hidden)]
#[macro_export]
macro_rules! __optional {
    (:) => {
        false
    };
    (?) => {
        true
    };
}

/// What the list of the types says a type is.
#[doc(hidden)]
#[macro_export]
macro_rules! __kind {
    (= $is:tt) => {
        $crate::inspect::Kind::Is($crate::__shape!($is))
    };
    (sequence { $($member:literal $presence:tt $shape:tt),* }) => {
        $crate::inspect::Kind::Sequence(&[$((
            $member,
            $crate::__shape!($shape),
            $crate::__optional!($presence),
        )),*])
    };
    (choice { $($member:literal $presence:tt $shape:tt),* }) => {
        $crate::inspect::Kind::Choice(&[$((
            $member,
            $crate::__shape!($shape),
            $crate::__optional!($presence),
        )),*])
    };
}

/// The types of a protocol, which its `src/inspect_registry.rs` lists from the bindings in
/// the order of their names: each SEQUENCE with its members and each CHOICE with its
/// alternatives, by the names that ASN.1 gives them, and each type that is a list or a
/// bit string; then each ENUMERATED with the names of its values. A member is written
/// with `?` when a value may be without it, and its value as `_` when it has no members,
/// `bits` for a bit string whose size varies, the name of its type, a list in brackets,
/// a container by its kind, or in parentheses the type that the octets of a transfer
/// contain.
///
/// The module `names` has the names of each SEQUENCE, CHOICE and ENUMERATED, by the
/// name of its type: those that its tree writes, and that the paths read in `TYPES` and
/// `ENUMERATIONS`.
#[doc(hidden)]
#[macro_export]
macro_rules! __types {
    (
        types { $($name:ident $kind:tt $body:tt;)* }
        enumerations { $($enumerated:ident [$($value:literal),*];)* }
    ) => {
        pub(crate) const TYPES: &[(&str, $crate::inspect::Kind)] = &[
            $((stringify!($name), $crate::__kind!($kind $body)),)*
        ];
        pub(crate) const ENUMERATIONS: &[(&str, &[&str])] = &[
            $((stringify!($enumerated), names::$enumerated),)*
        ];
        #[allow(non_upper_case_globals)]
        pub(crate) mod names {
            $($crate::__names!($name $kind $body);)*
            $(pub(crate) const $enumerated: &[&str] = &[$($value),*];)*
        }
    };
}

/// The names of the members of a SEQUENCE or the alternatives of a CHOICE of
/// [`__types!`], as a constant named after its type.
#[doc(hidden)]
#[macro_export]
macro_rules! __names {
    ($name:ident = $is:tt) => {};
    ($name:ident $kind:ident { $($member:literal $presence:tt $shape:tt),* }) => {
        pub(crate) const $name: &[&str] = &[$($member),*];
    };
}

/// The functions of a type of the bindings of a protocol for its trees,
/// which is written with the lifetime `'a` when it borrows, or without one in a function.
#[doc(hidden)]
#[macro_export]
// A protocol calls it in its crate, whose `inspect` module `crate::` names.
#[allow(clippy::crate_in_macro_def)]
macro_rules! __typed {
    ($type:ty) => {{
        // A type that borrows nothing leaves the lifetime without a use.
        #[allow(clippy::needless_lifetimes)]
        fn decode<'a>(raw: &'a [u8]) -> Result<$crate::inspect::Decoded, $crate::Error> {
            crate::inspect::__decode_typed::<$type>(raw)
        }
        #[allow(clippy::needless_lifetimes)]
        fn encode<'a>(value: &'a $crate::json::Value) -> Result<Vec<u8>, $crate::Error> {
            crate::inspect::__encode_typed::<$type>(value)
        }
        $crate::inspect::Typed { decode, encode }
    }};
}

/// The transfers of a protocol: the member that holds each, and the type that its octets
/// contain. A member that has several types has its name alone.
#[doc(hidden)]
#[macro_export]
macro_rules! __transfers {
    ($($field:literal $($transfer:path)?;)*) => {
        pub(crate) const TRANSFER_FIELDS: &[&str] = &[$($field,)*];
        #[allow(clippy::match_single_binding)]
        pub(crate) fn transfer(field: &str) -> Result<$crate::inspect::Typed, $crate::Error> {
            match field {
                $($($field => Ok($crate::__typed!($transfer)),)?)*
                _ => Err($crate::Error::new($crate::ErrorKind::Value, format!(
                    "{PROTOCOL} has several types for {field}, or none: it is read and \
                     written as its octets"
                ))),
            }
        }
    };
}

/// The IEs of a protocol whose octets may carry a type that the specification names and
/// the ASN.1 does not: the identifier of each, the name that ASN.1 gives the type, and the
/// type; then, after `of`, those whose octets may carry a type of another protocol, with
/// the name of that protocol.
#[doc(hidden)]
#[macro_export]
macro_rules! __carried {
    ($($id:literal $name:literal $carried:path;)* $(of $protocol:literal $fid:literal $fname:literal;)*) => {
        pub(crate) const CARRIED: &[(u16, &str, &str)] =
            &[$(($id, $name, stringify!($carried)),)*];
        pub(crate) const FOREIGN: &[(u16, &str, &str)] = &[$(($fid, $fname, $protocol),)*];
        pub(crate) fn carried(name: &str) -> Option<$crate::inspect::Typed> {
            $(if $crate::inspect::same_name($name, name) {
                return Some($crate::__typed!($carried));
            })*
            None
        }
    };
}

/// The tables of a protocol for its trees and their paths, and the functions of
/// its module `inspect` that read them, written once: the protocol calls it in that module,
/// with the module of its bindings, the type of its PDU, the cfg of its hooks, and a
/// message that is a container of protocol IEs and nothing else.
#[doc(hidden)]
#[macro_export]
// A protocol calls it in its crate, whose items `crate::` names.
#[allow(clippy::crate_in_macro_def)]
macro_rules! __inspect {
    (bindings $bindings:ident, pdu $pdu:ident, hooks $hooks:meta, any $any:ty) => {
use std::collections::HashMap;
use std::sync::LazyLock;

use $crate::inspect::{self as shared, Protocol};
pub(crate) use $crate::inspect::{Kind, Shape, same_name};
use $crate::json::{self, Tree};
use $crate::{AsnType, Decode, Encode};

use crate::Error;
// The name of the protocol, the first segment of the paths of the IEs of the message of
// a PDU, and what the ASN.1 says of the messages, the IEs and the types.
use crate::inspect_registry::{
    CARRIED, ENUMERATIONS, FOREIGN, IE_NAMES, IE_TYPES, ITEM_CRITICALITIES, MESSAGES, PROTOCOL,
    ROOT, TRANSFER_FIELDS, TYPES,
};
// The type that an IE of an extension container has where its identifier alone does not
// say it: the one that the type which the container extends gives it.
use crate::registry::EXTENSION_TYPES;
// The transfers of protocol IEs, with their object sets.
use crate::registry::TRANSFERS;
/// A tree: the JSON value of `oxirush-utils`.
pub use $crate::json::Value;

/// The tables of the protocol, as the paths read them.
pub(crate) static TABLES: Protocol = Protocol {
    name: PROTOCOL,
    root: ROOT,
    types: TYPES,
    messages: MESSAGES,
    ie_names: IE_NAMES,
    ie_types: IE_TYPES,
    extension_types: EXTENSION_TYPES,
    carried: CARRIED,
    foreign: FOREIGN,
    enumerations: ENUMERATIONS,
    item_criticalities: ITEM_CRITICALITIES,
    transfers: TRANSFERS,
    ie_id,
    kind_of,
    ie_typed: crate::inspect_registry::ie,
    extension_typed: crate::registry::extension,
    ie_contents: crate::inspect_registry::ie_contents,
    type_form: crate::inspect_registry::type_form,
    transfer_fields: TRANSFER_FIELDS,
    transfer_typed: crate::inspect_registry::transfer,
    carried_typed: crate::inspect_registry::carried,
    any_message,
};

/// The letters and the digits of a name, in lower case.
fn letters(name: &str) -> impl Iterator<Item = u8> + '_ {
    let letters = name.bytes().filter(u8::is_ascii_alphanumeric);
    letters.map(|letter| letter.to_ascii_lowercase())
}

/// The number of the IE that ASN.1 names `id-<name>`, however the name is written.
pub(crate) fn ie_id(name: &str) -> Option<u16> {
    /// The identifier of each IE by the letters and the digits of its name, in lower
    /// case, which is how two names are the same: the first IE of a name, as a search
    /// of the names in their order finds it.
    static IDS: LazyLock<HashMap<Vec<u8>, u16>> = LazyLock::new(|| {
        let mut ids = HashMap::new();
        for (id, name) in IE_NAMES {
            ids.entry(letters(name).collect()).or_insert(*id);
        }
        ids
    });
    // The letters of the name, on the stack for a name of the length that an IE has:
    // the longest has 51 of them.
    let mut short = [0; 64];
    let mut length = 0;
    for letter in letters(name) {
        let Some(place) = short.get_mut(length) else {
            let long: Vec<u8> = letters(name).collect();
            return IDS.get(&long).copied();
        };
        *place = letter;
        length += 1;
    }
    IDS.get(&short[..length]).copied()
}

/// What a type of the list of the types is; `None` for a name that the list does not
/// have, which is that of a value without members.
fn kind_of(name: &str) -> Option<&'static Kind> {
    static KINDS: LazyLock<HashMap<&'static str, &'static Kind>> =
        LazyLock::new(|| TYPES.iter().map(|(name, kind)| (*name, kind)).collect());
    KINDS.get(name).copied()
}

/// The values at `path` in `tree`, the tree of a PDU, in the order of the tree. See
/// [Paths](self#paths).
///
/// An IE that the message does not have selects nothing, and so does a member that its
/// type can have and the value does not have: an OPTIONAL member that is absent, or
/// another alternative of a CHOICE. A name that the type of the value cannot have is an
/// error, which says the members that it has. `*` is each entry of a list or each member
/// of a value; after it, a segment is an error when none of the values can have it.
///
/// After an IE, its name selects its value again: `0/Cause` is the first IE when it is a
/// Cause. A value that is its `octets`, as one that did not decode, has nothing under it.
pub fn select<'a>(tree: &'a Value, path: &str) -> Result<Vec<&'a Value>, Error> {
    shared::select(&TABLES, tree, path)
}

/// Whether `path` can select a value in a message that ASN.1 names `message`: what
/// [`select`] says of a path without a tree to select in.
///
/// The name is taken as [`message_named`](crate::inspect::message_named) takes it. Under
/// the root, the name of an IE has to be that of an IE of the object set of the message;
/// under an IE, each segment has to be a member that the type of the value has, through
/// the lists, the alternatives of a CHOICE and the transfers. A position or `*` stands for
/// each entry that a list could have, and `-` for the end of a list.
///
/// What is not known is not refused: the value of an IE selected by `@id=N` when the
/// identifier is unknown or has no type where it is, a segment under it, and whether a
/// position is within the bounds of its list.
pub fn check_path(message: &str, path: &str) -> Result<(), Error> {
    shared::check_path(&TABLES, message, path)
}

/// The names that the value at `path` can have in a message that ASN.1 names `message`,
/// when its type is an ENUMERATED: the names of its values as the specification spells
/// them, in the order of its definition. A name is compared as [`select`] compares one,
/// whatever its case and whatever is between its letters. A name that is not one of them
/// is never the value of a message, so a caller can refuse it before it compares one.
///
/// `None` for a value of another type, and for one whose type is not known: what
/// [`check_path`] does not refuse for that reason. A path that it refuses is an error
/// here too.
pub fn enumerated_at(message: &str, path: &str) -> Result<Option<&'static [&'static str]>, Error> {
    shared::enumerated_at(&TABLES, message, path)
}

/// Each value of `tree`, the tree of a PDU, with the path that selects it, in the order
/// of the tree. A list of plain values is one value. The IEs of the message go by name
/// under the root, and those of a container by name in its place; an IE goes by its
/// position when its name is there twice.
///
/// What is nested deeper than 64 levels, the most segments that a path has, is one
/// value at the deepest path, whatever it holds: every path that is listed is one that
/// [`select`] takes.
pub fn paths(tree: &Value) -> Vec<(String, Value)> {
    shared::paths(&TABLES, tree)
}

/// Give the value at `path` in `tree` the value `value`. `null` takes an optional member
/// out, and an alternative of a CHOICE takes the place of the one that is there. A value
/// is written as a tree writes it, and the value of an IE may be its `octets` in
/// hexadecimal: `{"octets": "…"}`.
///
/// A path that selects nothing is an error, and the tree is then as it was: an IE that
/// the message does not have is inserted at `/<root>/-`, as `{"<IE>": value}`. So is a
/// name that the type of a value cannot have, and a member that its type always has and
/// that would be taken out.
pub fn set(tree: &mut Value, path: &str, value: Value) -> Result<(), Error> {
    shared::set(&TABLES, tree, path, value)
}

/// Take the values at `path` out of `tree`: an IE with its criticality, a member or the
/// entries of a list. A path that selects nothing is an error, and so is a member that
/// its type always has.
pub fn remove(tree: &mut Value, path: &str) -> Result<(), Error> {
    shared::remove(&TABLES, tree, path)
}

/// Add `value` to a list of `tree`: before each entry that `path` selects, or at the end
/// for a path that ends with `-`. An IE is written as the tree writes it, `{"<IE>":
/// value}`, with its `"criticality"` where it is not the one that the tables assign.
pub fn insert(tree: &mut Value, path: &str, value: Value) -> Result<(), Error> {
    shared::insert(&TABLES, tree, path, value)
}

/// The entry shape of a list of `shape`, which the tests of the tables read.
#[cfg(test)]
fn entry_shape(shape: Shape) -> Shape {
    TABLES.entry_shape(shape)
}

/// The IEs of the message that ASN.1 names `message`, which a view has as `ies`, as a
/// tree has them, which the tests of the tables read.
#[cfg(test)]
fn named_ies(message: &str, ies: Value) -> Result<Value, Error> {
    shared::named_ies(&TABLES, message, ies)
}

/// The entries that a view has for the IEs of the message that ASN.1 names `message`,
/// which `ies` has as a tree has them, which the tests of the tables read.
#[cfg(test)]
fn listed_ies(message: &str, ies: &Value) -> Result<Value, Error> {
    shared::listed_ies(&TABLES, message, ies)
}

#[cfg(test)]
mod tests {
    use super::*;
    use $crate::inspect::{Member, type_name};
    use oxirush_utils::tree;

    /// The member of a message or of a container that holds its IEs.
    const IES: &str = "protocolIEs";

    /// Every member and every alternative of every type of the list.
    fn all_members() -> impl Iterator<Item = (&'static str, &'static [Member])> {
        TYPES.iter().filter_map(|(name, kind)| match kind {
            Kind::Sequence(members) | Kind::Choice(members) => Some((*name, *members)),
            Kind::Is(_) => None,
        })
    }

    /// The member of `members` that a segment names.
    fn member<'m>(members: &'m [Member], part: &str) -> Option<&'m Member> {
        let exact = members.iter().find(|(name, ..)| *name == part);
        exact.or_else(|| members.iter().find(|(name, ..)| same_name(name, part)))
    }

    #[test]
    fn the_types_are_listed_in_the_order_of_their_names_and_name_types_of_the_list() {
        assert!(TYPES.windows(2).all(|pair| pair[0].0 < pair[1].0));
        fn check(of: &str, shape: Shape) {
            match shape {
                // A name that the list does not have is that of an ENUMERATED, which
                // has no members and has its names, or of a well-known type, which has
                // its readable form.
                Shape::Named(name) => assert!(
                    kind_of(name).is_some()
                        || TABLES.enumeration(name).is_some()
                        || (TABLES.type_form)(name).is_some(),
                    "{of}: {name}"
                ),
                Shape::Transfer(name) if !name.is_empty() => {
                    assert!(kind_of(name).is_some(), "{of}: {name}")
                }
                Shape::List(inner) => check(of, *inner),
                _ => {}
            }
        }
        for (name, kind) in TYPES {
            match kind {
                Kind::Is(shape) => check(name, *shape),
                Kind::Sequence(members) | Kind::Choice(members) => {
                    members.iter().for_each(|(_, shape, _)| check(name, *shape))
                }
            }
        }
        // The types of the messages and of the IEs are types of the bindings.
        for (.., ty) in MESSAGES {
            assert!(kind_of(type_name(ty)).is_some(), "{ty}");
        }
        assert!(
            IE_TYPES
                .iter()
                .any(|(_, ty, _)| kind_of(type_name(ty)).is_some())
        );
        // The identifiers of the IEs are in their order, which their names are read in.
        assert!(IE_NAMES.windows(2).all(|pair| pair[0].0 < pair[1].0));
    }

    #[test]
    fn the_enumerations_are_listed_in_the_order_of_their_names() {
        assert!(ENUMERATIONS.windows(2).all(|pair| pair[0].0 < pair[1].0));
    }

    #[test]
    fn no_two_names_that_a_path_takes_are_the_same() {
        let distinct = |of: &str, names: &mut dyn Iterator<Item = &str>| {
            let names: Vec<_> = names.collect();
            for (at, name) in names.iter().enumerate() {
                let other = names[..at].iter().find(|other| same_name(other, name));
                assert!(other.is_none(), "{of}: {name} and {other:?}");
            }
        };
        for (name, members) in all_members() {
            distinct(name, &mut members.iter().map(|(member, ..)| *member));
        }
        distinct("the IEs", &mut IE_NAMES.iter().map(|(_, name)| *name));
        let mut messages = MESSAGES.iter().map(|(set, ..)| set.name);
        distinct("the messages", &mut messages);
        // The names of an IE and of a member do not meet: a message that a container
        // holds has its IEs alone.
        for (name, members) in all_members() {
            if members.iter().any(|(member, ..)| *member == IES) {
                assert_eq!(members.len(), 1, "{name}");
            }
        }
        // A name is read whatever its case and its separators, and a name of an IE
        // finds its identifier.
        assert!(same_name("AMF-UE-NGAP-ID", "amf_ue ngap.id"));
        assert!(!same_name("-", "_") && !same_name("", ""));
        for (id, name) in IE_NAMES {
            assert_eq!(ie_id(&name.to_lowercase()), Some(*id), "{name}");
        }
    }

    #[test]
    fn no_two_names_of_an_enumerated_are_the_same() {
        // A name is read whatever its case and its separators: no two values of one
        // type are then one name.
        for (ty, names) in ENUMERATIONS {
            for (at, name) in names.iter().enumerate() {
                let other = names[..at].iter().find(|other| same_name(other, name));
                assert!(other.is_none(), "{ty}: {name} and {other:?}");
            }
        }
    }

    #[test]
    fn an_extension_has_the_type_that_the_type_it_extends_gives_it() {
        // An extension container extends the type that has it, whatever type that is.
        let mut containers = 0;
        for (name, _) in all_members() {
            let members = TABLES.members(Shape::Named(name), None).unwrap_or_default();
            for (_, shape, _) in members {
                if let Shape::Extensions(extended) = shape {
                    assert_eq!(extended, name);
                    assert_eq!(entry_shape(shape), Shape::Extension(None, name));
                    containers += 1;
                }
            }
        }
        assert!(containers > 0);
        // An IE whose identifier has a type for each type that it extends is of that type
        // there, and of none elsewhere: its identifier alone does not say its type.
        for (id, extended, ty) in EXTENSION_TYPES {
            assert!(kind_of(extended).is_some(), "{extended}");
            assert!(IE_TYPES.iter().all(|(known, ..)| known != id), "{id}");
            assert_eq!(
                TABLES.ie_value(Some(*id), extended),
                Shape::Named(type_name(ty))
            );
            assert_eq!(TABLES.ie_value(Some(*id), ""), Shape::Unknown);
            let extension = Shape::Extension(Some(*id), extended);
            let value = TABLES.members(extension, None).unwrap();
            let (_, value, _) = member(&value, "extensionValue").unwrap();
            assert_eq!(*value, Shape::Named(type_name(ty)), "{id} in {extended}");
        }
    }

    #[test]
    fn the_ies_that_a_value_holds_are_written_by_name_and_read_back() {
        let cause = ie_id("Cause").expect("an IE of the protocol");
        let value = tree!({"radioNetwork": "unspecified"});
        // The entry of a view, with what it keeps of what was received, and as it is
        // written.
        let entry = |received: bool| {
            let mut entry = tree!({"id": cause, "criticality": "ignore", "value": &value});
            if received {
                entry["_raw_value"] = tree!("0000");
            }
            entry
        };
        // No table has the criticality of the IEs of a message that is not one of the
        // protocol, as that of a transfer: it is written, and asked for.
        let shown = named_ies("no message", tree!([entry(true)])).unwrap();
        assert_eq!(shown, tree!([{"Cause": &value, "criticality": "ignore"}]));
        let listed = listed_ies("no message", &shown).unwrap();
        assert_eq!(listed, tree!([entry(false)]));
        let refused = listed_ies("no message", &tree!([{"Cause": &value}])).unwrap_err();
        assert_eq!(refused.kind(), crate::ErrorKind::Value);
        let text = refused.to_string();
        assert!(text.contains("says its \"criticality\""), "{refused}");
        // One that did not decode has its octets, which are sent as they are.
        let mut undecoded = entry(true);
        undecoded["value"] = tree!("0000");
        undecoded["_decode_error"] = tree!("why");
        let shown = named_ies("no message", tree!([undecoded])).unwrap();
        let octets = tree!({"Cause": {"octets": "0000"}, "criticality": "ignore"});
        assert_eq!(shown, tree!([octets]));
        let listed = listed_ies("no message", &shown).unwrap();
        let octets = tree!({"id": cause, "criticality": "ignore", "octets": "0000"});
        assert_eq!(listed, tree!([octets]));
        // A tree is not read deeper than a view is encoded.
        let mut deep = tree!(1);
        for _ in 0..258 {
            deep = tree!([deep]);
        }
        let deep = tree!([{"Cause": deep, "criticality": "ignore"}]);
        let refused = listed_ies("no message", &deep).unwrap_err();
        assert_eq!(refused.kind(), crate::ErrorKind::Value);
        assert!(refused.to_string().contains("nests deeper"), "{refused}");
    }

    #[test]
    fn the_octets_of_an_ie_carry_a_type_of_the_list_by_a_name_that_no_member_has() {
        for (id, name, ty) in CARRIED {
            // The value of the IE is octets, and what they carry a type of the list, by
            // the name that ASN.1 gives it.
            let (_, octets, contained) = IE_TYPES.iter().find(|(known, ..)| known == id).unwrap();
            assert!(contained.is_empty() && kind_of(type_name(octets)).is_none());
            assert!(kind_of(type_name(ty)).is_some() && same_name(name, type_name(ty)));
            assert_eq!(TABLES.ie_value(Some(*id), ""), Shape::Carrier(*id));
            // A view is encoded with any member of that name as that type.
            for (of, members) in all_members() {
                let same = members.iter().any(|(member, ..)| same_name(member, name));
                assert!(!same, "{of}: {name}");
            }
        }
    }

    #[test]
    fn what_is_nested_deeper_than_a_path_goes_is_one_value() {
        // Lists in lists, ten levels deeper than a path goes: the path of the one value
        // has a segment for each level that is walked, and the value is what is under
        // them, whole.
        const SEGMENTS: usize = 64;
        let mut deep = tree!([{"a": 1}]);
        for _ in 0..SEGMENTS + 10 {
            deep = tree!([deep]);
        }
        let found = paths(&deep);
        assert_eq!(found.len(), 1);
        let (path, value) = &found[0];
        assert_eq!(path.matches('/').count(), SEGMENTS);
        assert_eq!(select(&deep, path), Ok(vec![value]));
        // What is not that deep is listed value by value.
        let mut shallow = tree!([{"a": 1}]);
        for _ in 0..SEGMENTS - 10 {
            shallow = tree!([shallow]);
        }
        let found = paths(&shallow);
        assert_eq!(found.len(), 1);
        assert!(found[0].0.ends_with("/0/a") && found[0].1 == tree!(1));
    }
}

/// The functions of the message of a procedure that the specification does not have in
/// that direction, which is read and written as what every message of the protocol but
/// the private one is: a container of protocol IEs and nothing else.
pub(crate) fn any_message() -> Typed {
    $crate::__typed!($any)
}

/// The value of the type `T` that the octets `raw` are, in its JSON form, and whether it
/// encodes back to them: it does not when they hold what a later version of the type
/// adds, which the value has no place for, or are in another form than the canonical
/// one. `name` is the name of the type in the bindings.
pub(crate) fn __decode_typed<'a, T>(raw: &'a [u8]) -> Result<$crate::inspect::Decoded, Error>
where
    T: Decode<'a> + Encode + Tree,
{
    let read: Result<T, Error> = $crate::decode(raw);
    read.map(|value| {
        let exact = $crate::encode(&value).is_ok_and(|octets| octets == raw);
        // Octets that do not come back are read again by the reader of the one encoding,
        // which says what of them is in another form.
        let refused = match exact {
            true => None,
            false => $crate::decode_strict::<T>(raw).err(),
        };
        (value.to_tree(), exact, refused)
    })
}

/// The octets of the value of the type `T` that `value` writes in its JSON form.
pub(crate) fn __encode_typed<T>(value: &Value) -> Result<Vec<u8>, Error>
where
    T: Encode + Tree + AsnType,
{
    let typed: T = json::from_value(value)?;
    // A value that is read and does not encode is outside a constraint of its type.
    $crate::encode(&typed).map_err(|error| error.outside_its_type(<T as AsnType>::NAME))
}

/// How a tree shows the values of a well-known type.
pub(crate) use $crate::inspect::Form;

/// The functions of a type of the registry.
pub(crate) use $crate::inspect::Typed;

/// Decode, in `tree`, the octets that `path` takes as a type that they may carry: those
/// of an IE whose content the specification gives by reference, as the transparent
/// container of a handover, where the segment after the IE is the name that ASN.1 gives
/// one of these types. See [Octets that carry a type](self#octets-that-carry-a-type).
///
/// The IE then has, in place of its octets, that type under that name, which [`select`],
/// [`paths`] and the edits go into and `from_tree` of the PDU encodes back. Nothing in a
/// message says which type octets carry, so `to_tree` leaves them as they are, and a path
/// through the name of a type on octets that were not opened is an error that says so.
///
/// A path that goes through no such name changes nothing. A name that the type of a
/// value cannot have is an error, and the tree is then as it was.
pub fn open(tree: &mut Value, path: &str) -> Result<(), Error> {
    shared::open(&TABLES, tree, path)
}

/// Register the tables of this protocol, so that the trees and the paths of the other
/// protocol of the family read and write the handover containers of this one
/// that the messages of that protocol carry: those of a handover between 5GS and EPS. A
/// program that has them calls it once; a call after the first changes nothing.
///
/// Without it, such a container is written and read as its octets alone, and its name
/// is an error that says to call this function.
pub fn register() {
    shared::register(&TABLES);
}

/// The tree of a view whose values another implementation wrote in the JSON Encoding
/// Rules of ITU-T X.697, as pycrate writes them: `from_tree` of the PDU reads it. The tests
/// read the values of their reference with it.
#[doc(hidden)]
pub fn __tree_of_jer(view: &Value) -> Result<Value, Error> {
    shared::tree_of_jer(&TABLES, view)
}
    };
}

/// The module `hooks` of a protocol crate, which its fuzz targets and the tests of its
/// oracle take from the inside of the crate: the protocol calls it at the root of the
/// crate, with the configuration name that it has the module with, the module of its
/// bindings and the types of its PDU.
#[doc(hidden)]
#[macro_export]
// A protocol calls it in its crate, whose items `crate::` names.
#[allow(clippy::crate_in_macro_def)]
macro_rules! __hooks {
    (cfg $cfg:ident, bindings $bindings:ident, pdu $pdu:ident) => {
        #[cfg($cfg)]
        #[doc(hidden)]
        #[doc = concat!(
                    "What the fuzz targets of `fuzz/` take from the inside of the crate. It is \
     compiled with `--cfg ", stringify!($cfg), "`, which that package is built with, and \
     is no part of the crate without: the fuzz build of another program, for which \
     cargo-fuzz sets `fuzzing`, has the crate as every other build has it.\n\n\
     It gives them what needs the type of a message, which no function gives from a PDU, \
     and the two ways in which the crate writes a message."
                )]
        pub mod hooks {
            use crate::Error;
            use crate::$bindings::$pdu;

            /// The bindings, by a name that is the same for both protocols.
            pub use crate::$bindings as bindings;
            /// A PDU that owns its octets, by a name that is the same for both protocols.
            pub type Pdu = crate::$bindings::$pdu<'static>;
            /// A PDU that borrows its message from the octets that it is decoded from.
            pub use crate::$bindings::$pdu as BorrowedPdu;

            /// The name of the type of a PDU in the bindings.
            pub const PDU_TYPE: &str = stringify!($pdu);

            /// The root of the paths of a tree, which the module of the bindings names.
            pub const ROOT: &str = concat!("/", stringify!($bindings));

            /// The octets of the message of `pdu`.
            pub fn message_octets<'p>(pdu: &'p $pdu<'_>) -> &'p [u8] {
                $crate::protocol::Pdu::value(pdu)
            }

            /// The message of `pdu` as its type: the name of the type in the bindings, and the
            /// octets that the message encodes to once `decode_value` has read it, or why it does
            /// not decode. `None` when the protocol has no message for the PDU.
            pub fn message_read_and_written(
                pdu: &$pdu<'_>,
            ) -> Option<(&'static str, Result<Vec<u8>, Error>)> {
                let name =
                    crate::registry::message_type_name(pdu.direction(), pdu.procedure_code())?;
                Some((name, crate::registry::message_read_and_written(pdu)?))
            }
        }
    };
}
