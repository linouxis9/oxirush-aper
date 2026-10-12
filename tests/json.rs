//! The JSON form of each construct, written and read back, and what the reading refuses
//! with the words that it says. The types are written by hand as generated bindings
//! write theirs.

use std::borrow::Cow;

use oxirush_aper::json::{self, Json, Reading, Refused, from_value, hex, to_value, unhex};
use oxirush_aper::{AsnType, BitString, Bits, ErrorKind, ObjectIdentifier, Octets};
use serde_json::{Value, json};

/// `Kind ::= ENUMERATED { user-inactivity, unspecified, ..., not-in-the-root }`
#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    UserInactivity,
    Unspecified,
    NotInTheRoot,
}

impl Kind {
    const VALUES: [Self; 3] = [Self::UserInactivity, Self::Unspecified, Self::NotInTheRoot];
    const NAMES: [&'static str; 3] = ["user-inactivity", "unspecified", "not-in-the-root"];
}

impl Json for Kind {
    fn to_json(&self) -> Value {
        json::text(Self::NAMES[*self as usize])
    }
    fn from_json(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
        Ok(Self::VALUES[reading.enumerated(value, &Self::NAMES)?])
    }
}

/// `Either ::= CHOICE { number INTEGER (0..255), kind Kind, home-ID BIT STRING (SIZE (28)),
/// nothing NULL }`
#[derive(Debug, Clone, PartialEq)]
enum Either {
    Number(u8),
    Kind(Kind),
    HomeId(Bits),
    Nothing,
}

impl Json for Either {
    fn to_json(&self) -> Value {
        match self {
            Self::Number(value) => json::alternative("number", json::unsigned((*value).into())),
            Self::Kind(value) => json::alternative("kind", value.to_json()),
            Self::HomeId(value) => json::alternative("home-ID", json::bits(*value, 28, 28)),
            Self::Nothing => json::alternative("nothing", Value::Null),
        }
    }
    fn from_json(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
        let names = &["number", "kind", "home-ID", "nothing"];
        let (position, value) = reading.alternative(value, names)?;
        Ok(match position {
            0 => Self::Number(reading.within("number", value, |value, reading| {
                Ok(reading.unsigned(value, 0, 255)? as u8)
            })?),
            1 => Self::Kind(reading.within("kind", value, Kind::from_json)?),
            2 => Self::HomeId(reading.within("home-ID", value, |value, reading| {
                reading.bits(value, 28, 28)
            })?),
            _ => {
                reading.within("nothing", value, |value, reading| reading.null(value))?;
                Self::Nothing
            }
        })
    }
}

/// A SEQUENCE with a member of each kind, and an extension marker before `later`.
#[derive(Debug, Clone, PartialEq)]
struct Sample {
    flag: bool,
    /// `INTEGER (0..1099511627775)`
    id: u64,
    /// `INTEGER (0..4000000000000, ...)`
    rate: u64,
    /// `INTEGER (-8..7)`
    small: i8,
    kind: Kind,
    either: Either,
    octets: Octets<'static>,
    /// `OCTET STRING (SIZE (3))`
    plmn: [u8; 3],
    /// `BIT STRING (SIZE (12))`
    fixed_bits: Bits,
    /// `BIT STRING (SIZE (16, ...))`
    mask: BitString<'static>,
    /// `BIT STRING (SIZE (1..160, ...))`
    address: BitString<'static>,
    /// `BIT STRING (SIZE (7)) OPTIONAL`
    day_of_week: Option<Bits>,
    /// `BIT STRING (SIZE (1..32))`
    few: Bits,
    name: Cow<'static, str>,
    printable: Cow<'static, str>,
    visible: Cow<'static, str>,
    ia5: Cow<'static, str>,
    object: ObjectIdentifier,
    open: Octets<'static>,
    list: Vec<Either>,
    /// `INTEGER (0..255) OPTIONAL`
    absent: Option<u8>,
    later: Option<Kind>,
}

impl AsnType for Sample {
    const NAME: &'static str = "Sample";
}

const MEMBERS: &[&str] = &[
    "flag",
    "id",
    "rate",
    "small",
    "kind",
    "either",
    "octets",
    "plmn",
    "fixed-bits",
    "mask",
    "address",
    "day-of-week",
    "few",
    "name",
    "printable",
    "visible",
    "ia5",
    "object",
    "open",
    "list",
    "absent",
    "later",
];

impl Json for Sample {
    fn to_json(&self) -> Value {
        let mut members = json::Members::new();
        members.put("flag", Value::Bool(self.flag));
        members.put("id", json::unsigned(self.id));
        members.put("rate", json::unsigned(self.rate));
        members.put("small", json::signed(self.small.into()));
        members.put("kind", self.kind.to_json());
        members.put("either", self.either.to_json());
        members.put("octets", json::octets(&self.octets));
        members.put("plmn", json::octets(&self.plmn));
        members.put("fixed-bits", json::bits(self.fixed_bits, 12, 12));
        members.put("mask", json::bit_string(&self.mask, 16, 16, true));
        members.put("address", json::bit_string(&self.address, 1, 160, true));
        let day = self.day_of_week.map(|bits| json::bits(bits, 7, 7));
        members.put_optional("day-of-week", day);
        members.put("few", json::bits(self.few, 1, 32));
        members.put("name", json::text(&self.name));
        members.put("printable", json::text(&self.printable));
        members.put("visible", json::text(&self.visible));
        members.put("ia5", json::text(&self.ia5));
        members.put("object", json::object_identifier(&self.object));
        members.put("open", json::octets(&self.open));
        let list = self.list.iter().map(|item| item.to_json()).collect();
        members.put("list", Value::Array(list));
        let absent = self.absent.map(|value| json::unsigned(value.into()));
        members.put_optional("absent", absent);
        members.put_optional("later", self.later.map(|value| value.to_json()));
        members.finish()
    }

