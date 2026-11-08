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

//! How an identity prints. It is printed whole, and as it is read: a PLMN identity as
//! its MCC and its MNC, and a fixed number of octets, as a TAC, in hexadecimal, where
//! `Debug` derived for a type gave a list of numbers.

use oxirush_toy::identity::Plmn;
use oxirush_toy::toy::*;

/// The PLMN identity of an MCC and an MNC.
fn plmn(mcc: &str, mnc: &str) -> PLMNIdentity {
    PLMNIdentity::from(format!("{mcc}-{mnc}").parse::<Plmn>().unwrap())
}

#[test]
fn a_plmn_identity_prints_as_its_mcc_and_its_mnc() {
    let identity = plmn("208", "93");
    assert_eq!(identity.0, [0x02, 0xf8, 0x39]);
    assert_eq!(
        format!("{identity} {identity:?}"),
        "208-93 PLMNIdentity(208-93)"
    );
    // An MNC of three digits, and one that starts with a zero
    assert_eq!(plmn("310", "410").to_string(), "310-410");
    assert_eq!(plmn("208", "093").to_string(), "208-093");

    // In a value that holds one
    let area = AreaItem {
        tac: TAC([0, 0, 1]),
        broadcast_plmns: BroadcastPLMNs(vec![identity]),
        ie_extensions: None,
    };
    let printed = format!("{area:?}");
    assert!(printed.contains("PLMNIdentity(208-93)"), "{printed}");
    assert!(printed.contains("TAC(000001)"), "{printed}");

    // Octets that are no PLMN identity print as the octets that they are
    let none = PLMNIdentity([0xab, 0xcd, 0xef]);
    assert_eq!(format!("{none} {none:?}"), "abcdef PLMNIdentity(abcdef)");
}

/// TS 38.413 clause 9.3.3.5 and TS 36.413 clause 9.2.3.8: octet 2 has the first digit
/// of an MNC of three digits, and octet 3 its second and its third.
#[test]
fn a_plmn_identity_converts_to_and_from_a_plmn() {
    let network: Plmn = "310-410".parse().unwrap();
    let identity = PLMNIdentity::from(network);
    assert_eq!(identity.0, [0x13, 0x40, 0x01]);
    assert_eq!(Plmn::try_from(identity), Ok(network));
    assert_eq!((network.mcc(), network.mnc()), ("310", "410"));

    // A filler in the MNC: no PLMN, and the error says the member
    let error = Plmn::try_from(PLMNIdentity([0x02, 0xf8, 0xf9])).unwrap_err();
    assert_eq!(error.member, "mnc");
}

#[test]
fn a_fixed_number_of_octets_prints_in_hexadecimal() {
    let tac = TAC([0x00, 0x00, 0x01]);
    assert_eq!(format!("{tac} {tac:?}"), "000001 TAC(000001)");
    assert_eq!(format!("{:?}", GTPTEID([0, 0, 0, 1])), "GTPTEID(00000001)");
}
