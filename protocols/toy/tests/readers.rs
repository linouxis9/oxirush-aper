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

//! The readers of the IEs of a PDU and of a message: `ie::<T>()`, `ies::<T>()` and
//! `items::<T>()`, and the macro that extracts IEs from a PDU without `decode_value`.
//! They walk the octets of the message, and read what the codec of the message reads.

use oxirush_toy::aper::protocol::{Pdu, Source};
use oxirush_toy::toy::*;
use oxirush_toy::{Deviation, Direction, Error, ErrorKind, Faults, Octets, build_toy};
use oxirush_toy::{Ie, extract_toy_ies, ie};

/// A RELEASE REQUEST of UE-Core-ID 42, UE-RAN-ID `ran` and a NAS-PDU `nas`.
fn release(ran: u32, nas: &[u8]) -> TOYPDU<'static> {
    build_toy!(
        ReleaseRequest,
        UE_Core_ID(42u64),
        UE_RAN_ID(ran),
        Cause(Cause::radioNetwork(CauseRadioNetwork::user_inactivity)),
        NAS_PDU(nas.to_vec()),
    )
    .unwrap()
}

/// The PDU of these IEs, as a RELEASE REQUEST.
fn request(ies: Vec<ProtocolIEField<'static>>) -> TOYPDU<'static> {
    let message = ReleaseRequest {
        protocol_ies: ProtocolIEContainer(ies),
    };
    TOYPDU::initiatingMessage(InitiatingMessage {
        procedure_code: ProcedureCode(2),
        criticality: Criticality::Ignore,
        value: Octets::owned(oxirush_toy::encode(&message).unwrap()),
    })
}

/// The PDU whose message is `octets`, as a RELEASE REQUEST.
fn of_octets(octets: &[u8]) -> TOYPDU<'static> {
    let mut pdu = release(7, b"");
    pdu.__set_value(Octets::from(octets.to_vec()));
    pdu
}

/// An IE of an identifier and a criticality whose value is `octets`.
fn field(id: u16, criticality: Criticality, octets: &[u8]) -> ProtocolIEField<'static> {
    ProtocolIEField {
        id: ProtocolIEID(id),
        criticality,
        value: Octets::from(octets.to_vec()),
    }
}

/// UE-RAN-ID 7 as an IE of the criticality `reject`.
const RAN_7: [u8; 6] = [0x00, 0x0b, 0x00, 0x02, 0x00, 0x07];

/// A UE-RAN-ID whose length, in its first two bits, is four octets, and that ends there.
fn spoiled() -> ProtocolIEField<'static> {
    field(11, Criticality::Reject, &[0xc0])
}

/// The UE-RAN-ID and the NAS-PDU of a RELEASE REQUEST, by the macro.
fn read(pdu: &TOYPDU<'_>) -> Result<(u32, Option<Vec<u8>>), Error> {
    extract_toy_ies!(pdu, ReleaseRequest,
        req ran: u32 = UE_RAN_ID(id),
        opt nas: Vec<u8> = NAS_PDU(nas) => nas.0.to_vec(),
    );
    Ok((ran, nas))
}

#[test]
fn an_ie_is_read_from_the_pdu_as_from_its_message() -> Result<(), Error> {
    let octets = release(7, b"nas").encode()?;
    let pdu = TOYPDU::decode(&octets)?;
    assert_eq!(pdu.ie::<ie::UE_RAN_ID>().unwrap()?, UERANID(7));
    assert_eq!(pdu.ie::<ie::UE_Core_ID>().unwrap()?, UECoreID(42));
    assert!(pdu.ie::<ie::UE_IDs>().is_none());
    assert_eq!(read(&pdu)?, (7, Some(b"nas".to_vec())));
    // The message, by the readers and by the macro
    let message: ReleaseRequest = pdu.decode_value()?;
    assert_eq!(message.ie::<ie::UE_RAN_ID>().unwrap()?, UERANID(7));
    let from_message = || -> Result<(u32, Option<Vec<u8>>), Error> {
        extract_toy_ies!(message, ReleaseRequest,
            req ran: u32 = UE_RAN_ID(id),
            opt nas: Vec<u8> = NAS_PDU(nas) => nas.0.to_vec(),
        );
        Ok((ran, nas))
    };
    assert_eq!(from_message()?, (7, Some(b"nas".to_vec())));
    // A value borrows from the octets of the PDU.
    let nas = pdu.ie::<ie::NAS_PDU>().unwrap()?;
    assert!(nas.0.is_borrowed());
    // A reader of the PDU itself
    let source = Source::__of_pdu(&pdu);
    assert_eq!(source.ie::<ie::UE_RAN_ID>().unwrap()?, UERANID(7));
    // The name of a message in its errors, from the path of its type.
    let named = oxirush_toy::aper::protocol::__type_name("crate::toy::ReleaseRequest<'a>");
    assert_eq!(named, "ReleaseRequest");
    Ok(())
}

