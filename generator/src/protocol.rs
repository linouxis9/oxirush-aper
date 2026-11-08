//! The generator of the bindings of a protocol crate, which its `build/` runs with the
//! names of its protocol: rasn-compiler on the ASN.1 of the specification, then the
//! bindings, the registry, the inspection and the oracle of the tests, written by the
//! modules of this crate, and what a PDU of the protocol does.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, ensure};
use rasn_compiler::OutputMode;
use rasn_compiler::prelude::{Compiler, RasnBackend, RasnConfig};
use regex::Regex;

use crate::bindings::Keys;
use crate::registry::Registry;

/// A protocol, as its generator names it.
pub struct Protocol {
    /// Its name, as `NGAP`: the bindings are `src/ngap.rs`, its PDU `NGAPPDU`, and its
    /// marker type `Ngap`.
    pub name: &'static str,
    /// The prefix of the modules that rasn-compiler writes for its ASN.1 modules, as
    /// `ngap` for `ngap_containers`.
    pub modules: &'static str,
    /// The IEs whose octets ASN.1 leaves open, each with the type of the protocol that
    /// the specification gives them for a handover between its nodes.
    pub carried: &'static [(&'static str, &'static str)],
    /// The IEs whose octets ASN.1 leaves open, each with another protocol and the type
    /// of that protocol that the specification gives them for a handover to its system:
    /// the tables of that protocol read it, once a program registers them.
    pub foreign: &'static [(&'static str, &'static str, &'static str)],
    /// The types of the keys of the protocol, of which `Debug` prints no octet, as K_gNB
    /// or K_eNB and NH.
    pub keys: &'static [&'static str],
    /// The type of a PLMN identity, which has its readable form.
    pub plmn: &'static str,
    /// What the protocol corrects in the bindings that rasn-compiler resolved, before the
    /// others are written from them.
    pub correct: fn(String) -> Result<String>,
}

impl Protocol {
    /// The name in lower case, as the module of the bindings.
    fn module(&self) -> String {
        self.name.to_ascii_lowercase()
    }

    /// The marker type, as `Ngap`.
    fn marker(&self) -> String {
        let module = self.module();
        self.name
            .chars()
            .take(1)
            .chain(module.chars().skip(1))
            .collect()
    }
}

/// The bindings of the oracle.
const ORACLE: &str = "oracle/src/bindings.rs";

/// The modules that rasn-compiler writes for the ASN.1 modules of a protocol.
const MODULES: [&str; 6] = [
    "common_data_types",
    "constants",
    "containers",
    "ies",
    "pdu_contents",
    "pdu_descriptions",
];

/// Generate the bindings of `protocol`, from the crate directory.
pub fn generate(protocol: &Protocol) -> Result<()> {
    // The modules that `build/asn1.sha256` records, and no other.
    let mut files = crate::asn1::modules()?;
    files.sort();

    let config = RasnConfig {
        // rasn-compiler's typed open types are experimental and currently emit invalid
        // bindings for these specifications. An open type is opaque: the macros over the
        // generated registry give it its type.
        opaque_open_types: true,
        generate_from_impls: true,
        ..RasnConfig::default()
    };
    // What rasn-compiler resolves is written as bindings for rasn, which become the
    // oracle of the tests.
    let output = Path::new(ORACLE);

    let warnings = Compiler::<RasnBackend, _>::new_with_config(config)
        .add_asn_sources_by_path(files.iter())
        .set_output_mode(OutputMode::SingleFile(output.into()))
        .compile()
        .map_err(|error| anyhow!("compile {} ASN.1 definitions: {error}", protocol.name))?;

    for warning in warnings {
        eprintln!("warning: {warning}");
    }

    post_process(protocol, output, &files)
}

