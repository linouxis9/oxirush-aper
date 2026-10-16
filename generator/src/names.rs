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

//! The name of a field of the bindings, from the identifier that ASN.1 gives the
//! member: its words in lower case, with `_` between them.
//!
//! The words of an identifier are what a reader of the specification says:
//!
//! 1. A hyphen ends a word, and so does a lower-case letter or a digit before a
//!    capital: `procedureCode` is `procedure_code`, `gTP-TEID` is `gtp_teid`.
//! 2. An identifier starts with a lower-case letter whatever its first word is, so
//!    that letter is of the abbreviation that the capitals after it are: `tAC` is
//!    `tac`, `pLMNIdentity` is `plmn_identity`, `iE-Extensions` is `ie_extensions`.
//! 3. Capitals in a row are an abbreviation. The last of them starts a word when a
//!    lower-case letter follows it, `nRCellIdentity` is `nr_cell_identity`, unless
//!    the capitals are an abbreviation of [`ABBREVIATIONS`] without it and none
//!    with it: `pLMNidentity` is `plmn_identity` too. An `s` after them is their
//!    plural: `protocolIEs` is `protocol_ies`.
//! 4. Capitals in a row that are made of several abbreviations of [`ABBREVIATIONS`]
//!    are these: `uERLFReportContainer` is `ue_rlf_report_container`.
//! 5. An abbreviation that the specifications write with a lower-case letter is one
//!    word wherever it is, [`MIXED`]: `gNB-ID` is `gnb_id`, `globalgNB-ID` is
//!    `global_gnb_id`, `qosFlowLevelQoSParameters` is `qos_flow_level_qos_parameters`.
//! 6. `fiveG` and `fiveGS` are the words `five_g` and `five_gs`, and `5GS` is one
//!    word.
//!
//! A few identifiers are written otherwise than the others of their specification, as
//! `parentTImeSource`: [`WRITTEN_OTHERWISE`] has their words.

/// Abbreviations that the specifications write with a lower-case letter: each is one
/// word, wherever it is in an identifier.
pub const MIXED: &[&str] = &[
    "a2X", "AIoT", "IoT", "QoS", "QoE", "ProSe", "eNB", "gNB", "IPsec", "IPv4", "IPv6", "NRPPa",
    "LPPa", "n3IWF", "eDRX", "eCGI",
];

/// The abbreviations that the identifiers of TS 38.413 and TS 36.413 write in capitals.
/// Capitals in a row that are made of them are cut into them, and one of them before a
/// word in lower case ends there.
pub const ABBREVIATIONS: &[&str] = &[
    "5G", "5GS", "A2", "A2X", "AGF", "AIOT", "AIOTF", "AMF", "ARFCN", "ARP", "AUN3", "BAT", "CAG",
    "CCE", "CE", "CGI", "CI", "CN", "COUNT", "CSG", "D2R", "DAPS", "DCN", "DL", "DRB", "DRX",
    "EAI", "EPS", "EUTRA", "EUTRAN", "GBR", "GNSS", "GS", "GTP", "GUAMI", "GUMMEI", "HFC", "HFN",
    "HO", "ID", "IE", "IP", "IRAT", "IWF", "LA", "LAC", "LAI", "LPWUS", "LTE", "MAC", "MBS",
    "MBSFN", "MDT", "MICO", "MIMO", "MME", "MMEC", "MRB", "NAS", "NB", "NCGI", "NG", "NGAP",
    "NGRAN", "NGU", "NH", "NID", "NODE", "NPI", "NPN", "NR", "NSAG", "NSSAI", "NTN", "PC5", "PCI",
    "PDCCH", "PDCP", "PDU", "PER", "PLMN", "PNI", "PQI", "PRB", "PS", "QCI", "QI", "QMC", "R15",
    "RAB", "RAC", "RAN", "RAT", "RFP", "RFSP", "RIM", "RLF", "RNC", "RNTI", "RRC", "RS", "RSN",
    "RSPP", "RSRP", "RSRQ", "RSSI", "RV", "S1AP", "SD", "SDU", "SINR", "SL", "SMTC", "SN", "SNPN",
    "SON", "SSB", "SST", "TA", "TAC", "TAI", "TEID", "TL", "TLA", "TMGI", "TMSI", "TNAP", "TNGF",
    "TNL", "TSC", "TWAP", "TWIF", "UDP", "UE", "UL", "UP", "UTC", "UTF8", "UTRAN", "UU", "WLAN",
    "X2", "XMEID", "XMSI",
];

