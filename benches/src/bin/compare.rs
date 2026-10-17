/*
   OxiRush
   Copyright 2025-2026 Valentin D'Emmanuele

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

   http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

//! One operation of the crate or of rasn 0.29 on one value of NGAP, as
//! `benches/compare.py` runs it:
//!
//! ```text
//! compare CODEC OPERATION VALUE ITERATIONS
//! ```
//!
//! `CODEC` is `aper`, the crate with the types written as the generated bindings of
//! oxirush-ngap write them, or `rasn`, the derive of rasn 0.29 on the same types of
//! TS 38.413 clause 9.4. `OPERATION` is `decode` or `encode`, and `VALUE` one of those of
//! [`VALUES`]. It runs the operation `ITERATIONS` times as a warm-up, then three times
//! `ITERATIONS` times, and prints `{"ns": N, "allocations": A, "bytes": B}`: the time of a
//! call in the fastest of the three, and the allocations and the octets of a call.
//! `compare list` prints the values with their octets, which both read and write alike.

use oxirush_aper::{AsnType, Bits, Decode, Encode, Error, Octets, Reader, UNBOUNDED, Writer};
use std::alloc::{GlobalAlloc, Layout, System};
use std::borrow::Cow;
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::time::Instant;

/// An allocator that counts, for the allocations of one call.
struct Counting;
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static OCTETS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Relaxed);
        OCTETS.fetch_add(layout.size(), Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Relaxed);
        OCTETS.fetch_add(size, Relaxed);
        unsafe { System.realloc(pointer, layout, size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// The types as the generated bindings of oxirush-ngap have them on the crate.
mod ours {
    use super::*;

    /// `ProtocolExtensionField ::= SEQUENCE { id, criticality, extensionValue }`
    #[derive(Debug, Clone, PartialEq)]
    pub struct Field<'a> {
        pub id: u16,
        pub criticality: u64,
        pub value: Octets<'a>,
    }

    /// `ProtocolExtensionContainer ::= SEQUENCE (SIZE (1..65535)) OF Field`
    #[derive(Debug, Clone, PartialEq)]
    pub struct Extensions<'a>(pub Vec<Field<'a>>);

    impl AsnType for Extensions<'_> {
        const NAME: &'static str = "ProtocolExtensionContainer";
    }

    impl<'a> Decode<'a> for Extensions<'a> {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let count = reader.size(1, 65535)?;
            let mut list = reader.list(count);
            for _ in 0..count {
                let id = reader.unsigned(0, 65535)? as u16;
                let criticality = reader.enumerated(3, 0, false)?;
                let value = reader.open_type()?;
                list.push(Field {
                    id,
                    criticality,
                    value,
                });
            }
            Ok(Self(list))
        }
    }

    impl Encode for Extensions<'_> {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.size(self.0.len(), 1, 65535)?;
            for field in &self.0 {
                writer.unsigned(field.id.into(), 0, 65535)?;
                writer.index(field.criticality, 3, false);
                writer.open_type(&field.value);
            }
            Ok(())
        }
    }

    /// `AMF-UE-NGAP-ID ::= INTEGER (0..1099511627775)`
    #[derive(Debug, Clone, PartialEq)]
    pub struct AmfUeNgapId(pub u64);

    impl AsnType for AmfUeNgapId {
        const NAME: &'static str = "AMF-UE-NGAP-ID";
    }

    impl<'a> Decode<'a> for AmfUeNgapId {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self(reader.unsigned(0, 1099511627775)?))
        }
    }

    impl Encode for AmfUeNgapId {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.unsigned(self.0, 0, 1099511627775)
        }
    }

    /// `NAS-PDU ::= OCTET STRING`
    #[derive(Debug, Clone, PartialEq)]
    pub struct NasPdu<'a>(pub Octets<'a>);

    impl AsnType for NasPdu<'_> {
        const NAME: &'static str = "NAS-PDU";
    }

    impl<'a> Decode<'a> for NasPdu<'a> {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self(reader.octet_string(0, UNBOUNDED)?))
        }
    }

    impl Encode for NasPdu<'_> {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.octet_string(&self.0, 0, UNBOUNDED)
        }
    }

    /// `AMFName ::= PrintableString (SIZE(1..150, ...))`
    #[derive(Debug, Clone, PartialEq)]
    pub struct AmfName<'a>(pub Cow<'a, str>);

    impl AsnType for AmfName<'_> {
        const NAME: &'static str = "AMFName";
    }

    impl<'a> Decode<'a> for AmfName<'a> {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            Ok(Self(reader.printable_string_extensible(1, 150)?))
        }
    }

    impl Encode for AmfName<'_> {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.printable_string_extensible(&self.0, 1, 150)
        }
    }

    /// `GUAMI ::= SEQUENCE { pLMNIdentity, aMFRegionID, aMFSetID, aMFPointer,
    /// iE-Extensions OPTIONAL, ... }`
    #[derive(Debug, Clone, PartialEq)]
    pub struct Guami {
        pub plmn: [u8; 3],
        pub region: Bits,
        pub set: Bits,
        pub pointer: Bits,
        pub extensions: Option<Extensions<'static>>,
    }

    impl AsnType for Guami {
        const NAME: &'static str = "GUAMI";
    }

    impl<'a> Decode<'a> for Guami {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let extended = reader.bit()?;
            let present = reader.bits(1)?;
            let plmn = reader.octet_array::<3>()?;
            let region = reader.bits_sized(8, 8)?;
            let set = reader.bits_sized(10, 10)?;
            let pointer = reader.bits_sized(6, 6)?;
            let extensions = match present & 1 != 0 {
                true => Some(Extensions::decode(reader)?.into_owned()),
                false => None,
            };
            if extended {
                reader.skip_additions()?;
            }
            Ok(Self {
                plmn,
                region,
                set,
                pointer,
                extensions,
            })
        }
    }

    impl Encode for Guami {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.bit(false);
            writer.bit(self.extensions.is_some());
            writer.octet_array(&self.plmn);
            writer.bits_sized(self.region, 8, 8)?;
            writer.bits_sized(self.set, 10, 10)?;
            writer.bits_sized(self.pointer, 6, 6)?;
            if let Some(extensions) = &self.extensions {
                extensions.encode(writer)?;
            }
            Ok(())
        }
    }

    /// `NR-CGI ::= SEQUENCE { pLMNIdentity, nRCellIdentity BIT STRING (SIZE(36)),
    /// iE-Extensions OPTIONAL, ... }`
    #[derive(Debug, Clone, PartialEq)]
    pub struct NrCgi {
        pub plmn: [u8; 3],
        pub cell: Bits,
        pub extensions: Option<Extensions<'static>>,
    }

    /// `TAI ::= SEQUENCE { pLMNIdentity, tAC OCTET STRING (SIZE(3)), iE-Extensions
    /// OPTIONAL, ... }`
    #[derive(Debug, Clone, PartialEq)]
    pub struct Tai {
        pub plmn: [u8; 3],
        pub tac: [u8; 3],
        pub extensions: Option<Extensions<'static>>,
    }

    /// `UserLocationInformationNR ::= SEQUENCE { nR-CGI, tAI, timeStamp OCTET STRING
    /// (SIZE(4)) OPTIONAL, iE-Extensions OPTIONAL, ... }`
    #[derive(Debug, Clone, PartialEq)]
    pub struct Location {
        pub cell: NrCgi,
        pub area: Tai,
        pub time: Option<[u8; 4]>,
        pub extensions: Option<Extensions<'static>>,
    }

    impl AsnType for Location {
        const NAME: &'static str = "UserLocationInformationNR";
    }

    impl<'a> Decode<'a> for Location {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            let extended = reader.bit()?;
            let present = reader.bits(2)?;
            let cell = {
                let extended = reader.bit()?;
                let present = reader.bits(1)?;
                let plmn = reader.octet_array::<3>()?;
                let cell = reader.bits_sized(36, 36)?;
                let extensions = match present & 1 != 0 {
                    true => Some(Extensions::decode(reader)?.into_owned()),
                    false => None,
                };
                if extended {
                    reader.skip_additions()?;
                }
                NrCgi {
                    plmn,
                    cell,
                    extensions,
                }
            };
            let area = {
                let extended = reader.bit()?;
                let present = reader.bits(1)?;
                let plmn = reader.octet_array::<3>()?;
                let tac = reader.octet_array::<3>()?;
                let extensions = match present & 1 != 0 {
                    true => Some(Extensions::decode(reader)?.into_owned()),
                    false => None,
                };
                if extended {
                    reader.skip_additions()?;
                }
                Tai {
                    plmn,
                    tac,
                    extensions,
                }
            };
            let time = match present & 2 != 0 {
                true => Some(reader.octet_array::<4>()?),
                false => None,
            };
            let extensions = match present & 1 != 0 {
                true => Some(Extensions::decode(reader)?.into_owned()),
                false => None,
            };
            if extended {
                reader.skip_additions()?;
            }
            Ok(Self {
                cell,
                area,
                time,
                extensions,
            })
        }
    }

    impl Encode for Location {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.bit(false);
            writer.bit(self.time.is_some());
            writer.bit(self.extensions.is_some());
            writer.bit(false);
            writer.bit(self.cell.extensions.is_some());
            writer.octet_array(&self.cell.plmn);
            writer.bits_sized(self.cell.cell, 36, 36)?;
            if let Some(extensions) = &self.cell.extensions {
                extensions.encode(writer)?;
            }
            writer.bit(false);
            writer.bit(self.area.extensions.is_some());
            writer.octet_array(&self.area.plmn);
            writer.octet_array(&self.area.tac);
            if let Some(extensions) = &self.area.extensions {
                extensions.encode(writer)?;
            }
            if let Some(time) = &self.time {
                writer.octet_array(time);
            }
            if let Some(extensions) = &self.extensions {
                extensions.encode(writer)?;
            }
            Ok(())
        }
    }

    impl Extensions<'_> {
        pub fn into_owned(self) -> Extensions<'static> {
            let fields = self.0.into_iter().map(|field| Field {
                id: field.id,
                criticality: field.criticality,
                value: field.value.into_owned(),
            });
            Extensions(fields.collect())
        }
    }
}

/// The same types with the derive of rasn 0.29.
mod theirs {
    use rasn::prelude::*;

    #[derive(AsnType, Debug, Clone, Copy, Decode, Encode, PartialEq)]
    #[rasn(enumerated)]
    pub enum Criticality {
        Reject = 0,
        Ignore = 1,
        Notify = 2,
    }

    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(automatic_tags)]
    pub struct Field {
        #[rasn(value("0..=65535"))]
        pub id: u16,
        pub criticality: Criticality,
        pub value: Any,
    }

    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(delegate, size("1..=65535"))]
    pub struct Extensions(pub SequenceOf<Field>);

    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(delegate, value("0..=1099511627775"))]
    pub struct AmfUeNgapId(pub u64);

    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(delegate)]
    pub struct NasPdu(pub OctetString);

    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(delegate, size("1..=150", extensible))]
    pub struct AmfName(pub PrintableString);

    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(automatic_tags)]
    #[non_exhaustive]
    pub struct Guami {
        pub plmn: FixedOctetString<3>,
        pub region: FixedBitString<8>,
        pub set: FixedBitString<10>,
        pub pointer: FixedBitString<6>,
        pub extensions: Option<Extensions>,
    }

    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(automatic_tags)]
    #[non_exhaustive]
    pub struct NrCgi {
        pub plmn: FixedOctetString<3>,
        pub cell: FixedBitString<36>,
        pub extensions: Option<Extensions>,
    }

    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(automatic_tags)]
    #[non_exhaustive]
    pub struct Tai {
        pub plmn: FixedOctetString<3>,
        pub tac: FixedOctetString<3>,
        pub extensions: Option<Extensions>,
    }

    #[derive(AsnType, Debug, Clone, Decode, Encode, PartialEq)]
    #[rasn(automatic_tags)]
    #[non_exhaustive]
    pub struct Location {
        pub cell: NrCgi,
        pub area: Tai,
        pub time: Option<FixedOctetString<4>>,
        pub extensions: Option<Extensions>,
    }
}

/// The values, by name, with the octets that the crate writes of them.
fn values() -> Vec<(&'static str, Vec<u8>)> {
    use ours::*;
    let encode = |value: &dyn Fn(&mut Writer<'_>) -> Result<(), Error>| {
        let mut octets = Vec::new();
        value(&mut Writer::new(&mut octets)).unwrap();
        octets
    };
    let nas: Vec<u8> = (0..60).collect();
    let guami = Guami {
        plmn: [0x02, 0xf8, 0x39],
        region: Bits::new(0xca, 8).unwrap(),
        set: Bits::new(0x3fe, 10).unwrap(),
        pointer: Bits::new(1, 6).unwrap(),
        extensions: None,
    };
    let location = Location {
        cell: NrCgi {
            plmn: [0x02, 0xf8, 0x39],
            cell: Bits::new(0x1_2345_6001, 36).unwrap(),
            extensions: None,
        },
        area: Tai {
            plmn: [0x02, 0xf8, 0x39],
            tac: [0, 0, 1],
            extensions: None,
        },
        time: Some([0xe9, 0x8f, 0x2c, 0x10]),
        extensions: None,
    };
    vec![
        ("AMF-UE-NGAP-ID", encode(&|w| AmfUeNgapId(42).encode(w))),
        (
            "NAS-PDU",
            encode(&|w| NasPdu(Octets::from(&nas[..])).encode(w)),
        ),
        (
            "AMFName",
            encode(&|w| AmfName("amf.oxirush.test".into()).encode(w)),
        ),
        ("GUAMI", encode(&|w| guami.encode(w))),
        ("UserLocationInformationNR", encode(&|w| location.encode(w))),
    ]
}

/// The call of an operation of a codec on a value: the octets are read, or the value
/// that they hold is written.
fn call(codec: &str, operation: &str, name: &str, octets: &'static [u8]) -> Box<dyn FnMut()> {
    macro_rules! of {
        ($ours:ty, $theirs:ty) => {{
            let ours: $ours = oxirush_aper::decode(octets).unwrap();
            let theirs: $theirs = rasn::aper::decode(octets).unwrap();
            // Both read the octets, and write them again alike.
            assert_eq!(oxirush_aper::encode(&ours).unwrap(), octets, "{name}");
            assert_eq!(rasn::aper::encode(&theirs).unwrap(), octets, "{name}");
            match (codec, operation) {
                ("aper", "decode") => Box::new(move || {
                    black_box(oxirush_aper::decode::<$ours>(black_box(octets)).unwrap());
                }) as Box<dyn FnMut()>,
                ("aper", _) => Box::new(move || {
                    black_box(oxirush_aper::encode(black_box(&ours)).unwrap());
                }),
                ("rasn", "decode") => Box::new(move || {
                    black_box(rasn::aper::decode::<$theirs>(black_box(octets)).unwrap());
                }),
                _ => Box::new(move || {
                    black_box(rasn::aper::encode(black_box(&theirs)).unwrap());
                }),
            }
        }};
    }
    match name {
        "AMF-UE-NGAP-ID" => of!(ours::AmfUeNgapId, theirs::AmfUeNgapId),
        "NAS-PDU" => of!(ours::NasPdu<'static>, theirs::NasPdu),
        "AMFName" => of!(ours::AmfName<'static>, theirs::AmfName),
        "GUAMI" => of!(ours::Guami, theirs::Guami),
        _ => of!(ours::Location, theirs::Location),
    }
}

fn main() {
    let arguments: Vec<String> = std::env::args().collect();
    if arguments.get(1).map(String::as_str) == Some("list") {
        for (name, octets) in values() {
            println!("{name}\t{}", hex(&octets));
        }
        return;
    }
    let [_, codec, operation, name, iterations] = &arguments[..] else {
        panic!("compare CODEC OPERATION VALUE ITERATIONS, or compare list");
    };
    let iterations: u32 = iterations.parse().unwrap();
    let octets = values()
        .into_iter()
        .find(|(known, _)| known == name)
        .unwrap()
        .1;
    let mut call = call(codec, operation, name, Box::leak(octets.into_boxed_slice()));
    for _ in 0..iterations {
        call();
    }
    let before = (ALLOCATIONS.load(Relaxed), OCTETS.load(Relaxed));
    call();
    let (count, bytes) = (
        ALLOCATIONS.load(Relaxed) - before.0,
        OCTETS.load(Relaxed) - before.1,
    );
    let mut fastest = f64::MAX;
    for _ in 0..3 {
        let started = Instant::now();
        for _ in 0..iterations {
            call();
        }
        fastest = fastest.min(started.elapsed().as_secs_f64() * 1e9 / f64::from(iterations));
    }
    println!(r#"{{"ns": {fastest:.1}, "allocations": {count}, "bytes": {bytes}}}"#);
}

fn hex(octets: &[u8]) -> String {
    octets.iter().map(|octet| format!("{octet:02x}")).collect()
}