#[test]
fn a_pdu_of_another_message_is_an_error_and_lax_fields_are_none() -> Result<(), Error> {
    let notification = build_toy!(Notification, UE_IDs(UEIDs::uE_Core_ID(UECoreID(42))),)?;
    assert_eq!(
        read(&notification).unwrap_err().kind(),
        ErrorKind::WrongMessage
    );
    let lax = |pdu: &TOYPDU<'_>| {
        extract_toy_ies!(pdu, ReleaseRequest, lax ran: u32 = UE_RAN_ID(id));
        ran
    };
    assert_eq!(lax(&notification), None);
    assert_eq!(lax(&release(7, b"")), Some(7));
    // The readers of the PDU take any message of protocol IEs.
    let ids = notification.ie::<ie::UE_IDs>().unwrap()?;
    assert_eq!(ids, UEIDs::uE_Core_ID(UECoreID(42)));
    Ok(())
}

#[test]
fn a_value_in_fragments_is_read_and_kept() -> Result<(), Error> {
    // 20,000 octets come in two fragments in the open type of the IE, and in two more in
    // the OCTET STRING of the NAS-PDU.
    let nas: Vec<u8> = (0..20_000).map(|at| (at % 253) as u8).collect();
    let octets = release(7, &nas).encode()?;
    let pdu = TOYPDU::decode(&octets)?;
    assert_eq!(pdu.ie::<ie::NAS_PDU>().unwrap()?.0[..], nas[..]);
    assert_eq!(read(&pdu)?, (7, Some(nas.clone())));
    let message: ReleaseRequest = pdu.decode_value()?;
    assert_eq!(message.ie::<ie::NAS_PDU>().unwrap()?.0[..], nas[..]);
    assert!(pdu.check()?.is_empty());
    Ok(())
}

#[test]
fn a_repeated_ie_is_there_twice_for_ies_and_once_for_the_macro() -> Result<(), Error> {
    let pdu = build_toy!(
        ReleaseRequest,
        UE_Core_ID(42u64),
        UE_RAN_ID(7u32),
        UE_RAN_ID(8u32),
        Cause(Cause::radioNetwork(CauseRadioNetwork::user_inactivity)),
    )?;
    let every: Vec<_> = pdu.ies::<ie::UE_RAN_ID>().collect::<Result<_, _>>()?;
    assert_eq!(every, [UERANID(7), UERANID(8)]);
    assert_eq!(read(&pdu)?.0, 7);
    assert!(matches!(
        pdu.check_structure()?[..],
        [Deviation::RepeatedIe {
            id: 11,
            position: 2,
            first: 1,
            ..
        }]
    ));
    // The message: the first one changes.
    let mut message: ReleaseRequest = pdu.decode_value()?;
    assert_eq!(message.set_ie::<ie::UE_RAN_ID>(9u32), Ok(true));
    let every: Vec<_> = message.ies::<ie::UE_RAN_ID>().collect::<Result<_, _>>()?;
    assert_eq!(every, [UERANID(9), UERANID(8)]);
    Ok(())
}

#[test]
fn an_ie_that_does_not_decode_is_an_error_of_the_readers_and_of_check_alone() -> Result<(), Error> {
    let nas = ie::NAS_PDU::field_of::<ReleaseRequest>(b"nas".to_vec())?;
    let pdu = request(vec![spoiled(), nas]);
    let error = read(&pdu).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::UndecodableIe);
    assert_eq!(
        error.ie().map(|ie| (ie.id, ie.position)),
        Some((11, Some(0)))
    );
    // The iterator goes on after it.
    let mut identities = pdu.ies::<ie::UE_RAN_ID>();
    assert_eq!(
        identities.next().unwrap().unwrap_err().kind(),
        ErrorKind::UndecodableIe
    );
    assert!(identities.next().is_none());
    assert_eq!(pdu.ie::<ie::NAS_PDU>().unwrap()?.0[..], *b"nas");
    // `check` decodes the value, and `check_structure` does not: both report the IEs
    // that the message misses.
    let missing = |deviation: &&Deviation| !matches!(deviation, Deviation::MissingIe { .. });
    let checked = pdu.check()?;
    let found: Vec<_> = checked.iter().filter(missing).collect();
    assert!(matches!(
        found[..],
        [Deviation::UndecodableIe { id: 11, .. }]
    ));
    let structure = pdu.check_structure()?;
    assert_eq!(structure.iter().filter(missing).count(), 0);
    // An error of a reader names the IE and its position, whatever comes before it: here
    // a NAS-PDU in fragments, which the walk reads whole.
    let long = ie::NAS_PDU::field_of::<ReleaseRequest>(vec![0x61; 20_000])?;
    let error = request(vec![long, spoiled()])
        .ie::<ie::UE_RAN_ID>()
        .unwrap()
        .unwrap_err();
    assert_eq!(
        error.ie().map(|ie| (ie.id, ie.position)),
        Some((11, Some(1)))
    );
    assert!(error.to_string().contains("UE-RAN-ID"), "{error}");
    Ok(())
}

