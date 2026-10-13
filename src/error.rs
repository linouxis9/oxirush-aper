//! The error of the crate and of the bindings built on it: a kind that a caller matches
//! on, and a message for the one who reads it.

use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

/// What went wrong: the part of an [`Error`] that a caller matches on.
///
/// The codec of this crate gives `Decode` and `Encode`. The other kinds are those of
/// the bindings that are built on it, which have one error type for their messages,
/// their IEs and their JSON forms.
///
/// More kinds may come with a later version: match the ones that matter, and leave the
/// others to `_`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Octets are not the value that they are decoded as, in aligned PER: they end
    /// before the value does, go on after it, or hold a value outside its constraint.
    Decode,
    /// A value does not encode in aligned PER, as an integer outside its constraint.
    Encode,
    /// A PDU that holds another message than the one that it is decoded as, and the
    /// protocol IEs of a PDU whose message has none.
    WrongMessage,
    /// A required IE that the message does not have.
    MissingIe,
    /// An IE, or a transfer, that is there and does not decode as its type.
    UndecodableIe,
    /// An argument that is not what a function takes, as an MCC of two digits.
    Argument,
    /// A name of a message, or a direction and a procedure code, that the protocol
    /// does not have.
    UnknownMessage,
    /// A name or an identifier of an IE that the protocol does not have, or has no
    /// type for where it is.
    UnknownIe,
    /// A path that is not well formed.
    Path,
    /// A name that the type at that place cannot have: a member of a value, an IE of
    /// a message, or a name where a list goes by position.
    NoSuchMember,
    /// An edit whose path selects nothing.
    NothingSelected,
    /// A path through the type that octets carry, before they were opened.
    NotOpened,
    /// A JSON value that is not a value of its ASN.1 type: of another form, with a
    /// name that an ENUMERATED does not have, or outside a constraint.
    Value,
    /// JSON that is not written as a tree or as a view.
    Form,
    /// An edit that is refused: a member that its type always has is not taken out,
    /// and the octets of a value that was edited since are not read.
    Edit,
    /// Nesting or a number of occurrences over the limits of the inspection, or
    /// what it does not represent.
    Limit,
}

/// The criticality of an IE or of a procedure in the application protocols of 3GPP, as
/// their ASN.1 has it (`Criticality ::= ENUMERATED { reject, ignore, notify }`): what a
/// receiver does with what it does not comprehend. The bindings of a protocol have a
/// type of their own for it, which converts to and from this one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Criticality {
    /// The receiver rejects the procedure.
    Reject,
    /// The receiver ignores the IE.
    Ignore,
    /// The receiver ignores the IE, and notifies the sender.
    Notify,
}

/// The IE of a message that an error is about: [`Error::ie`] of an error of the kind
/// `MissingIe` or `UndecodableIe` that the bindings of a protocol made.
///
/// It is what the Criticality Diagnostics of an answer say of an IE (3GPP TS 38.413
/// clauses 10.3.4 and 10.3.5, TS 36.413 clause 10.3): its identifier, and its
/// criticality.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ErrorIe {
    /// The identifier of the IE.
    pub id: u16,
    /// Its criticality: the one that it was received with, for an IE that is there,
    /// and the one that the object set of the message assigns it, for an IE that is
    /// missing.
    pub criticality: Criticality,
    /// Its position among the IEs of the message, which starts at 0; `None` for an IE
    /// that is missing.
    pub position: Option<usize>,
}

impl ErrorIe {
    /// The IE of an identifier and a criticality, at a position when it is there.
    pub fn new(id: u16, criticality: Criticality, position: Option<usize>) -> Self {
        Self {
            id,
            criticality,
            position,
        }
    }
}

