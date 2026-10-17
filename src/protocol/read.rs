//! The readers of the IEs of a container: one pass over the octets of a message, or over
//! the IEs of a value that is decoded, which decodes the IEs that are asked for alone.

use alloc::borrow::Cow;
use core::marker::PhantomData;

use super::errors::{__undecodable_ie, __wrong_message};
use super::{Container, CriticalityOf, Field, Ie, InProtocol, List, Message, Pdu, Protocol};
use crate::{Decode, Error, Reader};

/// What a container's IEs are read from: the octets of the message of a PDU, which are
/// scanned as they are read, or the IEs of a message or of a transfer that is decoded.
/// `ies`, `ie` and `items` of a PDU and of a message make one.
pub struct Source<'s, P: Protocol> {
    /// The name that ASN.1 gives the message, which an error names.
    message: &'static str,
    fields: Fields<'s, P>,
}

/// Where the IEs of a source are.
enum Fields<'s, P: Protocol> {
    /// The octets of a message: its extension bit, the count of its IEs, then the IEs.
    Octets(&'s [u8]),
    /// The IEs of a value.
    Decoded(&'s [P::Field<'s>]),
    /// Why the IEs cannot be read.
    Failed(Error),
}

impl<'s, P: Protocol> Source<'s, P> {
    /// The IEs of the octets of a message, which ASN.1 names `message`: a SEQUENCE of a
    /// container of protocol IEs, and what a later release adds.
    #[doc(hidden)]
    pub fn __of_octets(message: &'static str, octets: &'s [u8]) -> Self {
        let fields = Fields::Octets(octets);
        Self { message, fields }
    }

    /// The IEs of a value that is decoded, which ASN.1 names `message`.
    #[doc(hidden)]
    pub fn __of_fields(message: &'static str, fields: &'s [P::Field<'s>]) -> Self {
        let fields = Fields::Decoded(fields);
        Self { message, fields }
    }

    /// No IE, for `error`.
    #[doc(hidden)]
    pub fn __failed(message: &'static str, error: Error) -> Self {
        let fields = Fields::Failed(error);
        Self { message, fields }
    }

    /// The IEs of the message of `pdu`: of a message that ASN.1 names `message` when the
    /// specification has one for it, and an error for a message that has no container of
    /// protocol IEs.
    #[doc(hidden)]
    pub fn __of_pdu<X: Pdu<'s> + InProtocol<Protocol = P>>(pdu: &'s X) -> Source<'s, P> {
        let set = super::object_set::<P>(pdu.direction(), pdu.procedure_code());
        match set {
            Some(set) if set.ies.is_none() => {
                Self::__failed(set.name, Error::no_protocol_ies(set.name))
            }
            set => {
                let name = set.map_or("message", |set| set.name);
                let value = pdu.value();
                Self::__of_octets(name, value.as_slice())
            }
        }
    }

    /// Every occurrence of the IE `T`, in the order of the container, each decoded as its
    /// type.
    pub fn ies<T: Ie<Protocol = P>>(self) -> Ies<'s, T> {
        let message = self.message;
        Ies {
            walk: self.walk(),
            message,
            ie: PhantomData,
        }
    }

    /// The first occurrence of the IE `T`, decoded as its type; `None` when the container
    /// has none.
    pub fn ie<T: Ie<Protocol = P>>(self) -> Option<Result<T::Value<'s>, Error>> {
        self.ies::<T>().next()
    }

    /// The items of the first occurrence of the list `L`, each decoded as the IE of its
    /// single container: nothing when the container has no such list.
    pub fn items<L: List<Protocol = P>>(self) -> Items<'s, L> {
        let message = self.message;
        let state = match self.ies_entries::<L>() {
            None => State::Done,
            Some(Err(error)) => State::Failed(Some(error)),
            Some(Ok(entry)) => match Walking::of(entry, message, L::__SIZE) {
                Ok(walking) => State::Walking(walking),
                Err(error) => State::Failed(Some(error)),
            },
        };
        Items {
            state,
            list: PhantomData,
        }
    }

    /// The first occurrence of the IE `T`, as it is in the container.
    fn ies_entries<T: Ie<Protocol = P>>(self) -> Option<Result<__Entry<'s, P>, Error>> {
        let mut walk = self.walk();
        loop {
            match walk.next_of(T::ID)? {
                Ok(entry) if entry.id == T::ID => return Some(Ok(entry)),
                Ok(_) => {}
                Err(error) => return Some(Err(error)),
            }
        }
    }

    /// The IEs of the source, in their order.
    #[doc(hidden)]
    #[inline]
    pub fn __walk(self) -> __Walk<'s, P> {
        self.walk()
    }

    /// The IEs of the source, in their order.
    #[inline]
    fn walk(self) -> __Walk<'s, P> {
        match self.fields {
            Fields::Octets(octets) => __Walk::of_octets(self.message, octets),
            Fields::Decoded(fields) => __Walk::Decoded {
                fields,
                position: 0,
            },
            Fields::Failed(error) => __Walk::Failed(Some(error)),
        }
    }
}