#[test]
fn octets_of_the_message_that_end_early_are_an_error_of_every_reader() -> Result<(), Error> {
    let pdu = release(7, b"nas");
    // The message without its last octet, which its NAS-PDU takes.
    let mut octets = pdu.value().to_vec();
    octets.pop();
    let mut cut = pdu.clone();
    cut.__set_value(Octets::from(octets));
    assert_eq!(cut.check_structure().unwrap_err().kind(), ErrorKind::Decode);
    assert_eq!(cut.check().unwrap_err().kind(), ErrorKind::Decode);
    assert_eq!(read(&cut).unwrap_err().kind(), ErrorKind::Decode);
    // The IEs before it read, then the error is the last item.
    let ids: Vec<_> = cut.ies::<ie::UE_RAN_ID>().collect();
    assert_eq!(ids[0], Ok(UERANID(7)));
    assert_eq!(ids[1].as_ref().unwrap_err().kind(), ErrorKind::Decode);
    Ok(())
}

#[test]
fn octets_after_the_message_are_an_error_once_the_walk_reaches_them() -> Result<(), Error> {
    let pdu = release(7, b"nas");
    let mut octets = pdu.value().to_vec();
    octets.push(0);
    let mut longer = pdu.clone();
    longer.__set_value(Octets::from(octets));
    // The first IE is read before the walk reaches the end.
    assert_eq!(longer.ie::<ie::UE_RAN_ID>().unwrap()?, UERANID(7));
    let every: Vec<_> = longer.ies::<ie::UE_RAN_ID>().collect();
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
    // The extension bit, the count of the IEs, then a UE-RAN-ID whose length of 5 is in
    // two octets: the length of the number, 4, in two bits, then its octets.
    let mut octets = vec![
        0x80, 0x00, 0x01, 0x00, 0x0b, 0x00, 0x80, 0x05, 0xc0, 0x01, 0x02, 0x03, 0x04,
    ];
    // The bitmap of one addition, which is present, after its length less one in a
    // normally small number, then its open type.
    octets.extend_from_slice(&[0x01, 0x01, 0xaa]);
    let pdu = of_octets(&octets);
    let message: ReleaseRequest = pdu.decode_value()?;
    let typed = oxirush_toy::decode::<UERANID>(&message.protocol_ies.0[0].value)?;
    assert_eq!(typed, UERANID(0x0102_0304));
    assert_eq!(pdu.ie::<ie::UE_RAN_ID>().unwrap()?, typed);
    assert_eq!(read(&pdu)?, (typed.0, None));
    let every: Vec<_> = pdu.ies::<ie::UE_RAN_ID>().collect::<Result<_, _>>()?;
    assert_eq!(every, [typed]);
    Ok(())
}

