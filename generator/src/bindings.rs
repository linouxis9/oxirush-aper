//! The bindings on `oxirush-aper`: the types that rasn-compiler resolved from the
//! ASN.1, each written with its codec in aligned PER, its conversions and, with the
//! `inspect` feature, its JSON form.
//!
//! rasn-compiler parses the modules and resolves their references, their parameters
//! and their constraints, and writes what it resolved as bindings for rasn. Those
//! bindings are the description that this backend reads: a struct is a SEQUENCE, an
//! enum an ENUMERATED or a CHOICE, a tuple struct a type with a name, and the `rasn`
//! attributes have the names that ASN.1 gives and the constraints. The same
//! description becomes the oracle of the tests, with the codec of rasn, so that both
//! have the same types by construction.
//!
//! What the two protocols do not have is refused, and not written wrongly: a member
//! with a default value, an extension addition of a SEQUENCE, a size with an
//! extension marker on a list, a constraint that is no range.
use std::collections::BTreeSet;
use std::fmt::Write;

use anyhow::{Context, Result, bail, ensure};
use syn::{Attribute, Fields, Item, Type};

/// A constraint of a size or of a value: its bounds, where it has them, and whether it
/// has an extension marker.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Range {
    lower: Option<i128>,
    upper: Option<i128>,
    extensible: bool,
}

/// What the `rasn` attributes of an item say.
#[derive(Clone, Debug, Default)]
struct Attributes {
    /// The name that ASN.1 gives, where it is not the name of the binding.
    identifier: Option<String>,
    size: Option<Range>,
    value: Option<Range>,
    addition: bool,
    choice: bool,
    enumerated: bool,
    delegate: bool,
    docs: Vec<String>,
    non_exhaustive: bool,
}

/// A type of ASN.1, as the bindings hold its values.
#[derive(Clone, Debug, PartialEq)]
enum Kind {
    Boolean,
    Null,
    /// A whole number of a native type, with its constraint.
    Integer {
        native: &'static str,
        range: Range,
    },
    /// An OCTET STRING of one size: an array.
    OctetArray(usize),
    /// Another OCTET STRING, with its size constraint.
    Octets(Range),
    /// A BIT STRING of 64 bits at most, without an extension marker.
    Bits(Range),
    /// Another BIT STRING.
    BitString(Range),
    /// A character string: the name of its type in the reader, and its size.
    Text {
        of: &'static str,
        range: Range,
    },
    ObjectIdentifier,
    /// An open type.
    Open,
    List {
        of: Box<Kind>,
        range: Range,
    },
    /// A type of the bindings, by its name.
    Named(String),
}

struct Member {
    name: String,
    /// The name that ASN.1 gives the member or the alternative.
    asn: String,
    kind: Kind,
    optional: bool,
    addition: bool,
    /// The value of a value of an ENUMERATED.
    number: i128,
    docs: Vec<String>,
}

enum Shape {
    Sequence(Vec<Member>),
    Choice(Vec<Member>),
    Enumerated(Vec<Member>),
    /// A type with a name: what it is.
    Alias(Kind),
}

struct Definition {
    name: String,
    /// The name that ASN.1 gives the type.
    asn: String,
    extensible: bool,
    shape: Shape,
    docs: Vec<String>,
    /// The parameters of `new` that take what converts into their type.
    into: BTreeSet<String>,
    /// The alternatives that a `From` gives, by the type that it converts.
    from: Vec<String>,
}

enum Entry {
    Definition(Box<Definition>),
    /// An item that is written as it is: a constant, and the alias of a type.
    Constant {
        docs: Vec<String>,
        text: String,
    },
    TypeAlias {
        docs: Vec<String>,
        name: String,
        target: String,
    },
}

struct Module {
    name: String,
    entries: Vec<Entry>,
}

/// The bindings on oxirush-aper, and which of their types borrow from the octets that
/// they are decoded from: those have a lifetime.
pub struct Written {
    pub text: String,
    pub borrowing: BTreeSet<String>,
}

/// The bindings on oxirush-aper of the description that rasn-compiler gave.
pub fn bindings(description: &str) -> Result<Written> {
    let file = syn::parse_file(description).context("parse the resolved bindings")?;
    let mut modules = Vec::new();
    for item in &file.items {
        let Item::Mod(module) = item else {
            bail!("the resolved bindings have an item that is no module");
        };
        let (_, items) = module
            .content
            .as_ref()
            .context("a module without content")?;
        modules.push(Module {
            name: module.ident.to_string(),
            entries: entries(items).with_context(|| format!("in {}", module.ident))?,
        });
    }
    let bindings = Bindings::new(&modules)?;
    let text = bindings.write(&modules)?;
    Ok(Written {
        text,
        borrowing: bindings.borrowing,
    })
}

// ── Reading the description ─────────────────────────────────────────────────

fn entries(items: &[Item]) -> Result<Vec<Entry>> {
    let named_number = regex::Regex::new(r"Integer :: from \((-?\d+)i128\)")?;
    let converted = regex::Regex::new(r"Self :: (\w+) \(value\)")?;
    let mut entries: Vec<Entry> = Vec::new();
    for item in items {
        match item {
            Item::ExternCrate(_) | Item::Use(_) => {}
            Item::Const(constant) => {
                let docs = attributes(&constant.attrs)?.docs;
                let (name, ty) = (&constant.ident, tokens(&constant.ty));
                let value = tokens(&constant.expr);
                let text = format!("pub const {name}: {ty} = {value};");
                entries.push(Entry::Constant { docs, text });
            }
            // A value of an INTEGER that has a name: a constant of a native type.
            Item::Static(value) => {
                let docs = attributes(&value.attrs)?.docs;
                let written = tokens(&value.expr);
                let number = named_number
                    .captures(&written)
                    .and_then(|captures| captures[1].parse::<i128>().ok())
                    .with_context(|| format!("{} is no number: {written}", value.ident))?;
                let native = if number < 0 { "i64" } else { "u64" };
                let text = format!("pub const {}: {native} = {number};", value.ident);
                entries.push(Entry::Constant { docs, text });
            }
            Item::Type(alias) => {
                let docs = attributes(&alias.attrs)?.docs;
                entries.push(Entry::TypeAlias {
                    docs,
                    name: alias.ident.to_string(),
                    target: tokens(&alias.ty).replace(' ', ""),
                });
            }
            Item::Struct(item) => {
                let attributes = attributes(&item.attrs)?;
                let name = item.ident.to_string();
                let shape = match &item.fields {
                    Fields::Named(fields) => {
                        ensure!(!attributes.delegate, "{name} delegates to named members");
                        let members: Result<Vec<_>> = fields.named.iter().map(member).collect();
                        Shape::Sequence(members.with_context(|| format!("in {name}"))?)
                    }
                    Fields::Unnamed(fields) => {
                        ensure!(
                            attributes.delegate && fields.unnamed.len() == 1,
                            "{name} is no type with a name"
                        );
                        let inner = &fields.unnamed[0];
                        let own = self::attributes(&inner.attrs)?;
                        ensure!(own.size.is_none() && own.value.is_none(), "{name}");
                        Shape::Alias(
                            kind(&inner.ty, &attributes).with_context(|| format!("in {name}"))?,
                        )
                    }
                    Fields::Unit => bail!("{name} has no member"),
                };
                entries.push(Entry::Definition(Box::new(Definition {
                    asn: attributes
                        .identifier
                        .clone()
                        .unwrap_or_else(|| name.clone()),
                    name,
                    extensible: attributes.non_exhaustive,
                    shape,
                    docs: attributes.docs,
                    into: BTreeSet::new(),
                    from: Vec::new(),
                })));
            }
            Item::Enum(item) => {
                let attributes = attributes(&item.attrs)?;
                let name = item.ident.to_string();
                let mut members = Vec::new();
                for variant in &item.variants {
                    let own = self::attributes(&variant.attrs)?;
                    let asn = own.identifier.clone();
                    let mut member = Member {
                        name: variant.ident.to_string(),
                        asn: asn.unwrap_or_else(|| variant.ident.to_string()),
                        kind: Kind::Null,
                        optional: false,
                        addition: own.addition,
                        number: 0,
                        docs: own.docs.clone(),
                    };
                    if attributes.enumerated {
                        let (_, number) = variant
                            .discriminant
                            .as_ref()
                            .with_context(|| format!("{name} has a value without a number"))?;
                        member.number = tokens(number).replace(' ', "").parse()?;
                    } else {
                        let Fields::Unnamed(fields) = &variant.fields else {
                            bail!("{name} has an alternative without a type");
                        };
                        ensure!(fields.unnamed.len() == 1, "{name}");
                        member.kind = kind(&fields.unnamed[0].ty, &own)
                            .with_context(|| format!("in {name}"))?;
                    }
                    members.push(member);
                }
                ensure!(
                    attributes.enumerated != attributes.choice,
                    "{name} is neither an ENUMERATED nor a CHOICE"
                );
                let shape = match attributes.enumerated {
                    true => Shape::Enumerated(members),
                    false => Shape::Choice(members),
                };
                entries.push(Entry::Definition(Box::new(Definition {
                    asn: attributes
                        .identifier
                        .clone()
                        .unwrap_or_else(|| name.clone()),
                    name,
                    extensible: attributes.non_exhaustive,
                    shape,
                    docs: attributes.docs,
                    into: BTreeSet::new(),
                    from: Vec::new(),
                })));
            }
            // What rasn-compiler writes beside a type: its `new`, and the conversions
            // into a CHOICE from the types that one alternative alone has.
            Item::Impl(item) => {
                let of = tokens(&item.self_ty).replace(' ', "");
                let definition = entries.iter_mut().rev().find_map(|entry| match entry {
                    Entry::Definition(definition) if definition.name == of => Some(definition),
                    _ => None,
                });
                let definition = definition.with_context(|| format!("an impl of {of}"))?;
                match &item.trait_ {
                    None => {
                        for inner in &item.items {
                            let syn::ImplItem::Fn(function) = inner else {
                                continue;
                            };
                            for input in &function.sig.inputs {
                                let syn::FnArg::Typed(input) = input else {
                                    continue;
                                };
                                if tokens(&input.ty).starts_with("impl Into") {
                                    definition.into.insert(tokens(&input.pat));
                                }
                            }
                        }
                    }
                    Some((_, path, _)) => {
                        let written = tokens(path);
                        ensure!(
                            written.starts_with("From <"),
                            "an impl of {written} for {of}"
                        );
                        let body = tokens(item);
                        let alternative = converted
                            .captures(&body)
                            .map(|captures| captures[1].to_string())
                            .with_context(|| format!("the From of {of}"))?;
                        definition.from.push(alternative);
                    }
                }
            }
            other => bail!("an item that the backend does not read: {}", tokens(other)),
        }
    }
    Ok(entries)
}