fn post_process(protocol: &Protocol, path: &Path, asn_files: &[PathBuf]) -> Result<()> {
    let name = protocol.name;
    let prefix = protocol.modules;
    let mut resolved =
        fs::read_to_string(path).with_context(|| format!("read the resolved {name} bindings"))?;

    // Plain bracketed specification references are otherwise parsed as rustdoc links.
    resolved = resolved.replace("[16]", "(reference 16)");

    resolved = crate::containers::share(&resolved, &format!("{prefix}_containers"))?;
    resolved = (protocol.correct)(resolved)?;
    resolved = constrain_container_lists(&resolved, asn_files)?;
    resolved = private_ie_id(&resolved, prefix)?;

    // The bindings of the crate, on oxirush-aper. The types whose open type holds the
    // value of an IE or a message may hold a key, as its identifier or its procedure says.
    let marker = protocol.marker();
    let of_ie = "crate::registry::KEY_IES.contains(&self.id.0)".to_string();
    let of_message = |direction: &str| {
        format!(
            "oxirush_aper::protocol::holds_key::<crate::{marker}>(\
             oxirush_aper::Direction::{direction}, self.procedure_code.0)"
        )
    };
    let holders = [
        ("ProtocolIEField", of_ie.clone()),
        ("ProtocolExtensionField", of_ie),
        ("InitiatingMessage", of_message("Initiating")),
        ("SuccessfulOutcome", of_message("Successful")),
        ("UnsuccessfulOutcome", of_message("Unsuccessful")),
    ];
    let holders: Vec<(&str, &str)> = holders.iter().map(|(ty, of)| (*ty, of.as_str())).collect();
    let keys = Keys {
        types: protocol.keys,
        holders: &holders,
    };
    let bindings = crate::bindings::bindings(&resolved, keys, &[protocol.plmn])?;

    // The oracle of the tests: the same types, with the codec of rasn and the
    // corrections that it needs.
    let mut oracle = resolved;
    oracle = crate::rasn_fix::fix_constrained_sequences(&oracle)?;
    oracle = crate::rasn_fix::fix_utf8_strings(&oracle)?;
    oracle = crate::rasn_fix::fix_fixed_bit_strings(&oracle)?;
    oracle = crate::rasn_fix::fix_long_inline_strings(&oracle)?;
    oracle = crate::rasn_fix::fix_sequences(&oracle)?;
    oracle.push_str(&oracle_support(protocol, &oracle)?);

    let asn = asn_files
        .iter()
        .map(fs::read_to_string)
        .collect::<std::io::Result<Vec<_>>>()?
        .join("\n");
    // The procedures and the IEs, once for the macros and for the inspection.
    let registry = crate::registry::Registry::read(
        name,
        &oracle,
        &asn,
        bindings.borrowing.clone(),
        bindings.keys.clone(),
    )?;
    let list = registry.list()?;
    let inspection = crate::inspection::generate(
        name,
        &oracle,
        &asn,
        &registry,
        protocol.carried,
        protocol.foreign,
    )?;
    // The types that the crate decodes by a name or an identifier: the oracle decodes
    // the same.
    let module = protocol.module();
    let named = Regex::new(&format!(r"crate::{module}::([A-Za-z][A-Za-z0-9_]*)"))?;
    let pdu = format!("{name}PDU");
    let mut types: BTreeSet<&str> = BTreeSet::from([pdu.as_str()]);
    types.extend(registry.message_types());
    for text in [&list, &inspection] {
        types.extend(
            named
                .captures_iter(text)
                .map(|c| c.get(1).expect("a name").as_str()),
        );
    }
    let mut oracle_types = String::from(
        "// Auto-generated by build/main.rs from ASN.1; do not edit.\ncrate::oracle_types! {\n",
    );
    for name in &types {
        writeln!(oracle_types, "    {name}")?;
    }
    oracle_types.push_str("}\n");
    fs::write("oracle/src/types.rs", oracle_types)?;
    fs::write("src/registry.rs", &list)?;
    fs::write("src/inspect_registry.rs", inspection)?;
    fs::write(path, oracle).context("write the bindings of the oracle")?;
    // A PDU is serialized as the tree of its message, which `inspect.rs` writes.
    let serde = format!("#[cfg(feature = \"serde\")]\noxirush_utils::impl_serde!({pdu}<'a>);\n");
    ensure!(
        bindings.text.matches(&serde).count() == 1,
        "the serde of a PDU"
    );
    let mut text = bindings.text.replacen(&serde, "", 1);
    text.push_str(&support(protocol, &bindings.borrowing)?);
    transfers(&text, &registry)?;
    let file = format!("src/{module}.rs");
    fs::write(file, text).with_context(|| format!("write the {name} bindings"))
}

