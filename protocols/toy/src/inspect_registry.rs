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
        ContainerBlob sequence { "tAC": TAC, "nodeName"? _, "message"? _, "iE-Extensions"? extensions };
        Counts sequence { "countUL": _, "countDL": _, "iE-Extensions"? extensions };
        CriticalityDiagnostics sequence { "procedureCode"? _, "triggeringMessage"? TriggeringMessage, "procedureCriticality"? Criticality, "iEsCriticalityDiagnostics"? CriticalityDiagnosticsIEList, "iE-Extensions"? extensions };
        CriticalityDiagnosticsIEItem sequence { "iECriticality": Criticality, "iE-ID": _, "typeOfError": TypeOfError, "iE-Extensions"? extensions };
        CriticalityDiagnosticsIEList = [CriticalityDiagnosticsIEItem];
        ErrorIndication sequence { "protocolIEs": ies };
        FailureTransfer sequence { "cause": Cause, "iE-Extensions"? extensions };
        GTPTunnel sequence { "transportLayerAddress": TransportLayerAddress, "gTP-TEID": GTPTEID, "iE-Extensions"? extensions };
        InitiatingMessage sequence { "procedureCode": _, "criticality": Criticality, "value": _ };
        Item sequence { "item-ID": _, "cause"? Cause, "iE-Extensions"? extensions };
        Items = [ie];
        NodeID choice { "macro-NodeID": _, "home-NodeID": _, "choice-Extensions": ie };
        Notification sequence { "protocolIEs": ies };
        PrivateIEID choice { "local": _, "global": _ };
        PrivateMessage sequence { "privateIEs": PrivateMessagePrivateIEs };
        PrivateMessagePrivateIEs = [AnonymousPrivateMessagePrivateIEs];
        ReleaseRequest sequence { "protocolIEs": ies };
        SetupFailure sequence { "protocolIEs": ies };
        SetupRequest sequence { "protocolIEs": ies };
        SetupResponse sequence { "protocolIEs": ies };
        SetupTransfer sequence { "protocolIEs": ies };
        SuccessfulOutcome sequence { "procedureCode": _, "criticality": Criticality, "value": _ };
        TOYPDU choice { "initiatingMessage": InitiatingMessage, "successfulOutcome": SuccessfulOutcome, "unsuccessfulOutcome": UnsuccessfulOutcome };
        TransferItem sequence { "transfer-ID": _, "setupTransfer": (SetupTransfer), "failureTransfer"? (FailureTransfer), "iE-Extensions"? extensions };
        TransferList = [TransferItem];
        TransportLayerAddress = bits;
        Tunnel choice { "gTPTunnel": GTPTunnel, "choice-Extensions": ie };
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
        TransferKind ["ipv4", "ipv6"];
        TriggeringMessage ["initiating-message", "successful-outcome", "unsuccessful-outcome"];
        TypeOfError ["not-understood", "missing"];
    }
}