fn tokens(node: &impl quote::ToTokens) -> String {
    node.to_token_stream().to_string()
}

fn member(field: &syn::Field) -> Result<Member> {
    let attributes = attributes(&field.attrs)?;
    let name = field.ident.as_ref().context("a member without a name")?;
    let name = name.to_string();
    let written = tokens(&field.ty).replace(' ', "");
    let (optional, ty) = match written
        .strip_prefix("Option<")
        .and_then(|inner| inner.strip_suffix('>'))
    {
        Some(inner) => (true, syn::parse_str::<Type>(inner)?),
        None => (false, field.ty.clone()),
    };
    ensure!(!attributes.addition, "{name} is an extension addition");
    Ok(Member {
        asn: attributes
            .identifier
            .clone()
            .unwrap_or_else(|| name.clone()),
        kind: kind(&ty, &attributes).with_context(|| format!("in {name}"))?,
        name,
        optional,
        addition: false,
        number: 0,
        docs: attributes.docs,
    })
}

fn attributes(attributes: &[Attribute]) -> Result<Attributes> {
    let mut read = Attributes::default();
    for attribute in attributes {
        if attribute.path().is_ident("doc") {
            if let syn::Meta::NameValue(doc) = &attribute.meta {
                read.docs.push(tokens(&doc.value));
            }
        } else if attribute.path().is_ident("non_exhaustive") {
            read.non_exhaustive = true;
        } else if attribute.path().is_ident("rasn") {
            attribute.parse_nested_meta(|meta| {
                let name = meta.path.get_ident().map(ToString::to_string);
                match name.as_deref() {
                    Some("identifier") => {
                        let value: syn::LitStr = meta.value()?.parse()?;
                        read.identifier = Some(value.value());
                    }
                    Some(of @ ("size" | "value")) => {
                        let content;
                        syn::parenthesized!(content in meta.input);
                        let written: syn::LitStr = content.parse()?;
                        let mut range = range(&written.value())
                            .map_err(|error| meta.error(format!("{error}")))?;
                        if content.parse::<Option<syn::Token![,]>>()?.is_some() {
                            let marker: syn::Ident = content.parse()?;
                            if marker != "extensible" {
                                return Err(meta.error("a constraint with another marker"));
                            }
                            range.extensible = true;
                        }
                        match of {
                            "size" => read.size = Some(range),
                            _ => read.value = Some(range),
                        }
                    }
                    Some("extension_addition") => read.addition = true,
                    Some("choice") => read.choice = true,
                    Some("enumerated") => read.enumerated = true,
                    Some("delegate") => read.delegate = true,
                    Some("automatic_tags") => {}
                    _ => return Err(meta.error("an attribute that the backend does not read")),
                }
                Ok(())
            })?;
        } else if !attribute.path().is_ident("derive") && !attribute.path().is_ident("allow") {
            bail!(
                "an attribute that the backend does not read: {}",
                tokens(attribute)
            );
        }
    }
    Ok(read)
}

/// The range that rasn-compiler writes: `3`, `1..=150`, `0..`.
fn range(written: &str) -> Result<Range> {
    let bound = |bound: &str| -> Result<Option<i128>> {
        match bound {
            "" => Ok(None),
            bound => Ok(Some(
                bound
                    .parse()
                    .with_context(|| format!("the bound {bound}"))?,
            )),
        }
    };
    let (lower, upper) = match written.split_once("..") {
        Some((lower, upper)) => (bound(lower)?, bound(upper.trim_start_matches('='))?),
        None => (bound(written)?, bound(written)?),
    };
    Ok(Range {
        lower,
        upper,
        extensible: false,
    })
}