/// The private IE container uses PrivateIE-ID, which rasn-compiler omits from the import
/// list of the module of the containers.
fn private_ie_id(resolved: &str, prefix: &str) -> Result<String> {
    let import = Regex::new(&format!(
        r"    use super::{prefix}_common_data_types::\{{([^}}]*)\}};\n"
    ))?;
    let found = import
        .captures_iter(resolved)
        .find(|c| {
            let names = &c[1];
            names.contains("Presence")
                && names.contains("ProtocolIEID")
                && !names.contains("PrivateIEID")
        })
        .map(|c| (c[0].to_string(), c[1].to_string()));
    let (line, names) = found.ok_or_else(|| anyhow!("no import of the common data types"))?;
    let mut names: Vec<&str> = names.split(',').map(str::trim).collect();
    names.retain(|name| !name.is_empty());
    names.push("PrivateIEID");
    names.sort_unstable();
    let imported = format!(
        "    use super::{prefix}_common_data_types::{{{}}};\n",
        names.join(", ")
    );
    Ok(resolved.replacen(&line, &imported, 1))
}

/// Restore the SIZE constraint of the lists declared through a parameterized
/// `ProtocolIE-ContainerList` or `ProtocolIE-ContainerPairList`.
///
/// rasn-compiler drops the bounds these take as value parameters, so
/// `E-RABAdmittedList ::= E-RAB-IE-ContainerList { {...} }`, a
/// `SEQUENCE (SIZE(1..maxnoofE-RABs)) OF ProtocolIE-SingleContainer`, would get an
/// unconstrained length determinant. A protocol that declares none has none to restore.
fn constrain_container_lists(generated: &str, asn_files: &[PathBuf]) -> Result<String> {
    let mut asn = String::new();
    for file in asn_files {
        asn.push_str(&fs::read_to_string(file)?);
        asn.push('\n');
    }

    let constant = Regex::new(r"(?m)^([A-Za-z][A-Za-z0-9-]*)\s+INTEGER\s*::=\s*(\d+)")?;
    let constants: BTreeMap<&str, &str> = constant
        .captures_iter(&asn)
        .map(|captures| {
            let (_, [name, value]) = captures.extract();
            (name, value)
        })
        .collect();
    let bound = |value: &str| -> Result<String> {
        if value.bytes().all(|byte| byte.is_ascii_digit()) {
            return Ok(value.to_string());
        }
        constants
            .get(value)
            .map(|value| value.to_string())
            .ok_or_else(|| anyhow!("unknown container list bound {value}"))
    };

    let list_definition = Regex::new(
        r"(?m)^([A-Za-z][A-Za-z0-9-]*)\s*\{[^}]*\}\s*::=\s*ProtocolIE-Container(?:Pair)?List\s*\{\s*([A-Za-z0-9-]+)\s*,\s*([A-Za-z0-9-]+)\s*,",
    )?;
    let mut generated = generated.to_string();
    for captures in list_definition.captures_iter(&asn) {
        let (_, [list, lower, upper]) = captures.extract();
        let size = format!("size(\"{}..={}\")", bound(lower)?, bound(upper)?);
        let list_use = Regex::new(&format!(
            r"(?m)^([A-Za-z][A-Za-z0-9-]*)\s*::=\s*{}\s*\{{",
            regex::escape(list)
        ))?;
        for captures in list_use.captures_iter(&asn) {
            // Each of these ASN.1 names needs a rasn `identifier`.
            let name = &captures[1];
            let declaration = format!("    #[rasn(delegate, identifier = \"{name}\")]\n");
            ensure!(
                generated.matches(&declaration).count() == 1,
                "no unique declaration of {name}"
            );
            generated = generated.replacen(
                &declaration,
                &format!("    #[rasn(delegate, {size}, identifier = \"{name}\")]\n"),
                1,
            );
        }
    }
    Ok(generated)
}

