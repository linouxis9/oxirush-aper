//! The worked examples of ITU-T X.691 (02/2021) Annex A: each record value is written
//! to the octets that the annex gives for the ALIGNED variant, and read back from them.
//!
//! The types are written by hand, as generated bindings are: a function that writes
//! and one that reads, with the fields of the reader and of the writer. What the
//! crate has no method for is written here with bits, to show the encoding whole: a
//! permitted alphabet of digits (A.2, A.3), a NumericString and an extension addition
//! group (A.4). A SET is written in the canonical order of its tags (X.691 §21).

use oxirush_aper::{Error, Reader, UNBOUNDED, Writer};

fn octets(hexadecimal: &str) -> Vec<u8> {
    let digits: Vec<u8> = hexadecimal
        .bytes()
        .filter(|digit| !digit.is_ascii_whitespace())
        .collect();
    digits
        .chunks(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
struct Name {
    given_name: String,
    initial: String,
    family_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Sex {
    Male,
    Female,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
struct Child {
    name: Name,
    date_of_birth: String,
    /// The extension addition of A.3.
    sex: Option<Sex>,
}

#[derive(Debug, Clone, PartialEq)]
struct PersonnelRecord {
    name: Name,
    title: String,
    number: i64,
    date_of_hire: String,
    name_of_spouse: Name,
    children: Option<Vec<Child>>,
}

fn name(given_name: &str, initial: &str, family_name: &str) -> Name {
    Name {
        given_name: given_name.into(),
        initial: initial.into(),
        family_name: family_name.into(),
    }
}

/// The record value of A.1.2, A.2.2 and A.3.2: the last one has the sex of a child.
fn record(sex: Option<Sex>) -> PersonnelRecord {
    PersonnelRecord {
        name: name("John", "P", "Smith"),
        title: "Director".into(),
        number: 51,
        date_of_hire: "19710917".into(),
        name_of_spouse: name("Mary", "T", "Smith"),
        children: Some(vec![
            Child {
                name: name("Ralph", "T", "Smith"),
                date_of_birth: "19571111".into(),
                sex: None,
            },
            Child {
                name: name("Susan", "B", "Jones"),
                date_of_birth: "19590717".into(),
                sex,
            },
        ]),
    }
}

// ── A.1: no subtype constraint ──────────────────────────────────────────────

mod a1 {
    use super::*;

    fn write_name(name: &Name, w: &mut Writer<'_>) -> Result<(), Error> {
        w.visible_string(&name.given_name, 0, UNBOUNDED)?;
        w.visible_string(&name.initial, 0, UNBOUNDED)?;
        w.visible_string(&name.family_name, 0, UNBOUNDED)
    }

    fn read_name(r: &mut Reader<'_>) -> Result<Name, Error> {
        Ok(Name {
            given_name: r.visible_string(0, UNBOUNDED)?.into_owned(),
            initial: r.visible_string(0, UNBOUNDED)?.into_owned(),
            family_name: r.visible_string(0, UNBOUNDED)?.into_owned(),
        })
    }

    pub fn write(record: &PersonnelRecord, w: &mut Writer<'_>) -> Result<(), Error> {
        // `children` has a default value: one bit says whether it is there.
        w.bit(record.children.is_some());
        write_name(&record.name, w)?;
        w.unconstrained(record.number);
        w.visible_string(&record.title, 0, UNBOUNDED)?;
        w.visible_string(&record.date_of_hire, 0, UNBOUNDED)?;
        write_name(&record.name_of_spouse, w)?;
        if let Some(children) = &record.children {
            let (count, more) = w.length(children.len());
            assert_eq!((count, more), (children.len(), false));
            for child in children {
                write_name(&child.name, w)?;
                w.visible_string(&child.date_of_birth, 0, UNBOUNDED)?;
            }
        }
        Ok(())
    }

    pub fn read(r: &mut Reader<'_>) -> Result<PersonnelRecord, Error> {
        let has_children = r.bit()?;
        let name = read_name(r)?;
        let number = r.unconstrained()?;
        let title = r.visible_string(0, UNBOUNDED)?.into_owned();
        let date_of_hire = r.visible_string(0, UNBOUNDED)?.into_owned();
        let name_of_spouse = read_name(r)?;
        let children = match has_children {
            true => {
                let (count, more) = r.length()?;
                assert!(!more);
                let mut children = r.list(count);
                for _ in 0..count {
                    children.push(Child {
                        name: read_name(r)?,
                        date_of_birth: r.visible_string(0, UNBOUNDED)?.into_owned(),
                        sex: None,
                    });
                }
                Some(children)
            }
            false => None,
        };
        Ok(PersonnelRecord {
            name,
            title,
            number,
            date_of_hire,
            name_of_spouse,
            children,
        })
    }
}

/// A.1.3.1: 94 octets.
const A1: &str = "
    80044A6F 686E0150 05536D69 74680133 08446972 6563746F 72083139 37313039
    3137044D 61727901 5405536D 69746802 0552616C 70680154 05536D69 74680831
    39353731 31313105 53757361 6E014205 4A6F6E65 73083139 35393037 3137";

#[test]
fn a1_a_record_that_does_not_use_subtype_constraints() {
    let expected = octets(A1);
    assert_eq!(expected.len(), 94);
    let mut written = Vec::new();
    a1::write(&record(None), &mut Writer::new(&mut written)).unwrap();
    assert_eq!(written, expected);
    let mut reader = Reader::new(&expected);
    assert_eq!(a1::read(&mut reader).unwrap(), record(None));
    reader.finish().unwrap();
}

// ── A.2 and A.3: subtype constraints, then extension markers ───────────────

/// `VisibleString (FROM ("0".."9") ^ SIZE (8))`: ten characters take four bits each, and
/// a character is its place among them (X.691 §30.5.2, §30.5.4 b). Thirty-two bits are
/// octet-aligned (§30.5.6).
fn write_date(date: &str, w: &mut Writer<'_>) {
    assert_eq!(date.len(), 8);
    w.align();
    for digit in date.bytes() {
        w.bits(u64::from(digit - b'0'), 4);
    }
}

fn read_date(r: &mut Reader<'_>) -> Result<String, Error> {
    r.align();
    let mut date = String::new();
    for _ in 0..8 {
        date.push(char::from(b'0' + r.bits(4)? as u8));
    }
    Ok(date)
}

/// A.2 when the types have no extension marker, and A.3 when they have one.
mod constrained {
    use super::*;

    /// `NameString`: `VisibleString (FROM ("a".."z" | "A".."Z" | "-.") ^ SIZE (1..64))`.
    /// Fifty-four characters take six bits, and eight in the aligned variant, where
    /// each is its own code (X.691 §30.5.2, §30.5.4 a).
    fn write_name_string(text: &str, extensible: bool, w: &mut Writer<'_>) -> Result<(), Error> {
        match extensible {
            true => w.visible_string_extensible(text, 1, 64),
            false => w.visible_string(text, 1, 64),
        }
    }

    fn read_name_string(extensible: bool, r: &mut Reader<'_>) -> Result<String, Error> {
        let text = match extensible {
            true => r.visible_string_extensible(1, 64)?,
            false => r.visible_string(1, 64)?,
        };
        Ok(text.into_owned())
    }

    fn write_name(name: &Name, extensible: bool, w: &mut Writer<'_>) -> Result<(), Error> {
        if extensible {
            w.bit(false);
        }
        write_name_string(&name.given_name, extensible, w)?;
        // `NameString (SIZE (1))`: the size constraint of the type is replaced, and
        // with it its extension marker.
        w.visible_string(&name.initial, 1, 1)?;
        write_name_string(&name.family_name, extensible, w)
    }

    fn read_name(extensible: bool, r: &mut Reader<'_>) -> Result<Name, Error> {
        let extended = extensible && r.bit()?;
        let name = Name {
            given_name: read_name_string(extensible, r)?,
            initial: r.visible_string(1, 1)?.into_owned(),
            family_name: read_name_string(extensible, r)?,
        };
        if extended {
            r.skip_additions()?;
        }
        Ok(name)
    }

    fn write_child(child: &Child, extensible: bool, w: &mut Writer<'_>) -> Result<(), Error> {
        if extensible {
            w.bit(child.sex.is_some());
        }
        write_name(&child.name, extensible, w)?;
        if extensible {
            // The size of a date is within the root of its constraint.
            w.bit(false);
        }
        write_date(&child.date_of_birth, w);
        if let Some(sex) = child.sex {
            // One addition, which is present: an open type that holds the index of
            // the value among the three.
            w.additions(&[true])?;
            let start = w.begin_open_type();
            w.index(sex as u64, 3, false);
            w.end_open_type(start);
        }
        Ok(())
    }

    fn read_child(extensible: bool, r: &mut Reader<'_>) -> Result<Child, Error> {
        let extended = extensible && r.bit()?;
        let name = read_name(extensible, r)?;
        if extensible {
            assert!(!r.bit()?);
        }
        let date_of_birth = read_date(r)?;
        let mut sex = None;
        if extended {
            let mut additions = r.additions()?;
            if additions.present() {
                let value = r.open_type()?;
                let mut inner = Reader::new(&value);
                sex = Some(
                    [Sex::Male, Sex::Female, Sex::Unknown][inner.enumerated(3, 0, false)? as usize],
                );
            }
            additions.skip_rest(r)?;
        }
        Ok(Child {
            name,
            date_of_birth,
            sex,
        })
    }

    pub fn write(
        record: &PersonnelRecord,
        extensible: bool,
        w: &mut Writer<'_>,
    ) -> Result<(), Error> {
        if extensible {
            w.bit(false);
        }
        w.bit(record.children.is_some());
        write_name(&record.name, extensible, w)?;
        match extensible {
            // `INTEGER (0..9999, ...)`.
            true => w.unsigned_extensible(record.number as u64, 0, 9999),
            false => w.unconstrained(record.number),
        }
        w.visible_string(&record.title, 0, UNBOUNDED)?;
        if extensible {
            w.bit(false);
        }
        write_date(&record.date_of_hire, w);
        write_name(&record.name_of_spouse, extensible, w)?;
        if let Some(children) = &record.children {
            match extensible {
                // `SEQUENCE (SIZE (2, ...)) OF`: a bit, and no count for the one size.
                true => {
                    w.bit(children.len() != 2);
                    w.size(children.len(), 2, 2)?;
                }
                false => {
                    w.length(children.len());
                }
            }
            for child in children {
                write_child(child, extensible, w)?;
            }
        }
        Ok(())
    }

    pub fn read(extensible: bool, r: &mut Reader<'_>) -> Result<PersonnelRecord, Error> {
        let extended = extensible && r.bit()?;
        let has_children = r.bit()?;
        let name = read_name(extensible, r)?;
        let number = match extensible {
            true => r.unsigned_extensible(0, 9999)? as i64,
            false => r.unconstrained()?,
        };
        let title = r.visible_string(0, UNBOUNDED)?.into_owned();
        if extensible {
            assert!(!r.bit()?);
        }
        let date_of_hire = read_date(r)?;
        let name_of_spouse = read_name(extensible, r)?;
        let children = match has_children {
            true => {
                let count = match extensible {
                    true => {
                        assert!(!r.bit()?);
                        r.size(2, 2)?
                    }
                    false => r.length()?.0,
                };
                let mut children = r.list(count);
                for _ in 0..count {
                    children.push(read_child(extensible, r)?);
                }
                Some(children)
            }
            false => None,
        };
        if extended {
            r.skip_additions()?;
        }
        Ok(PersonnelRecord {
            name,
            title,
            number,
            date_of_hire,
            name_of_spouse,
            children,
        })
    }
}

/// A.2.3.1: 74 octets.
const A2: &str = "
    864A6F68 6E501053 6D697468 01330844 69726563 746F7219 7109170C 4D617279
    5410536D 69746802 1052616C 70685410 536D6974 68195711 11105375 73616E42
    104A6F6E 65731959 0717";

#[test]
fn a2_a_record_that_uses_subtype_constraints() {
    let expected = octets(A2);
    assert_eq!(expected.len(), 74);
    let mut written = Vec::new();
    constrained::write(&record(None), false, &mut Writer::new(&mut written)).unwrap();
    assert_eq!(written, expected);
    let mut reader = Reader::new(&expected);
    assert_eq!(constrained::read(false, &mut reader).unwrap(), record(None));
    reader.finish().unwrap();
}

/// A.3.3.1: 83 octets.
const A3: &str = "
    40C04A6F 686E5008 536D6974 68000033 08446972 6563746F 72001971 0917034D
    61727954 08536D69 74680100 52616C70 68540853 6D697468 00195711 11820053
    7573616E 42084A6F 6E657300 19590717 010140";

#[test]
fn a3_a_record_that_uses_extension_markers() {
    let expected = octets(A3);
    assert_eq!(expected.len(), 83);
    let value = record(Some(Sex::Female));
    let mut written = Vec::new();
    constrained::write(&value, true, &mut Writer::new(&mut written)).unwrap();
    assert_eq!(written, expected);
    let mut reader = Reader::new(&expected);
    assert_eq!(constrained::read(true, &mut reader).unwrap(), value);
    reader.finish().unwrap();
}

// ── A.4: extension addition groups ──────────────────────────────────────────

/// `{ a 253, b TRUE, c e : TRUE, g "123", h TRUE }` of the type `Ax` of A.4.1.
#[derive(Debug, PartialEq)]
struct Ax {
    a: u8,
    b: bool,
    /// The alternative `e` of the CHOICE `c`, which is its first addition.
    c_e: bool,
    /// The members of the addition group.
    g: String,
    h: Option<bool>,
}

/// A character of a NumericString takes four bits, and is its place among the space
/// and the ten digits (X.691 §30.5.4 b).
const NUMERIC: &[u8] = b" 0123456789";

fn write_ax(value: &Ax, w: &mut Writer<'_>) -> Result<(), Error> {
    // Additions follow, and neither `i` nor `j` is there.
    w.bit(true);
    w.bits(0, 2);
    w.unsigned(value.a.into(), 250, 253)?;
    w.bit(value.b);
    // `c`: its only alternative of the root is `d`, and `e` is the first addition. An
    // addition is an open type.
    w.index(1, 1, true);
    let start = w.begin_open_type();
    w.bit(value.c_e);
    w.end_open_type(start);
    // One addition, the group, which is a SEQUENCE in an open type: a bit for `h`,
    // which may be absent, then `g`, three characters of four bits that are not
    // aligned (X.691 §30.5.6), then `h`.
    w.additions(&[true])?;
    let start = w.begin_open_type();
    w.bit(value.h.is_some());
    for character in value.g.bytes() {
        let place = NUMERIC.iter().position(|known| *known == character);
        w.bits(place.unwrap() as u64, 4);
    }
    if let Some(h) = value.h {
        w.bit(h);
    }
    w.end_open_type(start);
    Ok(())
}

fn read_ax(r: &mut Reader<'_>) -> Result<Ax, Error> {
    let extended = r.bit()?;
    assert_eq!(r.bits(2)?, 0);
    let a = r.unsigned(250, 253)? as u8;
    let b = r.bit()?;
    assert_eq!(r.choice(1, 1, true)?, 1);
    let alternative = r.open_type()?;
    let c_e = Reader::new(&alternative).bit()?;
    assert!(extended);
    let mut additions = r.additions()?;
    assert!(additions.present());
    let group = r.open_type()?;
    let mut inner = Reader::new(&group);
    let has_h = inner.bit()?;
    let mut g = String::new();
    for _ in 0..3 {
        g.push(char::from(NUMERIC[inner.bits(4)? as usize]));
    }
    let h = match has_h {
        true => Some(inner.bit()?),
        false => None,
    };
    additions.skip_rest(r)?;
    Ok(Ax { a, b, c_e, g, h })
}

#[test]
fn a4_a_record_that_uses_extension_addition_groups() {
    // A.4.3.1: 8 octets.
    let expected = octets("9E000180 010291A4");
    let value = Ax {
        a: 253,
        b: true,
        c_e: true,
        g: "123".into(),
        h: Some(true),
    };
    let mut written = Vec::new();
    write_ax(&value, &mut Writer::new(&mut written)).unwrap();
    assert_eq!(written, expected);
    let mut reader = Reader::new(&expected);
    assert_eq!(read_ax(&mut reader).unwrap(), value);
    reader.finish().unwrap();
}