/// The octets of the value of an IE, borrowed from what the IEs are read from: in one
/// piece, or in fragments, which a read of the IE from its first octet gathers.
#[derive(Clone, Copy)]
pub(super) enum Value<'s> {
    /// The octets of the value.
    Whole(&'s [u8]),
    /// The octets of the IE, from its first one, whose value came in fragments.
    Fragmented(&'s [u8]),
}

impl<'s> Value<'s> {
    /// The octets of the value: borrowed when they are in one piece, and gathered from
    /// their fragments otherwise, by the read that the walk made of the IE.
    fn octets<P: Protocol>(&self) -> Result<Cow<'s, [u8]>, Error> {
        match self {
            Value::Whole(octets) => Ok(Cow::Borrowed(octets)),
            Value::Fragmented(field) => {
                let field = P::Field::decode(&mut Reader::new(field))?;
                Ok(Cow::Owned(field.value().to_vec()))
            }
        }
    }
}

/// One IE of a container, as it is there: what the macro that extracts IEs keeps of the
/// first occurrence of each that a field asks for, before it decodes it.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct __Entry<'s, P: Protocol> {
    position: usize,
    id: u16,
    criticality: CriticalityOf<P>,
    value: Value<'s>,
}

impl<'s, P: Protocol> __Entry<'s, P> {
    /// The identifier of the IE.
    #[inline]
    pub fn id(&self) -> u16 {
        self.id
    }

    /// The value of the IE, decoded as the type of `T`, in a message that ASN.1 names
    /// `message`: an error of the kind `UndecodableIe` when it does not decode.
    #[inline]
    pub fn decode<T: Ie<Protocol = P>>(&self, message: &str) -> Result<T::Value<'s>, Error> {
        let read = match self.value {
            Value::Whole(octets) => crate::decode::<T::Value<'s>>(octets),
            Value::Fragmented(_) => self
                .value
                .octets::<P>()
                .and_then(|octets| gathered::<T>(&octets)),
        };
        match read {
            Ok(value) => Ok(value),
            Err(error) => Err(self.undecodable(error, message)),
        }
    }
}

/// The value of the IE `T` that octets gathered from fragments hold, which borrows nothing
/// from them: seldom, and apart from the usual path.
#[cold]
#[inline(never)]
fn gathered<'s, T: Ie>(octets: &[u8]) -> Result<T::Value<'s>, Error> {
    T::__owned(octets)
}

/// The IEs of a container, one after the other: what the macro that extracts IEs walks.
#[doc(hidden)]
pub enum __Walk<'s, P: Protocol> {
    /// The octets of a message, as they are read: the IEs start on an octet, `at`.
    Octets {
        octets: &'s [u8],
        at: usize,
        message: &'static str,
        left: usize,
        position: usize,
        extended: bool,
    },
    /// The IEs of a value.
    Decoded {
        fields: &'s [P::Field<'s>],
        position: usize,
    },
    /// An error, which is given once.
    Failed(Option<Error>),
}

