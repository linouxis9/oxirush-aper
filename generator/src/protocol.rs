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
    /// The types of the keys of the protocol, of which `Debug` prints no octet, as K_gNB
    /// or K_eNB and NH.
    pub keys: &'static [&'static str],
    /// The type of a PLMN identity, which has its readable form.
    pub plmn: &'static str,
    /// The aliases of the bindings, each with its type: the identifiers of a UE.
    pub aliases: &'static [(&'static str, &'static str)],
    /// The octets of a UE CONTEXT RELEASE REQUEST, for the examples of the documentation
    /// of a PDU.
    pub release_request: &'static str,
    /// The name of another message than that UE CONTEXT RELEASE REQUEST.
    pub other_message: &'static str,
    /// The name of a type that is no message.
    pub no_message: &'static str,
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
            "oxirush_aper::protocol::holds_key::<crate::{marker}>(\"{direction}\", \
             self.procedure_code.0)"
        )
    };
    let holders = [
        ("ProtocolIEField", of_ie.clone()),
        ("ProtocolExtensionField", of_ie),
        ("InitiatingMessage", of_message("InitiatingMessage")),
        ("SuccessfulOutcome", of_message("SuccessfulOutcome")),
        ("UnsuccessfulOutcome", of_message("UnsuccessfulOutcome")),
    ];
    let holders: Vec<(&str, &str)> = holders.iter().map(|(ty, of)| (*ty, of.as_str())).collect();
    let keys = Keys {
        types: protocol.keys,
        holders: &holders,
    };
    let bindings = crate::bindings::bindings(
        &resolved,
        keys,
        &[protocol.plmn],
        Some("oxirush_aper::protocol::__Field"),
    )?;

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
    let inspection = crate::inspection::generate(name, &oracle, &asn, &registry, protocol.carried)?;
    // The types that the crate decodes by a name or an identifier: the oracle decodes
    // the same.
    let module = protocol.module();
    let named = Regex::new(&format!(r"crate::{module}::([A-Za-z][A-Za-z0-9_]*)"))?;
    let pdu = format!("{name}PDU");
    let mut types: BTreeSet<&str> = BTreeSet::from([pdu.as_str()]);
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
    // The name of each field, and the one that it had before the rule of the generator.
    fs::write("build/fields.tsv", &bindings.fields)?;
    // A PDU is serialized as the tree of its message, which `inspect.rs` writes.
    let serde = format!("#[cfg(feature = \"serde\")]\noxirush_utils::impl_serde!({pdu}<'a>);\n");
    ensure!(
        bindings.text.matches(&serde).count() == 1,
        "the serde of a PDU"
    );
    let mut text = bindings.text.replacen(&serde, "", 1);
    text.push_str(&support(protocol, &bindings.borrowing)?);
    text.push_str(&transfers(protocol, &text, &list)?);
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