/// The error of every function of the crate that can fail, and of the bindings that
/// are built on it.
///
/// [`kind`](Self::kind) is what a caller matches on. `Display` says what is wrong, in
/// the words of the specification where it has them, then why, and for octets that do
/// not decode the field of the bindings where the decoding stopped:
///
/// ```
/// use oxirush_aper::{ErrorKind, Reader};
///
/// // A number of the constraint (0..65535) is two octets, and one is there
/// let error = Reader::new(&[0x12]).unsigned(0, 65535).unwrap_err();
/// assert_eq!(error.kind(), ErrorKind::Decode);
/// assert_eq!(
///     error.to_string(),
///     "the octets do not decode: the octets end before the value does"
/// );
/// ```
///
/// Two errors are equal when they have the same kind and the same message. An error is
/// one pointer, so that a `Result` of a small value stays small.
#[derive(Clone)]
pub struct Error(Box<Inner>);

#[derive(Clone)]
struct Inner {
    kind: ErrorKind,
    /// What is wrong, and after it the reason, where the message has one apart: what
    /// the codec says of octets or of a value.
    what: String,
    why: Option<String>,
    /// The fields of the bindings on the way to where a decoding stopped, the
    /// innermost first.
    fields: Vec<&'static str>,
    /// The IE of a message that the error is about.
    ie: Option<ErrorIe>,
}

impl Error {
    /// An error of a kind, which says `what` is wrong.
    pub fn new(kind: ErrorKind, what: impl Into<String>) -> Self {
        Self(Box::new(Inner {
            kind,
            what: what.into(),
            why: None,
            fields: Vec::new(),
            ie: None,
        }))
    }

    /// What went wrong, as a caller matches on it.
    pub fn kind(&self) -> ErrorKind {
        self.0.kind
    }

    /// The same error, with the reason of what is wrong, which the message has after
    /// it.
    #[must_use]
    pub fn because(mut self, why: impl Into<String>) -> Self {
        self.0.why = Some(why.into());
        self
    }

    /// The same error, told of `what`: what it said is wrong becomes the reason, where
    /// it gave none apart.
    #[must_use]
    pub fn of(mut self, what: impl Into<String>) -> Self {
        let was = core::mem::replace(&mut self.0.what, what.into());
        if self.0.why.is_none() {
            self.0.why = Some(was);
        }
        self
    }

    /// The same error, of another kind.
    #[must_use]
    pub fn with_kind(mut self, kind: ErrorKind) -> Self {
        self.0.kind = kind;
        self
    }

    /// The IE of a message that the error is about, when it is about one: an error of
    /// the kind `MissingIe` or `UndecodableIe` that the bindings of a protocol made.
    ///
    /// ```
    /// use oxirush_aper::{Criticality, Error, ErrorIe};
    ///
    /// // The IE 21 of an NG SETUP REQUEST, received with the criticality `ignore`
    /// let ie = ErrorIe::new(21, Criticality::Ignore, Some(3));
    /// let error = Error::undecodable("the DefaultPagingDRX does not decode").about_ie(ie);
    /// assert_eq!(error.ie(), Some(ie));
    /// assert_eq!(Error::argument("an MCC has three digits").ie(), None);
    /// ```
    pub fn ie(&self) -> Option<ErrorIe> {
        self.0.ie
    }

    /// The same error, about an IE of a message.
    #[must_use]
    pub fn about_ie(mut self, ie: ErrorIe) -> Self {
        self.0.ie = Some(ie);
        self
    }

    /// The same error, with the message that `reword` makes of the one that it has.
    #[must_use]
    pub fn reworded(self, reword: impl FnOnce(&str) -> String) -> Self {
        let mut reworded = Self::new(self.0.kind, reword(&self.to_string()));
        reworded.0.ie = self.0.ie;
        reworded
    }

    /// The reason of what is wrong, where the message has one apart, and what is wrong
    /// otherwise, with the field of the bindings where a decoding stopped.
    pub fn reason(&self) -> String {
        let mut reason = self.0.why.clone().unwrap_or_else(|| self.0.what.clone());
        self.write_fields(&mut reason);
        reason
    }

    /// The fields of the bindings on the way to where a decoding stopped, the outermost
    /// first: none for an error that is no decoding error, or that stopped in no field.
    pub fn fields(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.0.fields.iter().rev().copied()
    }