/// The next IE of the octets of a container, which starts on the octet `at`: its
/// identifier, its criticality, its value and the octet after it. The usual layout is read
/// octet by octet: the identifier in two octets, the criticality in the next, and the
/// value after its length in one octet or in two. What is otherwise, a value in fragments
/// or octets that end early, the codec of the IE reads, as it reads it in a message: the
/// two read the same IE, which the tests hold them to.
#[inline(always)]
pub(super) fn field<'s, P: Protocol>(
    octets: &'s [u8],
    at: usize,
) -> Result<(u16, CriticalityOf<P>, Value<'s>, usize), Error> {
    if let [high, low, criticality, rest @ ..] = &octets[at..]
        && let Some(criticality) = criticality_of(*criticality >> 6)
    {
        let (length, header) = match rest {
            [length @ 0..=0x7f, ..] => (usize::from(*length), 4),
            [first @ 0x80..=0xbf, second, ..] => {
                (usize::from(first & 0x3f) << 8 | usize::from(*second), 5)
            }
            _ => (usize::MAX, 0),
        };
        let start = at + header;
        if header > 0 && length <= octets.len() - start {
            let id = u16::from_be_bytes([*high, *low]);
            let value = Value::Whole(&octets[start..start + length]);
            return Ok((id, criticality.into(), value, start + length));
        }
    }
    unusual::<P>(octets, at)
}

/// The IE that starts on the octet `at` of the octets of a container, in another layout
/// than the usual one, as the codec of the IE reads it.
#[cold]
#[inline(never)]
fn unusual<'s, P: Protocol>(
    octets: &'s [u8],
    at: usize,
) -> Result<(u16, CriticalityOf<P>, Value<'s>, usize), Error> {
    let mut reader = Reader::new(&octets[at..]);
    let field = P::Field::decode(&mut reader)?;
    // An open type ends on an octet.
    let next = at + reader.position() / 8;
    let value = match field.value().as_borrowed() {
        Some(value) => Value::Whole(value),
        None => Value::Fragmented(&octets[at..next]),
    };
    Ok((field.id(), field.criticality(), value, next))
}

/// The identifier of the IE that starts on the octet `at` of the octets of a container,
/// and the octet after it, in the usual layout: none for what is otherwise, a length in
/// fragments, a criticality of no value, or octets that end early.
#[inline]
fn skip(octets: &[u8], at: usize) -> Option<(u16, usize)> {
    let [high, low, criticality, rest @ ..] = octets.get(at..)? else {
        return None;
    };
    if criticality >> 6 == 3 {
        return None;
    }
    let (length, header) = match rest {
        [length @ 0..=0x7f, ..] => (usize::from(*length), 4),
        [first @ 0x80..=0xbf, second, ..] => {
            (usize::from(first & 0x3f) << 8 | usize::from(*second), 5)
        }
        _ => return None,
    };
    let next = at + header + length;
    (next <= octets.len()).then_some((u16::from_be_bytes([*high, *low]), next))
}

/// The criticality of an index, as `Criticality ::= ENUMERATED { reject, ignore, notify }`
/// has it, without an extension marker: none for the fourth value of its two bits.
#[inline]
fn criticality_of(index: u8) -> Option<crate::Criticality> {
    match index {
        0 => Some(crate::Criticality::Reject),
        1 => Some(crate::Criticality::Ignore),
        2 => Some(crate::Criticality::Notify),
        _ => None,
    }
}

impl<'s, P: Protocol> __Walk<'s, P> {
    /// The IEs of the octets of a message: its extension bit, then its container, whose
    /// count comes first, in the two octets after the first (X.691 §11.9.4.1, §11.5.7.3).
    fn of_octets(message: &'static str, octets: &'s [u8]) -> Self {
        match head(octets) {
            Ok((extended, left)) => Self::Octets {
                octets,
                at: 3,
                message,
                left,
                position: 0,
                extended,
            },
            Err(error) => Self::Failed(Some(error.of_message(Some(message)))),
        }
    }