    fn from_json(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
        let members = reading.members(value, MEMBERS)?;
        Ok(Self {
            flag: reading.member(&members, 0, |v, r| r.boolean(v))?,
            id: reading.member(&members, 1, |v, r| r.unsigned(v, 0, 1_099_511_627_775))?,
            rate: reading.member(&members, 2, |v, r| r.unsigned(v, u64::MIN, u64::MAX))?,
            small: reading.member(&members, 3, |v, r| Ok(r.signed(v, -8, 7)? as i8))?,
            kind: reading.member(&members, 4, Kind::from_json)?,
            either: reading.member(&members, 5, Either::from_json)?,
            octets: reading.member(&members, 6, |v, r| r.octets(v))?,
            plmn: reading.member(&members, 7, |v, r| r.octet_array(v))?,
            fixed_bits: reading.member(&members, 8, |v, r| r.bits(v, 12, 12))?,
            mask: reading.member(&members, 9, |v, r| r.bit_string(v, 16, 16, true))?,
            address: reading.member(&members, 10, |v, r| r.bit_string(v, 1, 160, true))?,
            day_of_week: reading.optional(&members, 11, |v, r| r.bits(v, 7, 7))?,
            few: reading.member(&members, 12, |v, r| r.bits(v, 1, 32))?,
            name: reading.member(&members, 13, |v, r| r.utf8_string(v))?,
            printable: reading.member(&members, 14, |v, r| r.printable_string(v))?,
            visible: reading.member(&members, 15, |v, r| r.visible_string(v))?,
            ia5: reading.member(&members, 16, |v, r| r.ia5_string(v))?,
            object: reading.member(&members, 17, |v, r| r.object_identifier(v))?,
            open: reading.member(&members, 18, |v, r| r.octets(v))?,
            list: reading.member(&members, 19, |v, r| r.list(v, Either::from_json))?,
            absent: reading.optional(&members, 20, |v, r| Ok(r.unsigned(v, 0, 255)? as u8))?,
            later: reading.optional(&members, 21, Kind::from_json)?,
        })
    }
}

fn sample() -> Sample {
    Sample {
        flag: true,
        id: 1_099_511_627_775,
        rate: 4_000_000_000_000,
        small: -8,
        kind: Kind::UserInactivity,
        either: Either::HomeId(Bits::new(0x1234567, 28).unwrap()),
        octets: vec![0x00, 0xff].into(),
        plmn: [0x02, 0xf8, 0x39],
        fixed_bits: Bits::new(0xabc, 12).unwrap(),
        mask: BitString::from([0xc0, 0x00]),
        address: BitString::from([10, 0, 0, 1]),
        day_of_week: Some(Bits::new(0x7f, 7).unwrap()),
        few: Bits::new(0b101, 3).unwrap(),
        name: "gnb-1".into(),
        printable: "AMF 1".into(),
        visible: "visible".into(),
        ia5: "a@b".into(),
        object: ObjectIdentifier::new(vec![1, 2, 840]).unwrap(),
        open: vec![0x05, 0x00].into(),
        list: vec![
            Either::Number(7),
            Either::Kind(Kind::Unspecified),
            Either::Nothing,
        ],
        absent: None,
        later: Some(Kind::NotInTheRoot),
    }
}

fn written() -> Value {
    json!({
        "flag": true,
        "id": 1_099_511_627_775u64,
        "rate": 4_000_000_000_000u64,
        "small": -8,
        "kind": "user-inactivity",
        "either": {"home-ID": "12345670"},
        "octets": "00FF",
        "plmn": "02F839",
        "fixed-bits": "ABC0",
        "mask": "C000",
        "address": {"value": "0A000001", "length": 32},
        "day-of-week": "FE",
        "few": {"value": "A0", "length": 3},
        "name": "gnb-1",
        "printable": "AMF 1",
        "visible": "visible",
        "ia5": "a@b",
        "object": "1.2.840",
        "open": "0500",
        "list": [{"number": 7}, {"kind": "unspecified"}, {"nothing": null}],
        "later": "not-in-the-root",
    })
}

/// Why `value` is not a `Sample`: the kind of the error and what it says.
fn refused(value: &Value) -> (ErrorKind, String) {
    let error = from_value::<Sample>(value).unwrap_err();
    (error.kind(), error.to_string())
}

/// The sample with `value` for its member `name`.
fn with(name: &str, value: Value) -> Value {
    let mut sample = written();
    sample[name] = value;
    sample
}

#[test]
fn each_construct_has_its_form_and_is_read_back() {
    assert_eq!(to_value(&sample()), written());
    assert_eq!(from_value::<Sample>(&written()).unwrap(), sample());
}

#[test]
fn a_bit_string_of_one_size_is_its_octets_in_a_member_that_may_be_absent() {
    // X.697 (02/2021) §24.1: a bit string whose type has one size is a JSON string,
    // wherever a value of the type is.
    assert_eq!(to_value(&sample())["day-of-week"], json!("FE"));
    let read = from_value::<Sample>(&with("day-of-week", json!("fe"))).unwrap();
    assert_eq!(read.day_of_week, sample().day_of_week);
    let (kind, why) = refused(&with("day-of-week", json!({"value": "FE", "length": 7})));
    assert_eq!(kind, ErrorKind::Value);
    assert!(
        why.contains("at day-of-week: a bit string of 7 bits is expected, in hexadecimal"),
        "{why}"
    );
    // A number that may be absent has the constraint of its member too.
    let (_, why) = refused(&with("absent", json!(256)));
    assert!(
        why.ends_with("at absent: 256 is outside its constraint (0..255)"),
        "{why}"
    );
}

#[test]
fn hexadecimal_is_written_in_upper_case_and_read_in_either() {
    assert_eq!(hex(&[0x00, 0x9a, 0xff]), "009AFF");
    assert_eq!(unhex("009aFF"), Some(vec![0x00, 0x9a, 0xff]));
    assert_eq!(unhex(""), Some(vec![]));
    // Each octet, alone and among others, and each character that is no digit.
    let every: Vec<u8> = (0..=255).collect();
    let text: String = every.iter().map(|octet| format!("{octet:02X}")).collect();
    assert_eq!(hex(&every), text);
    assert_eq!(unhex(&text), Some(every.clone()));
    assert_eq!(unhex(&text.to_lowercase()), Some(every.clone()));
    for length in 0..=every.len() {
        assert_eq!(
            unhex(&hex(&every[..length])).as_deref(),
            Some(&every[..length])
        );
    }
    for other in (0..=255u8).filter(|other| !other.is_ascii_hexdigit()) {
        for at in [0, 1, 62, 63] {
            let mut spoiled = text.clone().into_bytes();
            spoiled[at] = other;
            if let Ok(spoiled) = String::from_utf8(spoiled) {
                assert_eq!(unhex(&spoiled), None, "{other:#04x} at {at}");
            }
        }
    }
    for text in ["0", "0g", "000", " 00", "é0"] {
        assert_eq!(unhex(text), None, "{text}");
    }
    let lower = with("octets", json!("00ff"));
    assert_eq!(from_value::<Sample>(&lower).unwrap(), sample());
    let (kind, why) = refused(&with("octets", json!("0")));
    assert_eq!(kind, ErrorKind::Value);
    assert_eq!(
        why,
        "this is not a value of Sample, at octets: octets in hexadecimal, two digits for each, \
         are expected, and \"0\" is written"
    );
    let (_, why) = refused(&with("plmn", json!("02F8")));
    assert!(
        why.ends_with("at plmn: 2 octets are written for an OCTET STRING of 3"),
        "{why}"
    );
}

#[test]
fn an_integer_is_a_number_of_64_bits() {
    let read = |value: Value| from_value::<Sample>(&with("rate", value)).map(|read| read.rate);
    assert_eq!(read(json!(u64::MAX)).unwrap(), u64::MAX);
    assert_eq!(json::unsigned(u64::MAX), json!(u64::MAX));
    assert_eq!(json::signed(i64::MIN), json!(i64::MIN));

    // The decimal digits of a number are no number, whatever its size.
    for other in [
        json!("18446744073709551616"),
        json!("5"),
        json!("+1"),
        json!(""),
        json!(1.5),
        json!(null),
    ] {
        let error = read(other.clone()).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Value);
        assert_eq!(
            error.to_string(),
            format!(
                "this is not a value of Sample, at rate: a number is expected, and {other} is \
                 written"
            )
        );
    }
    // A number without a sign is not below zero.
    let error = read(json!(-1)).unwrap_err();
    assert_eq!(
        error.to_string(),
        "this is not a value of Sample, at rate: -1 is outside its constraint \
         (0..18446744073709551615)"
    );
}