/// What a type of the description is, with the constraints of where it is written.
fn kind(ty: &Type, attributes: &Attributes) -> Result<Kind> {
    let written = tokens(ty).replace(' ', "");
    let fixed = |prefix: &str| -> Option<usize> {
        let size = written.strip_prefix(prefix)?.strip_suffix("usize>")?;
        size.parse().ok()
    };
    let size = attributes.size.unwrap_or_default();
    let sized = |range: Range| -> Result<Range> {
        ensure!(
            range.lower.is_none_or(|lower| lower >= 0),
            "a negative size"
        );
        Ok(range)
    };
    let no_size = || -> Result<()> {
        ensure!(attributes.size.is_none(), "a size constraint on {written}");
        Ok(())
    };
    let no_value = || -> Result<()> {
        ensure!(
            attributes.value.is_none(),
            "a value constraint on {written}"
        );
        Ok(())
    };
    if let Some(inner) = written
        .strip_prefix("SequenceOf<")
        .and_then(|inner| inner.strip_suffix('>'))
    {
        no_value()?;
        ensure!(
            !size.extensible,
            "a list whose size has an extension marker"
        );
        let of = kind(&syn::parse_str::<Type>(inner)?, &Attributes::default())?;
        return Ok(Kind::List {
            of: Box::new(of),
            range: sized(size)?,
        });
    }
    if let Some(octets) = fixed("FixedOctetString<") {
        no_value()?;
        ensure!(
            attributes.size.is_none_or(|size| size == one(octets)),
            "an OCTET STRING of {octets} octets with another size"
        );
        return Ok(Kind::OctetArray(octets));
    }
    if let Some(bits) = fixed("FixedBitString<") {
        no_value()?;
        let size = attributes.size.unwrap_or(one(bits));
        ensure!(
            (size.lower, size.upper) == (Some(bits as i128), Some(bits as i128)),
            "a BIT STRING of {bits} bits with another size"
        );
        return Ok(bit_string(size));
    }
    Ok(match written.as_str() {
        "bool" => Kind::Boolean,
        "()" => Kind::Null,
        "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" | "Integer" => {
            no_size()?;
            integer(attributes.value.unwrap_or_default())?
        }
        "OctetString" => {
            // rasn-compiler writes `value("0..")` for an OCTET STRING that is constrained
            // to contain a type, which constrains nothing in PER.
            let contains = Range {
                lower: Some(0),
                upper: None,
                extensible: false,
            };
            ensure!(
                attributes.value.is_none_or(|value| value == contains),
                "a value constraint on {written}"
            );
            match (size.lower, size.upper) {
                (Some(lower), Some(upper))
                    if lower == upper && upper < 65536 && !size.extensible =>
                {
                    Kind::OctetArray(upper as usize)
                }
                _ => Kind::Octets(sized(size)?),
            }
        }
        "BitString" => {
            no_value()?;
            bit_string(sized(size)?)
        }
        "PrintableString" | "VisibleString" | "Ia5String" | "Utf8String" => {
            no_value()?;
            let of = match written.as_str() {
                "PrintableString" => "printable_string",
                "VisibleString" => "visible_string",
                "Ia5String" => "ia5_string",
                _ => "utf8_string",
            };
            Kind::Text {
                of,
                range: sized(size)?,
            }
        }
        "ObjectIdentifier" => Kind::ObjectIdentifier,
        "Any" => Kind::Open,
        name => {
            ensure!(
                name.chars()
                    .all(|letter| letter.is_ascii_alphanumeric() || letter == '_'),
                "a type that the backend does not read: {name}"
            );
            no_size()?;
            no_value()?;
            Kind::Named(name.to_string())
        }
    })
}

fn one(size: usize) -> Range {
    Range {
        lower: Some(size as i128),
        upper: Some(size as i128),
        extensible: false,
    }
}

/// A BIT STRING of a size constraint: a number with a length when it has 64 bits at
/// most and no extension marker.
fn bit_string(size: Range) -> Kind {
    match size.upper {
        Some(upper) if upper <= 64 && !size.extensible => Kind::Bits(size),
        _ => Kind::BitString(size),
    }
}

/// An INTEGER of a value constraint: the smallest native type that holds its range,
/// and 64 bits where an extension marker lets a later version go past it.
fn integer(range: Range) -> Result<Kind> {
    let signed = range.lower.is_none_or(|lower| lower < 0);
    let native = match (range.lower, range.upper, range.extensible) {
        (Some(lower), Some(upper), false) => {
            ensure!(lower <= upper, "a constraint whose bounds are not in order");
            let fits = |least: i128, most: i128| least <= lower && upper <= most;
            match signed {
                false if fits(0, u8::MAX.into()) => "u8",
                false if fits(0, u16::MAX.into()) => "u16",
                false if fits(0, u32::MAX.into()) => "u32",
                false if fits(0, u64::MAX.into()) => "u64",
                true if fits(i8::MIN.into(), i8::MAX.into()) => "i8",
                true if fits(i16::MIN.into(), i16::MAX.into()) => "i16",
                true if fits(i32::MIN.into(), i32::MAX.into()) => "i32",
                true if fits(i64::MIN.into(), i64::MAX.into()) => "i64",
                _ => bail!("an INTEGER of more than 64 bits: {lower}..{upper}"),
            }
        }
        (Some(lower), Some(upper), true) => {
            let most: i128 = if signed {
                i64::MAX.into()
            } else {
                u64::MAX.into()
            };
            ensure!(
                lower >= i128::from(i64::MIN) && upper <= most,
                "an INTEGER of more than 64 bits: {lower}..{upper}"
            );
            if signed { "i64" } else { "u64" }
        }
        (_, None, _) => {
            ensure!(
                !range.extensible,
                "an extension marker without an upper bound"
            );
            if signed { "i64" } else { "u64" }
        }
        (None, Some(_), _) => bail!("an INTEGER with an upper bound and no lower one"),
    };
    Ok(Kind::Integer { native, range })
}

// ── Writing the bindings ────────────────────────────────────────────────────

struct Bindings {
    /// The types that borrow: those that hold octets, a long bit string, a character
    /// string or an open type, and those that hold such a type.
    borrowing: BTreeSet<String>,
    /// The types whose values are copied: numbers, arrays and short bit strings with a
    /// name, and the values of an ENUMERATED.
    copied: BTreeSet<String>,
    /// The SEQUENCE types whose members are all plain fields: their codec is read and
    /// written where it is used.
    plain: BTreeSet<String>,
}

impl Bindings {
    fn new(modules: &[Module]) -> Result<Self> {
        let definitions = || {
            modules
                .iter()
                .flat_map(|module| &module.entries)
                .filter_map(|entry| match entry {
                    Entry::Definition(definition) => Some(definition.as_ref()),
                    _ => None,
                })
        };
        let mut names = BTreeSet::new();
        for definition in definitions() {
            ensure!(
                names.insert(definition.name.as_str()),
                "{} is defined twice",
                definition.name
            );
        }
        let mut bindings = Self {
            borrowing: BTreeSet::new(),
            copied: BTreeSet::new(),
            plain: BTreeSet::new(),
        };
        // The types that hold a type that borrows borrow too: until none is added.
        loop {
            let before = bindings.borrowing.len();
            for definition in definitions() {
                let borrows = match &definition.shape {
                    Shape::Sequence(members) | Shape::Choice(members) => {
                        members.iter().any(|member| bindings.borrows(&member.kind))
                    }
                    Shape::Enumerated(_) => false,
                    Shape::Alias(kind) => bindings.borrows(kind),
                };
                if borrows {
                    bindings.borrowing.insert(definition.name.clone());
                }
            }
            if bindings.borrowing.len() == before {
                break;
            }
        }
        // A name for a type that is copied is copied too, as the PLMN identity that
        // is a TBCD string: until none is added. A SEQUENCE and a CHOICE are not, as a
        // later release may give either a member that is not.
        loop {
            let before = bindings.copied.len();
            for definition in definitions() {
                let copied = match &definition.shape {
                    Shape::Enumerated(_) => true,
                    Shape::Alias(Kind::Named(name)) => bindings.copied.contains(name),
                    Shape::Alias(kind) => matches!(
                        kind,
                        Kind::Boolean | Kind::Integer { .. } | Kind::OctetArray(_) | Kind::Bits(_)
                    ),
                    _ => false,
                };
                if copied {
                    bindings.copied.insert(definition.name.clone());
                }
            }
            if bindings.copied.len() == before {
                break;
            }
        }
        for definition in definitions() {
            if let Shape::Sequence(members) = &definition.shape
                && members.iter().all(|member| bindings.is_field(&member.kind))
            {
                bindings.plain.insert(definition.name.clone());
            }
        }
        Ok(bindings)
    }

    /// Whether a value of a kind is one field: a number, a string, an open type, or a
    /// name for one of these.
    fn is_field(&self, kind: &Kind) -> bool {
        match kind {
            Kind::List { .. } => false,
            Kind::Named(name) => self.copied.contains(name),
            _ => true,
        }
    }

    /// Whether a value of a kind is one field, or a SEQUENCE of them.
    fn is_plain(&self, kind: &Kind) -> bool {
        match kind {
            Kind::Named(name) if self.plain.contains(name) => true,
            kind => self.is_field(kind),
        }
    }

    fn borrows(&self, kind: &Kind) -> bool {
        match kind {
            Kind::Octets(_) | Kind::BitString(_) | Kind::Text { .. } | Kind::Open => true,
            Kind::List { of, .. } => self.borrows(of),
            Kind::Named(name) => self.borrowing.contains(name),
            _ => false,
        }
    }