    /// The next IE of the identifier `id`, or why the IEs cannot be read, which is given
    /// once: the IEs of other identifiers are passed over in one loop.
    #[inline]
    fn next_of(&mut self, id: u16) -> Option<Result<__Entry<'s, P>, Error>> {
        match self {
            Self::Decoded { fields, position } => {
                let found = fields[*position..]
                    .iter()
                    .position(|field| field.id() == id);
                let Some(found) = found else {
                    *position = fields.len();
                    return None;
                };
                *position += found;
                self.next()
            }
            Self::Octets {
                octets,
                at,
                left,
                position,
                ..
            } => {
                let (mut here, mut count, mut index) = (*at, *left, *position);
                while count > 0 {
                    // The identifier of the IE at `here`, and the octet after it, in the
                    // usual layout; the walk reads what is otherwise.
                    let Some((found, next)) = skip(octets, here) else {
                        break;
                    };
                    if found == id {
                        break;
                    }
                    here = next;
                    count -= 1;
                    index += 1;
                }
                (*at, *left, *position) = (here, count, index);
                self.next()
            }
            Self::Failed(_) => self.next(),
        }
    }

    /// The next IE, or why the IEs cannot be read, which is given once.
    #[inline(always)]
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<Result<__Entry<'s, P>, Error>> {
        match self {
            Self::Decoded { fields, position } => {
                let field = fields.get(*position)?;
                let entry = __Entry {
                    position: *position,
                    id: field.id(),
                    criticality: field.criticality(),
                    value: Value::Whole(field.value().as_slice()),
                };
                *position += 1;
                Some(Ok(entry))
            }
            Self::Failed(error) => error.take().map(Err),
            Self::Octets {
                octets,
                at,
                message,
                left,
                position,
                extended,
            } => {
                let message = *message;
                if *left == 0 {
                    // What a later release adds to the message, after the container, and no
                    // octet after the message.
                    let end = tail(octets, *at, *extended);
                    let end = end.map_err(|error| error.of_message(Some(message)));
                    *self = Self::Failed(end.err());
                    return self.next();
                }
                match field::<P>(octets, *at) {
                    Ok((id, criticality, value, next)) => {
                        let entry = __Entry {
                            position: *position,
                            id,
                            criticality,
                            value,
                        };
                        *at = next;
                        *left -= 1;
                        *position += 1;
                        Some(Ok(entry))
                    }
                    Err(error) => {
                        let error = error.in_field("protocol_ies").of_message(Some(message));
                        *self = Self::Failed(Some(error));
                        self.next()
                    }
                }
            }
        }
    }
}

/// Every occurrence of the IE `T` in a container, in its order, each decoded as the type
/// of `T`: `ies::<T>()` of a PDU or of a message.
///
/// The octets of the message of a PDU are read as the iterator goes: an IE of another
/// identifier is passed over, and its value is not decoded. An error ends it: the value of
/// an occurrence that does not decode is an error of the kind `UndecodableIe`, after which
/// the next occurrence is read, and octets that are no container of protocol IEs an error
/// of the kind `Decode`, which is the last item.
pub struct Ies<'s, T: Ie> {
    walk: __Walk<'s, T::Protocol>,
    message: &'static str,
    ie: PhantomData<T>,
}

impl<'s, T: Ie> Iterator for Ies<'s, T> {
    type Item = Result<T::Value<'s>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.walk.next_of(T::ID)? {
                Ok(entry) if entry.id == T::ID => return Some(entry.decode::<T>(self.message)),
                Ok(_) => {}
                Err(error) => return Some(Err(error)),
            }
        }
    }
}

/// The items of a list of single containers: `items::<L>()` of a PDU or of a message.
/// Each item is the IE of its single container, decoded as the type of `L::Item`; a single
/// container of another IE is passed over. The list is read as the iterator goes, and is
/// not built: octets that are no such list are an error of the kind `UndecodableIe` of
/// the list, which is the last item.
pub struct Items<'s, L: List> {
    state: State<'s, L::Protocol>,
    list: PhantomData<L>,
}

/// Where the items of a list are.
enum State<'s, P: Protocol> {
    Walking(Walking<'s, P>),
    Failed(Option<Error>),
    Done,
}

/// A list of single containers, as it is read: the octets of the list IE, which the
/// errors name, where the next item is, and how many are left.
struct Walking<'s, P: Protocol> {
    list: __Entry<'s, P>,
    octets: Cow<'s, [u8]>,
    message: &'static str,
    at: usize,
    left: usize,
    index: usize,
}

impl<'s, P: Protocol> Walking<'s, P> {
    /// The list of the IE `list`, whose size is in `size`: its count is read.
    fn of(
        list: __Entry<'s, P>,
        message: &'static str,
        size: (usize, usize),
    ) -> Result<Self, Error> {
        let octets = match list.value.octets::<P>() {
            Ok(octets) => octets,
            Err(error) => return Err(list.undecodable(error, message)),
        };
        let mut reader = Reader::new(&octets);
        match reader.size(size.0, size.1) {
            Ok(left) => {
                // An item starts on an octet: its identifier is octet-aligned.
                let at = reader.position().div_ceil(8);
                Ok(Self {
                    list,
                    octets,
                    message,
                    at,
                    left,
                    index: 0,
                })
            }
            Err(error) => Err(list.undecodable(error, message)),
        }
    }