#[test]
fn an_integer_outside_its_constraint_is_refused_where_it_is() {
    let (kind, why) = refused(&with("id", json!(1u64 << 40)));
    assert_eq!(kind, ErrorKind::Value);
    assert_eq!(
        why,
        "this is not a value of Sample, at id: 1099511627776 is outside its constraint \
         (0..1099511627775)"
    );
    for (value, why) in [
        (json!(8), "8 is outside its constraint (-8..7)"),
        (json!(-9), "-9 is outside its constraint (-8..7)"),
        (
            json!(u64::MAX),
            "18446744073709551615 is outside its constraint (-8..7)",
        ),
    ] {
        let (_, text) = refused(&with("small", value));
        let expected = format!("this is not a value of Sample, at small: {why}");
        assert_eq!(text, expected);
    }
    let (_, why) = refused(&with("id", json!("one")));
    assert!(
        why.ends_with("at id: a number is expected, and \"one\" is written"),
        "{why}"
    );
    // A constraint that has an extension marker takes any number of 64 bits.
    let beyond = with("rate", json!(4_000_000_000_001u64));
    assert!(from_value::<Sample>(&beyond).is_ok());
}

#[test]
fn a_name_of_an_enumerated_is_taken_whatever_its_case_and_separators() {
    for name in ["user-inactivity", "User_Inactivity", "userinactivity"] {
        let sample = from_value::<Sample>(&with("kind", json!(name))).unwrap();
        assert_eq!(sample.kind, Kind::UserInactivity, "{name}");
    }
    let extended = from_value::<Sample>(&with("kind", json!("NOT in the root"))).unwrap();
    assert_eq!(extended.kind, Kind::NotInTheRoot);
    assert!(json::same_name("user-inactivity", "UserInactivity"));
    assert!(!json::same_name("user-inactivity", "user-activity"));
    assert!(!json::same_name("-", "-"));
    let (kind, why) = refused(&with("kind", json!("no-such-kind")));
    assert_eq!(kind, ErrorKind::Value);
    assert_eq!(
        why,
        "this is not a value of Sample, at kind: \"no-such-kind\" is no value of the ENUMERATED"
    );
    let (_, why) = refused(&with("kind", json!(0)));
    assert!(
        why.ends_with("the name of a value of the ENUMERATED is expected, and 0 is written"),
        "{why}"
    );
}