    /// The type of the values of a kind, with the lifetime `life` where it borrows.
    fn ty(&self, kind: &Kind, life: &str) -> String {
        match kind {
            Kind::Boolean => "bool".into(),
            Kind::Null => "()".into(),
            Kind::Integer { native, .. } => (*native).into(),
            Kind::OctetArray(size) => format!("[u8; {size}]"),
            Kind::Octets(_) | Kind::Open => format!("Octets<{life}>"),
            Kind::Bits(_) => "Bits".into(),
            Kind::BitString(_) => format!("BitString<{life}>"),
            Kind::Text { .. } => format!("Cow<{life}, str>"),
            Kind::ObjectIdentifier => "ObjectIdentifier".into(),
            Kind::List { of, .. } => format!("Vec<{}>", self.ty(of, life)),
            Kind::Named(name) => match self.borrowing.contains(name) {
                true => format!("{name}<{life}>"),
                false => name.clone(),
            },
        }
    }

    fn write(&self, modules: &[Module]) -> Result<String> {
        let mut out = String::new();
        writeln!(
            out,
            "// Auto-generated from ASN.1 by the generator of oxirush-aper; do not edit."
        )?;
        writeln!(out, "#![allow(clippy::large_enum_variant)]\n")?;
        for module in modules {
            writeln!(
                out,
                "#[allow(non_camel_case_types, non_snake_case, non_upper_case_globals, unused, \
                 clippy::too_many_arguments)]"
            )?;
            writeln!(out, "pub mod {} {{", module.name)?;
            writeln!(out, "use super::*;")?;
            writeln!(out, "use alloc::borrow::Cow;")?;
            writeln!(out, "use alloc::vec::Vec;")?;
            writeln!(
                out,
                "use oxirush_aper::{{AsnType, BitString, Bits, Decode, Encode, Error, \
                 ObjectIdentifier, Octets, Reader, UNBOUNDED, Writer}};"
            )?;
            writeln!(out, "#[cfg(feature = \"inspect\")]")?;
            writeln!(
                out,
                "use oxirush_aper::json::{{self, Json, Reading, Refused}};"
            )?;
            writeln!(out, "#[cfg(feature = \"inspect\")]")?;
            writeln!(out, "use serde_json::Value;")?;
            for entry in &module.entries {
                match entry {
                    Entry::Constant { docs, text } => {
                        write_docs(&mut out, docs)?;
                        writeln!(out, "{text}")?;
                    }
                    Entry::TypeAlias { docs, name, target } => {
                        write_docs(&mut out, docs)?;
                        match self.borrowing.contains(target) {
                            true => writeln!(out, "pub type {name}<'a> = {target}<'a>;")?,
                            false => writeln!(out, "pub type {name} = {target};")?,
                        }
                    }
                    Entry::Definition(definition) => self
                        .definition(&mut out, definition)
                        .with_context(|| format!("in {}", definition.name))?,
                }
            }
            writeln!(out, "}}")?;
        }
        Ok(out)
    }

    fn definition(&self, out: &mut String, definition: &Definition) -> Result<()> {
        let name = &definition.name;
        let borrows = self.borrowing.contains(name);
        // `Name<'a>` where the type is declared, `Name<'_>` where it is named.
        let (declared, named, life) = match borrows {
            true => (format!("{name}<'a>"), format!("{name}<'_>"), "<'a>"),
            false => (name.clone(), name.clone(), ""),
        };
        write_docs(out, &definition.docs)?;
        let copy = if self.copied.contains(name) {
            ", Copy"
        } else {
            ""
        };
        writeln!(out, "#[derive(Debug, Clone, PartialEq, Eq, Hash{copy})]")?;
        if definition.extensible {
            writeln!(out, "#[non_exhaustive]")?;
        }
        match &definition.shape {
            Shape::Sequence(members) => {
                writeln!(out, "pub struct {declared} {{")?;
                for member in members {
                    write_docs(out, &member.docs)?;
                    writeln!(
                        out,
                        "pub {}: {},",
                        member.name,
                        self.member_ty(member, "'a")
                    )?;
                }
                writeln!(out, "}}")?;
                // `new`, as rasn-compiler writes it.
                writeln!(out, "impl{life} {declared} {{")?;
                let parameters: Vec<String> = members
                    .iter()
                    .map(|member| {
                        let ty = self.member_ty(member, "'a");
                        match definition.into.contains(&member.name) {
                            true => format!("{}: impl Into<{ty}>", member.name),
                            false => format!("{}: {ty}", member.name),
                        }
                    })
                    .collect();
                writeln!(
                    out,
                    "pub fn new({}) -> Self {{ Self {{",
                    parameters.join(", ")
                )?;
                for member in members {
                    match definition.into.contains(&member.name) {
                        true => writeln!(out, "{0}: {0}.into(),", member.name)?,
                        false => writeln!(out, "{},", member.name)?,
                    }
                }
                writeln!(out, "}} }}")?;
                if borrows {
                    self.owned_doc(out, name)?;
                    if self.plain.contains(name) {
                        writeln!(out, "#[inline]")?;
                    }
                    writeln!(
                        out,
                        "pub fn into_owned(self) -> {name}<'static> {{ {name} {{"
                    )?;
                    for member in members {
                        let value = format!("self.{}", member.name);
                        let owned = match member.optional {
                            true => self.owned_optional(&member.kind, &value),
                            false => self.owned(&member.kind, &value),
                        };
                        writeln!(out, "{}: {owned},", member.name)?;
                    }
                    writeln!(out, "}} }}")?;
                }
                writeln!(out, "}}")?;
                self.asn_type(out, &named, &definition.asn)?;
                self.sequence_codec(out, definition, members, &declared, &named)?;
                self.sequence_json(out, members, &named)?;
            }
            Shape::Choice(members) => {
                writeln!(out, "pub enum {declared} {{")?;
                for member in members {
                    write_docs(out, &member.docs)?;
                    writeln!(out, "{}({}),", member.name, self.ty(&member.kind, "'a"))?;
                }
                writeln!(out, "}}")?;
                if borrows {
                    writeln!(out, "impl{life} {declared} {{")?;
                    self.owned_doc(out, name)?;
                    if members.iter().all(|member| self.is_plain(&member.kind)) {
                        writeln!(out, "#[inline]")?;
                    }
                    writeln!(
                        out,
                        "pub fn into_owned(self) -> {name}<'static> {{ match self {{"
                    )?;
                    for member in members {
                        let owned = self.owned(&member.kind, "value");
                        writeln!(
                            out,
                            "Self::{0}(value) => {name}::{0}({owned}),",
                            member.name
                        )?;
                    }
                    writeln!(out, "}} }} }}")?;
                }
                // A conversion from the type that one alternative alone has.
                for alternative in &definition.from {
                    let member = members
                        .iter()
                        .find(|member| &member.name == alternative)
                        .with_context(|| format!("no alternative {alternative}"))?;
                    let ty = self.ty(&member.kind, "'a");
                    let alone = members
                        .iter()
                        .filter(|other| self.ty(&other.kind, "'a") == ty);
                    if alone.count() != 1 {
                        continue;
                    }
                    writeln!(
                        out,
                        "impl{life} From<{ty}> for {declared} {{ fn from(value: {ty}) -> Self {{ \
                         Self::{alternative}(value) }} }}"
                    )?;
                }
                self.asn_type(out, &named, &definition.asn)?;
                self.choice_codec(out, definition, members, &declared, &named)?;
                self.choice_json(out, members, &named)?;
            }
            Shape::Enumerated(members) => {
                writeln!(out, "pub enum {name} {{")?;
                for member in members {
                    write_docs(out, &member.docs)?;
                    writeln!(out, "{} = {},", member.name, member.number)?;
                }
                writeln!(out, "}}")?;
                self.asn_type(out, &named, &definition.asn)?;
                self.enumerated_codec(out, definition, members)?;
                writeln!(out, "#[cfg(feature = \"inspect\")]")?;
                writeln!(out, "impl Json for {name} {{")?;
                writeln!(
                    out,
                    "fn to_json(&self) -> Value {{ json::text(Self::NAMES[self.index() as usize]) }}"
                )?;
                writeln!(
                    out,
                    "fn from_json(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {{ \
                     Ok(Self::VALUES[reading.enumerated(value, &Self::NAMES)?]) }}"
                )?;
                writeln!(out, "}}")?;
            }
            Shape::Alias(kind) => {
                let ty = self.ty(kind, "'a");
                writeln!(out, "pub struct {declared}(pub {ty});")?;
                if borrows {
                    writeln!(out, "impl{life} {declared} {{")?;
                    self.owned_doc(out, name)?;
                    let owned = self.owned(kind, "self.0");
                    if !matches!(kind, Kind::List { .. }) {
                        writeln!(out, "#[inline]")?;
                    }
                    writeln!(
                        out,
                        "pub fn into_owned(self) -> {name}<'static> {{ {name}({owned}) }}"
                    )?;
                    writeln!(out, "}}")?;
                }
                let from = |out: &mut String, of: &str, value: &str| {
                    writeln!(
                        out,
                        "impl{life} From<{of}> for {declared} {{ fn from(value: {of}) -> Self {{ \
                         Self({value}) }} }}"
                    )
                };
                from(out, &ty, "value")?;
                match kind {
                    Kind::Octets(_) | Kind::Open => {
                        from(out, "Vec<u8>", "Octets::from(value)")?;
                        from(out, "&'a [u8]", "Octets::from(value)")?;
                    }
                    // A string of whole octets is written by them.
                    Kind::BitString(range) => {
                        if let (Some(lower), Some(upper)) = (range.lower, range.upper)
                            && lower == upper
                            && !range.extensible
                            && upper % 8 == 0
                        {
                            let octets = upper / 8;
                            from(out, &format!("[u8; {octets}]"), "BitString::from(value)")?;
                        }
                    }
                    Kind::Text { .. } => {
                        from(out, "&'a str", "Cow::Borrowed(value)")?;
                        from(out, "alloc::string::String", "Cow::Owned(value)")?;
                    }
                    // A short string of one size is written by the number that its bits
                    // are: the type has the size.
                    Kind::Bits(range) => {
                        if let (Some(lower), Some(upper)) = (range.lower, range.upper)
                            && lower == upper
                        {
                            writeln!(
                                out,
                                "impl {name} {{\n/// The string of the {upper} bits of `value`, \
                                 the most significant first.\n///\n/// # Errors\n///\n/// An \
                                 error of the kind `Argument` for a value that {upper} bits do \
                                 not hold.\npub fn new(value: u64) -> Result<Self, Error> {{ \
                                 Bits::new(value, {upper}).map(Self) }}\n}}"
                            )?;
                        }
                    }
                    _ => {}
                }
                self.asn_type(out, &named, &definition.asn)?;
                // A name for one field is that field where the name is read and
                // written: a list is a loop, which stays a function.
                let inline = match kind {
                    Kind::List { .. } => "",
                    _ => "#[inline]\n",
                };
                writeln!(out, "impl<'a> Decode<'a> for {declared} {{")?;
                writeln!(
                    out,
                    "{inline}fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {{"
                )?;
                let value = self.decode(kind, &|result| format!("{result}?"))?;
                writeln!(out, "Ok(Self({value}))")?;
                writeln!(out, "}} }}")?;
                writeln!(out, "impl Encode for {named} {{")?;
                writeln!(
                    out,
                    "{inline}fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {{"
                )?;
                self.encode(out, kind, "self.0", false)?;
                writeln!(out, "Ok(())")?;
                writeln!(out, "}} }}")?;
                writeln!(out, "#[cfg(feature = \"inspect\")]")?;
                writeln!(out, "impl Json for {named} {{")?;
                writeln!(
                    out,
                    "fn to_json(&self) -> Value {{ {} }}",
                    self.to_json(kind, "self.0", false)
                )?;
                writeln!(
                    out,
                    "fn from_json(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {{ \
                     Ok(Self({}?)) }}",
                    self.read_json(kind)
                )?;
                writeln!(out, "}}")?;
            }
        }
        Ok(())
    }

