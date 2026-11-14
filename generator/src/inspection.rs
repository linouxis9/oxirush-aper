//! Generate dynamic open-type dispatch from the same ASN.1 as the codec.
use std::collections::BTreeMap;
use std::fmt::Write;

use anyhow::{Result, anyhow, bail};
use regex::Regex;

use crate::registry::{Registry, alone, canonical};

fn items<'a>(input: &'a [syn::Item], prefix: &str, output: &mut Vec<(String, &'a syn::Item)>) {
    for item in input {
        match item {
            syn::Item::Mod(module) => {
                if let Some((_, inner)) = &module.content {
                    items(inner, &format!("{prefix}{}::", module.ident), output);
                }
            }
            syn::Item::Struct(_) | syn::Item::Enum(_) => output.push((prefix.to_string(), item)),
            _ => {}
        }
    }
}

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

/// The ASN.1 identifier that `rasn` has for a type, a field or an alternative.
fn rasn(attributes: &[syn::Attribute]) -> Result<Option<String>> {
    use syn::{Meta, Token, punctuated::Punctuated};
    for attribute in attributes.iter().filter(|a| a.path().is_ident("rasn")) {
        let metas = attribute.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
        for meta in metas {
            if let Meta::NameValue(pair) = meta
                && pair.path.is_ident("identifier")
                && let syn::Expr::Lit(literal) = pair.value
                && let syn::Lit::Str(name) = literal.lit
            {
                return Ok(Some(name.value()));
            }
        }
    }
    Ok(None)
}

/// The last segment of a type's path: its name and its generic arguments.
fn named(ty: &syn::Type) -> Option<(String, Vec<&syn::GenericArgument>)> {
    let syn::Type::Path(path) = ty else {
        return None;
    };
    let segment = path.path.segments.last()?;
    let arguments = match &segment.arguments {
        syn::PathArguments::AngleBracketed(arguments) => arguments.args.iter().collect(),
        _ => Vec::new(),
    };
    Some((segment.ident.to_string(), arguments))
}

/// The structs of one unnamed member, by name: the ASN.1 name of each and the type that
/// it wraps.
type Wrapped<'a> = BTreeMap<String, (String, &'a syn::Type)>;

/// The number of bits of the fixed OCTET STRING or BIT STRING that `name` is.
fn bits(name: &str, wrapped: &Wrapped) -> Option<u32> {
    let (inner, arguments) = named(wrapped.get(name)?.1)?;
    let size = arguments.iter().find_map(|argument| match argument {
        syn::GenericArgument::Const(syn::Expr::Lit(literal)) => match &literal.lit {
            syn::Lit::Int(size) => size.base10_parse::<u32>().ok(),
            _ => None,
        },
        _ => None,
    });
    match inner.as_str() {
        "FixedOctetString" => Some(size? * 8),
        "FixedBitString" => size,
        _ => bits(&inner, wrapped),
    }
}

/// The readable form of the values of a type that ASN.1 names `asn`, when it is one of
/// the well-known types: `Plmn`, `Digits`, `Address` or `Number`.
pub(crate) fn well_known(asn: &str) -> Option<&'static str> {
    let mut readable = READABLE.iter();
    let known =
        readable.find(|(_, types)| types.iter().any(|known| canonical(known) == canonical(asn)));
    known.map(|(form, _)| *form)
}

/// The readable form of each well-known type, by the name that the bindings give it.
fn generate_readable(generated: &str, out: &mut String) -> Result<()> {
    let syntax = syn::parse_file(generated)?;
    let mut declarations = Vec::new();
    items(&syntax.items, "", &mut declarations);
    let mut wrapped = Wrapped::new();
    for (_, item) in &declarations {
        if let syn::Item::Struct(item) = item
            && let syn::Fields::Unnamed(fields) = &item.fields
            && fields.unnamed.len() == 1
        {
            let asn = rasn(&item.attrs)?;
            let asn = asn.unwrap_or_else(|| item.ident.to_string());
            wrapped.insert(item.ident.to_string(), (asn, &fields.unnamed[0].ty));
        }
    }
    // One line for each well-known type, in the order of the names of the types.
    let mut forms = Vec::new();
    for (name, (asn, _)) in &wrapped {
        let form = match well_known(asn) {
            Some("Number") => match bits(name, &wrapped) {
                Some(bits) => format!("Form::Number({bits})"),
                None => bail!("{asn}: a number of a size that the type does not give"),
            },
            Some(form) => format!("Form::{form}"),
            None => continue,
        };
        forms.push(format!("// {asn}\n{name:?} => {form},"));
    }
    match forms.is_empty() {
        true => writeln!(
            out,
            "pub(crate) fn type_form(_: &str) -> Option<Form> {{ None }}"
        )?,
        false => writeln!(
            out,
            "pub(crate) fn type_form(name: &str) -> Option<Form> {{ Some(match name {{\n{}\n_ \
             => return None, }}) }}",
            forms.join("\n")
        )?,
    }
    Ok(())
}

/// The tables that only the inspection has: the transfers, the readable forms and the
/// open types. The messages and the IEs are those of the registry; the members of the
/// types are read from the table of the types.
///
/// `carried` are the IEs whose octets ASN.1 leaves open and the specification gives a
/// type of the protocol, each as the name of the IE after `id-` and the name of that
/// type: the ASN.1 does not say them. `foreign` are those that it gives a type of another
/// protocol, each as the name of the IE, the name of that protocol and the name that it
/// gives the type.
pub fn generate(
    protocol: &str,
    generated: &str,
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
        "// Auto-generated from ASN.1 by the generator of oxirush-aper; do not edit.\nuse oxirush_aper::inspect::Form;\npub(crate) use crate::registry::{IE_NAMES, IE_TYPES, ITEM_CRITICALITIES, MESSAGES, ie, ie_contents};\n",
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
    generate_readable(generated, &mut out)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The identifier is what `identifier` names, and no other pair names one.
    #[test]
    fn the_identifier_of_a_member_is_the_one_that_its_attribute_names() {
        let named = |attribute: &str| {
            let item: syn::ItemStruct =
                syn::parse_str(&format!("struct Holder {{ {attribute} count: u8 }}")).unwrap();
            let field = item.fields.iter().next().unwrap();
            rasn(&field.attrs).unwrap()
        };
        assert_eq!(
            named(r#"#[rasn(identifier = "Count")]"#).as_deref(),
            Some("Count")
        );
        assert_eq!(named(r#"#[rasn(other = "Count")]"#), None);
        assert_eq!(named(r#"#[rasn(value("0..=10"), default = "count_default")]"#), None);
    }
}