/// Identifiers that are written otherwise than the others of their specification, each
/// with the name that its words give: a capital inside a word, `ProSe` and `pc5` or
/// `PQI` without the capital that starts them, capitals that no hyphen cuts where the
/// other identifiers have one, `engNB` for `en-gNB`, and `sNSSAI` for `s-NSSAI`.
pub const WRITTEN_OTHERWISE: &[(&str, &str)] = &[
    ("parentTImeSource", "parent_time_source"),
    ("fiveGProSepc5QoSFlowList", "five_g_prose_pc5_qos_flow_list"),
    (
        "fiveGProSepc5LinkAggregateBitRates",
        "five_g_prose_pc5_link_aggregate_bit_rates",
    ),
    ("fiveGproSepFI", "five_g_prose_pfi"),
    (
        "fiveGproSepc5FlowBitRates",
        "five_g_prose_pc5_flow_bit_rates",
    ),
    ("fiveGproSepQI", "five_g_prose_pqi"),
    ("lPWUScNsubgroupID", "lpwus_cn_subgroup_id"),
    ("gTPTLAa", "gtp_tlaa"),
    (
        "additionalNG-UUPTNLInformation",
        "additional_ng_u_up_tnl_information",
    ),
    ("sSRSSIMeasurement", "ss_rssi_measurement"),
    ("sourceengNB", "source_en_gnb"),
    ("targetengNB", "target_en_gnb"),
    ("sNSSAI", "s_nssai"),
];

/// The words of Rust that are no name of a field.
const KEYWORDS: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "crate",
    "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl",
    "in", "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref",
    "return", "self", "static", "struct", "super", "trait", "true", "try", "type", "typeof",
    "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];

/// The name of the field of the member that ASN.1 names `identifier`: see the
/// [rule](self). A word of Rust has `_` after it.
pub fn field_name(identifier: &str) -> String {
    assert!(identifier.is_ascii(), "{identifier}");
    if let Some((_, name)) = WRITTEN_OTHERWISE
        .iter()
        .find(|(written, _)| *written == identifier)
    {
        return name.to_string();
    }
    let identifier = identifier
        .replace("fiveGS", "five-GS-")
        .replace("fiveG", "five-G-");
    let parts = identifier.split('-').filter(|part| !part.is_empty());
    let words: Vec<String> = parts.flat_map(words).collect();
    let name = words.join("_");
    match KEYWORDS.contains(&name.as_str()) {
        true => name + "_",
        false => name,
    }
}

/// The abbreviations that a run of capitals and digits is made of, the longest first;
/// `None` when it is not made of them.
fn cut(run: &str) -> Option<Vec<&str>> {
    if run.is_empty() {
        return Some(Vec::new());
    }
    for end in (1..=run.len()).rev() {
        if ABBREVIATIONS.contains(&&run[..end])
            && let Some(rest) = cut(&run[end..])
        {
            let mut cut = vec![&run[..end]];
            cut.extend(rest);
            return Some(cut);
        }
    }
    None
}

/// The abbreviations that a run of capitals is made of, or the run when it is not made
/// of them.
fn whole(run: &str) -> Vec<String> {
    let pieces = cut(run).unwrap_or_else(|| vec![run]);
    pieces.into_iter().map(str::to_string).collect()
}

/// How many of the first characters of `text` are what `of` takes.
fn span(text: &str, of: impl Fn(u8) -> bool) -> usize {
    text.bytes().take_while(|byte| of(*byte)).count()
}

/// `word` with its first letter in the other case.
fn with_first(word: &str, upper: bool) -> String {
    let first = match upper {
        true => word[..1].to_ascii_uppercase(),
        false => word[..1].to_ascii_lowercase(),
    };
    first + &word[1..]
}

/// `lower`, the letters in lower case that `rest` has after `from`, up to where an
/// abbreviation of [`MIXED`] starts in them, and with the digits after them otherwise,
/// unless those start `5G`.
fn lower_word(rest: &str, from: usize, lower: usize) -> usize {
    let starts = MIXED.iter().filter_map(|mixed| {
        let at = rest[from.max(1)..].find(mixed)? + from.max(1);
        (at < from + lower).then_some(at)
    });
    if let Some(start) = starts.min() {
        return start - from;
    }
    let after = &rest[from + lower..];
    match after.starts_with("5G") {
        true => lower,
        false => lower + span(after, |byte| byte.is_ascii_digit()),
    }
}