    /// The item after the one that was read, of `used` octets.
    fn advance(&mut self, used: usize) -> usize {
        self.at += used;
        self.left -= 1;
        self.index += 1;
        self.index - 1
    }

    /// `error` is why the item at `index` does not decode as the IE `id`.
    fn item_undecodable(
        &self,
        error: Error,
        id: u16,
        criticality: CriticalityOf<P>,
        index: usize,
    ) -> Error {
        let list = super::ie_name::<P>(self.list.id);
        __undecodable_ie::<P>(error, &list, id, criticality, index)
    }
}

impl<'s, P: Protocol> __Entry<'s, P> {
    /// `error` is why the value of this IE does not decode.
    #[cold]
    fn undecodable(&self, error: Error, message: &str) -> Error {
        __undecodable_ie::<P>(error, message, self.id, self.criticality, self.position)
    }
}

impl<'s, L: List> Iterator for Items<'s, L> {
    type Item = Result<<L::Item as Ie>::Value<'s>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let walking = match &mut self.state {
                State::Walking(walking) => walking,
                State::Failed(error) => return error.take().map(Err),
                State::Done => return None,
            };
            if walking.left == 0 {
                // No octet after the last item.
                let end = Reader::new(&walking.octets[walking.at..]).finish();
                let end = end.map_err(|error| walking.list.undecodable(error, walking.message));
                self.state = State::Failed(end.err());
                continue;
            }
            // The single container of the next item: its IE and its value, decoded as the
            // type of the item when it is that IE, and the octets that it takes.
            let read = match &walking.octets {
                Cow::Borrowed(octets) => {
                    let octets: &'s [u8] = octets;
                    let mut reader = Reader::new(&octets[walking.at..]);
                    <L::Protocol as Protocol>::Field::decode(&mut reader).map(|field| {
                        let value = (field.id() == L::Item::ID).then(|| {
                            match field.value().as_borrowed() {
                                Some(value) => crate::decode(value),
                                None => L::Item::__owned(field.value()),
                            }
                        });
                        let used = reader.position() / 8;
                        (value, field.id(), field.criticality(), used)
                    })
                }
                Cow::Owned(octets) => {
                    let mut reader = Reader::new(&octets[walking.at..]);
                    <L::Protocol as Protocol>::Field::decode(&mut reader).map(|field| {
                        let value =
                            (field.id() == L::Item::ID).then(|| L::Item::__owned(field.value()));
                        let used = reader.position() / 8;
                        (value, field.id(), field.criticality(), used)
                    })
                }
            };
            let read = read.map(|(value, id, criticality, used)| {
                let index = walking.advance(used);
                value.map(|value| (value, id, criticality, index))
            });
            match read {
                // The single container of another IE.
                Ok(None) => {}
                Ok(Some((Ok(value), ..))) => return Some(Ok(value)),
                Ok(Some((Err(error), id, criticality, index))) => {
                    return Some(Err(walking.item_undecodable(error, id, criticality, index)));
                }
                Err(error) => {
                    let error = walking.list.undecodable(error, walking.message);
                    self.state = State::Failed(None);
                    return Some(Err(error));
                }
            }
        }
    }
}

/// For the macro that changes an IE: the first occurrence of the IE `T` in `container`,
/// with its position, decoded as its type; `None` when the container has none.
#[doc(hidden)]
pub fn __first<'s, 'a: 's, T: Ie, M: Container<'a> + InProtocol<Protocol = T::Protocol>>(
    container: &'s M,
) -> Result<Option<(usize, T::Value<'s>)>, Error> {
    let source = Source::<T::Protocol>::__of_fields(M::NAME, container.__fields());
    match source.ies_entries::<T>() {
        None => Ok(None),
        Some(Err(error)) => Err(error),
        Some(Ok(entry)) => entry
            .decode::<T>(M::NAME)
            .map(|value| Some((entry.position, value))),
    }
}

/// The items of the first occurrence of the list `L` among `fields`, the IEs of a value
/// that ASN.1 names `message`.
pub fn items_of_fields<'s, L: List>(
    message: &'static str,
    fields: &'s [<L::Protocol as Protocol>::Field<'s>],
) -> Items<'s, L> {
    Source::__of_fields(message, fields).items::<L>()
}

