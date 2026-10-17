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

//! `oxirush_aper::protocol`, on the protocol of `toy/mod.rs`, which is written as the
//! protocol crates are: the macros that build, read and change IEs, the readers of one IE
//! and of the items of a list, and the checks of a PDU against its object set.

include!("toy/mod.rs");

use oxirush_aper::check::Wrong;
use oxirush_aper::{ErrorKind, Faults, Octets};
use toy::*;

type Error = oxirush_aper::Error;

/// A SETUP REQUEST of an identity, a name and two items, as a PDU that owns its octets.
fn setup(identity: u32, name: &'static str) -> TOY_PDU {
    let first = build_toy_ie!(REJECT Item(1u8)).unwrap();
    let second = build_toy_ie!(REJECT Item(2u8)).unwrap();
    build_toy!(
        InitiatingMessage,
        Setup,
        SetupRequest,
        Identity(identity),
        Name(name),
        Items(vec![first, second]),
    )
    .unwrap()
}

/// The PDU of these IEs, as a SETUP REQUEST.
fn request(ies: Vec<ProtocolIEField<'static>>) -> TOY_PDU {
    let message = SetupRequest::new(ProtocolIEContainer(ies));
    TOYPDU::initiatingMessage(InitiatingMessage {
        procedure_code: ProcedureCode(1),
        criticality: Criticality::reject,
        value: Octets::owned(oxirush_aper::encode(&message).unwrap()),
    })
}

/// An IE of an identifier and a criticality whose value is `octets`.
fn field(id: u16, criticality: Criticality, octets: &[u8]) -> ProtocolIEField<'static> {
    ProtocolIEField::new(id, criticality, Octets::from(octets.to_vec()))
}

/// The identity and the name of a SETUP REQUEST, by the macro.
fn read(pdu: &TOYPDU<'_>) -> Result<(u32, Option<Vec<u8>>), Error> {
    extract_toy_ies!(pdu, SetupRequest,
        req identity: u32 = Identity(id),
        opt name: Vec<u8> = Name(name) => name.0.to_vec(),
    );
    Ok((identity, name))
}

#[test]
fn a_pdu_is_built_with_the_criticalities_that_the_object_set_assigns() -> Result<(), Error> {
    let pdu = setup(7, "toy");
    // The procedure assigns reject, and so do the object set to Identity and Items, and
    // ignore to Name.
    let ies: Vec<_> = {
        let message: SetupRequest = pdu.decode_value()?;
        message
            .protocol_ies
            .0
            .iter()
            .map(|ie| (ie.id.0, ie.criticality))
            .collect()
    };
    assert_eq!(
        ies,
        [
            (1, Criticality::reject),
            (2, Criticality::ignore),
            (3, Criticality::reject)
        ]
    );
    assert_eq!(
        <SetupRequest as oxirush_aper::protocol::Message>::CRITICALITY,
        Criticality::reject
    );
    assert!(pdu.check()?.is_empty());
    assert!(pdu.check_structure()?.is_empty());
    // The octets that the codec of the type writes.
    let octets = pdu.encode()?;
    assert_eq!(TOYPDU::decode(&octets)?, pdu);
    Ok(())
}

#[test]
fn the_macro_reads_a_pdu_and_a_message_alike() -> Result<(), Error> {
    let octets = setup(7, "toy").encode()?;
    let pdu = TOYPDU::decode(&octets)?;
    assert_eq!(read(&pdu)?, (7, Some(b"toy".to_vec())));
    let message: SetupRequest = pdu.decode_value()?;
    let from_message = || -> Result<(u32, Option<Vec<u8>>), Error> {
        extract_toy_ies!(message, SetupRequest,
            req identity: u32 = Identity(id),
            opt name: Vec<u8> = Name(name) => name.0.to_vec(),
        );
        Ok((identity, name))
    };
    assert_eq!(from_message()?, (7, Some(b"toy".to_vec())));
    Ok(())
}

#[test]
fn a_value_borrows_from_the_pdu() -> Result<(), Error> {
    let octets = setup(7, "toy").encode()?;
    let pdu = TOYPDU::decode(&octets)?;
    let name = pdu.ie::<ie::Name>().unwrap()?;
    assert!(name.0.is_borrowed());
    assert_eq!(&name.0[..], b"toy");
    Ok(())
}

