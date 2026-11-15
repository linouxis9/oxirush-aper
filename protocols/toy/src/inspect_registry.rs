// Auto-generated from ASN.1 by the generator of oxirush-aper; do not edit.
pub(crate) use crate::registry::{
    IE_NAMES, IE_TYPES, ITEM_CRITICALITIES, MESSAGES, ie, ie_contents,
};
pub(crate) const PROTOCOL: &str = "TOY";
pub(crate) const ROOT: &str = "toy";
oxirush_aper::__transfers! {
    "failureTransfer" crate::toy::FailureTransfer;
    "setupTransfer" crate::toy::SetupTransfer;
}
oxirush_aper::__carried! {
    25 "ContainerBlob" crate::toy::ContainerBlob;
}
oxirush_aper::__extension_sets! {
    "AreaItem" [];
    "ConnectionItem" [];
    "ContainerBlob" [];
    "Counts" [];
    "CriticalityDiagnostics" [];
    "CriticalityDiagnosticsIEItem" [];
    "FailureTransfer" [];
    "GTPTunnel" [];
    "Item" [40 ignore optional];
    "SecurityContext" [];
    "TransferItem" [40 ignore optional];
    "UEIDPair" [];
}
/// Each IE by the letters and the digits of its name, in lower case, as an index of `IE_NAMES`.
#[rustfmt::skip]
pub(crate) static IE_HASH: oxirush_aper::inspect::Hashed = oxirush_aper::inspect::Hashed {
    keys: &["uecoreid", "ueranid", "ueids", "cause", "criticalitydiagnostics", "nodename", "securitykey", "items", "item", "transferlist", "container", "areas", "algorithms", "counts", "tunnel", "naspdu", "transferkind", "nodeid", "securitycontext", "failuretransfer", "setuptransfer", "extensionindex", "extendedcause", "tagged", "connections"],
    displacements: &[49, 17, 0, 2, 11, 54, 2],
    slots: &[4, 16, 15, 1, 19, 0, 17, 5, 11, 22, 12, 6, 21, 13, 65535, 65535, 20, 2, 3, 8, 65535, 24, 18, 14, 23, 10, 9, 65535, 65535, 7],
};
use oxirush_aper::UNBOUNDED;
#[allow(unused_imports)]
use oxirush_aper::inspect::Form;
#[allow(unused_imports)]
use oxirush_aper::inspect::schema::{Field as F, Kind as K, Member as M, Text, Type as T};
/// The types of the protocol, in the order of the names that ASN.1 gives them, which the walker of the trees reads: a type is named by its row.
#[rustfmt::skip]
pub(crate) static SCHEMA: &[T] = &[
/* 0 */ T { name: "Algorithms", rust: "Algorithms", kind: K::Alias { field: F::BitString(16, 16, true), form: None, secret: false } },
/* 1 */ T { name: "AreaItem", rust: "AreaItem", kind: K::Sequence { names: &["tAC", "broadcastPLMNs", "iE-Extensions"], members: &[M { field: "tac", ty: F::Named(64), optional: false }, M { field: "broadcast_plmns", ty: F::Named(3), optional: false }, M { field: "ie_extensions", ty: F::Named(48), optional: true }], extensible: true } },
/* 2 */ T { name: "Areas", rust: "Areas", kind: K::Alias { field: F::List(&F::Named(1), 1, 16), form: None, secret: false } },
/* 3 */ T { name: "BroadcastPLMNs", rust: "BroadcastPLMNs", kind: K::Alias { field: F::List(&F::Named(42), 1, 6), form: None, secret: false } },
/* 4 */ T { name: "Cause", rust: "Cause", kind: K::Choice { names: &["radioNetwork", "nas", "protocol", "misc", "choice-Extensions"], alternatives: &[F::Named(8), F::Named(6), F::Named(7), F::Named(5), F::Named(52)], root: 5, extensible: false } },
/* 5 */ T { name: "CauseMisc", rust: "CauseMisc", kind: K::Enumerated { names: &["control-processing-overload", "unspecified"], root: 2, extensible: true } },
/* 6 */ T { name: "CauseNas", rust: "CauseNas", kind: K::Enumerated { names: &["normal-release", "deregister", "unspecified"], root: 3, extensible: true } },
/* 7 */ T { name: "CauseProtocol", rust: "CauseProtocol", kind: K::Enumerated { names: &["transfer-syntax-error", "abstract-syntax-error-reject", "abstract-syntax-error-ignore-and-notify", "message-not-compatible-with-receiver-state", "semantic-error", "abstract-syntax-error-falsely-constructed-message", "unspecified"], root: 7, extensible: true } },
/* 8 */ T { name: "CauseRadioNetwork", rust: "CauseRadioNetwork", kind: K::Enumerated { names: &["unspecified", "user-inactivity", "release-due-to-ran-generated-reason"], root: 3, extensible: true } },
/* 9 */ T { name: "CellIdentity", rust: "CellIdentity", kind: K::Alias { field: F::Bits(28, 28), form: Some(Form::Number(28)), secret: false } },
/* 10 */ T { name: "Code", rust: "Code", kind: K::Alias { field: F::OctetArray(2), form: None, secret: false } },
/* 11 */ T { name: "ConnectionItem", rust: "ConnectionItem", kind: K::Sequence { names: &["uE-Core-ID", "uE-RAN-ID", "iE-Extensions"], members: &[M { field: "ue_core_id", ty: F::Named(74), optional: true }, M { field: "ue_ran_id", ty: F::Named(77), optional: true }, M { field: "ie_extensions", ty: F::Named(48), optional: true }], extensible: true } },
/* 12 */ T { name: "Connections", rust: "Connections", kind: K::Alias { field: F::List(&F::Named(11), 1, 65536), form: None, secret: false } },
/* 13 */ T { name: "Container", rust: "Container", kind: K::Alias { field: F::Octets(0, UNBOUNDED, false), form: None, secret: false } },
/* 14 */ T { name: "ContainerBlob", rust: "ContainerBlob", kind: K::Sequence { names: &["tAC", "nodeName", "message", "kinds", "mixed", "iE-Extensions"], members: &[M { field: "tac", ty: F::Named(64), optional: false }, M { field: "node_name", ty: F::Named(39), optional: true }, M { field: "message", ty: F::Octets(1, 9600, false), optional: true }, M { field: "kinds", ty: F::Named(33), optional: true }, M { field: "mixed", ty: F::Named(35), optional: true }, M { field: "ie_extensions", ty: F::Named(48), optional: true }], extensible: true } },
/* 15 */ T { name: "CoreID", rust: "CoreID", kind: K::Alias { field: F::Named(74), form: None, secret: false } },
/* 16 */ T { name: "Counts", rust: "Counts", kind: K::Sequence { names: &["countUL", "counts", "countDL", "iE-Extensions"], members: &[M { field: "count_ul", ty: F::Unsigned(0, 18446744073709551615, false), optional: false }, M { field: "counts", ty: F::Unsigned(0, 255, false), optional: true }, M { field: "count_dl", ty: F::Unsigned(0, 4000000000000, true), optional: false }, M { field: "ie_extensions", ty: F::Named(48), optional: true }], extensible: true } },
/* 17 */ T { name: "Criticality", rust: "Criticality", kind: K::Enumerated { names: &["reject", "ignore", "notify"], root: 3, extensible: false } },
/* 18 */ T { name: "CriticalityDiagnostics", rust: "CriticalityDiagnostics", kind: K::Sequence { names: &["procedureCode", "triggeringMessage", "procedureCriticality", "iEsCriticalityDiagnostics", "iE-Extensions"], members: &[M { field: "procedure_code", ty: F::Named(47), optional: true }, M { field: "triggering_message", ty: F::Named(71), optional: true }, M { field: "procedure_criticality", ty: F::Named(17), optional: true }, M { field: "ies_criticality_diagnostics", ty: F::Named(20), optional: true }, M { field: "ie_extensions", ty: F::Named(48), optional: true }], extensible: true } },
/* 19 */ T { name: "CriticalityDiagnostics-IE-Item", rust: "CriticalityDiagnosticsIEItem", kind: K::Sequence { names: &["iECriticality", "iE-ID", "typeOfError", "iE-Extensions"], members: &[M { field: "ie_criticality", ty: F::Named(17), optional: false }, M { field: "ie_id", ty: F::Named(53), optional: false }, M { field: "type_of_error", ty: F::Named(73), optional: false }, M { field: "ie_extensions", ty: F::Named(48), optional: true }], extensible: true } },
/* 20 */ T { name: "CriticalityDiagnostics-IE-List", rust: "CriticalityDiagnosticsIEList", kind: K::Alias { field: F::List(&F::Named(19), 1, 256), form: None, secret: false } },
/* 21 */ T { name: "ErrorIndication", rust: "ErrorIndication", kind: K::Sequence { names: &["protocolIEs"], members: &[M { field: "protocol_ies", ty: F::Named(51), optional: false }], extensible: true } },
/* 22 */ T { name: "ExtendedCause", rust: "ExtendedCause", kind: K::Enumerated { names: &["congestion", "overload"], root: 2, extensible: true } },
/* 23 */ T { name: "FailureTransfer", rust: "FailureTransfer", kind: K::Sequence { names: &["cause", "iE-Extensions"], members: &[M { field: "cause", ty: F::Named(4), optional: false }, M { field: "ie_extensions", ty: F::Named(48), optional: true }], extensible: true } },
/* 24 */ T { name: "GTP-TEID", rust: "GTPTEID", kind: K::Alias { field: F::OctetArray(4), form: Some(Form::Number(32)), secret: false } },
/* 25 */ T { name: "GTPTunnel", rust: "GTPTunnel", kind: K::Sequence { names: &["transportLayerAddress", "gTP-TEID", "iE-Extensions"], members: &[M { field: "transport_layer_address", ty: F::Named(70), optional: false }, M { field: "gtp_teid", ty: F::Named(24), optional: false }, M { field: "ie_extensions", ty: F::Named(48), optional: true }], extensible: true } },
/* 26 */ T { name: "IMSI", rust: "IMSI", kind: K::Alias { field: F::Octets(3, 8, false), form: Some(Form::Digits), secret: false } },
/* 27 */ T { name: "IndexOfItem", rust: "IndexOfItem", kind: K::Alias { field: F::Unsigned(1, 8, true), form: None, secret: false } },
/* 28 */ T { name: "IndexOfTransfer", rust: "IndexOfTransfer", kind: K::Alias { field: F::Unsigned(1, 4, true), form: None, secret: false } },
/* 29 */ T { name: "InitiatingMessage", rust: "InitiatingMessage", kind: K::Sequence { names: &["procedureCode", "criticality", "value"], members: &[M { field: "procedure_code", ty: F::Named(47), optional: false }, M { field: "criticality", ty: F::Named(17), optional: false }, M { field: "value", ty: F::Open, optional: false }], extensible: false } },
/* 30 */ T { name: "Item", rust: "Item", kind: K::Sequence { names: &["item-ID", "cause", "iE-Extensions"], members: &[M { field: "item_id", ty: F::Unsigned(0, 255, false), optional: false }, M { field: "cause", ty: F::Named(4), optional: true }, M { field: "ie_extensions", ty: F::Named(48), optional: true }], extensible: true } },
/* 31 */ T { name: "Items", rust: "Items", kind: K::Alias { field: F::List(&F::Named(52), 1, 16), form: None, secret: false } },
/* 32 */ T { name: "KindOfTransfer", rust: "KindOfTransfer", kind: K::Alias { field: F::Named(68), form: None, secret: false } },
/* 33 */ T { name: "Kinds", rust: "Kinds", kind: K::Sequence { names: &["first", "second"], members: &[M { field: "first", ty: F::Named(68), optional: false }, M { field: "second", ty: F::Named(32), optional: false }], extensible: false } },
/* 34 */ T { name: "Level", rust: "Level", kind: K::Enumerated { names: &["low", "high", "top"], root: 2, extensible: true } },
/* 35 */ T { name: "Mixed", rust: "Mixed", kind: K::Sequence { names: &["kind", "cause"], members: &[M { field: "kind", ty: F::Named(68), optional: false }, M { field: "cause", ty: F::Named(6), optional: false }], extensible: false } },
/* 36 */ T { name: "Mode", rust: "Mode", kind: K::Choice { names: &["first", "second", "third"], alternatives: &[F::Unsigned(0, 7, false), F::Null, F::Boolean], root: 2, extensible: true } },
/* 37 */ T { name: "NAS-PDU", rust: "NASPDU", kind: K::Alias { field: F::Octets(0, UNBOUNDED, false), form: None, secret: false } },
/* 38 */ T { name: "NodeID", rust: "NodeID", kind: K::Choice { names: &["macro-NodeID", "home-NodeID", "unassigned", "choice-Extensions"], alternatives: &[F::Bits(20, 20), F::Bits(28, 28), F::Null, F::Named(52)], root: 4, extensible: false } },
/* 39 */ T { name: "NodeName", rust: "NodeName", kind: K::Alias { field: F::Text(Text::Printable, 1, 150, true), form: None, secret: false } },
/* 40 */ T { name: "Notification", rust: "Notification", kind: K::Sequence { names: &["protocolIEs"], members: &[M { field: "protocol_ies", ty: F::Named(51), optional: false }], extensible: true } },
/* 41 */ T { name: "OCTET STRING", rust: "Octets", kind: K::Alias { field: F::Octets(0, UNBOUNDED, false), form: None, secret: false } },
/* 42 */ T { name: "PLMNIdentity", rust: "PLMNIdentity", kind: K::Alias { field: F::OctetArray(3), form: Some(Form::Plmn), secret: false } },
/* 43 */ T { name: "Presence", rust: "Presence", kind: K::Enumerated { names: &["optional", "conditional", "mandatory"], root: 3, extensible: false } },
/* 44 */ T { name: "PrivateIE-ID", rust: "PrivateIEID", kind: K::Choice { names: &["local", "global"], alternatives: &[F::Unsigned(0, 65535, false), F::ObjectIdentifier], root: 2, extensible: false } },
/* 45 */ T { name: "PrivateMessage", rust: "PrivateMessage", kind: K::Sequence { names: &["privateIEs"], members: &[M { field: "private_ies", ty: F::Named(46), optional: false }], extensible: true } },
/* 46 */ T { name: "PrivateMessagePrivateIEs", rust: "PrivateMessagePrivateIEs", kind: K::Alias { field: F::List(&F::Named(56), 1, 65535), form: None, secret: false } },
/* 47 */ T { name: "ProcedureCode", rust: "ProcedureCode", kind: K::Alias { field: F::Unsigned(0, 255, false), form: None, secret: false } },
/* 48 */ T { name: "ProtocolExtensionContainer", rust: "ProtocolExtensionContainer", kind: K::Alias { field: F::List(&F::Named(49), 1, 65535), form: None, secret: false } },
/* 49 */ T { name: "ProtocolExtensionField", rust: "ProtocolExtensionField", kind: K::Sequence { names: &["id", "criticality", "extensionValue"], members: &[M { field: "id", ty: F::Named(50), optional: false }, M { field: "criticality", ty: F::Named(17), optional: false }, M { field: "extension_value", ty: F::Open, optional: false }], extensible: false } },
/* 50 */ T { name: "ProtocolExtensionID", rust: "ProtocolExtensionID", kind: K::Alias { field: F::Unsigned(0, 65535, false), form: None, secret: false } },
/* 51 */ T { name: "ProtocolIE-Container", rust: "ProtocolIEContainer", kind: K::Alias { field: F::List(&F::Named(52), 0, 65535), form: None, secret: false } },
/* 52 */ T { name: "ProtocolIE-Field", rust: "ProtocolIEField", kind: K::Sequence { names: &["id", "criticality", "value"], members: &[M { field: "id", ty: F::Named(53), optional: false }, M { field: "criticality", ty: F::Named(17), optional: false }, M { field: "value", ty: F::Open, optional: false }], extensible: false } },
/* 53 */ T { name: "ProtocolIE-ID", rust: "ProtocolIEID", kind: K::Alias { field: F::Unsigned(0, 65535, false), form: None, secret: false } },
/* 54 */ T { name: "RAC", rust: "RAC", kind: K::Alias { field: F::OctetArray(1), form: Some(Form::Number(8)), secret: false } },
/* 55 */ T { name: "ReleaseRequest", rust: "ReleaseRequest", kind: K::Sequence { names: &["protocolIEs"], members: &[M { field: "protocol_ies", ty: F::Named(51), optional: false }], extensible: true } },
/* 56 */ T { name: "SEQUENCE", rust: "AnonymousPrivateMessagePrivateIEs", kind: K::Sequence { names: &["id", "criticality", "value"], members: &[M { field: "id", ty: F::Named(44), optional: false }, M { field: "criticality", ty: F::Named(17), optional: false }, M { field: "value", ty: F::Open, optional: false }], extensible: false } },
/* 57 */ T { name: "SecurityContext", rust: "SecurityContext", kind: K::Sequence { names: &["nextHopChainingCount", "nextHopNH", "iE-Extensions"], members: &[M { field: "next_hop_chaining_count", ty: F::Unsigned(0, 7, false), optional: false }, M { field: "next_hop_nh", ty: F::Named(58), optional: false }, M { field: "ie_extensions", ty: F::Named(48), optional: true }], extensible: true } },
/* 58 */ T { name: "SecurityKey", rust: "SecurityKey", kind: K::Alias { field: F::BitString(256, 256, false), form: None, secret: true } },
/* 59 */ T { name: "SetupFailure", rust: "SetupFailure", kind: K::Sequence { names: &["protocolIEs"], members: &[M { field: "protocol_ies", ty: F::Named(51), optional: false }], extensible: true } },
/* 60 */ T { name: "SetupRequest", rust: "SetupRequest", kind: K::Sequence { names: &["protocolIEs"], members: &[M { field: "protocol_ies", ty: F::Named(51), optional: false }], extensible: true } },
/* 61 */ T { name: "SetupResponse", rust: "SetupResponse", kind: K::Sequence { names: &["protocolIEs"], members: &[M { field: "protocol_ies", ty: F::Named(51), optional: false }], extensible: true } },
/* 62 */ T { name: "SetupTransfer", rust: "SetupTransfer", kind: K::Sequence { names: &["protocolIEs"], members: &[M { field: "protocol_ies", ty: F::Named(51), optional: false }], extensible: true } },
/* 63 */ T { name: "SuccessfulOutcome", rust: "SuccessfulOutcome", kind: K::Sequence { names: &["procedureCode", "criticality", "value"], members: &[M { field: "procedure_code", ty: F::Named(47), optional: false }, M { field: "criticality", ty: F::Named(17), optional: false }, M { field: "value", ty: F::Open, optional: false }], extensible: false } },
/* 64 */ T { name: "TAC", rust: "TAC", kind: K::Alias { field: F::OctetArray(3), form: Some(Form::Number(24)), secret: false } },
/* 65 */ T { name: "TOY-PDU", rust: "TOYPDU", kind: K::Choice { names: &["initiatingMessage", "successfulOutcome", "unsuccessfulOutcome"], alternatives: &[F::Named(29), F::Named(63), F::Named(78)], root: 3, extensible: true } },
/* 66 */ T { name: "Tagged", rust: "Tagged", kind: K::Sequence { names: &["flag", "rac", "code", "cell", "level", "mode", "octet", "maybe", "some"], members: &[M { field: "flag", ty: F::Boolean, optional: false }, M { field: "rac", ty: F::Named(54), optional: false }, M { field: "code", ty: F::Named(10), optional: false }, M { field: "cell", ty: F::Named(9), optional: false }, M { field: "level", ty: F::Named(34), optional: false }, M { field: "mode", ty: F::Named(36), optional: false }, M { field: "octet", ty: F::Bits(8, 8), optional: false }, M { field: "maybe", ty: F::Bits(8, 8), optional: true }, M { field: "some", ty: F::Bits(1, 16), optional: false }], extensible: false } },
/* 67 */ T { name: "TransferItem", rust: "TransferItem", kind: K::Sequence { names: &["transfer-ID", "setupTransfer", "failureTransfer", "iE-Extensions"], members: &[M { field: "transfer_id", ty: F::Unsigned(0, 255, false), optional: false }, M { field: "setup_transfer", ty: F::Octets(0, UNBOUNDED, false), optional: false }, M { field: "failure_transfer", ty: F::Octets(0, UNBOUNDED, false), optional: true }, M { field: "ie_extensions", ty: F::Named(48), optional: true }], extensible: true } },
/* 68 */ T { name: "TransferKind", rust: "TransferKind", kind: K::Enumerated { names: &["ipv4", "ipv6", "ipv4v6"], root: 2, extensible: true } },
/* 69 */ T { name: "TransferList", rust: "TransferList", kind: K::Alias { field: F::List(&F::Named(67), 1, 8), form: None, secret: false } },
/* 70 */ T { name: "TransportLayerAddress", rust: "TransportLayerAddress", kind: K::Alias { field: F::BitString(1, 160, true), form: Some(Form::Address), secret: false } },
/* 71 */ T { name: "TriggeringMessage", rust: "TriggeringMessage", kind: K::Enumerated { names: &["initiating-message", "successful-outcome", "unsuccessful-outcome"], root: 3, extensible: false } },
/* 72 */ T { name: "Tunnel", rust: "Tunnel", kind: K::Choice { names: &["gTPTunnel", "unassigned", "choice-Extensions"], alternatives: &[F::Named(25), F::Null, F::Named(52)], root: 3, extensible: false } },
/* 73 */ T { name: "TypeOfError", rust: "TypeOfError", kind: K::Enumerated { names: &["not-understood", "missing"], root: 2, extensible: true } },
/* 74 */ T { name: "UE-Core-ID", rust: "UECoreID", kind: K::Alias { field: F::Unsigned(0, 1099511627775, false), form: None, secret: false } },
/* 75 */ T { name: "UE-ID-pair", rust: "UEIDPair", kind: K::Sequence { names: &["uE-Core-ID", "uE-RAN-ID", "iE-Extensions"], members: &[M { field: "ue_core_id", ty: F::Named(74), optional: false }, M { field: "ue_ran_id", ty: F::Named(77), optional: false }, M { field: "ie_extensions", ty: F::Named(48), optional: true }], extensible: true } },
/* 76 */ T { name: "UE-IDs", rust: "UEIDs", kind: K::Choice { names: &["uE-ID-pair", "uE-Core-ID", "choice-Extensions"], alternatives: &[F::Named(75), F::Named(74), F::Named(52)], root: 3, extensible: false } },
/* 77 */ T { name: "UE-RAN-ID", rust: "UERANID", kind: K::Alias { field: F::Unsigned(0, 4294967295, false), form: None, secret: false } },
/* 78 */ T { name: "UnsuccessfulOutcome", rust: "UnsuccessfulOutcome", kind: K::Sequence { names: &["procedureCode", "criticality", "value"], members: &[M { field: "procedure_code", ty: F::Named(47), optional: false }, M { field: "criticality", ty: F::Named(17), optional: false }, M { field: "value", ty: F::Open, optional: false }], extensible: false } },
];
/// Each type of `SCHEMA` by its name in the bindings, as its row.
#[rustfmt::skip]
pub(crate) static TYPE_HASH: oxirush_aper::inspect::Hashed = oxirush_aper::inspect::Hashed {
    keys: &["Algorithms", "AreaItem", "Areas", "BroadcastPLMNs", "Cause", "CauseMisc", "CauseNas", "CauseProtocol", "CauseRadioNetwork", "CellIdentity", "Code", "ConnectionItem", "Connections", "Container", "ContainerBlob", "CoreID", "Counts", "Criticality", "CriticalityDiagnostics", "CriticalityDiagnosticsIEItem", "CriticalityDiagnosticsIEList", "ErrorIndication", "ExtendedCause", "FailureTransfer", "GTPTEID", "GTPTunnel", "IMSI", "IndexOfItem", "IndexOfTransfer", "InitiatingMessage", "Item", "Items", "KindOfTransfer", "Kinds", "Level", "Mixed", "Mode", "NASPDU", "NodeID", "NodeName", "Notification", "Octets", "PLMNIdentity", "Presence", "PrivateIEID", "PrivateMessage", "PrivateMessagePrivateIEs", "ProcedureCode", "ProtocolExtensionContainer", "ProtocolExtensionField", "ProtocolExtensionID", "ProtocolIEContainer", "ProtocolIEField", "ProtocolIEID", "RAC", "ReleaseRequest", "AnonymousPrivateMessagePrivateIEs", "SecurityContext", "SecurityKey", "SetupFailure", "SetupRequest", "SetupResponse", "SetupTransfer", "SuccessfulOutcome", "TAC", "TOYPDU", "Tagged", "TransferItem", "TransferKind", "TransferList", "TransportLayerAddress", "TriggeringMessage", "Tunnel", "TypeOfError", "UECoreID", "UEIDPair", "UEIDs", "UERANID", "UnsuccessfulOutcome"],
    displacements: &[7, 4, 1, 36, 2, 2, 0, 23, 18, 36, 9, 4, 4, 25, 4, 6, 1, 17, 8, 116],
    slots: &[33, 65535, 53, 72, 2, 1, 21, 74, 65, 35, 52, 50, 15, 7, 65535, 56, 44, 63, 65535, 19, 10, 59, 29, 68, 9, 54, 51, 65535, 67, 0, 61, 27, 6, 40, 69, 46, 65535, 58, 65535, 25, 14, 77, 32, 5, 26, 65535, 4, 70, 60, 30, 34, 65535, 65535, 64, 38, 42, 23, 66, 3, 75, 20, 8, 36, 65535, 12, 65535, 65535, 57, 47, 11, 22, 41, 17, 31, 65535, 28, 65535, 78, 65535, 73, 18, 45, 16, 24, 39, 49, 76, 62, 43, 71, 48, 55, 13, 37],
};