/// For the macro that extracts IEs: that the IEs are read from a `N`, or from a PDU that
/// holds one, and the name that ASN.1 gives `N`.
#[diagnostic::on_unimplemented(
    message = "the IEs are read from a `{Self}`, which is no `{N}`, nor a PDU that holds one",
    label = "a `{N}`, or a PDU that holds one, is expected here",
    note = "the name after the first argument of the macro is the type of the message or of \
            the transfer that the IEs are read from"
)]
#[doc(hidden)]
pub trait __Holds<N> {
    /// The name that ASN.1 gives `N`, or why `self` does not hold one.
    fn __message(&self) -> Result<&'static str, Error>;
}

impl<'a, M: Container<'a>> __Holds<M> for M {
    #[inline]
    fn __message(&self) -> Result<&'static str, Error> {
        Ok(M::NAME)
    }
}

/// For the macro that extracts IEs: an IE of what the IEs are read from, which the macro
/// keeps until it decodes it.
#[doc(hidden)]
pub trait __Item<'s>: Clone {
    /// The protocol.
    type Protocol: Protocol;
    /// The identifier of the IE.
    fn id(&self) -> u16;
    /// Its value, decoded as the type of `T`, in a message that ASN.1 names `message`: an
    /// error of the kind `UndecodableIe` when it does not decode.
    fn decode<T: Ie<Protocol = Self::Protocol>>(
        &self,
        message: &str,
    ) -> Result<T::Value<'s>, Error>;
}

impl<'s, P: Protocol> __Item<'s> for __Entry<'s, P> {
    type Protocol = P;

    #[inline]
    fn id(&self) -> u16 {
        self.id
    }

    #[inline]
    fn decode<T: Ie<Protocol = P>>(&self, message: &str) -> Result<T::Value<'s>, Error> {
        __Entry::decode::<T>(self, message)
    }
}

/// For the macro that extracts IEs: an IE of a value that is decoded, with its position.
#[doc(hidden)]
pub struct __Decoded<'s, P: Protocol> {
    position: usize,
    field: &'s P::Field<'s>,
}

impl<P: Protocol> Clone for __Decoded<'_, P> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            position: self.position,
            field: self.field,
        }
    }
}

impl<'s, P: Protocol> __Item<'s> for __Decoded<'s, P> {
    type Protocol = P;

    #[inline]
    fn id(&self) -> u16 {
        self.field.id()
    }

    #[inline]
    fn decode<T: Ie<Protocol = P>>(&self, message: &str) -> Result<T::Value<'s>, Error> {
        let octets: &'s [u8] = self.field.value().as_slice();
        match crate::decode::<T::Value<'s>>(octets) {
            Ok(value) => Ok(value),
            Err(error) => Err(self.undecodable(error, message)),
        }
    }
}

impl<P: Protocol> __Decoded<'_, P> {
    /// `error` is why the value of this IE does not decode.
    #[cold]
    fn undecodable(&self, error: Error, message: &str) -> Error {
        let (id, criticality) = (self.field.id(), self.field.criticality());
        __undecodable_ie::<P>(error, message, id, criticality, self.position)
    }
}

/// For the macro that extracts IEs: what it walks the IEs of, in their order.
#[doc(hidden)]
pub trait __Each<'s> {
    /// An IE.
    type Item: __Item<'s>;
    /// Each IE, given to `each`, then what follows them, which is no container of protocol
    /// IEs; an error when the IEs cannot be read, as the codec of the message would say.
    fn __each(self, each: impl FnMut(Self::Item)) -> Result<(), Error>;
}

