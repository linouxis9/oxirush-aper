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

//! What the tables that the generator writes for a protocol hold, checked by the tests of
//! each protocol crate: `__registry_tests!` and `__inspect_tests!` write a test for each
//! check. Each check panics where the tables break what it says. The feature `__checks`,
//! which the protocol crates take for their tests alone, builds them: no part of the API.

use crate::Direction;
use crate::protocol::{Protocol, name, object_set};

/// The IEs are listed in the order of their identifiers, by which the name of an IE is
/// found with a binary search.
pub fn the_ies_are_listed_in_the_order_of_their_identifiers<P: Protocol>() {
    assert!(P::__IE_NAMES.windows(2).all(|pair| pair[0].0 < pair[1].0));
    for (id, known) in P::__IE_NAMES {
        assert_eq!(name::<P>(*id), Some(*known));
    }
    assert_eq!(name::<P>(u16::MAX), None);
}

/// Each message but the private one has an object set, which its direction and its
/// procedure code find.
pub fn each_message_but_the_private_one_has_an_object_set<P: Protocol>() {
    let without = P::__OBJECT_SETS.iter().filter(|set| set.ies.is_none());
    assert!(without.map(|set| set.name).eq(["PrivateMessage"]));
    for message in P::__OBJECT_SETS {
        let found = object_set::<P>(message.direction, message.procedure_code);
        assert_eq!(found.map(|found| found.name), Some(message.name));
    }
    assert!(object_set::<P>(Direction::Initiating, 255).is_none());
}

/// Each IE of an object set has a name, and is there once.
pub fn every_ie_of_an_object_set_has_a_name_and_is_there_once<P: Protocol>() {
    for message in P::__OBJECT_SETS {
        for (index, (id, ..)) in message.ies.into_iter().flatten().enumerate() {
            assert!(name::<P>(*id).is_some(), "IE {id} of {}", message.name);
            let before = &message.ies.unwrap_or_default()[..index];
            let again = before.iter().any(|(known, ..)| known == id);
            assert!(!again, "IE {id} is twice in the set of {}", message.name);
        }
    }
}

#[cfg(feature = "inspect")]
pub use self::inspect::*;

/// The checks of the tables of the trees and of the paths, which `Protocol` of
/// `inspect` holds.
#[cfg(feature = "inspect")]
mod inspect {
    use alloc::format;
    use alloc::string::{String, ToString};
    use alloc::vec::Vec;

    use crate::inspect::{
        Kind, Member, Protocol, Shape, listed_ies, named_ies, paths, same_name, schema, select,
        type_name,
    };
    use oxirush_utils::tree;

    /// The member of a message or of a container that holds its IEs.
    const IES: &str = "protocolIEs";

    /// Each type that `kind_of` knows, with its name in the bindings.
    fn named_kinds(p: &'static Protocol) -> impl Iterator<Item = (&'static str, &'static Kind)> {
        let kinds = p.schema.iter().map(|ty| ty.rust);
        kinds.filter_map(|name| Some((name, (p.kind_of)(name)?)))
    }

    /// Every member and every alternative of every type.
    fn all_members(
        p: &'static Protocol,
    ) -> impl Iterator<Item = (&'static str, &'static [Member])> {
        named_kinds(p).filter_map(|(name, kind)| match kind {
            Kind::Sequence(members) | Kind::Choice(members) => Some((name, &members[..])),
            Kind::Is(_) => None,
        })
    }