    /// The same error, which was met in the field `name` of the bindings. A decoder of a
    /// SEQUENCE names the member that did not decode.
    #[cold]
    #[must_use]
    pub fn in_field(mut self, name: &'static str) -> Self {
        self.0.fields.push(name);
        self
    }

    /// Octets that are not a value of the type that ASN.1 names `name`: this error is
    /// why.
    #[cold]
    #[must_use]
    pub fn decoding(mut self, name: &str) -> Self {
        let what = format!("these octets do not decode as {name}");
        let was = core::mem::replace(&mut self.0.what, what);
        if self.0.why.is_none() {
            self.0.why = Some(was);
        }
        self
    }

    /// A value of the type that ASN.1 names `name` that does not encode: this error is
    /// why.
    #[cold]
    #[must_use]
    pub fn encoding(mut self, name: &str) -> Self {
        let what = format!("a value of {name} does not encode");
        let was = core::mem::replace(&mut self.0.what, what);
        if self.0.why.is_none() {
            self.0.why = Some(was);
        }
        self
    }

    /// Octets that do not decode, and why.
    #[cold]
    pub(crate) fn decode(why: impl Into<String>) -> Self {
        Self::new(ErrorKind::Decode, "the octets do not decode").because(why)
    }

    /// A value that does not encode, and why.
    #[cold]
    pub(crate) fn encode(why: impl Into<String>) -> Self {
        Self::new(ErrorKind::Encode, "the value does not encode").because(why)
    }

    /// The octets end before the value does.
    #[cold]
    pub(crate) fn ended() -> Self {
        Self::decode("the octets end before the value does")
    }

    /// A number that is read outside its constraint.
    #[cold]
    pub(crate) fn number_read_outside(value: i128, lower: i128, upper: i128) -> Self {
        Self::decode(outside(value, lower, upper))
    }

    /// A number that is written outside its constraint.
    #[cold]
    pub(crate) fn number_written_outside(value: i128, lower: i128, upper: i128) -> Self {
        Self::encode(outside(value, lower, upper))
    }

    /// A size that is read outside its constraint.
    #[cold]
    pub(crate) fn size_read_outside(size: usize, lower: usize, upper: Option<usize>) -> Self {
        Self::decode(size_outside(size, lower, upper))
    }

    /// A size that is written outside its constraint.
    #[cold]
    pub(crate) fn size_written_outside(size: usize, lower: usize, upper: Option<usize>) -> Self {
        Self::encode(size_outside(size, lower, upper))
    }

    /// A number that is read and that the type of the bindings does not hold.
    #[cold]
    pub(crate) fn does_not_fit(bits: u32) -> Self {
        Self::decode(format!("the number does not fit in {bits} bits"))
    }

    /// A PDU of another message than the one that ASN.1 names `expected`: `held` is the
    /// name of the message of its `direction` and of the procedure `code`, when the
    /// specification has one.
    #[doc(hidden)]
    pub fn __wrong_message(expected: &str, held: Option<&str>, direction: &str, code: u8) -> Self {
        let message = match held {
            Some(held) => format!(
                "the message of this PDU is {held}, not {expected}: kind() says which \
                 message a PDU holds"
            ),
            None => format!(
                "the message of this PDU is not {expected}: the specification has no \
                 {direction} of procedure {code}"
            ),
        };
        Self::new(ErrorKind::WrongMessage, message)
    }

    /// A PDU of the message that ASN.1 names `name`, which has no container of
    /// protocol IEs.
    #[doc(hidden)]
    pub fn __no_protocol_ies(name: &str) -> Self {
        let what =
            format!("the message of this PDU is {name}, which has no container of protocol IEs");
        Self::new(ErrorKind::WrongMessage, what)
    }

    /// A PDU whose direction and procedure code the specification has no message for,
    /// and so no object set.
    #[doc(hidden)]
    pub fn __no_message_of(direction: &str, code: u8) -> Self {
        let what = format!(
            "the specification has no {direction} of procedure {code}, and so no object set \
             for its IEs"
        );
        Self::new(ErrorKind::UnknownMessage, what)
    }