#[test]
fn a_bit_string_has_its_length_unless_its_type_gives_it() {
    // A type of one size and an extension marker: a string of that size is its octets,
    // and one of another size has its length.
    let mask = |bits: BitString<'static>| json::bit_string(&bits, 16, 16, true);
    let read = |value: Value| from_value::<Sample>(&with("mask", value)).map(|read| read.mask);
    assert_eq!(mask(BitString::from([0xc0, 0x00])), json!("C000"));
    let twenty = BitString::new(vec![0xc0, 0x00, 0x00], 20).unwrap();
    let longer = json!({"value": "C00000", "length": 20});
    assert_eq!(mask(twenty.clone()), longer);
    assert_eq!(read(longer).unwrap(), twenty);
    assert_eq!(read(json!("c000")).unwrap(), BitString::from([0xc0, 0x00]));
    // The size of the type is also read with its length.
    let sized = json!({"value": "C000", "length": 16});
    assert_eq!(read(sized).unwrap(), BitString::from([0xc0, 0x00]));
    assert!(read(json!("C00000")).is_err());
    let error = read(json!(16)).unwrap_err();
    assert_eq!(
        error.to_string(),
        "this is not a value of Sample, at mask: a bit string of 16 bits is expected, in \
         hexadecimal, or one of another size as {\"value\": HEXADECIMAL, \"length\": BITS}, and \
         16 is written"
    );
    // A string of a type of one size and no marker is its octets, whatever its size.
    let other = BitString::new(vec![0xc0], 4).unwrap();
    assert_eq!(json::bit_string(&other, 16, 16, false), json!("C0"));

    // What is not the string that it says.
    for (member, value, why) in [
        (
            "fixed-bits",
            json!("ABC"),
            "\"ABC\" is not hexadecimal, two digits for each octet",
        ),
        (
            "fixed-bits",
            json!("ABC0FF"),
            "3 octets are written for a bit string of 12 bits, which has 2",
        ),
        (
            "fixed-bits",
            json!("ABC1"),
            "the last 4 bits of \"ABC1\" are after the 12 bits of the bit string, and are not \
             zero",
        ),
        (
            "fixed-bits",
            json!({"value": "ABC0", "length": 12}),
            "a bit string of 12 bits is expected, in hexadecimal, and \
             {\"length\":12,\"value\":\"ABC0\"} is written",
        ),
        (
            "address",
            json!("0A000001"),
            "a bit string whose size varies is expected, which is written as {\"value\": \
             HEXADECIMAL, \"length\": BITS}, and \"0A000001\" is written",
        ),
        (
            "address",
            json!({"value": "0A000001", "length": 32, "other": 1}),
            "a bit string whose size varies is expected, which is written as {\"value\": \
             HEXADECIMAL, \"length\": BITS}, and {\"length\":32,\"other\":1,\"value\":\"0A000001… \
             is written",
        ),
        (
            "few",
            json!({"value": "0000000000", "length": 33}),
            "a size of 33 is outside its constraint (1..32)",
        ),
        (
            "few",
            json!({"value": "", "length": 0}),
            "a size of 0 is outside its constraint (1..32)",
        ),
    ] {
        let (kind, text) = refused(&with(member, value));
        assert_eq!(kind, ErrorKind::Value);
        let expected = format!("this is not a value of Sample, at {member}: {why}");
        assert_eq!(text, expected);
    }
}