#[test]
fn the_first_of_a_repeated_ie_is_read_and_changed_and_check_reports_the_second() -> Result<(), Error>
{
    let identity = |id: u32| build_toy_ie!(SetupRequest, REJECT Identity(id)).unwrap();
    let pdu = request(vec![identity(1), identity(2)]);
    assert_eq!(read(&pdu)?.0, 1);
    let every: Vec<_> = pdu.ies::<ie::Identity>().collect::<Result<_, _>>()?;
    assert_eq!(every, [Identity(1), Identity(2)]);
    assert!(matches!(
        pdu.check_structure()?[..],
        [oxirush_aper::check::Deviation::RepeatedIe {
            id: 1,
            position: 1,
            first: 0,
            ..
        }]
    ));
    // The message: the first one changes.
    let mut message: SetupRequest = pdu.decode_value()?;
    assert_eq!(
        with_toy_ie_mut!(message, SetupRequest, Identity(id) = 9),
        Ok(true)
    );
    let every: Vec<_> = message.ies::<ie::Identity>().collect::<Result<_, _>>()?;
    assert_eq!(every, [Identity(9), Identity(2)]);
    Ok(())
}

#[test]
fn an_ie_that_does_not_decode_is_an_error_of_the_readers_and_of_check_alone() -> Result<(), Error> {
    // An identity whose length, in its first two bits, is four octets, and that ends there.
    let spoiled = field(1, Criticality::reject, &[0xc0]);
    let pdu = request(vec![spoiled, build_toy_ie!(SetupRequest, Name("toy"))?]);
    let error = read(&pdu).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::UndecodableIe);
    assert_eq!(
        error.ie().map(|ie| (ie.id, ie.position)),
        Some((1, Some(0)))
    );
    // The iterator goes on after it.
    let mut identities = pdu.ies::<ie::Identity>();
    assert_eq!(
        identities.next().unwrap().unwrap_err().kind(),
        ErrorKind::UndecodableIe
    );
    assert!(identities.next().is_none());
    assert_eq!(pdu.ie::<ie::Name>().unwrap()?.0[..], *b"toy");
    // `check` decodes the value, and `check_structure` does not.
    assert!(matches!(
        pdu.check()?[..],
        [oxirush_aper::check::Deviation::UndecodableIe { id: 1, .. }]
    ));
    assert!(pdu.check_structure()?.is_empty());
    Ok(())
}

#[test]
fn a_pdu_of_another_message_is_an_error_and_lax_fields_are_none() -> Result<(), Error> {
    let response = build_toy!(SuccessfulOutcome, Setup, SetupResponse, Identity(7u32))?;
    assert_eq!(read(&response).unwrap_err().kind(), ErrorKind::WrongMessage);
    let lax = |pdu: &TOYPDU<'_>| {
        extract_toy_ies!(pdu, SetupRequest, lax identity: u32 = Identity(id));
        identity
    };
    assert_eq!(lax(&response), None);
    assert_eq!(lax(&setup(7, "toy")), Some(7));
    // The readers of the PDU take any message of protocol IEs.
    assert_eq!(response.ie::<ie::Identity>().unwrap()?, Identity(7));
    Ok(())
}

#[test]
fn octets_that_are_no_container_are_an_error_of_the_readers() -> Result<(), Error> {
    let pdu = setup(7, "toy");
    let mut octets = oxirush_aper::protocol::Pdu::value(&pdu).to_vec();
    // The last IE ends early.
    octets.pop();
    let mut cut = pdu.clone();
    oxirush_aper::protocol::Pdu::__set_value(&mut cut, Octets::from(octets));
    assert_eq!(read(&cut).unwrap_err().kind(), ErrorKind::Decode);
    // The first IEs read, then the error is the last item.
    let names: Vec<_> = cut.ies::<ie::Name>().collect();
    assert_eq!(names.len(), 2);
    assert_eq!(names[0].as_ref().unwrap().0[..], *b"toy");
    assert_eq!(names[1].as_ref().unwrap_err().kind(), ErrorKind::Decode);
    assert_eq!(cut.check_structure().unwrap_err().kind(), ErrorKind::Decode);
    assert_eq!(cut.check().unwrap_err().kind(), ErrorKind::Decode);
    Ok(())
}

#[test]
fn octets_after_the_message_are_an_error_once_the_walk_reaches_them() -> Result<(), Error> {
    let pdu = setup(7, "toy");
    let mut octets = oxirush_aper::protocol::Pdu::value(&pdu).to_vec();
    octets.push(0);
    let mut longer = pdu.clone();
    oxirush_aper::protocol::Pdu::__set_value(&mut longer, Octets::from(octets));
    // The first IE is read before the walk reaches the end.
    assert_eq!(longer.ie::<ie::Identity>().unwrap()?, Identity(7));
    let every: Vec<_> = longer.ies::<ie::Identity>().collect();
    assert_eq!(
        every.last().unwrap().as_ref().unwrap_err().kind(),
        ErrorKind::Decode
    );
    assert!(longer.check_structure().is_err());
    Ok(())
}

