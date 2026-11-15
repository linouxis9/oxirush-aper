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

//! The walker of the table of the types reads the octets of a value as its tree, and
//! writes the tree back to the same octets: values that PER does not align, a number of
//! bits in its readable form, the values that a later version adds to an ENUMERATED and
//! a CHOICE, and a list whose size has no upper bound below 64K.

#![cfg(feature = "inspect")]

use oxirush_aper::inspect::schema;
use oxirush_toy::inspect::{value_from_tree, value_tree};
use oxirush_toy::toy::*;
use oxirush_toy::tree;
use oxirush_toy::tree::Value;

/// The tree of `value`, which writes it back.
fn read<T>(value: &T) -> Value
where
    T: oxirush_aper::Encode
        + oxirush_aper::AsnType
        + PartialEq
        + core::fmt::Debug
        + for<'a> oxirush_aper::Decode<'a>,
{
    let tree = value_tree(value).unwrap();
    assert_eq!(value_from_tree::<T>(&tree).as_ref(), Ok(value), "{tree}");
    tree
}

#[test]
fn values_after_a_bit_are_read_where_they_are() {
    let cell = oxirush_aper::Bits::new(0x123_4567, 28).unwrap();
    let tagged = Tagged {
        flag: true,
        rac: RAC([42]),
        code: Code([0xbe, 0xef]),
        cell: CellIdentity(cell),
        level: Level::top,
        mode: Mode::third(true),
        octet: oxirush_aper::Bits::new(0xa5, 8).unwrap(),
        maybe: None,
        some: oxirush_aper::Bits::new(5, 3).unwrap(),
    };
    assert_eq!(
        read(&tagged),
        tree!({"flag": true, "rac": 42, "code": "beef", "cell": 0x123_4567, "level": "top", "mode": {"third": true}, "octet": "a5", "some": {"value": "a0", "length": 3}})
    );
    let tagged = Tagged {
        flag: false,
        rac: RAC([0]),
        code: Code([0, 1]),
        cell: CellIdentity(oxirush_aper::Bits::new(1, 28).unwrap()),
        level: Level::high,
        mode: Mode::first(5),
        octet: oxirush_aper::Bits::new(0, 8).unwrap(),
        maybe: Some(oxirush_aper::Bits::new(1, 8).unwrap()),
        some: oxirush_aper::Bits::new(1, 1).unwrap(),
    };
    assert_eq!(
        read(&tagged),
        tree!({"flag": false, "rac": 0, "code": "0001", "cell": 1, "level": "high", "mode": {"first": 5}, "octet": "00", "maybe": "01", "some": {"value": "80", "length": 1}})
    );
    assert_eq!(read(&Mode::second(())), tree!({"second": null}));
    assert_eq!(read(&Level::low), tree!("low"));
}

#[test]
fn a_list_of_no_bound_below_64k_has_its_length_and_its_size() {
    let item = ConnectionItem {
        ue_core_id: Some(UECoreID(7)),
        ue_ran_id: None,
        ie_extensions: None,
    };
    let connections = Connections(vec![
        item.clone(),
        ConnectionItem {
            ue_core_id: None,
            ..item
        },
    ]);
    assert_eq!(read(&connections), tree!([{"uE-Core-ID": 7}, {}]));
    // A list of no entry is below its lower bound
    let tables = oxirush_toy::inspect::__tables();
    let row = schema::position(tables.schema, "Connections").unwrap();
    assert!(schema::tree(tables.schema, row, &[0x00]).is_err());
    assert!(schema::tree(tables.schema, row, &[0x01, 0x00]).is_ok());
}

/// What the paths take of each member: a bit string of one size that is always there is a
/// plain value, one that may be missing, and one whose size varies, a bit string; a type
/// that is another is that one, unless it has no members.
#[test]
fn the_kind_of_a_member_and_of_a_type_that_is_another() {
    use oxirush_aper::inspect::{Kind, Shape};
    let tables = oxirush_toy::inspect::__tables();
    let Some(Kind::Sequence(members)) = (tables.kind_of)("Tagged") else {
        panic!("Tagged is a SEQUENCE");
    };
    let shape = |name: &str| {
        members
            .iter()
            .find(|(member, ..)| *member == name)
            .map(|(_, shape, _)| *shape)
    };
    assert_eq!(shape("octet"), Some(Shape::Plain));
    assert_eq!(shape("maybe"), Some(Shape::Bits));
    assert_eq!(shape("some"), Some(Shape::Bits));
    assert!(matches!(
        (tables.kind_of)("KindOfTransfer"),
        Some(Kind::Is(Shape::Named("TransferKind")))
    ));
    assert!((tables.kind_of)("CoreID").is_none());
}

/// What the tree of a value of each type takes: a PLMN and digits in their readable form,
/// octets of one size, and of a size that varies, a bit string of one size as its octets,
/// and one of a size that varies as its octets and its length.
#[test]
fn what_a_type_of_each_field_takes() {
    use oxirush_toy::tree::Takes;
    let tables = oxirush_toy::inspect::__tables();
    let takes = oxirush_aper::inspect::takes(tables);
    let of = |name: &str| takes[schema::position(tables.schema, name).unwrap()];
    assert_eq!(of("PLMNIdentity"), &Takes::Plmn);
    assert_eq!(of("IMSI"), &Takes::Digits);
    assert_eq!(of("NAS-PDU"), &Takes::Octets(None));
    assert_eq!(of("Code"), &Takes::Octets(Some(2)));
    assert_eq!(of("SecurityKey"), &Takes::Octets(Some(32)));
    let tagged = of("Tagged");
    assert_eq!(tagged.member("octet"), Some(&Takes::Octets(Some(1))));
    let Some(Takes::Members(Some("value"), [("value", _), ("length", Takes::Number(16))])) =
        tagged.member("some")
    else {
        panic!("{tagged:?}");
    };
}
