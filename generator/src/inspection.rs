//! Generate dynamic open-type dispatch from the same ASN.1 as the codec.
use std::collections::BTreeMap;
use std::fmt::Write;

use anyhow::{Result, anyhow, bail, ensure};
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

/// The well-known types whose values a view and a tree show in a readable form: the
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

/// The ASN.1 identifier that `rasn` has for a type, a field or an alternative, and
/// whether the attributes of a type say that it is an ENUMERATED.
fn rasn(attributes: &[syn::Attribute]) -> Result<(Option<String>, bool)> {
    let (identifier, enumerated, _) = rasn_kind(attributes)?;
    Ok((identifier, enumerated))
}

/// Whether the attributes of a string give it one size and no extension marker: a bit
/// string of such a type is written as its octets alone, without its length.
fn one_size(attributes: &[syn::Attribute]) -> Result<bool> {
    use syn::{Meta, Token, punctuated::Punctuated};
    for attribute in attributes.iter().filter(|a| a.path().is_ident("rasn")) {
        let metas = attribute.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
        for meta in metas {
            if let Meta::List(list) = meta
                && list.path.is_ident("size")
            {
                let size = list.tokens.to_string();
                let mut parts = size.split(',').map(str::trim);
                let digits = parts.next().unwrap_or_default().trim_matches('"');
                let one = !digits.is_empty() && digits.bytes().all(|digit| digit.is_ascii_digit());
                return Ok(one && !parts.any(|part| part == "extensible"));
            }
        }
    }
    Ok(false)
}