impl<'s, P: Protocol> __Each<'s> for Source<'s, P> {
    type Item = __Entry<'s, P>;

    #[inline]
    fn __each(self, mut each: impl FnMut(__Entry<'s, P>)) -> Result<(), Error> {
        let message = self.message;
        let octets = match self.fields {
            Fields::Octets(octets) => octets,
            Fields::Failed(error) => return Err(error),
            Fields::Decoded(fields) => {
                for (position, field) in fields.iter().enumerate() {
                    each(__Entry {
                        position,
                        id: field.id(),
                        criticality: field.criticality(),
                        value: Value::Whole(field.value().as_slice()),
                    });
                }
                return Ok(());
            }
        };
        let (extended, count) = head(octets).map_err(|error| error.of_message(Some(message)))?;
        let mut at = 3;
        for position in 0..count {
            let (id, criticality, value, next) = match field::<P>(octets, at) {
                Ok(field) => field,
                Err(error) => {
                    return Err(error.in_field("protocol_ies").of_message(Some(message)));
                }
            };
            at = next;
            each(__Entry {
                position,
                id,
                criticality,
                value,
            });
        }
        tail(octets, at, extended).map_err(|error| error.of_message(Some(message)))
    }
}

/// The extension bit of the octets of a message, and the count of the IEs of its
/// container, in the two octets after the first (X.691 §11.9.4.1, §11.5.7.3).
#[inline]
pub(super) fn head(octets: &[u8]) -> Result<(bool, usize), Error> {
    match octets {
        [] => Err(Error::new(crate::ErrorKind::Decode, "there are no octets")),
        [first, high, low, ..] => {
            let count = usize::from(u16::from_be_bytes([*high, *low]));
            Ok((first & 0x80 != 0, count))
        }
        // The codec says why the octets are no container.
        _ => {
            let mut reader = Reader::new(octets);
            let extended = reader.bit()?;
            let count = reader.size(0, 65535);
            Ok((
                extended,
                count.map_err(|error| error.in_field("protocol_ies"))?,
            ))
        }
    }
}

/// What a later release adds to the message after its container, which ends on the octet
/// `at`, when its extension bit says so, and no octet after the message.
#[inline]
pub(super) fn tail(octets: &[u8], at: usize, extended: bool) -> Result<(), Error> {
    if !extended && at == octets.len() {
        return Ok(());
    }
    let mut reader = Reader::new(octets);
    reader.octets(at)?;
    if extended {
        reader.skip_additions()?;
    }
    reader.finish()
}

/// For the macro that extracts IEs: the IEs of a value that is decoded, which need no walk.
#[doc(hidden)]
pub struct __Fields<'s, P: Protocol> {
    fields: &'s [P::Field<'s>],
}

impl<'s, P: Protocol> __Each<'s> for __Fields<'s, P> {
    type Item = __Decoded<'s, P>;

    #[inline]
    fn __each(self, mut each: impl FnMut(__Decoded<'s, P>)) -> Result<(), Error> {
        for (position, field) in self.fields.iter().enumerate() {
            each(__Decoded { position, field });
        }
        Ok(())
    }
}

/// For the macro that extracts IEs: what the IEs are read from, a message or a transfer,
/// or a PDU, which the registry of a protocol implements this for.
#[doc(hidden)]
pub trait __Source<'s> {
    /// The protocol.
    type Protocol: Protocol;
    /// What the macro walks.
    type Walk: __Each<'s>;
    /// The IEs of `self` as a `N`.
    fn __source<N>(&'s self) -> Result<Self::Walk, Error>
    where
        Self: __Holds<N>;
}

impl<'s, 'a: 's, M: Container<'a>> __Source<'s> for M {
    type Protocol = <M as InProtocol>::Protocol;
    type Walk = __Fields<'s, <M as InProtocol>::Protocol>;

    #[inline]
    fn __source<N>(&'s self) -> Result<Self::Walk, Error>
    where
        Self: __Holds<N>,
    {
        <Self as __Holds<N>>::__message(self)?;
        Ok(__Fields {
            fields: self.__fields(),
        })
    }
}

/// For the registry of a protocol: that a PDU holds the message `M`, and the name that
/// ASN.1 gives it; an error of the kind `WrongMessage` for another message.
#[doc(hidden)]
pub fn __pdu_holds<'p, 'a, X: Pdu<'p>, M: Message<'a, Protocol = X::Protocol>>(
    pdu: &X,
) -> Result<&'static str, Error> {
    let (direction, code) = (pdu.direction(), pdu.procedure_code());
    if (direction, code) != (M::DIRECTION, M::PROCEDURE_CODE) {
        return Err(__wrong_message::<X::Protocol>(M::NAME, direction, code));
    }
    if !M::__OF_PROTOCOL_IES {
        return Err(Error::no_protocol_ies(M::NAME));
    }
    Ok(M::NAME)
}