/// The words of a part of an identifier that has no hyphen, in lower case.
fn words(part: &str) -> Vec<String> {
    // The first letter of an identifier is in lower case whatever it starts: an
    // abbreviation has it back.
    let bytes = part.as_bytes();
    let starts_mixed = MIXED.iter().any(|mixed| part.starts_with(mixed));
    let lowered = bytes.len() > 1 && bytes[0].is_ascii_lowercase() && bytes[1].is_ascii_uppercase();
    let part = match lowered && !starts_mixed {
        true => with_first(part, true),
        false => part.to_string(),
    };
    let mut found: Vec<String> = Vec::new();
    let mut at = 0;
    while at < part.len() {
        let rest = &part[at..];
        let mixed = MIXED.iter().find(|mixed| {
            let first = at == 0
                && (rest.starts_with(&with_first(mixed, true))
                    || rest.starts_with(&with_first(mixed, false)));
            rest.starts_with(**mixed) || first
        });
        if let Some(mixed) = mixed {
            found.push(mixed.to_string());
            at += mixed.len();
            continue;
        }
        // `5G` and `5GS` start with their digit.
        let five = ["5GS", "5G"].iter().find(|five| {
            let after = rest.strip_prefix(**five);
            after.is_some_and(|after| !after.starts_with(|next: char| next.is_ascii_lowercase()))
        });
        let digits = match five {
            Some(_) => 1,
            None => 0,
        };
        let capitals = &rest[digits..];
        let mut run = match capitals.starts_with(|first: char| first.is_ascii_uppercase()) {
            true => {
                digits
                    + span(capitals, |byte| {
                        byte.is_ascii_uppercase() || byte.is_ascii_digit()
                    })
            }
            false => 0,
        };
        if run == 0 {
            let letters = span(rest, |byte| byte.is_ascii_lowercase());
            let word = match letters {
                0 => span(rest, |byte| byte.is_ascii_digit()),
                letters => lower_word(rest, 0, letters),
            };
            assert!(word > 0, "{part}");
            found.push(rest[..word].to_string());
            at += word;
            continue;
        }
        // An abbreviation in mixed case may start in the run, which then ends there.
        for mixed in MIXED {
            if let Some(start) = rest[1..].find(mixed).map(|start| start + 1)
                && start < run
            {
                run = start;
            }
        }
        let after = &rest[run..];
        let lower_follows = after.starts_with(|next: char| next.is_ascii_lowercase());
        let plural = after.starts_with('s')
            && !after[1..].starts_with(|next: char| next.is_ascii_lowercase());
        if !lower_follows {
            // The run is whole abbreviations.
            found.extend(whole(&rest[..run]));
        } else if plural {
            found.extend(whole(&rest[..run]));
            found.last_mut().expect("a run has a word").push('s');
            at += 1;
        } else if run == 1 {
            // A capitalised word.
            let letters = span(after, |byte| byte.is_ascii_lowercase());
            let word = lower_word(rest, 1, letters);
            found.push(rest[..1 + word].to_string());
            at += word;
        } else if let Some(pieces) = cut(&rest[..run - 1]) {
            // Abbreviations, then a capitalised word.
            found.extend(pieces.into_iter().map(str::to_string));
            run -= 1;
        } else if let Some(pieces) = cut(&rest[..run]) {
            // Abbreviations, then a word in lower case.
            found.extend(pieces.into_iter().map(str::to_string));
        } else {
            // Capitals that the list does not have: the last starts a word.
            found.push(rest[..run - 1].to_string());
            run -= 1;
        }
        at += run;
    }
    found.iter().map(|word| word.to_ascii_lowercase()).collect()
}

#[cfg(test)]
mod tests {
    use super::{ABBREVIATIONS, field_name};