#[test]
fn a_member_that_the_type_has_not_and_one_that_it_lacks_are_refused() {
    let (kind, why) = refused(&with("other", json!(1)));
    assert_eq!(kind, ErrorKind::NoSuchMember);
    assert_eq!(
        why,
        "this is not a value of Sample: \"other\" is not a member that its type has"
    );

    let mut without = written();
    without.as_object_mut().unwrap().remove("flag");
    let (kind, why) = refused(&without);
    assert_eq!(kind, ErrorKind::Value);
    assert_eq!(
        why,
        "this is not a value of Sample, at flag: the member is missing"
    );

    // A member that may be absent is.
    without = written();
    without.as_object_mut().unwrap().remove("later");
    assert_eq!(from_value::<Sample>(&without).unwrap().later, None);

    let (_, why) = refused(&json!([1]));
    assert_eq!(
        why,
        "this is not a value of Sample: an object is expected, and [1] is written"
    );
}

#[test]
fn a_choice_is_one_alternative_that_its_type_has() {
    let (kind, why) = refused(&with("either", json!({"other": 1})));
    assert_eq!(kind, ErrorKind::NoSuchMember);
    assert_eq!(
        why,
        "this is not a value of Sample, at either: \"other\" is not an alternative that its \
         type has: it has number, kind, home-ID, nothing"
    );
    for other in [
        json!({}),
        json!({"number": 1, "kind": "unspecified"}),
        json!(1),
    ] {
        let (kind, why) = refused(&with("either", other));
        assert_eq!(kind, ErrorKind::Value);
        assert!(
            why.starts_with(
                "this is not a value of Sample, at either: an object with one member is \
                 expected, the name of an alternative with its value, and "
            ),
            "{why}"
        );
    }
    // The fault of an entry of a list is at its position.
    let list = json!([{"number": 7}, {"number": 256}]);
    let (_, why) = refused(&with("list", list));
    assert_eq!(
        why,
        "this is not a value of Sample, at list/1/number: 256 is outside its constraint (0..255)"
    );
    let (_, why) = refused(&with("list", json!({})));
    assert!(
        why.ends_with("at list: a list is expected, and {} is written"),
        "{why}"
    );
}

