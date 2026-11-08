// Auto-generated from ASN.1 by the generator of oxirush-aper; do not edit.
oxirush_aper::__protocol! { protocol Toy "TOY", bindings toy, pdu TOYPDU, triggering initiating_message successful_outcome unsuccessful_outcome }
oxirush_aper::__procedures! {
    protocol Toy "TOY", bindings toy, pdu TOYPDU, kind ToyPduKind, hooks oxirush_toy_hooks;
    1 Setup reject { InitiatingMessage SetupRequest "SetupRequest" [26 reject mandatory, 20 ignore optional, 32 reject optional, 22 reject optional, 24 reject optional, 25 ignore optional, 21 reject optional, 27 reject optional, 28 ignore optional, 29 ignore optional]; SuccessfulOutcome SetupResponse "SetupResponse" [20 ignore optional, 33 reject conditional, 16 ignore optional]; UnsuccessfulOutcome SetupFailure "SetupFailure" [15 ignore mandatory, 34 ignore optional, 16 ignore optional]; }
    2 Release ignore { InitiatingMessage ReleaseRequest "ReleaseRequest" [10 reject mandatory, 11 reject mandatory, 15 ignore mandatory, 30 reject optional]; }
    3 ErrorIndication ignore { InitiatingMessage ErrorIndication "ErrorIndication" [10 ignore optional, 11 ignore optional, 15 ignore optional, 16 ignore optional]; }
    4 PrivateMessage ignore { InitiatingMessage PrivateMessage "PrivateMessage"; }
    5 Notification ignore { InitiatingMessage Notification "Notification" [12 reject mandatory, 15 ignore optional]; }
}
oxirush_aper::__ies! {
    protocol Toy "TOY";
    10 "UE-Core-ID" crate::toy::UECoreID => UE_Core_ID;
    11 "UE-RAN-ID" crate::toy::UERANID => UE_RAN_ID;
    12 "UE-IDs" crate::toy::UEIDs => UE_IDs;
    15 "Cause" crate::toy::Cause => Cause;
    16 "CriticalityDiagnostics" crate::toy::CriticalityDiagnostics => CriticalityDiagnostics;
    20 "NodeName" crate::toy::NodeName<'a> => NodeName;
    21 "SecurityKey" @key crate::toy::SecurityKey<'a> => SecurityKey;
    22 "Items" crate::toy::Items<'a> => Items;
    23 reject "Item" crate::toy::Item => Item;
    24 "TransferList" crate::toy::TransferList<'a> => TransferList;
    25 "Container" crate::toy::Container<'a> => Container;
    26 "Areas" crate::toy::Areas => Areas;
    27 "Algorithms" crate::toy::Algorithms<'a> => Algorithms;
    28 "Counts" crate::toy::Counts => Counts;
    29 "Tunnel" crate::toy::Tunnel<'a> => Tunnel;
    30 "NAS-PDU" crate::toy::NASPDU<'a> => NAS_PDU;
    31 "TransferKind" crate::toy::TransferKind => TransferKind;
    32 "NodeID" crate::toy::NodeID => NodeID;
    33 "SecurityContext" @key crate::toy::SecurityContext<'a> => SecurityContext;
    34 "FailureTransfer" oxirush_aper::Octets<'a>, crate::toy::FailureTransfer => FailureTransfer;
    40 "ExtensionIndex" where [crate::toy::IndexOfItem: Item, crate::toy::IndexOfTransfer: TransferItem];
    41 ignore "ExtendedCause" crate::toy::ExtendedCause => ExtendedCause;
}
oxirush_aper::__lists! {
    Items Item 1 16;
}
oxirush_aper::__transfer_sets! {
    protocol Toy, bindings toy;
    "SetupTransfer" SetupTransfer [29 reject mandatory, 31 reject optional];
}