    /// This error is why the message of a PDU does not decode as a container of
    /// protocol IEs: the message that ASN.1 names `name`, when the specification has
    /// one for the PDU.
    #[doc(hidden)]
    #[must_use]
    pub fn __of_message(self, name: Option<&str>) -> Self {
        const UNNAMED: &str =
            "the message of this PDU does not decode as a container of protocol IEs";
        self.of(match name {
            Some(name) => format!("the {name} of this PDU does not decode"),
            None => UNNAMED.to_string(),
        })
    }

    /// This error is why `count` protocol IEs do not encode as a message.
    #[doc(hidden)]
    #[must_use]
    pub fn __of_protocol_ies(self, count: usize) -> Self {
        self.of(format!(
            "these {count} protocol IEs do not encode as a message"
        ))
    }

    /// The IE that a message names `ie`, which the field `field` requires, is not in a
    /// message of the type `message`.
    #[doc(hidden)]
    pub fn __missing_ie(message: &str, ie: &str, field: &str) -> Self {
        let what = format!("this {message} has no {ie}, which the field `{field}` requires");
        Self::new(ErrorKind::MissingIe, what)
    }

    /// This error is why the IE that a message names `ie` does not decode, in a message
    /// of the type `message`.
    #[doc(hidden)]
    #[must_use]
    pub fn __undecodable_ie(self, message: &str, ie: &str) -> Self {
        // What the codec said of the octets, without the type that it took them for.
        self.of(format!("the {ie} of this {message} does not decode"))
            .with_kind(ErrorKind::UndecodableIe)
    }

    /// A value of the type that ASN.1 names `name` that is written in JSON and does
    /// not encode: it is outside a constraint of its type, and so no value of that
    /// type. This error is what the codec said of it.
    #[doc(hidden)]
    #[must_use]
    pub fn __outside_its_type(self, name: &str) -> Self {
        self.of(format!("this is not a value of {name}"))
            .with_kind(ErrorKind::Value)
    }

    fn write_fields(&self, text: &mut String) {
        if self.0.fields.is_empty() {
            return;
        }
        text.push_str(", in its field `");
        for (at, field) in self.fields().enumerate() {
            if at > 0 {
                text.push('.');
            }
            text.push_str(field);
        }
        text.push('`');
    }
}

/// Why a number is outside its constraint.
fn outside(value: i128, lower: i128, upper: i128) -> String {
    format!("{value} is outside its constraint ({lower}..{upper})")
}

/// Why a size is outside its constraint.
fn size_outside(size: usize, lower: usize, upper: Option<usize>) -> String {
    match upper {
        Some(upper) if upper == lower => {
            format!("a size of {size} is outside its constraint ({lower})")
        }
        Some(upper) => format!("a size of {size} is outside its constraint ({lower}..{upper})"),
        None => format!("a size of {size} is outside its constraint ({lower}..MAX)"),
    }
}

/// An error of each kind, with what is wrong.
macro_rules! kinds {
    ($($(#[$doc:meta])* $name:ident $kind:ident;)*) => {
        impl Error {
            $(
                $(#[$doc])*
                pub fn $name(what: impl Into<String>) -> Self {
                    Self::new(ErrorKind::$kind, what)
                }
            )*
        }
    };
}

kinds! {
    /// An argument that a function does not take, and what it takes.
    argument Argument;
    /// A name of a message, or a direction and a procedure code, that the protocol
    /// does not have.
    unknown_message UnknownMessage;
    /// A name or an identifier of an IE that the protocol does not have, or has no
    /// type for where it is.
    unknown_ie UnknownIe;
    /// A path that is not well formed.
    path Path;
    /// A name that the type at that place cannot have.
    no_such_member NoSuchMember;
    /// An edit whose path selects nothing.
    nothing_selected NothingSelected;
    /// A path through the type that octets carry, before they are opened.
    not_opened NotOpened;
    /// A path into an IE or a transfer that is there and did not decode.
    undecodable UndecodableIe;
    /// A JSON value that is not a value of its ASN.1 type.
    value Value;
    /// JSON that is not written as a tree or as a view.
    form Form;
    /// An edit that is refused.
    edit Edit;
    /// Nesting or a number of occurrences over the limits of the inspection.
    limit Limit;
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut text = self.0.what.to_string();
        if let Some(why) = &self.0.why {
            text.push_str(": ");
            text.push_str(why);
        }
        self.write_fields(&mut text);
        formatter.write_str(&text)
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut error = formatter.debug_struct("Error");
        error.field("kind", &self.0.kind);
        error.field("message", &self.to_string());
        error.finish()
    }
}

impl core::error::Error for Error {}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        self.0.kind == other.0.kind && self.to_string() == other.to_string()
    }
}