#[test]
fn the_other_values_are_refused_as_what_they_are_not() {
    for (member, value, why) in [
        (
            "flag",
            json!(1),
            "true or false is expected, and 1 is written",
        ),
        ("name", json!(1), "a string is expected, and 1 is written"),
        (
            "printable",
            json!("a_b"),
            "\"a_b\" has a character that a PrintableString has not",
        ),
        (
            "visible",
            json!("a\tb"),
            "\"a\\tb\" has a character that a VisibleString has not",
        ),
        (
            "ia5",
            json!("é"),
            "\"é\" has a character that an IA5String has not",
        ),
        (
            "object",
            json!("1.x"),
            "an object identifier is expected, its arcs in decimal with a full stop between \
             them, and \"1.x\" is written",
        ),
        (
            "open",
            json!({"number": 1}),
            "octets in hexadecimal, two digits for each, are expected, and {\"number\":1} is \
             written",
        ),
    ] {
        let (kind, text) = refused(&with(member, value));
        assert_eq!(kind, ErrorKind::Value);
        let expected = format!("this is not a value of Sample, at {member}: {why}");
        assert_eq!(text, expected);
    }
    let (_, why) = refused(&with("either", json!({"nothing": 0})));
    assert!(
        why.ends_with("at either/nothing: null is expected, and 0 is written"),
        "{why}"
    );
}