/// The walk reads what is not in the usual layout as the codec reads it: an IE that ends
/// past the octets, a criticality of no value and a value past the end are errors, and an
/// extension bit says that additions follow the container.
#[test]
fn the_walk_reads_what_is_otherwise_as_the_codec_does() {
    // An IE whose value of one octet is not there
    let pdu = of_octets(&[0x00, 0x00, 0x01, 0x00, 0x1e, 0x00, 0x01]);
    assert_eq!(
        pdu.ie::<ie::NAS_PDU>().unwrap().unwrap_err().kind(),
        ErrorKind::Decode
    );
    // A NAS-PDU of the criticality of no value, before the identifier
    let mut octets = vec![0x00, 0x00, 0x02, 0x00, 0x1e, 0xc0, 0x01, 0x00];
    octets.extend(RAN_7);
    let pdu = of_octets(&octets);
    assert!(pdu.ie::<ie::UE_RAN_ID>().unwrap().is_err());
    // A NAS-PDU that claims sixteen octets of the one that there is, before the
    // identifier
    let mut octets = vec![0x00, 0x00, 0x02, 0x00, 0x1e, 0x00, 0x10, 0x61];
    octets.extend(RAN_7);
    let pdu = of_octets(&octets);
    let read_ran = pdu.ie::<ie::UE_RAN_ID>().unwrap();
    assert_eq!(read_ran.unwrap_err().kind(), ErrorKind::Decode);
    // An identifier, then an addition of three octets
    let mut octets = vec![0x80, 0x00, 0x01];
    octets.extend(RAN_7);
    octets.extend([0x01, 0x03, 0x00, 0x00, 0x00]);
    let pdu = of_octets(&octets);
    assert!(pdu.ie::<ie::NAS_PDU>().is_none());
    assert_eq!(read(&pdu).ok(), Some((7, None)));
    // The extension bit, and no addition
    let mut octets = vec![0x80, 0x00, 0x01];
    octets.extend(RAN_7);
    let pdu = of_octets(&octets);
    let read_nas = pdu.ie::<ie::NAS_PDU>().unwrap();
    assert_eq!(read_nas.unwrap_err().kind(), ErrorKind::Decode);
    // No octets
    let pdu = of_octets(&[]);
    let error = pdu.ie::<ie::NAS_PDU>().unwrap().unwrap_err();
    assert!(error.to_string().contains("there are no octets"), "{error}");
}