/// What follows the modules in the bindings of the oracle: their names at the root, and
/// the conversions that its tests build values with.
fn oracle_support(protocol: &Protocol, generated: &str) -> Result<String> {
    let newtype = Regex::new(r"(?m)^    pub struct ([A-Za-z][A-Za-z0-9_]*)\(pub ([^;\n]+)\);$")?;
    let mut newtypes = BTreeMap::new();
    for captures in newtype.captures_iter(generated) {
        let name = captures[1].to_string();
        let inner = captures[2].to_string();
        if !inner.contains(", pub ") {
            newtypes.entry(name).or_insert(inner);
        }
    }

    let prefix = protocol.modules;
    let mut out = String::new();
    writeln!(out, "\n// The names of the modules at the root.")?;
    for module in MODULES {
        writeln!(out, "pub use {prefix}_{module}::*;")?;
    }

    writeln!(out, "#[allow(unused_imports)]")?;
    writeln!(
        out,
        "use rasn::prelude::{{BitString, FixedBitString, FixedOctetString, Integer, OctetString, PrintableString, SequenceOf, Utf8String, VisibleString}};"
    )?;
    writeln!(out, "\n// The conversions of the types that have a name.")?;
    for (name, inner) in &newtypes {
        writeln!(
            out,
            "impl From<{inner}> for {name} {{ fn from(value: {inner}) -> Self {{ Self(value) }} }}"
        )?;
        if inner == "OctetString" {
            writeln!(
                out,
                "impl From<Vec<u8>> for {name} {{ fn from(value: Vec<u8>) -> Self {{ Self(value.into()) }} }}"
            )?;
        } else if let Some(size) = inner
            .strip_prefix("FixedOctetString<")
            .and_then(|value| value.strip_suffix('>'))
        {
            writeln!(
                out,
                "impl From<[u8; {size}]> for {name} {{ fn from(value: [u8; {size}]) -> Self {{ Self(value.into()) }} }}"
            )?;
        }
    }
    Ok(out)
}

/// Whether the transfers of protocol IEs of the registry, which it reads from the ASN.1,
/// are the types of the bindings that are a container of protocol IEs and no message of
/// the registry: each SEQUENCE whose one member is `protocolIEs`. A protocol may have
/// none.
fn transfers(bindings: &str, registry: &Registry) -> Result<()> {
    let containers = Regex::new(
        r"pub struct (\w+)<'a>\s*\{\s*pub protocol_ies:\s*ProtocolIEContainer<'a>,?\s*\}",
    )?;
    let messages: BTreeSet<&str> = registry.message_types().collect();
    let found: BTreeSet<&str> = (containers.captures_iter(bindings))
        .map(|c| c.get(1).expect("a name").as_str())
        .filter(|name| !messages.contains(name))
        .collect();
    let listed = registry.transfers.iter();
    let listed: BTreeSet<&str> = listed
        .map(|transfer| transfer.ty.rsplit(':').next().unwrap_or(&transfer.ty))
        .collect();
    ensure!(
        found == listed,
        "the transfers of protocol IEs of the bindings, {found:?}, are not those of the \
         ASN.1, {listed:?}"
    );
    Ok(())
}

/// What follows the modules in the bindings of the crate: their names at the root, and
/// the kind of a PDU. The methods of a PDU are those of `oxirush_aper::__pdu!`, which the
/// crate calls.
fn support(protocol: &Protocol, borrowing: &BTreeSet<String>) -> Result<String> {
    let (prefix, name) = (protocol.modules, protocol.name);
    ensure!(
        borrowing.contains(&format!("{name}PDU")),
        "a PDU that borrows nothing"
    );
    let mut out = String::new();
    writeln!(out, "\n// The names of the modules at the root.")?;
    for module in MODULES {
        writeln!(out, "pub use {prefix}_{module}::*;")?;
    }
    writeln!(
        out,
        "pub use crate::registry::{}PduKind;",
        protocol.marker()
    )?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The transfers of protocol IEs that the bindings have are those that the registry
    /// reads from the ASN.1, beside the messages.
    #[test]
    fn the_transfers_of_the_bindings_are_those_of_the_asn1() {
        let oracle = "pub enum TriggeringMessage { initiating_message = 0, }";
        let registry = Registry::read("Toy", oracle, "", BTreeSet::new(), BTreeSet::new());
        let mut registry = registry.unwrap();
        let bindings = "pub struct Holder<'a> { pub protocol_ies: ProtocolIEContainer<'a>, }";
        let error = transfers(bindings, &registry).unwrap_err();
        assert!(error.to_string().contains(r#"{"Holder"}"#), "{error}");
        // A message of a procedure is no transfer.
        let holder = crate::registry::Message {
            name: "Holder".into(),
            ty: "crate::toy::Holder".into(),
            ies: Some(Vec::new()),
        };
        let procedure = crate::registry::Procedure {
            name: "Hold".into(),
            criticality: None,
            messages: [Some(holder), None, None],
        };
        registry.procedures.insert(1, procedure);
        assert!(transfers(bindings, &registry).is_ok());
    }
}