#[test]
fn a_message_with_additions_and_lengths_in_two_octets_is_read_as_its_type_reads_it()
-> Result<(), Error> {
    // The extension bit, the count of the IEs, then an identity whose length of 5 is in two
    // octets, then an addition of one octet that is present.
    let mut octets = vec![
        0x80, 0x00, 0x01, 0x00, 0x01, 0x00, 0x80, 0x05, 0xc0, 0x01, 0x02, 0x03, 0x04,
    ];
    // Identity 0x01020304: the length of the number, 4, in two bits, then the octets.
    octets[8..].copy_from_slice(&[0xc0, 0x01, 0x02, 0x03, 0x04]);
    // The bitmap of one addition, which is present, after its length less one in a normally
    // small number, then its open type.
    octets.extend_from_slice(&[0x01, 0x01, 0xaa]);
    let mut pdu = setup(0, "");
    oxirush_aper::protocol::Pdu::__set_value(&mut pdu, Octets::from(octets.clone()));
    let message: SetupRequest = pdu.decode_value()?;
    let typed = oxirush_aper::decode::<Identity>(&message.protocol_ies.0[0].value)?;
    assert_eq!(pdu.ie::<ie::Identity>().unwrap()?, typed);
    assert_eq!(read(&pdu)?, (typed.0, None));
    let every: Vec<_> = pdu.ies::<ie::Identity>().collect::<Result<_, _>>()?;
    assert_eq!(every, [typed]);
    Ok(())
}

#[test]
fn a_value_in_fragments_is_gathered_and_kept() -> Result<(), Error> {
    // A name of 20,000 octets is in two fragments in its open type, and its octet string
    // in two more.
    let long: Vec<u8> = (0..20_000).map(|at| (at % 251) as u8).collect();
    let pdu = build_toy!(
        InitiatingMessage,
        Setup,
        SetupRequest,
        Identity(7u32),
        Name(long.clone()),
    )?;
    let octets = pdu.encode()?;
    let pdu = TOYPDU::decode(&octets)?;
    let name = pdu.ie::<ie::Name>().unwrap()?;
    assert_eq!(&name.0[..], &long[..]);
    assert_eq!(read(&pdu)?, (7, Some(long.clone())));
    let message: SetupRequest = pdu.decode_value()?;
    assert_eq!(message.ie::<ie::Name>().unwrap()?.0[..], long[..]);
    assert!(pdu.check()?.is_empty());
    Ok(())
}

#[test]
fn the_items_of_a_list_are_walked_and_each_is_decoded() -> Result<(), Error> {
    let pdu = setup(7, "toy");
    let items: Vec<_> = pdu.items::<ie::Items>().collect::<Result<_, _>>()?;
    assert_eq!(items, [Item(1), Item(2)]);
    let message: SetupRequest = pdu.decode_value()?;
    let items: Vec<_> = message.items::<ie::Items>().collect::<Result<_, _>>()?;
    assert_eq!(items, [Item(1), Item(2)]);
    // A single container of another IE is passed over, and an item that does not decode is
    // an error, after which the walk goes on.
    let other = field(11, Criticality::ignore, &[0x05]);
    let spoiled = field(10, Criticality::reject, &[]);
    let good = build_toy_ie!(REJECT Item(3u8))?;
    let list = build_toy_ie!(SetupRequest, Items(vec![other, spoiled, good]))?;
    let pdu = request(vec![build_toy_ie!(SetupRequest, Identity(7u32))?, list]);
    let items: Vec<_> = pdu.items::<ie::Items>().collect();
    assert_eq!(items.len(), 2);
    let error = items[0].as_ref().unwrap_err();
    assert_eq!(error.kind(), ErrorKind::UndecodableIe);
    assert_eq!(
        error.ie().map(|ie| (ie.id, ie.position)),
        Some((10, Some(1)))
    );
    assert_eq!(items[1], Ok(Item(3)));
    // No list, no item.
    assert_eq!(request(vec![]).items::<ie::Items>().count(), 0);
    Ok(())
}