    /// The member of `members` that a segment names.
    fn member<'m>(members: &'m [Member], part: &str) -> Option<&'m Member> {
        let exact = members.iter().find(|(name, ..)| *name == part);
        exact.or_else(|| members.iter().find(|(name, ..)| same_name(name, part)))
    }

    /// The types name types that have members, names or a readable form.
    pub fn the_types_name_types_that_have_members_names_or_a_form(p: &'static Protocol) {
        let check = |of: &str, shape: Shape| {
            let mut shape = shape;
            while let Shape::List(inner) = shape {
                shape = *inner;
            }
            match shape {
                // A name that the list does not have is that of an ENUMERATED, which
                // has no members and has its names, or of a well-known type, which has
                // its readable form.
                Shape::Named(name) => assert!(
                    (p.kind_of)(name).is_some()
                        || p.enumeration(name).is_some()
                        || (p.type_form)(name).is_some(),
                    "{of}: {name}"
                ),
                Shape::Transfer(name) if !name.is_empty() => {
                    assert!((p.kind_of)(name).is_some(), "{of}: {name}")
                }
                _ => {}
            }
        };
        for (name, kind) in named_kinds(p) {
            match kind {
                Kind::Is(shape) => check(name, *shape),
                Kind::Sequence(members) | Kind::Choice(members) => {
                    members.iter().for_each(|(_, shape, _)| check(name, *shape))
                }
            }
        }
        // The types of the messages and of the IEs are types of the bindings.
        for (.., ty) in p.messages {
            assert!((p.kind_of)(type_name(ty)).is_some(), "{ty}");
        }
        let typed = |(_, ty, _): &(u16, &'static str, &'static str)| (p.kind_of)(type_name(ty));
        assert!(p.ie_types.iter().any(|ie| typed(ie).is_some()));
        // The identifiers of the IEs are in their order, which their names are read in.
        assert!(p.ie_names.windows(2).all(|pair| pair[0].0 < pair[1].0));
    }

    /// No two names that a path takes are the same: the members of a type, the IEs and
    /// the messages.
    pub fn no_two_names_that_a_path_takes_are_the_same(p: &'static Protocol) {
        let distinct = |of: &str, names: &mut dyn Iterator<Item = &str>| {
            let names: Vec<_> = names.collect();
            for (at, name) in names.iter().enumerate() {
                let other = names[..at].iter().find(|other| same_name(other, name));
                assert!(other.is_none(), "{of}: {name} and {other:?}");
            }
        };
        for (name, members) in all_members(p) {
            distinct(name, &mut members.iter().map(|(member, ..)| *member));
        }
        distinct("the IEs", &mut p.ie_names.iter().map(|(_, name)| *name));
        let mut messages = p.messages.iter().map(|(set, ..)| set.name);
        distinct("the messages", &mut messages);
        // The names of an IE and of a member do not meet: a message that a container
        // holds has its IEs alone.
        for (name, members) in all_members(p) {
            if members.iter().any(|(member, ..)| *member == IES) {
                assert_eq!(members.len(), 1, "{name}");
            }
        }
        // A name is read whatever its case and its separators, and a name of an IE
        // finds its identifier.
        assert!(same_name("AMF-UE-NGAP-ID", "amf_ue ngap.id"));
        assert!(!same_name("-", "_") && !same_name("", ""));
        for (id, name) in p.ie_names {
            assert_eq!((p.ie_id)(&name.to_lowercase()), Some(*id), "{name}");
        }
    }

    /// A name finds in the perfect hash of its table what a search of the names finds:
    /// the IE whose name has its letters and its digits, and the type of its name, with
    /// its kind, its readable form and the values of an ENUMERATED.
    pub fn a_name_finds_in_its_hash_what_a_search_of_the_names_finds(p: &'static Protocol) {
        // Each name, and names that are close to it and are not it.
        let near = |name: &str| {
            let mut shorter = name.to_string();
            shorter.pop();
            [
                name.to_string(),
                name.to_uppercase(),
                format!("{name}x"),
                format!("x{name}"),
                shorter,
                String::new(),
            ]
        };
        let letters = |name: &str| -> Vec<u8> {
            let letters = name.bytes().filter(u8::is_ascii_alphanumeric);
            letters.map(|letter| letter.to_ascii_lowercase()).collect()
        };
        for (_, name) in p.ie_names {
            for name in near(name) {
                let found = p.ie_names.iter().find(|(_, known)| {
                    !letters(&name).is_empty() && letters(known) == letters(&name)
                });
                assert_eq!((p.ie_id)(&name), found.map(|(id, _)| *id), "{name}");
            }
        }
        let kinds = crate::inspect::kinds(p);
        for ty in p.schema {
            for name in near(ty.rust) {
                let found = p.schema.iter().position(|known| known.rust == name);
                let kind = found.and_then(|row| kinds[row].as_ref());
                let kind_of = (p.kind_of)(&name);
                assert_eq!(format!("{kind_of:?}"), format!("{kind:?}"), "{name}");
                let (form, values) = match found.map(|row| &p.schema[row].kind) {
                    Some(schema::Kind::Alias { form, .. }) => (*form, None),
                    Some(schema::Kind::Enumerated { names, .. }) => (None, Some(*names)),
                    _ => (None, None),
                };
                assert_eq!((p.type_form)(&name), form, "{name}");
                assert_eq!((p.enumerated)(&name), values, "{name}");
            }
        }
    }

    /// No two names of the values of an ENUMERATED are the same, whatever their case and
    /// their separators.
    pub fn no_two_names_of_an_enumerated_are_the_same(p: &'static Protocol) {
        let enumerations = p.schema.iter().filter_map(|ty| match ty.kind {
            schema::Kind::Enumerated { names, .. } => Some((ty.rust, names)),
            _ => None,
        });
        for (ty, names) in enumerations {
            for (at, name) in names.iter().enumerate() {
                let other = names[..at].iter().find(|other| same_name(other, name));
                assert!(other.is_none(), "{ty}: {name} and {other:?}");
            }
        }
    }

    /// An extension has the type that the type it extends gives it.
    pub fn an_extension_has_the_type_that_the_type_it_extends_gives_it(p: &'static Protocol) {
        // An extension container extends the type that has it, whatever type that is.
        let mut containers = 0;
        for (name, _) in all_members(p) {
            let members = p.members(Shape::Named(name), None).unwrap_or_default();
            for (_, shape, _) in members {
                if let Shape::Extensions(extended) = shape {
                    assert_eq!(extended, name);
                    assert_eq!(p.entry_shape(shape), Shape::Extension(None, name));
                    containers += 1;
                }
            }
        }
        assert!(containers > 0);
        // An IE whose identifier has a type for each type that it extends is of that type
        // there, and of none elsewhere: its identifier alone does not say its type.
        for (id, extended, ty) in p.extension_types {
            assert!((p.kind_of)(extended).is_some(), "{extended}");
            assert!(p.ie_types.iter().all(|(known, ..)| known != id), "{id}");
            assert_eq!(p.ie_value(Some(*id), extended), Shape::Named(type_name(ty)));
            assert_eq!(p.ie_value(Some(*id), ""), Shape::Unknown);
            let extension = Shape::Extension(Some(*id), extended);
            let value = p.members(extension, None).unwrap();
            let (_, value, _) = member(&value, "extensionValue").unwrap();
            assert_eq!(*value, Shape::Named(type_name(ty)), "{id} in {extended}");
        }
    }

    /// The IEs that a value holds are written by name, and read back.
    pub fn the_ies_that_a_value_holds_are_written_by_name_and_read_back(p: &'static Protocol) {
        let cause = (p.ie_id)("Cause").expect("an IE of the protocol");
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
        let shown = named_ies(p, "no message", tree!([entry(true)])).unwrap();
        assert_eq!(shown, tree!([{"Cause": &value, "criticality": "ignore"}]));
        let listed = listed_ies(p, "no message", &shown).unwrap();
        assert_eq!(listed, tree!([entry(false)]));
        let refused = listed_ies(p, "no message", &tree!([{"Cause": &value}])).unwrap_err();
        assert_eq!(refused.kind(), crate::ErrorKind::Value);
        let text = refused.to_string();
        assert!(text.contains("says its \"criticality\""), "{refused}");
        // One that did not decode has its octets, which are sent as they are.
        let mut undecoded = entry(true);
        undecoded["value"] = tree!("0000");
        undecoded["_decode_error"] = tree!("why");
        let shown = named_ies(p, "no message", tree!([undecoded])).unwrap();
        let octets = tree!({"Cause": {"octets": "0000"}, "criticality": "ignore"});
        assert_eq!(shown, tree!([octets]));
        let listed = listed_ies(p, "no message", &shown).unwrap();
        let octets = tree!({"id": cause, "criticality": "ignore", "octets": "0000"});
        assert_eq!(listed, tree!([octets]));
        // A tree is not read deeper than a view is encoded.
        let mut deep = tree!(1);
        for _ in 0..258 {
            deep = tree!([deep]);
        }
        let deep = tree!([{"Cause": deep, "criticality": "ignore"}]);
        let refused = listed_ies(p, "no message", &deep).unwrap_err();
        assert_eq!(refused.kind(), crate::ErrorKind::Value);
        assert!(refused.to_string().contains("nests deeper"), "{refused}");
    }

    /// The octets of an IE carry a type of the list by a name that no member has.
    pub fn the_octets_of_an_ie_carry_a_type_of_the_list_by_a_name_that_no_member_has(
        p: &'static Protocol,
    ) {
        for (id, name, ty) in p.carried {
            // The value of the IE is octets, and what they carry a type of the list, by
            // the name that ASN.1 gives it.
            let found = p.ie_types.iter().find(|(known, ..)| known == id);
            let (_, octets, contained) = found.unwrap();
            assert!(contained.is_empty() && (p.kind_of)(type_name(octets)).is_none());
            assert!((p.kind_of)(type_name(ty)).is_some() && same_name(name, type_name(ty)));
            assert_eq!(p.ie_value(Some(*id), ""), Shape::Carrier(*id));
            // A view is encoded with any member of that name as that type.
            for (of, members) in all_members(p) {
                let same = members.iter().any(|(member, ..)| same_name(member, name));
                assert!(!same, "{of}: {name}");
            }
        }
    }

    /// What is nested deeper than a path goes is one value.
    pub fn what_is_nested_deeper_than_a_path_goes_is_one_value(p: &'static Protocol) {
        // Lists in lists, ten levels deeper than a path goes: the path of the one value
        // has a segment for each level that is walked, and the value is what is under
        // them, whole.
        const SEGMENTS: usize = 64;
        let mut deep = tree!([{"a": 1}]);
        for _ in 0..SEGMENTS + 10 {
            deep = tree!([deep]);
        }
        let found = paths(p, &deep);
        assert_eq!(found.len(), 1);
        let (path, value) = &found[0];
        assert_eq!(path.matches('/').count(), SEGMENTS);
        assert_eq!(select(p, &deep, path), Ok(alloc::vec![value]));
        // What is not that deep is listed value by value.
        let mut shallow = tree!([{"a": 1}]);
        for _ in 0..SEGMENTS - 10 {
            shallow = tree!([shallow]);
        }
        let found = paths(p, &shallow);
        assert_eq!(found.len(), 1);
        assert!(found[0].0.ends_with("/0/a") && found[0].1 == tree!(1));
    }
}