    /// The documentation of `into_owned` of the type `name`. A value that is kept is
    /// what a caller looks for by other words, which find it in the documentation.
    fn owned_doc(&self, out: &mut String, name: &str) -> Result<()> {
        writeln!(
            out,
            "/// The same value, which borrows nothing and is kept as long as it is needed: \
             a `{name}<'static>`, which is the type to write in a struct. What the value \
             borrowed from the octets that it was decoded from is copied."
        )?;
        writeln!(out, "#[doc(alias(\"to_owned\", \"owned\", \"keep\"))]")?;
        Ok(())
    }

    fn asn_type(&self, out: &mut String, named: &str, asn: &str) -> Result<()> {
        writeln!(
            out,
            "impl AsnType for {named} {{ const NAME: &'static str = {asn:?}; }}"
        )?;
        Ok(())
    }

    fn member_ty(&self, member: &Member, life: &str) -> String {
        let ty = self.ty(&member.kind, life);
        match member.optional {
            true => format!("Option<{ty}>"),
            false => ty,
        }
    }

    /// The expression that makes `value`, of a kind, one that borrows nothing.
    fn owned(&self, kind: &Kind, value: &str) -> String {
        if !self.borrows(kind) {
            return value.to_string();
        }
        match kind {
            Kind::Text { .. } => format!("Cow::Owned({value}.into_owned())"),
            Kind::List { of, .. } => {
                let item = self.owned(of, "item");
                format!("{value}.into_iter().map(|item| {item}).collect()")
            }
            _ => format!("{value}.into_owned()"),
        }
    }

    fn owned_optional(&self, kind: &Kind, value: &str) -> String {
        if !self.borrows(kind) {
            return value.to_string();
        }
        format!("{value}.map(|value| {})", self.owned(kind, "value"))
    }

    // ── SEQUENCE ────────────────────────────────────────────────────────────

    fn sequence_codec(
        &self,
        out: &mut String,
        definition: &Definition,
        members: &[Member],
        declared: &str,
        named: &str,
    ) -> Result<()> {
        let optional: Vec<&Member> = members.iter().filter(|member| member.optional).collect();
        ensure!(
            optional.len() <= 64,
            "more than 64 members that may be absent"
        );
        // A SEQUENCE of a few fields, none of which is a value with members of its
        // own, is read and written where it is used: the field of a protocol IE
        // container is one, and a message is a list of them.
        let inline = match self.plain.contains(&definition.name) {
            true => "#[inline]\n",
            false => "",
        };
        writeln!(out, "impl<'a> Decode<'a> for {declared} {{")?;
        writeln!(
            out,
            "{inline}fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {{"
        )?;
        if definition.extensible {
            writeln!(out, "let extended = reader.bit()?;")?;
        }
        if !optional.is_empty() {
            writeln!(out, "let present = reader.bits({})?;", optional.len())?;
        }
        let mut position = 0;
        for member in members {
            let name = &member.name;
            let in_field = |result: &str| {
                format!(
                    "match {result} {{ Ok(value) => value, \
                     Err(error) => return Err(error.in_field({name:?})) }}"
                )
            };
            let value = self.decode(&member.kind, &in_field)?;
            match member.optional {
                true => {
                    let bit = 1u64 << (optional.len() - 1 - position);
                    position += 1;
                    writeln!(
                        out,
                        "let {name} = if present & {bit} != 0 {{ Some({value}) }} else {{ None }};"
                    )?;
                }
                false => writeln!(out, "let {name} = {value};")?,
            }
        }
        if definition.extensible {
            // The additions of a later version are skipped.
            writeln!(out, "if extended {{ reader.skip_additions()?; }}")?;
        }
        let names: Vec<&str> = members.iter().map(|member| member.name.as_str()).collect();
        writeln!(out, "Ok(Self {{ {} }})", names.join(", "))?;
        writeln!(out, "}} }}")?;

        writeln!(out, "impl Encode for {named} {{")?;
        writeln!(
            out,
            "{inline}fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {{"
        )?;
        if definition.extensible {
            writeln!(out, "writer.bit(false);")?;
        }
        for member in &optional {
            writeln!(out, "writer.bit(self.{}.is_some());", member.name)?;
        }
        for member in members {
            match member.optional {
                true => {
                    writeln!(out, "if let Some(value) = &self.{} {{", member.name)?;
                    self.encode(out, &member.kind, "value", true)?;
                    writeln!(out, "}}")?;
                }
                false => self.encode(out, &member.kind, &format!("self.{}", member.name), false)?,
            }
        }
        writeln!(out, "Ok(())")?;
        writeln!(out, "}} }}")?;
        Ok(())
    }