/// What follows the modules in the bindings of the oracle: their names at the root, the
/// aliases of the crate, and the conversions that its tests build values with.
fn oracle_support(protocol: &Protocol, generated: &str) -> Result<String> {
    let generated_type = Regex::new(r"(?m)^    pub (?:struct|enum|type) ([A-Za-z][A-Za-z0-9_]*)")?;
    let rust_types: BTreeSet<&str> = generated_type
        .captures_iter(generated)
        .map(|captures| captures.get(1).expect("a name").as_str())
        .collect();

    let newtype = Regex::new(r"(?m)^    pub struct ([A-Za-z][A-Za-z0-9_]*)\(pub ([^;\n]+)\);$")?;
    let mut newtypes = BTreeMap::new();
    for captures in newtype.captures_iter(generated) {
        let name = captures[1].to_string();
        let inner = captures[2].to_string();
        if !inner.contains(", pub ") {
            newtypes.entry(name).or_insert(inner);
        }
    }

    let (prefix, name) = (protocol.modules, protocol.name);
    let mut out = String::new();
    writeln!(
        out,
        "\n// The names of the modules at the root, and the aliases of the crate."
    )?;
    for module in MODULES {
        writeln!(out, "pub use {prefix}_{module}::*;")?;
    }
    writeln!(out, "#[allow(non_camel_case_types)]")?;
    writeln!(out, "pub type {name}_PDU = {name}PDU;")?;
    for (alias, ty) in protocol.aliases {
        if rust_types.contains(ty) {
            writeln!(out, "#[allow(non_camel_case_types)]")?;
            writeln!(out, "pub type {alias} = {ty};")?;
        }
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

/// The transfers whose octets hold a container of protocol IEs, which the macros read and
/// build IEs for as they do for a message: each SEQUENCE whose one member is `protocolIEs`
/// and that is no message of the registry `list`. A protocol may have none.
fn transfers(protocol: &Protocol, bindings: &str, list: &str) -> Result<String> {
    let containers = Regex::new(
        r"pub struct (\w+)<'a>\s*\{\s*pub protocol_ies:\s*ProtocolIEContainer<'a>,?\s*\}",
    )?;
    // A message of a procedure, as `"Name" crate::ngap::Type<'a> [IEs]`, whose name may
    // have a hyphen (`"E-RABSetupRequest"`): an IE has `=>`.
    let module = protocol.module();
    let messages = Regex::new(&format!(r#""[\w-]+" crate::{module}::(\w+)<'a>( \[|;)"#))?;
    let messages: BTreeSet<&str> = messages
        .captures_iter(list)
        .map(|c| c.get(1).expect("a name").as_str())
        .collect();
    let mut names = Vec::new();
    for name in containers
        .captures_iter(bindings)
        .map(|c| c.get(1).expect("a name").as_str())
    {
        if !messages.contains(name) {
            names.push(name);
        }
    }
    if names.is_empty() {
        return Ok(String::new());
    }
    let mut out =
        String::from("\n// The transfers of protocol IEs, whose IEs the macros read and build.\n");
    let marker = protocol.marker();
    writeln!(
        out,
        "oxirush_aper::__transfer!(crate::{marker}, {module}, {});",
        names.join(" ")
    )?;
    Ok(out)
}

/// What follows the modules in the bindings of the crate: their names at the root, the
/// aliases, and what a PDU does.
fn support(protocol: &Protocol, borrowing: &BTreeSet<String>) -> Result<String> {
    let (prefix, name) = (protocol.modules, protocol.name);
    let mut out = String::new();
    writeln!(
        out,
        "\n// The names of the modules at the root, and the aliases."
    )?;
    for module in MODULES {
        writeln!(out, "pub use {prefix}_{module}::*;")?;
    }
    writeln!(out, "use alloc::vec::Vec;")?;
    writeln!(out, "use oxirush_aper::{{Decode, Encode, Octets}};")?;
    let pdu = format!("{name}PDU");
    ensure!(borrowing.contains(&pdu), "a PDU that borrows nothing");
    for (alias, ty) in protocol.aliases {
        ensure!(!borrowing.contains(*ty), "{ty} borrows");
        writeln!(out, "#[allow(non_camel_case_types)]")?;
        writeln!(out, "pub type {alias} = {ty};")?;
    }
    let text = SUPPORT
        .replace("{RELEASE_REQUEST}", protocol.release_request)
        .replace("{OTHER_MESSAGE}", protocol.other_message)
        .replace("{NO_MESSAGE}", protocol.no_message)
        .replace("{PDU_ALIAS}", &format!("{name}_PDU"))
        .replace("{PDU}", &pdu)
        .replace("{KIND}", &format!("{}PduKind", protocol.marker()))
        .replace("{CRATE}", &format!("oxirush_{}", protocol.module()))
        .replace("{MARKER}", &protocol.marker())
        .replace("{MODULE}", &protocol.module());
    out.push_str(&text);
    Ok(out)
}

/// What a PDU does, with the names of the protocol in braces.
const SUPPORT: &str = r####"/// A PDU that owns its octets: what `decode` gives, what the macros build, and what
/// is kept and sent. It borrows nothing, and so has no lifetime to write.
#[allow(non_camel_case_types)]
pub type {PDU_ALIAS} = {PDU}<'static>;
pub use crate::registry::{KIND};
impl {PDU}<'static> {
    /// The PDU that `bytes` are, in aligned PER: the octets of one PDU, whole, and no
    /// octet after it. The PDU owns the octets of its message, which are copied once.
    ///
    /// The envelope is read, and the octets of the message are not looked into: what
    /// does not decode in them is an error of `decode_value`, of `protocol_ies` and of
    /// `check`. The crate has no limit on the octets of a PDU, and a call on a PDU takes
    /// time and memory in proportion to them: the caller bounds what it reads from a
    /// peer.
    pub fn decode(bytes: &[u8]) -> Result<Self, crate::Error> {
        match {PDU}::usual(bytes) {
            Some((alternative, code, criticality, message)) => Ok(Self::of_alternative(
                alternative,
                code,
                criticality,
                Octets::owned(message.to_vec()),
            )),
            None => oxirush_aper::decode(bytes).map({PDU}::into_owned),
        }
    }
    /// The PDU that `bytes` are, as `decode` reads it, with the octets of its message
    /// borrowed from `bytes`: nothing is copied, and the PDU, a `{PDU}<'_>`, lives as
    /// long as they do. `into_owned()` gives the PDU that `decode` gives.
    ///
    /// [`message`](Self::message) gives the message of the PDU, which borrows from
    /// `bytes` as the PDU does and is kept after it: a function that is given octets
    /// returns their message.
    ///
    /// ```
    /// use {CRATE}::{MODULE}::*;
    ///
    /// // A UE CONTEXT RELEASE REQUEST, read where it was received
    /// let received = hex::decode("{RELEASE_REQUEST}").unwrap();
    /// let pdu = {PDU_ALIAS}::decode_borrowed(&received)?;
    ///
    /// // Its message borrows from `received` too, and not from the PDU
    /// let request: UEContextReleaseRequest = pdu.message()?;
    ///
    /// // A PDU that is kept after `received` is gone
    /// let kept: {PDU_ALIAS} = pdu.into_owned();
    /// assert_eq!(request.protocol_ies.0.len(), 3);
    /// # let _ = kept;
    /// # Ok::<(), {CRATE}::Error>(())
    /// ```
    pub fn decode_borrowed<'b>(bytes: &'b [u8]) -> Result<{PDU}<'b>, crate::Error> {
        match {PDU}::usual(bytes) {
            Some((alternative, code, criticality, message)) => Ok({PDU}::of_alternative(
                alternative,
                code,
                criticality,
                Octets::borrowed(message),
            )),
            None => oxirush_aper::decode(bytes),
        }
    }
}
impl<'a> {PDU}<'a> {
    /// A PDU of the usual layout, read octet by octet: its alternative, its procedure
    /// code and its criticality, in an octet each, then its message in one piece up to
    /// the last octet. `None` for octets of another layout and for octets that are no
    /// PDU, which the codec of the type reads or refuses: this is a shorter way to
    /// what it gives, and the tests hold the two to the same PDU.
    #[inline]
    fn usual(bytes: &'a [u8]) -> Option<(u8, u8, Criticality, &'a [u8])> {
        let [alternative, code, criticality, ref rest @ ..] = *bytes else {
            return None;
        };
        // No extension bit, and one of the three alternatives.
        if alternative & 0x80 != 0 || alternative >> 5 > 2 {
            return None;
        }
        let criticality = *Criticality::VALUES.get(usize::from(criticality >> 6))?;
        // The length of the message in one octet or in two, and no fragment.
        let message = match *rest {
            [length @ 1..=0x7f, ref message @ ..] if message.len() == usize::from(length) => {
                message
            }
            [high @ 0x80..=0xbf, low, ref message @ ..]
                if !message.is_empty()
                    && message.len() == usize::from(high & 0x3f) << 8 | usize::from(low) =>
            {
                message
            }
            _ => return None,
        };
        Some((alternative >> 5, code, criticality, message))
    }
    /// The PDU of an alternative, which is one of the three.
    #[inline]
    fn of_alternative(alternative: u8, code: u8, criticality: Criticality, value: Octets<'a>) -> Self {
        let procedure_code = ProcedureCode(code);
        match alternative {
            0 => Self::initiatingMessage(InitiatingMessage {
                procedure_code,
                criticality,
                value,
            }),
            1 => Self::successfulOutcome(SuccessfulOutcome {
                procedure_code,
                criticality,
                value,
            }),
            _ => Self::unsuccessfulOutcome(UnsuccessfulOutcome {
                procedure_code,
                criticality,
                value,
            }),
        }
    }
    /// The octets of this PDU, in aligned PER.
    pub fn encode(&self) -> Result<Vec<u8>, crate::Error> {
        let mut octets = Vec::with_capacity(self.value().len() + 6);
        self.encode_into(&mut octets)?;
        Ok(octets)
    }
    /// Append the octets of this PDU, in aligned PER, to `octets`: a buffer that is
    /// kept from one PDU to the next is allocated once. After an error, `octets` are as
    /// they were.
    pub fn encode_into(&self, octets: &mut Vec<u8>) -> Result<(), crate::Error> {
        // Octet by octet, as `decode` reads the usual layout: the alternative, the
        // procedure code and the criticality, then the message in its open type. It
        // is what the codec of the type writes, which the tests hold it to.
        let (alternative, code, criticality, value) = match self {
            {PDU}::initiatingMessage(message) => {
                (0, message.procedure_code.0, message.criticality, &message.value)
            }
            {PDU}::successfulOutcome(message) => {
                (1, message.procedure_code.0, message.criticality, &message.value)
            }
            {PDU}::unsuccessfulOutcome(message) => {
                (2, message.procedure_code.0, message.criticality, &message.value)
            }
        };
        octets.extend_from_slice(&[alternative << 5, code, (criticality as u8) << 6]);
        oxirush_aper::Writer::new(octets).open_type(value);
        Ok(())
    }
    /// The message that this PDU holds, as its type `T`. What the message holds of
    /// octets is borrowed from the PDU: the PDU is a variable of its own, which the
    /// message does not outlive. [`message`](Self::message) gives one that is kept
    /// after the PDU, without a copy when the PDU itself borrows its octets.
    ///
    /// `T` is the type of the message that `kind()` says. For the type of another
    /// message the error has the kind `WrongMessage`, as the octets of one message
    /// may decode as another:
    ///
    /// ```
    /// use {CRATE}::{ErrorKind, {MODULE}::*};
    ///
    /// // A UE CONTEXT RELEASE REQUEST
    /// let octets = hex::decode("{RELEASE_REQUEST}").unwrap();
    /// let pdu = {PDU_ALIAS}::decode(&octets)?;
    /// let request: UEContextReleaseRequest = pdu.decode_value()?;
    /// assert_eq!(request.protocol_ies.0.len(), 3);
    ///
    /// let other = pdu.decode_value::<{OTHER_MESSAGE}>().unwrap_err();
    /// assert_eq!(other.kind(), ErrorKind::WrongMessage);
    /// # Ok::<(), {CRATE}::Error>(())
    /// ```
    ///
    /// A type that is no message, as that of a transfer or an IE, does not compile.
    /// `decode_open_type` decodes any type from an open type.
    ///
    /// ```compile_fail
    /// use {CRATE}::{MODULE}::*;
    ///
    /// fn held(pdu: &{PDU_ALIAS}) {
    ///     let _ = pdu.decode_value::<{NO_MESSAGE}>();
    /// }
    /// ```
    pub fn decode_value<'s, T: crate::Message<'s, Protocol = crate::{MARKER}>>(&'s self) -> Result<T, crate::Error> {
        let (direction, code) = (self.direction(), self.procedure_code());
        if (direction, code) != (T::DIRECTION, T::PROCEDURE_CODE) {
            return Err(oxirush_aper::protocol::__wrong_message::<crate::{MARKER}>(
                T::NAME, direction, code,
            ));
        }
        oxirush_aper::decode(self.value())
    }
    /// The message that this PDU holds, as its type `T`, which is kept after the PDU:
    /// it lives as long as the octets that the PDU was decoded from.
    ///
    /// The message of a PDU that borrows its octets, as `decode_borrowed` gives it,
    /// borrows from them too. Nothing is copied, and a function returns the message:
    ///
    /// ```
    /// use {CRATE}::{Error, {MODULE}::*};
    ///
    /// // The message borrows from `received`, and the PDU is gone
    /// fn read(received: &[u8]) -> Result<UEContextReleaseRequest<'_>, Error> {
    ///     {PDU_ALIAS}::decode_borrowed(received)?.message()
    /// }
    ///
    /// let received = hex::decode("{RELEASE_REQUEST}").unwrap();
    /// assert_eq!(read(&received)?.protocol_ies.0.len(), 3);
    /// # Ok::<(), {CRATE}::Error>(())
    /// ```
    ///
    /// The message of a PDU that owns its octets, as `decode` gives it, owns its own:
    /// the value of each IE is copied, as `into_owned()` of the message copies it.
    /// `decode_value` borrows from the PDU instead, and copies nothing. A PDU whose
    /// message came in fragments, which one of 16384 octets or more does, owns its
    /// octets however it was decoded.
    ///
    /// The errors are those of `decode_value`.
    pub fn message<T: crate::Message<'a, Protocol = crate::{MARKER}>>(&self) -> Result<T, crate::Error> {
        let (direction, code) = (self.direction(), self.procedure_code());
        if (direction, code) != (T::DIRECTION, T::PROCEDURE_CODE) {
            return Err(oxirush_aper::protocol::__wrong_message::<crate::{MARKER}>(
                T::NAME, direction, code,
            ));
        }
        match self.value().as_borrowed() {
            Some(octets) => oxirush_aper::decode(octets),
            None => T::__decode_owned(self.value()),
        }
    }
    /// Return the procedure code of this PDU.
    pub fn procedure_code(&self) -> u8 {
        match self {
            {PDU}::initiatingMessage(message) => message.procedure_code.0,
            {PDU}::successfulOutcome(message) => message.procedure_code.0,
            {PDU}::unsuccessfulOutcome(message) => message.procedure_code.0,
        }
    }
    /// Return the PDU direction as a human-readable string.
    pub fn direction(&self) -> &'static str {
        match self {
            {PDU}::initiatingMessage(_) => "InitiatingMessage",
            {PDU}::successfulOutcome(_) => "SuccessfulOutcome",
            {PDU}::unsuccessfulOutcome(_) => "UnsuccessfulOutcome",
        }
    }
    /// Returns `true` for an initiating message.
    pub fn is_initiating(&self) -> bool {
        matches!(self, {PDU}::initiatingMessage(_))
    }
    /// Returns `true` for a successful outcome.
    pub fn is_successful(&self) -> bool {
        matches!(self, {PDU}::successfulOutcome(_))
    }
    /// Returns `true` for an unsuccessful outcome.
    pub fn is_unsuccessful(&self) -> bool {
        matches!(self, {PDU}::unsuccessfulOutcome(_))
    }
}
impl core::fmt::Display for {PDU}<'_> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            formatter,
            "{} {} (code={})",
            self.direction(),
            self.procedure_name(),
            self.procedure_code()
        )
    }
}
/// The open type of a value: its complete encoding in aligned PER, in which an empty
/// encoding becomes one zero octet (X.691 (02/2021) §11.1.4, §11.2.1).
pub fn encode_open_type<T: Encode>(value: &T) -> Result<Octets<'static>, crate::Error> {
    oxirush_aper::encode(value).map(Octets::owned)
}
/// The value of the type `T` that the octets of an open type hold, in aligned PER: one
/// value, whole, and no octet after it. What the value holds of octets is borrowed
/// from `value`.
pub fn decode_open_type<'a, T: Decode<'a>>(value: &'a [u8]) -> Result<T, crate::Error> {
    oxirush_aper::decode(value)
}
"####;