    #[test]
    fn a_field_has_the_words_of_its_identifier() {
        for (identifier, name) in [
            // A hyphen and a capital after a lower-case letter end a word.
            ("procedureCode", "procedure_code"),
            ("gTP-TEID", "gtp_teid"),
            ("e-RAB-ID", "e_rab_id"),
            ("value", "value"),
            ("m1thresholdeventA2", "m1_thresholdevent_a2"),
            ("cdma2000OneXPilot", "cdma2000_one_x_pilot"),
            ("fiveGProSeLayer2RemoteUE", "five_g_prose_layer2_remote_ue"),
            // The first letter is of the abbreviation that the capitals after it are.
            ("tAC", "tac"),
            ("sST", "sst"),
            ("pLMNIdentity", "plmn_identity"),
            ("iE-Extensions", "ie_extensions"),
            ("aMFRegionID", "amf_region_id"),
            ("uE-NGAP-IDs", "ue_ngap_ids"),
            // The last capital of a row starts a word when a lower-case letter follows.
            ("nRCellIdentity", "nr_cell_identity"),
            ("pDUSessionID", "pdu_session_id"),
            ("tAIListForInactive", "tai_list_for_inactive"),
            ("mME-Code", "mme_code"),
            (
                "uELocationDerivedTACInNRNTN",
                "ue_location_derived_tac_in_nr_ntn",
            ),
            // Unless the capitals are an abbreviation without it and none with it.
            ("pLMNidentity", "plmn_identity"),
            ("nRencryptionAlgorithms", "nr_encryption_algorithms"),
            ("rejectedNSSAIinPLMN", "rejected_nssai_in_plmn"),
            ("sourceNGRANnodeID", "source_ngran_node_id"),
            ("dL-COUNTvalue", "dl_count_value"),
            ("sSBfrequencies", "ssb_frequencies"),
            (
                "fiveGProSeLayer2UEtoNetworkRelay",
                "five_g_prose_layer2_ue_to_network_relay",
            ),
            // An `s` after capitals is their plural.
            ("protocolIEs", "protocol_ies"),
            ("privateIEs", "private_ies"),
            ("iEsCriticalityDiagnostics", "ies_criticality_diagnostics"),
            ("servedMMECs", "served_mmecs"),
            ("gTP-TLAs", "gtp_tlas"),
            // Capitals that are several abbreviations.
            ("uERLFReportContainer", "ue_rlf_report_container"),
            ("aIoT-NASPDU", "aiot_nas_pdu"),
            (
                "signallingBasedMDTPLMNList",
                "signalling_based_mdt_plmn_list",
            ),
            (
                "additionalDLForwardingUPTNLInformation",
                "additional_dl_forwarding_up_tnl_information",
            ),
            ("receiveStatusofULPDCPSDUs", "receive_statusof_ul_pdcp_sdus"),
            ("cAGID", "cag_id"),
            ("nRCGI", "nr_cgi"),
            // Capitals that the list does not have are one word.
            ("aNPacketDelayBudgetUL", "an_packet_delay_budget_ul"),
            (
                "nR-paging-Long-eDRX-Cycle-for-RRC-INACTIVE",
                "nr_paging_long_edrx_cycle_for_rrc_inactive",
            ),
            // An abbreviation in mixed case is one word wherever it is.
            ("gNB-ID", "gnb_id"),
            ("globalgNB-ID", "global_gnb_id"),
            ("globaleNBID", "global_enb_id"),
            (
                "eNBX2TransportLayerAddresses",
                "enb_x2_transport_layer_addresses",
            ),
            ("recommendENBsForPaging", "recommend_enbs_for_paging"),
            ("qosFlowLevelQosParameters", "qos_flow_level_qos_parameters"),
            ("qoSFlowsTimedReportList", "qos_flows_timed_report_list"),
            ("qoEReference", "qoe_reference"),
            ("rVQoEReportingPath", "rv_qoe_reporting_path"),
            ("nB-IoT-Paging-eDRXCycle", "nb_iot_paging_edrx_cycle"),
            ("requestedAIoTAreaList", "requested_aiot_area_list"),
            ("a2X-PC5-QoS-Flow-List", "a2x_pc5_qos_flow_list"),
            ("n3IWF-ID", "n3iwf_id"),
            ("iPsecTLA", "ipsec_tla"),
            ("aggressoreCGI-List", "aggressor_ecgi_list"),
            // 5G and 5GS.
            ("fiveG-TMSI", "five_g_tmsi"),
            ("fiveQI", "five_qi"),
            ("fiveGSTAC", "five_gs_tac"),
            ("broadcast5GSTAI", "broadcast_5gs_tai"),
            // Identifiers that are written otherwise.
            ("parentTImeSource", "parent_time_source"),
            (
                "additionalNG-UUPTNLInformation",
                "additional_ng_u_up_tnl_information",
            ),
            ("sourceengNB", "source_en_gnb"),
            ("sNSSAI", "s_nssai"),
            ("s-NSSAI", "s_nssai"),
        ] {
            assert_eq!(field_name(identifier), name, "{identifier}");
        }
    }

    #[test]
    fn the_abbreviations_are_in_their_order() {
        assert!(ABBREVIATIONS.is_sorted(), "{ABBREVIATIONS:?}");
    }
}