    // ── CHOICE ──────────────────────────────────────────────────────────────

    fn choice_codec(
        &self,
        out: &mut String,
        definition: &Definition,
        members: &[Member],
        declared: &str,
        named: &str,
    ) -> Result<()> {
        let root = members.iter().filter(|member| !member.addition).count();
        let additions = members.len() - root;
        ensure!(root > 0, "a CHOICE without an alternative in its root");
        ensure!(
            members
                .iter()
                .skip_while(|member| !member.addition)
                .all(|member| member.addition),
            "an alternative of the root after an addition"
        );
        ensure!(
            additions == 0 || definition.extensible,
            "an addition without a marker"
        );
        let extensible = definition.extensible;
        // A CHOICE between plain fields and SEQUENCEs of plain fields is read and
        // written where it is used, as they are: a PDU is one.
        let inline = match members.iter().all(|member| self.is_plain(&member.kind)) {
            true => "#[inline]\n",
            false => "",
        };
        writeln!(out, "impl<'a> Decode<'a> for {declared} {{")?;
        // A CHOICE of one alternative has the index of its extension marker all the
        // same, and is written as the others are.
        if members.len() == 1 {
            writeln!(out, "#[allow(clippy::match_single_binding)]")?;
        }
        writeln!(
            out,
            "{inline}fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {{"
        )?;
        writeln!(
            out,
            "Ok(match reader.choice({root}, {additions}, {extensible})? {{"
        )?;
        for (index, member) in members.iter().enumerate() {
            // The last alternative is the one of every other index, which the reader
            // does not give.
            let pattern = match index + 1 == members.len() {
                true => "_".to_string(),
                false => index.to_string(),
            };
            let name = &member.name;
            if !member.addition {
                let value = self.decode(&member.kind, &|result| format!("{result}?"))?;
                writeln!(out, "{pattern} => Self::{name}({value}),")?;
                continue;
            }
            // An addition is in an open type (X.691 §23.8), whose octets are borrowed
            // unless they came in fragments.
            let value = self.decode(&member.kind, &|result| format!("{result}?"))?;
            writeln!(out, "{pattern} => {{ let octets = reader.open_type()?;")?;
            match self.borrows(&member.kind) {
                false => writeln!(
                    out,
                    "let reader = &mut Reader::new(&octets); Self::{name}({value}) }}"
                )?,
                true => {
                    let owned = self.owned(&member.kind, "value");
                    writeln!(
                        out,
                        "match octets.as_borrowed() {{ \
                         Some(octets) => {{ let reader = &mut Reader::new(octets); \
                         Self::{name}({value}) }} \
                         None => {{ let reader = &mut Reader::new(&octets); \
                         let value = {value}; Self::{name}({owned}) }} }} }}"
                    )?;
                }
            }
        }
        writeln!(out, "}})")?;
        writeln!(out, "}} }}")?;

        writeln!(out, "impl Encode for {named} {{")?;
        writeln!(
            out,
            "{inline}fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {{"
        )?;
        writeln!(out, "match self {{")?;
        for (index, member) in members.iter().enumerate() {
            writeln!(out, "Self::{}(value) => {{", member.name)?;
            writeln!(out, "writer.index({index}, {root}, {extensible});")?;
            if member.addition {
                writeln!(out, "let start = writer.begin_open_type();")?;
            }
            self.encode(out, &member.kind, "value", true)?;
            if member.addition {
                writeln!(out, "writer.end_open_type(start);")?;
            }
            writeln!(out, "}}")?;
        }
        writeln!(out, "}}")?;
        writeln!(out, "Ok(())")?;
        writeln!(out, "}} }}")?;
        Ok(())
    }

    // ── ENUMERATED ──────────────────────────────────────────────────────────

    fn enumerated_codec(
        &self,
        out: &mut String,
        definition: &Definition,
        members: &[Member],
    ) -> Result<()> {
        let name = &definition.name;
        // The index of a value is its place among the values of the root by their
        // numbers, then among the additions in their order (X.691 §14.1).
        let mut root: Vec<&Member> = members.iter().filter(|member| !member.addition).collect();
        root.sort_by_key(|member| member.number);
        let additions: Vec<&Member> = members.iter().filter(|member| member.addition).collect();
        ensure!(
            !root.is_empty(),
            "an ENUMERATED without a value in its root"
        );
        ensure!(
            additions.is_empty() || definition.extensible,
            "an addition without a marker"
        );
        ensure!(
            additions
                .windows(2)
                .all(|pair| pair[0].number < pair[1].number),
            "additions that are not in the order of their numbers"
        );
        let ordered: Vec<&Member> = root.iter().chain(&additions).copied().collect();
        let (root, additions) = (root.len(), additions.len());
        let extensible = definition.extensible;
        writeln!(out, "impl {name} {{")?;
        writeln!(out, "/// The values in the order of their indexes in PER.")?;
        let values: Vec<String> = ordered
            .iter()
            .map(|member| format!("Self::{}", member.name))
            .collect();
        writeln!(
            out,
            "pub(crate) const VALUES: [Self; {}] = [{}];",
            ordered.len(),
            values.join(", ")
        )?;
        writeln!(
            out,
            "/// The names that ASN.1 gives the values, in the same order."
        )?;
        let names: Vec<String> = ordered
            .iter()
            .map(|member| format!("{:?}", member.asn))
            .collect();
        writeln!(out, "#[cfg(feature = \"inspect\")]")?;
        writeln!(
            out,
            "pub(crate) const NAMES: [&'static str; {}] = [{}];",
            ordered.len(),
            names.join(", ")
        )?;
        // The number of a value is its index, unless ASN.1 numbers the values
        // otherwise.
        let numbered = ordered
            .iter()
            .enumerate()
            .all(|(index, member)| member.number == index as i128);
        writeln!(out, "/// The index of the value in PER.")?;
        writeln!(out, "#[inline]")?;
        match numbered {
            true => writeln!(out, "fn index(&self) -> u64 {{ *self as u64 }}")?,
            false => {
                writeln!(out, "fn index(&self) -> u64 {{ match self {{")?;
                for (index, member) in ordered.iter().enumerate() {
                    writeln!(out, "Self::{} => {index},", member.name)?;
                }
                writeln!(out, "}} }}")?;
            }
        }
        writeln!(out, "}}")?;
        writeln!(out, "impl<'a> Decode<'a> for {name} {{")?;
        writeln!(out, "#[inline]")?;
        writeln!(
            out,
            "fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {{"
        )?;
        writeln!(
            out,
            "Ok(Self::VALUES[reader.enumerated({root}, {additions}, {extensible})? as usize])"
        )?;
        writeln!(out, "}} }}")?;
        writeln!(out, "impl Encode for {name} {{")?;
        writeln!(out, "#[inline]")?;
        writeln!(
            out,
            "fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {{"
        )?;
        writeln!(out, "writer.index(self.index(), {root}, {extensible});")?;
        writeln!(out, "Ok(())")?;
        writeln!(out, "}} }}")?;
        Ok(())
    }

    // ── The JSON forms ──────────────────────────────────────────────────────