#[test]
fn a_list_whose_count_is_outside_its_size_is_an_error_of_the_list() {
    // A count of 17 items, where the list has 16 at most: four bits of 16, the count less
    // one, which no item follows.
    let list = field(3, Criticality::reject, &[0xf0]);
    let pdu = request(vec![list]);
    let items: Vec<_> = pdu.items::<ie::Items>().collect();
    assert_eq!(items.len(), 1);
    let error = items[0].as_ref().unwrap_err();
    assert_eq!(error.kind(), ErrorKind::UndecodableIe);
    assert_eq!(error.ie().map(|ie| ie.id), Some(3));
}

#[test]
fn a_private_message_has_no_ies_to_read() -> Result<(), Error> {
    let message = PrivateMessage {
        private_ies: Octets::from(vec![1, 2]),
    };
    let pdu = TOYPDU::initiatingMessage(InitiatingMessage {
        procedure_code: ProcedureCode(3),
        criticality: Criticality::ignore,
        value: Octets::owned(oxirush_aper::encode(&message)?),
    });
    let error = pdu.ie::<ie::Identity>().unwrap().unwrap_err();
    assert_eq!(error.kind(), ErrorKind::WrongMessage);
    // Its PDU is checked for its criticality alone.
    assert!(pdu.check()?.is_empty());
    Ok(())
}

#[test]
fn the_ies_of_a_procedure_that_the_protocol_does_not_have_are_read_too() -> Result<(), Error> {
    let mut pdu = setup(7, "toy");
    if let TOYPDU::initiatingMessage(message) = &mut pdu {
        message.procedure_code = ProcedureCode(200);
    }
    assert_eq!(pdu.ie::<ie::Identity>().unwrap()?, Identity(7));
    assert!(matches!(
        pdu.check_structure()?[..],
        [oxirush_aper::check::Deviation::UnknownProcedure {
            procedure_code: 200,
            ..
        }]
    ));
    Ok(())
}

/// The walk of the readers and the codec of the container read the same IEs, of values of
/// every length: in one octet of length, in two, and in fragments.
#[test]
fn the_readers_read_what_the_codec_of_the_container_reads() -> Result<(), Error> {
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    let mut next = move |bound: u64| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed % bound
    };
    for _ in 0..200 {
        let count = next(6) as usize;
        let mut ies = Vec::new();
        for _ in 0..count {
            let length = match next(4) {
                0 => next(128),
                1 => 128 + next(400),
                2 => next(3),
                _ => 16_384 + next(20_000),
            } as usize;
            let name: Vec<u8> = (0..length).map(|at| at as u8).collect();
            let id = [1u16, 2, 2, 4][next(4) as usize];
            ies.push(match id {
                2 => build_toy_ie!(SetupRequest, Name(name))?,
                1 => build_toy_ie!(SetupRequest, Identity(next(1 << 32) as u32))?,
                _ => field(id, Criticality::notify, &name),
            });
        }
        let octets = request(ies).encode()?;
        let pdu = TOYPDU::decode(&octets)?;
        let message: SetupRequest = pdu.decode_value()?;
        let names = message.protocol_ies.0.iter().filter(|ie| ie.id.0 == 2);
        let names: Vec<Vec<u8>> = names
            .map(|ie| oxirush_aper::decode::<Name>(&ie.value).unwrap().0.to_vec())
            .collect();
        let read: Vec<Vec<u8>> = pdu
            .ies::<ie::Name>()
            .map(|name| name.unwrap().0.to_vec())
            .collect();
        assert_eq!(read, names);
        let read: Vec<Vec<u8>> = message
            .ies::<ie::Name>()
            .map(|name| name.unwrap().0.to_vec())
            .collect();
        assert_eq!(read, names);
        // The structure that both read is the same.
        assert_eq!(pdu.check_structure()?.len(), {
            let mut found = Vec::new();
            let set = &oxirush_aper::protocol::object_set::<Toy>("InitiatingMessage", 1)
                .unwrap()
                .ies
                .unwrap();
            let listed: Vec<_> = message
                .protocol_ies
                .0
                .iter()
                .map(|ie| (ie.id.0, ie.criticality))
                .collect();
            oxirush_aper::check::deviations::<Toy>(
                set,
                &listed,
                &|_| (None, Faults::default()),
                &mut found,
            );
            found.len()
        });
    }
    Ok(())
}

/// What `check` reports of the IEs of a message, against an object set of four IEs: the
/// first and the third are mandatory, and the identifiers are not in their own order.
mod deviations {
    use super::*;
    use oxirush_aper::check::{Deviation, deviations};

    fn set() -> Vec<(u16, Criticality, bool)> {
        vec![
            (3, Criticality::reject, true),
            (1, Criticality::ignore, false),
            (2, Criticality::reject, true),
            (4, Criticality::notify, false),
        ]
    }

