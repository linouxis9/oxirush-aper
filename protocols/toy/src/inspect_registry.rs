// Auto-generated from ASN.1 by the generator of oxirush-aper; do not edit.
pub(crate) use crate::registry::{
    IE_NAMES, IE_TYPES, ITEM_CRITICALITIES, MESSAGES, ie, ie_contents,
};
use oxirush_aper::inspect::Form;
pub(crate) const PROTOCOL: &str = "TOY";
pub(crate) const ROOT: &str = "toy";
oxirush_aper::__transfers! {
    "failureTransfer" crate::toy::FailureTransfer;
    "setupTransfer" crate::toy::SetupTransfer;
}
oxirush_aper::__carried! {
    25 "ContainerBlob" crate::toy::ContainerBlob;
}
pub(crate) fn type_form(name: &str) -> Option<Form> {
    Some(match name {
        // GTP-TEID
        "GTPTEID" => Form::Number(32),
        // PLMNIdentity
        "PLMNIdentity" => Form::Plmn,
        // TAC
        "TAC" => Form::Number(24),
        // TransportLayerAddress
        "TransportLayerAddress" => Form::Address,
        _ => return None,
    })
}
oxirush_aper::__types! {
    types {
        Algorithms = bits;
        AnonymousPrivateMessagePrivateIEs sequence { "id": PrivateIEID, "criticality": Criticality, "value": _ };
        AreaItem sequence { "tAC": TAC, "broadcastPLMNs": BroadcastPLMNs, "iE-Extensions"? extensions };
        Areas = [AreaItem];
        BroadcastPLMNs = [PLMNIdentity];
        Cause choice { "radioNetwork": CauseRadioNetwork, "nas": CauseNas, "protocol": CauseProtocol, "misc": CauseMisc, "choice-Extensions": ie };
        ConnectionItem sequence { "uE-Core-ID"? _, "uE-RAN-ID"? _, "iE-Extensions"? extensions };
        Connections = [ConnectionItem];
        ContainerBlob sequence { "tAC": TAC, "nodeName"? _, "message"? _, "kinds"? Kinds, "mixed"? Mixed, "iE-Extensions"? extensions };
        Counts sequence { "countUL": _, "counts"? _, "countDL": _, "iE-Extensions"? extensions };
        CriticalityDiagnostics sequence { "procedureCode"? _, "triggeringMessage"? TriggeringMessage, "procedureCriticality"? Criticality, "iEsCriticalityDiagnostics"? CriticalityDiagnosticsIEList, "iE-Extensions"? extensions };
        CriticalityDiagnosticsIEItem sequence { "iECriticality": Criticality, "iE-ID": _, "typeOfError": TypeOfError, "iE-Extensions"? extensions };
        CriticalityDiagnosticsIEList = [CriticalityDiagnosticsIEItem];
        ErrorIndication sequence { "protocolIEs": ies };
        FailureTransfer sequence { "cause": Cause, "iE-Extensions"? extensions };
        GTPTunnel sequence { "transportLayerAddress": TransportLayerAddress, "gTP-TEID": GTPTEID, "iE-Extensions"? extensions };
        InitiatingMessage sequence { "procedureCode": _, "criticality": Criticality, "value": _ };
        Item sequence { "item-ID": _, "cause"? Cause, "iE-Extensions"? extensions };
        Items = [ie];
        KindOfTransfer = TransferKind;
        Kinds sequence { "first": TransferKind, "second": KindOfTransfer };
        Mixed sequence { "kind": TransferKind, "cause": CauseNas };
        NodeID choice { "macro-NodeID": _, "home-NodeID": _, "unassigned": null, "choice-Extensions": ie };
        Notification sequence { "protocolIEs": ies };
        PrivateIEID choice { "local": _, "global": _ };
        PrivateMessage sequence { "privateIEs": PrivateMessagePrivateIEs };
        PrivateMessagePrivateIEs = [AnonymousPrivateMessagePrivateIEs];
        ReleaseRequest sequence { "protocolIEs": ies };
        SecurityContext sequence { "nextHopChainingCount": _, "nextHopNH": _, "iE-Extensions"? extensions };
        SetupFailure sequence { "protocolIEs": ies };
        SetupRequest sequence { "protocolIEs": ies };
        SetupResponse sequence { "protocolIEs": ies };
        SetupTransfer sequence { "protocolIEs": ies };
        SuccessfulOutcome sequence { "procedureCode": _, "criticality": Criticality, "value": _ };
        TOYPDU choice { "initiatingMessage": InitiatingMessage, "successfulOutcome": SuccessfulOutcome, "unsuccessfulOutcome": UnsuccessfulOutcome };
        TransferItem sequence { "transfer-ID": _, "setupTransfer": (SetupTransfer), "failureTransfer"? (FailureTransfer), "iE-Extensions"? extensions };
        TransferList = [TransferItem];
        TransportLayerAddress = bits;
        Tunnel choice { "gTPTunnel": GTPTunnel, "unassigned": null, "choice-Extensions": ie };
        UEIDPair sequence { "uE-Core-ID": _, "uE-RAN-ID": _, "iE-Extensions"? extensions };
        UEIDs choice { "uE-ID-pair": UEIDPair, "uE-Core-ID": _, "choice-Extensions": ie };
        UnsuccessfulOutcome sequence { "procedureCode": _, "criticality": Criticality, "value": _ };
    }
    enumerations {
        CauseMisc ["control-processing-overload", "unspecified"];
        CauseNas ["normal-release", "deregister", "unspecified"];
        CauseProtocol ["transfer-syntax-error", "abstract-syntax-error-reject", "abstract-syntax-error-ignore-and-notify", "message-not-compatible-with-receiver-state", "semantic-error", "abstract-syntax-error-falsely-constructed-message", "unspecified"];
        CauseRadioNetwork ["unspecified", "user-inactivity", "release-due-to-ran-generated-reason"];
        Criticality ["reject", "ignore", "notify"];
        ExtendedCause ["congestion", "overload"];
        Presence ["optional", "conditional", "mandatory"];
        TransferKind ["ipv4", "ipv6", "ipv4v6"];
        TriggeringMessage ["initiating-message", "successful-outcome", "unsuccessful-outcome"];
        TypeOfError ["not-understood", "missing"];
    }
}
use oxirush_aper::UNBOUNDED;
#[allow(unused_imports)]
use oxirush_aper::inspect::schema::{Field as F, Kind as K, Member as M, Text, Type as T};
/// The types of the protocol, in the order of the names that ASN.1 gives them, which the walker of the trees reads: a type is named by its row.
#[rustfmt::skip]
pub(crate) static SCHEMA: &[T] = &[
/* 0 */ T { name: "Algorithms", rust: "Algorithms", kind: K::Alias { field: F::BitString(16, 16, true), form: None, secret: false } },
/* 1 */ T { name: "AreaItem", rust: "AreaItem", kind: K::Sequence { names: &["tAC", "broadcastPLMNs", "iE-Extensions"], members: &[M { field: "tac", ty: F::Named(57), optional: false }, M { field: "broadcast_plmns", ty: F::Named(3), optional: false }, M { field: "ie_extensions", ty: F::Named(42), optional: true }], extensible: true } },
/* 2 */ T { name: "Areas", rust: "Areas", kind: K::Alias { field: F::List(&F::Named(1), 1, 16), form: None, secret: false } },
/* 3 */ T { name: "BroadcastPLMNs", rust: "BroadcastPLMNs", kind: K::Alias { field: F::List(&F::Named(36), 1, 6), form: None, secret: false } },
/* 4 */ T { name: "Cause", rust: "Cause", kind: K::Choice { names: &["radioNetwork", "nas", "protocol", "misc", "choice-Extensions"], alternatives: &[F::Named(8), F::Named(6), F::Named(7), F::Named(5), F::Named(46)], root: 5, extensible: false } },
/* 5 */ T { name: "CauseMisc", rust: "CauseMisc", kind: K::Enumerated { names: &["control-processing-overload", "unspecified"], root: 2, extensible: true } },
/* 6 */ T { name: "CauseNas", rust: "CauseNas", kind: K::Enumerated { names: &["normal-release", "deregister", "unspecified"], root: 3, extensible: true } },
/* 7 */ T { name: "CauseProtocol", rust: "CauseProtocol", kind: K::Enumerated { names: &["transfer-syntax-error", "abstract-syntax-error-reject", "abstract-syntax-error-ignore-and-notify", "message-not-compatible-with-receiver-state", "semantic-error", "abstract-syntax-error-falsely-constructed-message", "unspecified"], root: 7, extensible: true } },
/* 8 */ T { name: "CauseRadioNetwork", rust: "CauseRadioNetwork", kind: K::Enumerated { names: &["unspecified", "user-inactivity", "release-due-to-ran-generated-reason"], root: 3, extensible: true } },
/* 9 */ T { name: "ConnectionItem", rust: "ConnectionItem", kind: K::Sequence { names: &["uE-Core-ID", "uE-RAN-ID", "iE-Extensions"], members: &[M { field: "ue_core_id", ty: F::Named(66), optional: true }, M { field: "ue_ran_id", ty: F::Named(69), optional: true }, M { field: "ie_extensions", ty: F::Named(42), optional: true }], extensible: true } },
/* 10 */ T { name: "Connections", rust: "Connections", kind: K::Alias { field: F::List(&F::Named(9), 1, 65536), form: None, secret: false } },
/* 11 */ T { name: "Container", rust: "Container", kind: K::Alias { field: F::Octets(0, UNBOUNDED, false), form: None, secret: false } },
/* 12 */ T { name: "ContainerBlob", rust: "ContainerBlob", kind: K::Sequence { names: &["tAC", "nodeName", "message", "kinds", "mixed", "iE-Extensions"], members: &[M { field: "tac", ty: F::Named(57), optional: false }, M { field: "node_name", ty: F::Named(33), optional: true }, M { field: "message", ty: F::Octets(1, 9600, false), optional: true }, M { field: "kinds", ty: F::Named(29), optional: true }, M { field: "mixed", ty: F::Named(30), optional: true }, M { field: "ie_extensions", ty: F::Named(42), optional: true }], extensible: true } },
/* 13 */ T { name: "Counts", rust: "Counts", kind: K::Sequence { names: &["countUL", "counts", "countDL", "iE-Extensions"], members: &[M { field: "count_ul", ty: F::Unsigned(0, 18446744073709551615, false), optional: false }, M { field: "counts", ty: F::Unsigned(0, 255, false), optional: true }, M { field: "count_dl", ty: F::Unsigned(0, 4000000000000, true), optional: false }, M { field: "ie_extensions", ty: F::Named(42), optional: true }], extensible: true } },
/* 14 */ T { name: "Criticality", rust: "Criticality", kind: K::Enumerated { names: &["reject", "ignore", "notify"], root: 3, extensible: false } },
/* 15 */ T { name: "CriticalityDiagnostics", rust: "CriticalityDiagnostics", kind: K::Sequence { names: &["procedureCode", "triggeringMessage", "procedureCriticality", "iEsCriticalityDiagnostics", "iE-Extensions"], members: &[M { field: "procedure_code", ty: F::Named(41), optional: true }, M { field: "triggering_message", ty: F::Named(63), optional: true }, M { field: "procedure_criticality", ty: F::Named(14), optional: true }, M { field: "ies_criticality_diagnostics", ty: F::Named(17), optional: true }, M { field: "ie_extensions", ty: F::Named(42), optional: true }], extensible: true } },
/* 16 */ T { name: "CriticalityDiagnostics-IE-Item", rust: "CriticalityDiagnosticsIEItem", kind: K::Sequence { names: &["iECriticality", "iE-ID", "typeOfError", "iE-Extensions"], members: &[M { field: "ie_criticality", ty: F::Named(14), optional: false }, M { field: "ie_id", ty: F::Named(47), optional: false }, M { field: "type_of_error", ty: F::Named(65), optional: false }, M { field: "ie_extensions", ty: F::Named(42), optional: true }], extensible: true } },
/* 17 */ T { name: "CriticalityDiagnostics-IE-List", rust: "CriticalityDiagnosticsIEList", kind: K::Alias { field: F::List(&F::Named(16), 1, 256), form: None, secret: false } },
/* 18 */ T { name: "ErrorIndication", rust: "ErrorIndication", kind: K::Sequence { names: &["protocolIEs"], members: &[M { field: "protocol_ies", ty: F::Named(45), optional: false }], extensible: true } },
/* 19 */ T { name: "ExtendedCause", rust: "ExtendedCause", kind: K::Enumerated { names: &["congestion", "overload"], root: 2, extensible: true } },
/* 20 */ T { name: "FailureTransfer", rust: "FailureTransfer", kind: K::Sequence { names: &["cause", "iE-Extensions"], members: &[M { field: "cause", ty: F::Named(4), optional: false }, M { field: "ie_extensions", ty: F::Named(42), optional: true }], extensible: true } },
/* 21 */ T { name: "GTP-TEID", rust: "GTPTEID", kind: K::Alias { field: F::OctetArray(4), form: Some(Form::Number(32)), secret: false } },
/* 22 */ T { name: "GTPTunnel", rust: "GTPTunnel", kind: K::Sequence { names: &["transportLayerAddress", "gTP-TEID", "iE-Extensions"], members: &[M { field: "transport_layer_address", ty: F::Named(62), optional: false }, M { field: "gtp_teid", ty: F::Named(21), optional: false }, M { field: "ie_extensions", ty: F::Named(42), optional: true }], extensible: true } },
/* 23 */ T { name: "IndexOfItem", rust: "IndexOfItem", kind: K::Alias { field: F::Unsigned(1, 8, true), form: None, secret: false } },
/* 24 */ T { name: "IndexOfTransfer", rust: "IndexOfTransfer", kind: K::Alias { field: F::Unsigned(1, 4, true), form: None, secret: false } },
/* 25 */ T { name: "InitiatingMessage", rust: "InitiatingMessage", kind: K::Sequence { names: &["procedureCode", "criticality", "value"], members: &[M { field: "procedure_code", ty: F::Named(41), optional: false }, M { field: "criticality", ty: F::Named(14), optional: false }, M { field: "value", ty: F::Open, optional: false }], extensible: false } },
/* 26 */ T { name: "Item", rust: "Item", kind: K::Sequence { names: &["item-ID", "cause", "iE-Extensions"], members: &[M { field: "item_id", ty: F::Unsigned(0, 255, false), optional: false }, M { field: "cause", ty: F::Named(4), optional: true }, M { field: "ie_extensions", ty: F::Named(42), optional: true }], extensible: true } },
/* 27 */ T { name: "Items", rust: "Items", kind: K::Alias { field: F::List(&F::Named(46), 1, 16), form: None, secret: false } },
/* 28 */ T { name: "KindOfTransfer", rust: "KindOfTransfer", kind: K::Alias { field: F::Named(60), form: None, secret: false } },
/* 29 */ T { name: "Kinds", rust: "Kinds", kind: K::Sequence { names: &["first", "second"], members: &[M { field: "first", ty: F::Named(60), optional: false }, M { field: "second", ty: F::Named(28), optional: false }], extensible: false } },
/* 30 */ T { name: "Mixed", rust: "Mixed", kind: K::Sequence { names: &["kind", "cause"], members: &[M { field: "kind", ty: F::Named(60), optional: false }, M { field: "cause", ty: F::Named(6), optional: false }], extensible: false } },
/* 31 */ T { name: "NAS-PDU", rust: "NASPDU", kind: K::Alias { field: F::Octets(0, UNBOUNDED, false), form: None, secret: false } },
/* 32 */ T { name: "NodeID", rust: "NodeID", kind: K::Choice { names: &["macro-NodeID", "home-NodeID", "unassigned", "choice-Extensions"], alternatives: &[F::Bits(20, 20), F::Bits(28, 28), F::Null, F::Named(46)], root: 4, extensible: false } },
/* 33 */ T { name: "NodeName", rust: "NodeName", kind: K::Alias { field: F::Text(Text::Printable, 1, 150, true), form: None, secret: false } },
/* 34 */ T { name: "Notification", rust: "Notification", kind: K::Sequence { names: &["protocolIEs"], members: &[M { field: "protocol_ies", ty: F::Named(45), optional: false }], extensible: true } },
/* 35 */ T { name: "OCTET STRING", rust: "Octets", kind: K::Alias { field: F::Octets(0, UNBOUNDED, false), form: None, secret: false } },
/* 36 */ T { name: "PLMNIdentity", rust: "PLMNIdentity", kind: K::Alias { field: F::OctetArray(3), form: Some(Form::Plmn), secret: false } },
/* 37 */ T { name: "Presence", rust: "Presence", kind: K::Enumerated { names: &["optional", "conditional", "mandatory"], root: 3, extensible: false } },
/* 38 */ T { name: "PrivateIE-ID", rust: "PrivateIEID", kind: K::Choice { names: &["local", "global"], alternatives: &[F::Unsigned(0, 65535, false), F::ObjectIdentifier], root: 2, extensible: false } },
/* 39 */ T { name: "PrivateMessage", rust: "PrivateMessage", kind: K::Sequence { names: &["privateIEs"], members: &[M { field: "private_ies", ty: F::Named(40), optional: false }], extensible: true } },
/* 40 */ T { name: "PrivateMessagePrivateIEs", rust: "PrivateMessagePrivateIEs", kind: K::Alias { field: F::List(&F::Named(49), 1, 65535), form: None, secret: false } },
/* 41 */ T { name: "ProcedureCode", rust: "ProcedureCode", kind: K::Alias { field: F::Unsigned(0, 255, false), form: None, secret: false } },
/* 42 */ T { name: "ProtocolExtensionContainer", rust: "ProtocolExtensionContainer", kind: K::Alias { field: F::List(&F::Named(43), 1, 65535), form: None, secret: false } },
/* 43 */ T { name: "ProtocolExtensionField", rust: "ProtocolExtensionField", kind: K::Sequence { names: &["id", "criticality", "extensionValue"], members: &[M { field: "id", ty: F::Named(44), optional: false }, M { field: "criticality", ty: F::Named(14), optional: false }, M { field: "extension_value", ty: F::Open, optional: false }], extensible: false } },
/* 44 */ T { name: "ProtocolExtensionID", rust: "ProtocolExtensionID", kind: K::Alias { field: F::Unsigned(0, 65535, false), form: None, secret: false } },
/* 45 */ T { name: "ProtocolIE-Container", rust: "ProtocolIEContainer", kind: K::Alias { field: F::List(&F::Named(46), 0, 65535), form: None, secret: false } },
/* 46 */ T { name: "ProtocolIE-Field", rust: "ProtocolIEField", kind: K::Sequence { names: &["id", "criticality", "value"], members: &[M { field: "id", ty: F::Named(47), optional: false }, M { field: "criticality", ty: F::Named(14), optional: false }, M { field: "value", ty: F::Open, optional: false }], extensible: false } },
/* 47 */ T { name: "ProtocolIE-ID", rust: "ProtocolIEID", kind: K::Alias { field: F::Unsigned(0, 65535, false), form: None, secret: false } },
/* 48 */ T { name: "ReleaseRequest", rust: "ReleaseRequest", kind: K::Sequence { names: &["protocolIEs"], members: &[M { field: "protocol_ies", ty: F::Named(45), optional: false }], extensible: true } },
/* 49 */ T { name: "SEQUENCE", rust: "AnonymousPrivateMessagePrivateIEs", kind: K::Sequence { names: &["id", "criticality", "value"], members: &[M { field: "id", ty: F::Named(38), optional: false }, M { field: "criticality", ty: F::Named(14), optional: false }, M { field: "value", ty: F::Open, optional: false }], extensible: false } },
/* 50 */ T { name: "SecurityContext", rust: "SecurityContext", kind: K::Sequence { names: &["nextHopChainingCount", "nextHopNH", "iE-Extensions"], members: &[M { field: "next_hop_chaining_count", ty: F::Unsigned(0, 7, false), optional: false }, M { field: "next_hop_nh", ty: F::Named(51), optional: false }, M { field: "ie_extensions", ty: F::Named(42), optional: true }], extensible: true } },
/* 51 */ T { name: "SecurityKey", rust: "SecurityKey", kind: K::Alias { field: F::BitString(256, 256, false), form: None, secret: true } },
/* 52 */ T { name: "SetupFailure", rust: "SetupFailure", kind: K::Sequence { names: &["protocolIEs"], members: &[M { field: "protocol_ies", ty: F::Named(45), optional: false }], extensible: true } },
/* 53 */ T { name: "SetupRequest", rust: "SetupRequest", kind: K::Sequence { names: &["protocolIEs"], members: &[M { field: "protocol_ies", ty: F::Named(45), optional: false }], extensible: true } },
/* 54 */ T { name: "SetupResponse", rust: "SetupResponse", kind: K::Sequence { names: &["protocolIEs"], members: &[M { field: "protocol_ies", ty: F::Named(45), optional: false }], extensible: true } },
/* 55 */ T { name: "SetupTransfer", rust: "SetupTransfer", kind: K::Sequence { names: &["protocolIEs"], members: &[M { field: "protocol_ies", ty: F::Named(45), optional: false }], extensible: true } },
/* 56 */ T { name: "SuccessfulOutcome", rust: "SuccessfulOutcome", kind: K::Sequence { names: &["procedureCode", "criticality", "value"], members: &[M { field: "procedure_code", ty: F::Named(41), optional: false }, M { field: "criticality", ty: F::Named(14), optional: false }, M { field: "value", ty: F::Open, optional: false }], extensible: false } },
/* 57 */ T { name: "TAC", rust: "TAC", kind: K::Alias { field: F::OctetArray(3), form: Some(Form::Number(24)), secret: false } },
/* 58 */ T { name: "TOY-PDU", rust: "TOYPDU", kind: K::Choice { names: &["initiatingMessage", "successfulOutcome", "unsuccessfulOutcome"], alternatives: &[F::Named(25), F::Named(56), F::Named(70)], root: 3, extensible: true } },
/* 59 */ T { name: "TransferItem", rust: "TransferItem", kind: K::Sequence { names: &["transfer-ID", "setupTransfer", "failureTransfer", "iE-Extensions"], members: &[M { field: "transfer_id", ty: F::Unsigned(0, 255, false), optional: false }, M { field: "setup_transfer", ty: F::Octets(0, UNBOUNDED, false), optional: false }, M { field: "failure_transfer", ty: F::Octets(0, UNBOUNDED, false), optional: true }, M { field: "ie_extensions", ty: F::Named(42), optional: true }], extensible: true } },
/* 60 */ T { name: "TransferKind", rust: "TransferKind", kind: K::Enumerated { names: &["ipv4", "ipv6", "ipv4v6"], root: 2, extensible: true } },
/* 61 */ T { name: "TransferList", rust: "TransferList", kind: K::Alias { field: F::List(&F::Named(59), 1, 8), form: None, secret: false } },
/* 62 */ T { name: "TransportLayerAddress", rust: "TransportLayerAddress", kind: K::Alias { field: F::BitString(1, 160, true), form: Some(Form::Address), secret: false } },
/* 63 */ T { name: "TriggeringMessage", rust: "TriggeringMessage", kind: K::Enumerated { names: &["initiating-message", "successful-outcome", "unsuccessful-outcome"], root: 3, extensible: false } },
/* 64 */ T { name: "Tunnel", rust: "Tunnel", kind: K::Choice { names: &["gTPTunnel", "unassigned", "choice-Extensions"], alternatives: &[F::Named(22), F::Null, F::Named(46)], root: 3, extensible: false } },
/* 65 */ T { name: "TypeOfError", rust: "TypeOfError", kind: K::Enumerated { names: &["not-understood", "missing"], root: 2, extensible: true } },
/* 66 */ T { name: "UE-Core-ID", rust: "UECoreID", kind: K::Alias { field: F::Unsigned(0, 1099511627775, false), form: None, secret: false } },
/* 67 */ T { name: "UE-ID-pair", rust: "UEIDPair", kind: K::Sequence { names: &["uE-Core-ID", "uE-RAN-ID", "iE-Extensions"], members: &[M { field: "ue_core_id", ty: F::Named(66), optional: false }, M { field: "ue_ran_id", ty: F::Named(69), optional: false }, M { field: "ie_extensions", ty: F::Named(42), optional: true }], extensible: true } },
/* 68 */ T { name: "UE-IDs", rust: "UEIDs", kind: K::Choice { names: &["uE-ID-pair", "uE-Core-ID", "choice-Extensions"], alternatives: &[F::Named(67), F::Named(66), F::Named(46)], root: 3, extensible: false } },
/* 69 */ T { name: "UE-RAN-ID", rust: "UERANID", kind: K::Alias { field: F::Unsigned(0, 4294967295, false), form: None, secret: false } },
/* 70 */ T { name: "UnsuccessfulOutcome", rust: "UnsuccessfulOutcome", kind: K::Sequence { names: &["procedureCode", "criticality", "value"], members: &[M { field: "procedure_code", ty: F::Named(41), optional: false }, M { field: "criticality", ty: F::Named(14), optional: false }, M { field: "value", ty: F::Open, optional: false }], extensible: false } },
];