impl Eq for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_error_is_cloned_sent_and_compared_by_its_kind_and_its_message() {
        fn sent<T: Clone + Send + Sync + core::error::Error + 'static>() {}
        sent::<Error>();
        let error = Error::new(ErrorKind::Argument, "an MCC has three digits");
        assert_eq!(error.kind(), ErrorKind::Argument);
        assert_eq!(error.to_string(), "an MCC has three digits");
        assert_eq!(error, error.clone());
        assert_ne!(
            error,
            Error::new(ErrorKind::Value, "an MCC has three digits")
        );
        assert_ne!(error, Error::new(ErrorKind::Argument, "an MNC has two"));
        let shown = format!("{error:?}");
        assert_eq!(
            shown,
            "Error { kind: Argument, message: \"an MCC has three digits\" }"
        );
        // One pointer.
        assert_eq!(size_of::<Error>(), size_of::<usize>());
        assert_eq!(size_of::<Result<u64, Error>>(), 16);
    }

    #[test]
    fn a_decoding_error_names_its_type_and_the_field_where_it_stopped() {
        let error = Error::ended().in_field("id").in_field("protocol_ies");
        assert_eq!(error.kind(), ErrorKind::Decode);
        assert_eq!(
            error.to_string(),
            "the octets do not decode: the octets end before the value does, in its field \
             `protocol_ies.id`"
        );
        let named = error.clone().decoding("NGAP-PDU");
        assert_eq!(
            named.to_string(),
            "these octets do not decode as NGAP-PDU: the octets end before the value does, in \
             its field `protocol_ies.id`"
        );
        assert_eq!(named.fields().collect::<Vec<_>>(), ["protocol_ies", "id"]);
        // Told of something else, what the codec said is the reason.
        let told = named.of("the Cause of this message does not decode");
        assert_eq!(
            told.to_string(),
            "the Cause of this message does not decode: the octets end before the value does, \
             in its field `protocol_ies.id`"
        );
        assert_eq!(
            told.reason(),
            "the octets end before the value does, in its field `protocol_ies.id`"
        );
        let of_a_kind = told.with_kind(ErrorKind::UndecodableIe);
        assert_eq!(of_a_kind.kind(), ErrorKind::UndecodableIe);
    }

    #[test]
    fn a_constraint_is_told_as_asn1_writes_it() {
        assert_eq!(
            Error::number_read_outside(4096, 0, 4095).reason(),
            "4096 is outside its constraint (0..4095)"
        );
        assert_eq!(
            Error::number_written_outside(-9, -8, 7).to_string(),
            "the value does not encode: -9 is outside its constraint (-8..7)"
        );
        assert_eq!(
            Error::size_read_outside(4, 3, Some(3)).reason(),
            "a size of 4 is outside its constraint (3)"
        );
        assert_eq!(
            Error::size_written_outside(0, 1, Some(150)).reason(),
            "a size of 0 is outside its constraint (1..150)"
        );
        assert_eq!(
            Error::size_read_outside(0, 1, None).reason(),
            "a size of 0 is outside its constraint (1..MAX)"
        );
    }
}