    /// What a message of these IEs deviates in, when the value of each decodes.
    fn found(ies: &[(u16, Criticality)]) -> Vec<Deviation<Toy>> {
        let mut found = Vec::new();
        deviations::<Toy>(&set(), ies, &|_| (None, Faults::default()), &mut found);
        found
    }

    const REJECT: Criticality = Criticality::reject;
    const IGNORE: Criticality = Criticality::ignore;
    const NOTIFY: Criticality = Criticality::notify;

    #[test]
    fn a_message_as_its_object_set_has_it_deviates_in_nothing() {
        assert_eq!(found(&[(3, REJECT), (2, REJECT)]), []);
        assert_eq!(
            found(&[(3, REJECT), (1, IGNORE), (2, REJECT), (4, NOTIFY)]),
            []
        );
    }

    #[test]
    fn each_deviation_is_told_with_its_ie_and_its_position() {
        // A mandatory IE that is missing, and an optional one is not.
        let missing = found(&[(3, REJECT)]);
        assert!(matches!(missing[..], [Deviation::MissingIe { id: 2, .. }]));
        // An IE that the set does not have: one of the protocol or not.
        let other = found(&[(3, REJECT), (65000, IGNORE), (2, REJECT)]);
        let unexpected = Deviation::UnexpectedIe {
            id: 65000,
            name: None,
            position: 1,
            criticality: IGNORE,
        };
        assert_eq!(other, [unexpected]);
        // An IE that is there twice, whatever is between the two.
        let twice = found(&[(3, REJECT), (2, REJECT), (3, REJECT)]);
        assert!(matches!(
            twice[..],
            [Deviation::RepeatedIe {
                id: 3,
                position: 2,
                first: 0,
                ..
            }]
        ));
        // Two IEs in the other order: the second is after one that the set has after it.
        let swapped = found(&[(2, REJECT), (3, REJECT)]);
        assert!(matches!(
            swapped[..],
            [Deviation::OutOfOrder {
                id: 3,
                position: 1,
                after: 2,
                ..
            }]
        ));
        // A criticality that is not the one of the set.
        let criticality = found(&[(3, REJECT), (2, IGNORE)]);
        assert!(matches!(
            criticality[..],
            [Deviation::IeCriticality {
                id: 2,
                position: 1,
                criticality: IGNORE,
                assigned: REJECT,
                ..
            }]
        ));
    }

    #[test]
    fn the_order_is_that_of_the_set_and_leaves_out_what_the_set_does_not_have() {
        // The last IE of the set first: each of the others comes after it.
        let late = found(&[(4, NOTIFY), (3, REJECT), (1, IGNORE), (2, REJECT)]);
        let after: Vec<_> = late
            .iter()
            .map(|deviation| match deviation {
                Deviation::OutOfOrder {
                    id,
                    position,
                    after,
                    ..
                } => (*id, *position, *after),
                other => panic!("{other}"),
            })
            .collect();
        assert_eq!(after, [(3, 1, 4), (1, 2, 4), (2, 3, 4)]);
        // An IE that the set does not have is between two that are in order.
        let between = found(&[(3, REJECT), (65000, REJECT), (2, REJECT)]);
        assert!(matches!(between[..], [Deviation::UnexpectedIe { .. }]));
        // A repeated IE is told once, as repeated, and the missing ones come last.
        let all = found(&[(1, REJECT), (1, IGNORE), (3, REJECT)]);
        assert!(matches!(
            all[..],
            [
                Deviation::IeCriticality {
                    id: 1,
                    position: 0,
                    ..
                },
                Deviation::RepeatedIe {
                    id: 1,
                    position: 1,
                    first: 0,
                    ..
                },
                Deviation::OutOfOrder {
                    id: 3,
                    position: 2,
                    after: 1,
                    ..
                },
                Deviation::MissingIe { id: 2, .. },
            ]
        ));
    }

    #[test]
    fn a_value_that_does_not_decode_is_told_at_its_position() {
        let mut found = Vec::new();
        let error = Error::new(ErrorKind::UndecodableIe, "spoiled");
        let wrong = std::cell::RefCell::new(Some(error));
        let value = |position: usize| match position {
            1 => (
                wrong.borrow_mut().take().map(Wrong::Undecodable),
                Faults::default(),
            ),
            _ => (None, Faults::default()),
        };
        deviations::<Toy>(&set(), &[(3, REJECT), (2, REJECT)], &value, &mut found);
        assert!(matches!(
            found[..],
            [Deviation::UndecodableIe {
                id: 2,
                position: 1,
                ..
            }]
        ));
    }
}
