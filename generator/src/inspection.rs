//! Generate dynamic open-type dispatch from the same ASN.1 as the codec.
use std::collections::BTreeMap;
use std::fmt::Write;

use anyhow::{Result, anyhow};
use regex::Regex;

use crate::registry::{Registry, alone, canonical};

/// The well-known types whose values a tree shows in a readable form: the
/// form, and the names of the types that have it. A name is matched whatever its case
/// and its hyphens: TS 36.413 writes `PLMNidentity` and `Port-Number`.
const READABLE: &[(&str, &[&str])] = &[
    ("Plmn", &["PLMNIdentity"]),
    ("Digits", &["IMSI"]),
    ("Address", &["TransportLayerAddress"]),
    (
        "Number",
        &[
            "AMFPointer",
            "AMFRegionID",
            "AMFSetID",
            "CellIdentity",
            "CI",
            "EPS-TAC",
            "EUTRACellIdentity",
            "FiveG-TMSI",
            "FiveGSTAC",
            "GTP-TEID",
            "LAC",
            "M-TMSI",
            "MME-Code",
            "MME-Group-ID",
            "NRCellIdentity",
            "PortNumber",
            "RAC",
            "SST",
            "TAC",
            "UL-NAS-Count",
        ],
    ),
];

/// The readable form of the values of a type that ASN.1 names `asn`, when it is one of
/// the well-known types: `Plmn`, `Digits`, `Address` or `Number`.
pub(crate) fn well_known(asn: &str) -> Option<&'static str> {
    let mut readable = READABLE.iter();
    let known =
        readable.find(|(_, types)| types.iter().any(|known| canonical(known) == canonical(asn)));
    known.map(|(form, _)| *form)
}

/// The tables that only the inspection has: the transfers and the open types. The
/// messages and the IEs are those of the registry; the members of the types and the
/// readable forms are read from the table of the types.
///
/// `carried` are the IEs whose octets ASN.1 leaves open and the specification gives a
/// type of the protocol, each as the name of the IE after `id-` and the name of that
/// type: the ASN.1 does not say them. `foreign` are those that it gives a type of another
/// protocol, each as the name of the IE, the name of that protocol and the name that it
/// gives the type.
pub fn generate(
    protocol: &str,
    asn: &str,
    registry: &Registry,
    carried: &[(&str, &str)],
    foreign: &[(&str, &str, &str)],
) -> Result<String> {
    let module = protocol.to_ascii_lowercase();
    let contained = Regex::new(
        r"(?m)^\s*([A-Za-z][A-Za-z0-9-]*)\s+OCTET\s+STRING\s*\(\s*CONTAINING\s+([A-Za-z][A-Za-z0-9-]*)\s*\)",
    )?;
    let mut transfers: BTreeMap<String, std::collections::BTreeSet<String>> = BTreeMap::new();
    for c in contained.captures_iter(asn) {
        if let Some(ty) = registry.resolve(&c[2]) {
            transfers.entry(c[1].to_string()).or_default().insert(ty);
        }
    }
    // One line for each transfer: rustfmt leaves the lines of a macro called with braces
    // as they are.
    let mut out = String::from(
        "// Auto-generated from ASN.1 by the generator of oxirush-aper; do not edit.\npub(crate) use crate::registry::{IE_NAMES, IE_TYPES, ITEM_CRITICALITIES, MESSAGES, ie, ie_contents};\n",
    );
    writeln!(
        out,
        "pub(crate) const PROTOCOL: &str = {protocol:?};\npub(crate) const ROOT: &str = {module:?};"
    )?;
    // The member that holds a transfer, and the type that its octets contain.
    writeln!(out, "oxirush_aper::__transfers! {{")?;
    for (field, alternatives) in &transfers {
        match alone(alternatives) {
            Some(ty) => writeln!(out, "    {field:?} {ty};")?,
            None => writeln!(out, "    {field:?};")?,
        }
    }
    writeln!(out, "}}")?;
    // The identifier of an IE whose octets may carry a type, the name that ASN.1 gives
    // that type, and the type; then, after `of`, the protocol of a type of another one,
    // the identifier and the name that that protocol gives the type.
    let id_of = |ie: &str| {
        let mut ies = registry.ies.iter();
        let found = ies.find(|(_, known)| known.name == ie);
        found
            .map(|(id, _)| *id)
            .ok_or_else(|| anyhow!("{ie} names no IE"))
    };
    writeln!(out, "oxirush_aper::__carried! {{")?;
    for (ie, name) in carried {
        let id = id_of(ie)?;
        let ty = registry.resolve(name);
        let ty = ty.ok_or_else(|| anyhow!("{name} names no type"))?;
        writeln!(out, "    {id} {name:?} {ty};")?;
    }
    for (ie, other, name) in foreign {
        writeln!(out, "    of {other:?} {} {name:?};", id_of(ie)?)?;
    }
    writeln!(out, "}}")?;
    // The IEs that the extension container of each type may hold, by the name of the type
    // in the bindings, in the order of the names: those of its extension set, in its
    // order, with their criticality and their presence.
    let ids = crate::registry::identifiers(asn)?;
    let (extended, extensions) = crate::registry::extensions(protocol, asn)?;
    let rust = |asn: &str| {
        let ty = registry.resolve(asn)?;
        Some(ty.rsplit("::").next().expect("a path").to_string())
    };
    let mut sets: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for ty in extended.iter().filter_map(|ty| rust(ty)) {
        sets.entry(ty).or_default();
    }
    for extension in extensions {
        let Some(extended) = rust(&extension.extended) else {
            continue;
        };
        let (id, _) = (ids.get(&extension.constant))
            .ok_or_else(|| anyhow!("{} has no identifier", extension.constant))?;
        let object = format!("{id} {} {}", extension.criticality, extension.presence);
        let set = sets.entry(extended).or_default();
        if !set.contains(&object) {
            set.push(object);
        }
    }
    writeln!(out, "oxirush_aper::__extension_sets! {{")?;
    for (extended, objects) in &sets {
        writeln!(out, "    {extended:?} [{}];", objects.join(", "))?;
    }
    writeln!(out, "}}")?;
    // The IEs by the letters and the digits of their names, in lower case, which is how
    // two names are the same, in the order of `IE_NAMES`.
    let ies = registry.ies.values();
    let names: Vec<String> = ies.map(|ie| canonical(&ie.name)).collect();
    crate::hashing::write(
        &mut out,
        "Each IE by the letters and the digits of its name, in lower case, as an index of `IE_NAMES`.",
        "IE_HASH",
        &names,
    )?;
    Ok(out)
}