/// A test for each check of the registry of the protocol `$protocol`, its marker type.
#[doc(hidden)]
#[macro_export]
macro_rules! __registry_tests {
    ($protocol:ty) => {
        $crate::__registry_tests!(@each $protocol;
            the_ies_are_listed_in_the_order_of_their_identifiers,
            each_message_but_the_private_one_has_an_object_set,
            every_ie_of_an_object_set_has_a_name_and_is_there_once);
    };
    (@each $protocol:ty; $($check:ident),+) => {$(
        #[test]
        fn $check() {
            $crate::checks::$check::<$protocol>();
        }
    )+};
}

/// A test for each check of the tables of the trees and of the paths, which `$tables`,
/// a `&'static Protocol` of `inspect`, gives.
#[doc(hidden)]
#[macro_export]
macro_rules! __inspect_tests {
    ($tables:expr) => {
        $crate::__inspect_tests!(@each $tables;
            the_types_name_types_that_have_members_names_or_a_form,
            no_two_names_that_a_path_takes_are_the_same,
            a_name_finds_in_its_hash_what_a_search_of_the_names_finds,
            no_two_names_of_an_enumerated_are_the_same,
            an_extension_has_the_type_that_the_type_it_extends_gives_it,
            the_ies_that_a_value_holds_are_written_by_name_and_read_back,
            the_octets_of_an_ie_carry_a_type_of_the_list_by_a_name_that_no_member_has,
            what_is_nested_deeper_than_a_path_goes_is_one_value);
    };
    (@each $tables:expr; $($check:ident),+) => {$(
        #[test]
        fn $check() {
            $crate::checks::$check($tables);
        }
    )+};
}