    fn sequence_json(&self, out: &mut String, members: &[Member], named: &str) -> Result<()> {
        writeln!(out, "#[cfg(feature = \"inspect\")]")?;
        writeln!(out, "impl Json for {named} {{")?;
        writeln!(out, "fn to_json(&self) -> Value {{")?;
        writeln!(out, "let mut members = json::Members::new();")?;
        for member in members {
            let (name, asn) = (&member.name, &member.asn);
            match member.optional {
                true => {
                    let value = self.to_json(&member.kind, "value", true);
                    writeln!(
                        out,
                        "members.put_optional({asn:?}, self.{name}.as_ref().map(|value| {value}));"
                    )?;
                }
                false => {
                    let value = self.to_json(&member.kind, &format!("self.{name}"), false);
                    writeln!(out, "members.put({asn:?}, {value});")?;
                }
            }
        }
        writeln!(out, "members.finish()")?;
        writeln!(out, "}}")?;
        writeln!(
            out,
            "fn from_json(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {{"
        )?;
        let names: Vec<String> = members
            .iter()
            .map(|member| format!("{:?}", member.asn))
            .collect();
        writeln!(
            out,
            "let members = reading.members(value, &[{}])?;",
            names.join(", ")
        )?;
        for (position, member) in members.iter().enumerate() {
            let read = match &member.kind {
                Kind::Named(name) => format!("{name}::from_json"),
                kind => format!("|value, reading| {}", self.read_json(kind)),
            };
            let how = if member.optional {
                "optional"
            } else {
                "member"
            };
            writeln!(
                out,
                "let {} = reading.{how}(&members, {position}, {read})?;",
                member.name
            )?;
        }
        let names: Vec<&str> = members.iter().map(|member| member.name.as_str()).collect();
        writeln!(out, "Ok(Self {{ {} }})", names.join(", "))?;
        writeln!(out, "}} }}")?;
        Ok(())
    }

    fn choice_json(&self, out: &mut String, members: &[Member], named: &str) -> Result<()> {
        writeln!(out, "#[cfg(feature = \"inspect\")]")?;
        writeln!(out, "impl Json for {named} {{")?;
        writeln!(out, "fn to_json(&self) -> Value {{ match self {{")?;
        for member in members {
            let value = self.to_json(&member.kind, "value", true);
            writeln!(
                out,
                "Self::{}(value) => json::alternative({:?}, {value}),",
                member.name, member.asn
            )?;
        }
        writeln!(out, "}} }}")?;
        if members.len() == 1 {
            writeln!(out, "#[allow(clippy::match_single_binding)]")?;
        }
        writeln!(
            out,
            "fn from_json(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {{"
        )?;
        let names: Vec<String> = members
            .iter()
            .map(|member| format!("{:?}", member.asn))
            .collect();
        writeln!(
            out,
            "let (position, value) = reading.alternative(value, &[{}])?;",
            names.join(", ")
        )?;
        writeln!(out, "Ok(match position {{")?;
        for (position, member) in members.iter().enumerate() {
            let pattern = match position + 1 == members.len() {
                true => "_".to_string(),
                false => position.to_string(),
            };
            let read = match &member.kind {
                Kind::Named(name) => format!("{name}::from_json"),
                kind => format!("|value, reading| {}", self.read_json(kind)),
            };
            writeln!(
                out,
                "{pattern} => Self::{}(reading.within({:?}, value, {read})?),",
                member.name, member.asn
            )?;
        }
        writeln!(out, "}})")?;
        writeln!(out, "}} }}")?;
        Ok(())
    }

    /// The expression that is the JSON form of `value`, of a kind. `value` is a
    /// reference when `by_reference` says so.
    fn to_json(&self, kind: &Kind, value: &str, by_reference: bool) -> String {
        let copied = match by_reference {
            true => format!("*{value}"),
            false => value.to_string(),
        };
        let referred = match by_reference {
            true => value.to_string(),
            false => format!("&{value}"),
        };
        match kind {
            Kind::Boolean => format!("Value::Bool({copied})"),
            Kind::Null => "Value::Null".into(),
            Kind::Integer { native, .. } => match *native {
                "u64" => format!("json::unsigned({copied})"),
                "i64" => format!("json::signed({copied})"),
                native if native.starts_with('u') => format!("json::unsigned(u64::from({copied}))"),
                _ => format!("json::signed(i64::from({copied}))"),
            },
            Kind::OctetArray(_) | Kind::Octets(_) | Kind::Open => {
                format!("json::octets({referred})")
            }
            Kind::Bits(range) => format!("json::bits({copied}, {})", bounds(range)),
            Kind::BitString(range) => {
                format!(
                    "json::bit_string({referred}, {}, {})",
                    bounds(range),
                    range.extensible
                )
            }
            Kind::Text { .. } => format!("json::text({referred})"),
            Kind::ObjectIdentifier => format!("json::object_identifier({referred})"),
            Kind::Named(_) => format!("{value}.to_json()"),
            Kind::List { of, .. } => {
                let item = self.to_json(of, "item", true);
                format!("Value::Array({value}.iter().map(|item| {item}).collect())")
            }
        }
    }

    /// The expression that reads a value of a kind from the JSON `value`, in `reading`:
    /// a `Result`.
    fn read_json(&self, kind: &Kind) -> String {
        match kind {
            Kind::Boolean => "reading.boolean(value)".into(),
            Kind::Null => "reading.null(value)".into(),
            Kind::Integer { native, range } => {
                // A type whose constraint has an extension marker, or no upper bound,
                // holds what its native type holds.
                let closed = !range.extensible;
                let lower = range.lower.filter(|_| closed || range.upper.is_none());
                let upper = range.upper.filter(|_| closed);
                let signed = native.starts_with('i');
                // The functions take 64 bits: a narrower bound is widened.
                let into = match *native {
                    "u64" | "i64" => "",
                    _ => ".into()",
                };
                let lower = lower.map_or(format!("{native}::MIN{into}"), |lower| lower.to_string());
                let upper = upper.map_or(format!("{native}::MAX{into}"), |upper| upper.to_string());
                let read = match signed {
                    true => format!("reading.signed(value, {lower}, {upper})"),
                    false => format!("reading.unsigned(value, {lower}, {upper})"),
                };
                match *native {
                    "u64" | "i64" => read,
                    native => format!("{read}.map(|number| number as {native})"),
                }
            }
            Kind::OctetArray(size) => format!("reading.octet_array::<{size}>(value)"),
            Kind::Octets(_) | Kind::Open => "reading.octets(value)".into(),
            Kind::Bits(range) => format!("reading.bits(value, {})", bounds(range)),
            Kind::BitString(range) => {
                format!(
                    "reading.bit_string(value, {}, {})",
                    bounds(range),
                    range.extensible
                )
            }
            Kind::Text { of, .. } => format!("reading.{of}(value)"),
            Kind::ObjectIdentifier => "reading.object_identifier(value)".into(),
            Kind::Named(name) => format!("{name}::from_json(value, reading)"),
            Kind::List { of, .. } => {
                format!(
                    "reading.list(value, |value, reading| {})",
                    self.read_json(of)
                )
            }
        }
    }

    // ── The fields ──────────────────────────────────────────────────────────