/// A SETUP REQUEST with its Areas and these single containers as its Items.
fn setup_with_items(items: Vec<ProtocolIEField<'static>>) -> Result<TOYPDU<'static>, Error> {
    let area = AreaItem {
        tac: TAC([0, 0, 1]),
        broadcast_plmns: BroadcastPLMNs(vec![PLMNIdentity([0x02, 0xf8, 0x39])]),
        ie_extensions: None,
    };
    build_toy!(SetupRequest, Areas(Areas(vec![area])), Items(Items(items)),)
}

/// An Item of the identifier `id`.
fn item(id: u8) -> Item {
    Item {
        item_id: id,
        cause: None,
        ie_extensions: None,
    }
}

#[test]
fn the_items_of_a_list_are_walked_and_each_is_decoded() -> Result<(), Error> {
    let first = ie::Item::field(Criticality::Reject, item(1))?;
    let second = ie::Item::field(Criticality::Reject, item(2))?;
    let pdu = setup_with_items(vec![first, second])?;
    let items: Vec<_> = pdu.items::<ie::Items>().collect::<Result<_, _>>()?;
    assert_eq!(items, [item(1), item(2)]);
    let message: SetupRequest = pdu.decode_value()?;
    let items: Vec<_> = message.items::<ie::Items>().collect::<Result<_, _>>()?;
    assert_eq!(items, [item(1), item(2)]);
    // A single container of another IE is passed over, and an item that does not decode
    // is an error, after which the walk goes on.
    let other = field(11, Criticality::Ignore, &[0x00, 0x05]);
    let spoiled = field(23, Criticality::Reject, &[]);
    let good = ie::Item::field(Criticality::Reject, item(3))?;
    let pdu = setup_with_items(vec![other, spoiled, good])?;
    let items: Vec<_> = pdu.items::<ie::Items>().collect();
    assert_eq!(items.len(), 2);
    let error = items[0].as_ref().unwrap_err();
    assert_eq!(error.kind(), ErrorKind::UndecodableIe);
    assert_eq!(
        error.ie().map(|ie| (ie.id, ie.position)),
        Some((23, Some(1)))
    );
    assert_eq!(items[1], Ok(item(3)));
    // No list, no item.
    assert_eq!(request(vec![]).items::<ie::Items>().count(), 0);
    Ok(())
}

#[test]
fn a_list_that_ends_before_its_count_is_an_error_of_the_list() -> Result<(), Error> {
    // A count of 16 items, the most that the list has, in four bits of the count less
    // one, which no item follows.
    let list = field(22, Criticality::Reject, &[0xf0]);
    let mut pdu = setup_with_items(vec![ie::Item::field(Criticality::Reject, item(1))?])?;
    pdu.set_protocol_ies(ProtocolIEContainer(vec![list]))?;
    let items: Vec<_> = pdu.items::<ie::Items>().collect();
    assert_eq!(items.len(), 1);
    let error = items[0].as_ref().unwrap_err();
    assert_eq!(error.kind(), ErrorKind::UndecodableIe);
    assert_eq!(error.ie().map(|ie| ie.id), Some(22));
    Ok(())
}

/// The items of a list after a value in fragments, and those of a list in fragments,
/// which holds a long IE of another identifier, are walked and decoded.
#[test]
fn the_items_of_a_list_are_read_around_values_in_fragments() -> Result<(), Error> {
    let long = vec![0x5a; 20_000];
    let items = |ies| {
        field(
            22,
            Criticality::Reject,
            &oxirush_toy::encode(&Items(ies)).unwrap(),
        )
    };
    let one = |id: u8| ie::Item::field(Criticality::Reject, item(id)).unwrap();
    let mut pdu = setup_with_items(vec![one(1)])?;
    let name = field(
        25,
        Criticality::Ignore,
        &oxirush_toy::encode(&Container(long.clone().into()))?,
    );
    pdu.set_protocol_ies(ProtocolIEContainer(vec![name, items(vec![one(1), one(2)])]))?;
    let read: Vec<_> = pdu.items::<ie::Items>().collect::<Result<_, _>>()?;
    assert_eq!(read, [item(1), item(2)]);
    let other = field(99, Criticality::Ignore, &long);
    pdu.set_protocol_ies(ProtocolIEContainer(vec![items(vec![
        one(1),
        other,
        one(2),
    ])]))?;
    let read: Vec<_> = pdu.items::<ie::Items>().collect::<Result<_, _>>()?;
    assert_eq!(read, [item(1), item(2)]);
    Ok(())
}

#[test]
fn a_private_message_has_no_ies_to_read() -> Result<(), Error> {
    let message = PrivateMessage {
        private_ies: PrivateMessagePrivateIEs(vec![AnonymousPrivateMessagePrivateIEs {
            id: PrivateIEID::local(1),
            criticality: Criticality::Ignore,
            value: Octets::from(vec![1, 2]),
        }]),
    };
    let pdu = TOYPDU::initiatingMessage(InitiatingMessage {
        procedure_code: ProcedureCode(4),
        criticality: Criticality::Ignore,
        value: Octets::owned(oxirush_toy::encode(&message)?),
    });
    let error = pdu.ie::<ie::UE_RAN_ID>().unwrap().unwrap_err();
    assert_eq!(error.kind(), ErrorKind::WrongMessage);
    // Its PDU is checked for its criticality alone.
    assert!(pdu.check()?.is_empty());
    Ok(())
}

#[test]
fn the_ies_of_a_procedure_that_the_protocol_does_not_have_are_read_too() -> Result<(), Error> {
    let mut pdu = release(7, b"");
    if let TOYPDU::initiatingMessage(message) = &mut pdu {
        message.procedure_code = ProcedureCode(200);
    }
    assert_eq!(pdu.ie::<ie::UE_RAN_ID>().unwrap()?, UERANID(7));
    assert!(matches!(
        pdu.check_structure()?[..],
        [Deviation::UnknownProcedure {
            procedure_code: 200,
            ..
        }]
    ));
    Ok(())
}

/// The walk of the readers and the codec of the container read the same IEs, of values
/// of every length: in one octet of length, in two, and in fragments.
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
            let nas: Vec<u8> = (0..length).map(|at| at as u8).collect();
            ies.push(match next(4) {
                0 => ie::UE_RAN_ID::field_of::<ReleaseRequest>(next(1 << 32) as u32)?,
                1 => field(60_000, Criticality::Notify, &nas),
                _ => ie::NAS_PDU::field_of::<ReleaseRequest>(nas)?,
            });
        }
        let octets = request(ies).encode()?;
        let pdu = TOYPDU::decode(&octets)?;
        let message: ReleaseRequest = pdu.decode_value()?;
        let nas = message.protocol_ies.0.iter().filter(|ie| ie.id.0 == 30);
        let nas: Vec<Vec<u8>> = nas
            .map(|ie| oxirush_toy::decode::<NASPDU>(&ie.value).unwrap().0.to_vec())
            .collect();
        let read: Vec<Vec<u8>> = (pdu.ies::<ie::NAS_PDU>())
            .map(|nas| nas.unwrap().0.to_vec())
            .collect();
        assert_eq!(read, nas);
        let read: Vec<Vec<u8>> = (message.ies::<ie::NAS_PDU>())
            .map(|nas| nas.unwrap().0.to_vec())
            .collect();
        assert_eq!(read, nas);
        // The structure that both read is the same.
        let set =
            oxirush_toy::aper::protocol::object_set::<oxirush_toy::Toy>(Direction::Initiating, 2);
        let listed: Vec<_> = (message.protocol_ies.0.iter())
            .map(|ie| (ie.id.0, ie.criticality))
            .collect();
        let mut found = Vec::new();
        oxirush_toy::aper::check::deviations(
            set.unwrap().ies.unwrap(),
            &listed,
            &|_| (None, Faults::default()),
            &mut found,
        );
        assert_eq!(pdu.check_structure()?, found);
    }
    Ok(())
}