/// What [`rasn`] gives, and whether the attributes of a member give it a default value.
fn rasn_kind(attributes: &[syn::Attribute]) -> Result<(Option<String>, bool, bool)> {
    use syn::{Meta, Token, punctuated::Punctuated};
    let (mut identifier, mut enumerated, mut default) = (None, false, false);
    for attribute in attributes.iter().filter(|a| a.path().is_ident("rasn")) {
        let metas = attribute.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
        for meta in metas {
            match meta {
                Meta::Path(path) if path.is_ident("enumerated") => enumerated = true,
                Meta::Path(path) if path.is_ident("default") => default = true,
                Meta::NameValue(pair) if pair.path.is_ident("default") => default = true,
                Meta::NameValue(pair) if pair.path.is_ident("identifier") => {
                    let syn::Expr::Lit(literal) = pair.value else {
                        continue;
                    };
                    if let syn::Lit::Str(name) = literal.lit {
                        identifier = Some(name.value());
                    }
                }
                _ => {}
            }
        }
    }
    Ok((identifier, enumerated, default))
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

/// The readable form of each well-known type, by the name that the bindings give it,
/// and the names of the values of each ENUMERATED.
fn generate_readable(generated: &str, out: &mut String) -> Result<()> {
    use syn::ext::IdentExt;
    let syntax = syn::parse_file(generated)?;
    let mut declarations = Vec::new();
    items(&syntax.items, "", &mut declarations);
    let mut wrapped = Wrapped::new();
    for (_, item) in &declarations {
        if let syn::Item::Struct(item) = item
            && let syn::Fields::Unnamed(fields) = &item.fields
            && fields.unnamed.len() == 1
        {
            let asn = rasn(&item.attrs)?.0;
            let asn = asn.unwrap_or_else(|| item.ident.to_string());
            wrapped.insert(item.ident.to_string(), (asn, &fields.unnamed[0].ty));
        }
    }
    // The names of the values of each ENUMERATED, in the order of its definition.
    let mut enumerations: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (_, item) in &declarations {
        if let syn::Item::Enum(item) = item
            && rasn(&item.attrs)?.1
        {
            for value in &item.variants {
                let name = rasn(&value.attrs)?.0;
                let name = name.unwrap_or_else(|| value.ident.unraw().to_string());
                enumerations
                    .entry(item.ident.to_string())
                    .or_default()
                    .push(name);
            }
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
    writeln!(
        out,
        "/// The names of the values of each ENUMERATED, by the name of its type in the \
         bindings, in the order of the names of the types.\n\
         pub(crate) const ENUMERATIONS: &[(&str, &[&str])] = &["
    )?;
    for (ty, names) in &enumerations {
        writeln!(out, "({ty:?}, &{names:?}),")?;
    }
    writeln!(out, "];")?;
    Ok(())
}

/// The containers of the protocol, which a view shows as IEs, and how the list of the
/// types writes each.
const CONTAINERS: &[(&str, &str)] = &[
    ("ProtocolIEContainer", "ies"),
    ("ProtocolIEField", "ie"),
    ("ProtocolExtensionContainer", "extensions"),
    ("ProtocolExtensionField", "extension"),
];

/// How the list of the types writes a value of type `ty`, and whether a member of that
/// type may be absent: `_` for a value without members, `null` for the one value of the
/// type NULL, `bits` for a bit string whose size varies, the name of a SEQUENCE, of a
/// CHOICE, of an ENUMERATED or of a type that is a list, a list in brackets and a
/// container by its kind.
fn shape(ty: &syn::Type, declared: &BTreeMap<String, &syn::Item>, depth: u8) -> (String, bool) {
    let plain = || ("_".to_string(), false);
    if let syn::Type::Tuple(tuple) = ty
        && tuple.elems.is_empty()
    {
        return ("null".to_string(), false);
    }
    let Some((name, arguments)) = named(ty).filter(|_| depth < 16) else {
        return plain();
    };
    let inner = arguments.iter().find_map(|argument| match argument {
        syn::GenericArgument::Type(inner) => Some(inner),
        _ => None,
    });
    if let Some((_, written)) = CONTAINERS.iter().find(|(container, _)| *container == name) {
        return (written.to_string(), false);
    }
    match (name.as_str(), inner, declared.get(&name)) {
        ("Option", Some(inner), _) => (shape(inner, declared, depth + 1).0, true),
        ("Box", Some(inner), _) => shape(inner, declared, depth + 1),
        ("SequenceOf" | "SetOf" | "Vec", Some(inner), _) => {
            (format!("[{}]", shape(inner, declared, depth + 1).0), false)
        }
        ("BitString" | "SizedBitString", ..) => ("bits".to_string(), false),
        // A well-known type has its name, which gives its readable form.
        (_, _, Some(syn::Item::Struct(item)))
            if rasn(&item.attrs)
                .ok()
                .and_then(|(asn, _)| well_known(&asn.unwrap_or_else(|| name.clone())))
                .is_some() =>
        {
            (name.clone(), false)
        }
        // A type that only wraps another is what it wraps, when that has no members.
        (_, _, Some(syn::Item::Struct(item))) => match &item.fields {
            syn::Fields::Named(_) => (name.clone(), false),
            syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                match shape(&fields.unnamed[0].ty, declared, depth + 1).0.as_str() {
                    "_" => plain(),
                    "bits" if one_size(&item.attrs).unwrap_or(false) => plain(),
                    _ => (name.clone(), false),
                }
            }
            _ => plain(),
        },
        // A CHOICE has its alternatives, and an ENUMERATED its names.
        (_, _, Some(syn::Item::Enum(_))) => (name.clone(), false),
        _ => plain(),
    }
}

/// The list of the types: one line for each SEQUENCE and each CHOICE, with its members
/// or its alternatives by the names that ASN.1 gives them, and one for each type that is
/// a list or a bit string. A member that holds a transfer has the type that its
/// octets contain in parentheses.
fn generate_types(
    generated: &str,
    transfers: &BTreeMap<String, std::collections::BTreeSet<String>>,
    out: &mut String,
) -> Result<()> {
    use syn::ext::IdentExt;
    let syntax = syn::parse_file(generated)?;
    let mut declarations = Vec::new();
    items(&syntax.items, "", &mut declarations);
    let mut declared = BTreeMap::new();
    for (_, item) in &declarations {
        let name = match item {
            syn::Item::Struct(item) => &item.ident,
            syn::Item::Enum(item) => &item.ident,
            _ => unreachable!(),
        };
        ensure!(
            declared.insert(name.to_string(), *item).is_none(),
            "{name}: two types of the bindings have this name"
        );
    }
    let member = |attributes: &[syn::Attribute], ident: &syn::Ident, ty: &syn::Type| {
        let (name, _, default) = rasn_kind(attributes)?;
        let name = name.unwrap_or_else(|| ident.unraw().to_string());
        let (mut written, optional) = shape(ty, &declared, 0);
        // The constraint of a member that may be absent is not that of its value, as
        // rasn has it: a bit string has its length there, whatever its size.
        if written == "bits" && one_size(attributes)? && !(optional || default) {
            written = "_".to_string();
        }
        // The octets of a transfer are shown as the value that they contain.
        if let Some(contained) = transfers.get(&name) {
            ensure!(written == "_", "{name}: a transfer is a string of octets");
            let contained = alone(contained).unwrap_or_default();
            written = format!("({})", contained.rsplit("::").next().unwrap_or_default());
        }
        let presence = match optional || default {
            true => '?',
            false => ':',
        };
        Ok(format!("{name:?}{presence} {written}"))
    };
    // rustfmt leaves the lines of a macro called with braces as they are.
    writeln!(out, "types! {{")?;
    for (name, item) in &declared {
        if CONTAINERS.iter().any(|(container, _)| container == name) {
            continue;
        }
        match item {
            syn::Item::Struct(item) => match &item.fields {
                syn::Fields::Named(fields) => {
                    let members = fields.named.iter().map(|field| {
                        member(
                            &field.attrs,
                            field.ident.as_ref().expect("a name"),
                            &field.ty,
                        )
                    });
                    let members = members.collect::<Result<Vec<_>>>()?;
                    writeln!(out, "    {name} sequence {{ {} }};", members.join(", "))?;
                }
                syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                    let (written, _) = shape(&fields.unnamed[0].ty, &declared, 0);
                    if written != "_" && !(written == "bits" && one_size(&item.attrs)?) {
                        writeln!(out, "    {name} = {written};")?;
                    }
                }
                _ => {}
            },
            syn::Item::Enum(item) if !rasn(&item.attrs)?.1 => {
                let alternatives = item.variants.iter().map(|alternative| {
                    match alternative.fields.iter().next() {
                        Some(field) => member(&alternative.attrs, &alternative.ident, &field.ty),
                        None => {
                            let name = rasn(&alternative.attrs)?.0;
                            let name =
                                name.unwrap_or_else(|| alternative.ident.unraw().to_string());
                            Ok(format!("{name:?}: _"))
                        }
                    }
                });
                let alternatives = alternatives.collect::<Result<Vec<_>>>()?;
                writeln!(out, "    {name} choice {{ {} }};", alternatives.join(", "))?;
            }
            _ => {}
        }
    }
    writeln!(out, "}}")?;
    Ok(())
}

/// The tables that only the inspection has: the transfers, the readable forms, the
/// members of the types and the open types. The messages and the IEs are those of the
/// registry.
///
/// `carried` are the IEs whose octets ASN.1 leaves open and the specification gives a
/// type of the protocol, each as the name of the IE after `id-` and the name of that
/// type: the ASN.1 does not say them.
pub fn generate(
    protocol: &str,
    generated: &str,
    asn: &str,
    registry: &Registry,
    carried: &[(&str, &str)],
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
        "// Auto-generated from ASN.1 by the generator of oxirush-aper; do not edit.\nuse crate::inspect::{Form, Typed, carried, transfers};\nuse crate::inspect_paths::types;\npub(crate) use crate::registry::{IE_NAMES, IE_TYPES, ITEM_CRITICALITIES, MESSAGE_IE_CRITICALITIES, MESSAGES, PROCEDURE_CRITICALITIES, ie, ie_contents};\n",
    );
    writeln!(
        out,
        "pub(crate) const PROTOCOL: &str = {protocol:?};\npub(crate) const ROOT: &str = {module:?};"
    )?;
    // The member that holds a transfer, and the type that its octets contain.
    writeln!(out, "transfers! {{")?;
    for (field, alternatives) in &transfers {
        match alone(alternatives) {
            Some(ty) => writeln!(out, "    {field:?} {ty};")?,
            None => writeln!(out, "    {field:?};")?,
        }
    }
    writeln!(out, "}}")?;
    // The identifier of an IE whose octets may carry a type, the name that ASN.1 gives
    // that type, and the type.
    writeln!(out, "carried! {{")?;
    for (ie, name) in carried {
        let mut ies = registry.ies.iter();
        let (id, _) = ies
            .find(|(_, known)| known.name == *ie)
            .ok_or_else(|| anyhow!("{ie} names no IE"))?;
        let ty = registry.resolve(name);
        let ty = ty.ok_or_else(|| anyhow!("{name} names no type"))?;
        writeln!(out, "    {id} {name:?} {ty};")?;
    }
    writeln!(out, "}}")?;
    generate_readable(generated, &mut out)?;
    generate_types(generated, &transfers, &mut out)?;
    Ok(out)
}