    /// The expression that reads a value of a kind from `reader`. `unwrap` makes the
    /// value of a `Result`, and returns its error.
    fn decode(&self, kind: &Kind, unwrap: &dyn Fn(&str) -> String) -> Result<String> {
        Ok(match kind {
            Kind::Boolean => unwrap("reader.bit()"),
            Kind::Null => "()".into(),
            Kind::Integer { native, range } => {
                let read = match (range.lower, range.upper, range.extensible) {
                    (Some(lower), Some(upper), false) if lower < 0 => {
                        format!("reader.signed({lower}, {upper})")
                    }
                    (Some(lower), Some(upper), false) => {
                        format!("reader.unsigned({lower}, {upper})")
                    }
                    (Some(lower), Some(upper), true) if lower < 0 => {
                        format!("reader.signed_extensible({lower}, {upper})")
                    }
                    (Some(lower), Some(upper), true) => {
                        format!("reader.unsigned_extensible({lower}, {upper})")
                    }
                    (Some(lower), None, _) if lower < 0 => {
                        format!("reader.semi_constrained({lower})")
                    }
                    (Some(lower), None, _) => format!("reader.semi_constrained_unsigned({lower})"),
                    _ => "reader.unconstrained()".into(),
                };
                match *native {
                    "u64" | "i64" => unwrap(&read),
                    native => format!("{} as {native}", unwrap(&read)),
                }
            }
            Kind::OctetArray(size) => unwrap(&format!("reader.octet_array::<{size}>()")),
            Kind::Octets(range) => match range.extensible {
                true => unwrap(&format!(
                    "reader.octet_string_extensible({})",
                    bounds(range)
                )),
                false => unwrap(&format!("reader.octet_string({})", bounds(range))),
            },
            Kind::Bits(range) => unwrap(&format!("reader.bits_sized({})", bounds(range))),
            Kind::BitString(range) => match range.extensible {
                true => unwrap(&format!("reader.bit_string_extensible({})", bounds(range))),
                false => unwrap(&format!("reader.bit_string({})", bounds(range))),
            },
            // A size constraint on a UTF8String is not PER-visible (X.691 §10.3.7).
            Kind::Text {
                of: "utf8_string", ..
            } => unwrap("reader.utf8_string()"),
            Kind::Text { of, range } => match range.extensible {
                true => unwrap(&format!("reader.{of}_extensible({})", bounds(range))),
                false => unwrap(&format!("reader.{of}({})", bounds(range))),
            },
            Kind::ObjectIdentifier => unwrap("reader.object_identifier()"),
            Kind::Open => unwrap("reader.open_type()"),
            Kind::Named(name) => unwrap(&format!("{name}::decode(reader)")),
            Kind::List { of, range } => {
                let item = self.decode(of, unwrap)?;
                let (lower, upper) = (range.lower.unwrap_or(0), range.upper);
                match upper {
                    // A count below 64K is a constrained whole number (X.691 §20.6).
                    Some(upper) if upper < 65536 => {
                        let count = unwrap(&format!("reader.size({lower}, {upper})"));
                        format!(
                            "{{ let count = {count}; let mut list = reader.list(count); \
                             for _ in 0..count {{ list.push({item}); }} list }}"
                        )
                    }
                    // Otherwise an unconstrained one, in fragments from 16K elements.
                    upper => {
                        let length = unwrap("reader.length()");
                        let upper =
                            upper.map_or("UNBOUNDED".to_string(), |upper| upper.to_string());
                        let check =
                            unwrap(&format!("Reader::check_size(list.len(), {lower}, {upper})"));
                        format!(
                            "{{ let mut list = Vec::new(); loop {{ let (count, more) = {length}; \
                             for _ in 0..count {{ list.push({item}); }} if !more {{ break; }} }} \
                             {check}; list }}"
                        )
                    }
                }
            }
        })
    }

    /// The statements that write `value`, of a kind, to `writer`. `value` is a
    /// reference when `by_reference` says so.
    fn encode(&self, out: &mut String, kind: &Kind, value: &str, by_reference: bool) -> Result<()> {
        // A value that is copied, and a reference to one that is not.
        let copied = match by_reference {
            true => format!("*{value}"),
            false => value.to_string(),
        };
        let referred = match by_reference {
            true => value.to_string(),
            false => format!("&{value}"),
        };
        match kind {
            Kind::Boolean => writeln!(out, "writer.bit({copied});")?,
            Kind::Null => {}
            Kind::Integer { native, range } => {
                let wide = match *native {
                    "u64" | "i64" => copied.clone(),
                    native if native.starts_with('u') => format!("u64::from({copied})"),
                    _ => format!("i64::from({copied})"),
                };
                match (range.lower, range.upper, range.extensible) {
                    (Some(lower), Some(upper), false) if lower < 0 => {
                        writeln!(out, "writer.signed({wide}, {lower}, {upper})?;")?
                    }
                    (Some(lower), Some(upper), false) => {
                        writeln!(out, "writer.unsigned({wide}, {lower}, {upper})?;")?
                    }
                    (Some(lower), Some(upper), true) if lower < 0 => {
                        writeln!(out, "writer.signed_extensible({wide}, {lower}, {upper});")?
                    }
                    (Some(lower), Some(upper), true) => {
                        writeln!(out, "writer.unsigned_extensible({wide}, {lower}, {upper});")?
                    }
                    (Some(lower), None, _) if lower < 0 => {
                        writeln!(out, "writer.semi_constrained({wide}, {lower})?;")?
                    }
                    (Some(lower), None, _) => {
                        writeln!(out, "writer.semi_constrained_unsigned({wide}, {lower})?;")?
                    }
                    _ => writeln!(out, "writer.unconstrained({wide});")?,
                }
            }
            Kind::OctetArray(_) => writeln!(out, "writer.octet_array({referred});")?,
            Kind::Octets(range) => match range.extensible {
                true => writeln!(
                    out,
                    "writer.octet_string_extensible({referred}, {});",
                    bounds(range)
                )?,
                false => writeln!(out, "writer.octet_string({referred}, {})?;", bounds(range))?,
            },
            Kind::Bits(range) => writeln!(out, "writer.bits_sized({copied}, {})?;", bounds(range))?,
            Kind::BitString(range) => match range.extensible {
                true => writeln!(
                    out,
                    "writer.bit_string_extensible({referred}, {});",
                    bounds(range)
                )?,
                false => writeln!(out, "writer.bit_string({referred}, {})?;", bounds(range))?,
            },
            Kind::Text {
                of: "utf8_string", ..
            } => writeln!(out, "writer.utf8_string({referred});")?,
            Kind::Text { of, range } => match range.extensible {
                true => writeln!(
                    out,
                    "writer.{of}_extensible({referred}, {})?;",
                    bounds(range)
                )?,
                false => writeln!(out, "writer.{of}({referred}, {})?;", bounds(range))?,
            },
            Kind::ObjectIdentifier => writeln!(out, "writer.object_identifier({referred});")?,
            Kind::Open => writeln!(out, "writer.open_type({referred});")?,
            Kind::Named(_) => writeln!(out, "{value}.encode(writer)?;")?,
            Kind::List { of, range } => {
                let (lower, upper) = (range.lower.unwrap_or(0), range.upper);
                match upper {
                    Some(upper) if upper < 65536 => {
                        writeln!(out, "writer.size({value}.len(), {lower}, {upper})?;")?;
                        writeln!(out, "for item in {value}.iter() {{")?;
                        self.encode(out, of, "item", true)?;
                        writeln!(out, "}}")?;
                    }
                    upper => {
                        let upper =
                            upper.map_or("UNBOUNDED".to_string(), |upper| upper.to_string());
                        writeln!(out, "Writer::check_size({value}.len(), {lower}, {upper})?;")?;
                        writeln!(out, "let mut left = &{value}[..];")?;
                        writeln!(
                            out,
                            "loop {{ let (count, more) = writer.length(left.len());"
                        )?;
                        writeln!(out, "for item in &left[..count] {{")?;
                        self.encode(out, of, "item", true)?;
                        writeln!(out, "}}")?;
                        writeln!(out, "left = &left[count..]; if !more {{ break; }} }}")?;
                    }
                }
            }
        }
        Ok(())
    }
}

/// The bounds of a size constraint, as the arguments of a reader and of a writer.
fn bounds(range: &Range) -> String {
    let lower = range.lower.unwrap_or(0);
    match range.upper {
        Some(upper) => format!("{lower}, {upper}"),
        None => format!("{lower}, UNBOUNDED"),
    }
}

fn write_docs(out: &mut String, docs: &[String]) -> Result<()> {
    for doc in docs {
        writeln!(out, "#[doc = {doc}]")?;
    }
    Ok(())
}
