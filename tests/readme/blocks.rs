// The Rust blocks of the README and of the documentation, each as a function.
// `tests/readme.rs` writes this file and says how: do not edit it.
#![allow(unused)]

use std::cell::RefCell;

// The toy protocol, for the blocks that include it.
use super::{ie, toy};

thread_local!(static PRINTED: RefCell<String> = const { RefCell::new(String::new()) });

// What a block prints is kept, to be compared with what its page shows.
macro_rules! print {
    ($($argument:tt)*) => {{
        let text = format!($($argument)*);
        PRINTED.with(|printed| printed.borrow_mut().push_str(&text))
    }};
}

macro_rules! println {
    () => {
        print!("\n")
    };
    ($($argument:tt)*) => {{
        let line = format!($($argument)*);
        print!("{line}\n")
    }};
}

/// What `block` prints.
pub fn printed(block: fn()) -> String {
    PRINTED.with(|printed| printed.borrow_mut().clear());
    block();
    PRINTED.with(|printed| printed.take())
}

pub const BLOCKS: &[fn()] = &[
    block_1,
    block_2,
    block_3,
    block_4,
    block_5,
    block_6,
    block_7,
    block_8,
    block_9,
    block_10,
];

/// The Rust block 1 of README.md.
fn block_1() {
    use oxirush_aper::{Reader, Writer};

    fn main() -> Result<(), oxirush_aper::Error> {
        let mut octets = Vec::new();
        let mut writer = Writer::new(&mut octets);
        // INTEGER (0..7): three bits
        writer.unsigned(5, 0, 7)?;
        // INTEGER (0..255, ...): the extension bit, then one octet, aligned
        writer.unsigned_extensible(200, 0, 255)?;
        // ENUMERATED with 6 root values and an extension marker: the fourth
        writer.index(3, 6, true);
        // SIZE (1..256) of a SEQUENCE OF: its count, then its items
        writer.size(2, 1, 256)?;
        writer.unsigned(1, 0, 1)?;
        writer.unsigned(0, 0, 1)?;
        println!("{octets:02x?}");

        let mut reader = Reader::new(&octets);
        println!("{}", reader.unsigned(0, 7)?);
        println!("{}", reader.unsigned_extensible(0, 255)?);
        println!("{}", reader.enumerated(6, 0, true)?);
        println!("{} items: {} {}", reader.size(1, 256)?, reader.bit()?, reader.bit()?);

        println!("{}", Writer::new(&mut Vec::new()).unsigned(8, 0, 7).unwrap_err());
        Ok(())
    }

    main().unwrap();
}

/// The Rust block 2 of README.md.
fn block_2() {
    use oxirush_aper::{AsnType, Decode, Encode, Error, Reader, Writer};
    use std::borrow::Cow;

    // Cell ::= SEQUENCE {
    //     id          INTEGER (0..4095),
    //     kind        ENUMERATED { macro, small, ... },
    //     name        PrintableString (SIZE (1..150)) OPTIONAL,
    //     neighbours  SEQUENCE (SIZE (0..8)) OF INTEGER (0..4095),
    //     ...
    // }
    #[derive(Debug, PartialEq)]
    enum Kind {
        Macro,
        Small,
    }

    #[derive(Debug, PartialEq)]
    struct Cell<'a> {
        id: u16,
        kind: Kind,
        name: Option<Cow<'a, str>>,
        neighbours: Vec<u16>,
    }

    impl AsnType for Cell<'_> {
        const NAME: &'static str = "Cell";
    }

    impl<'a> Decode<'a> for Cell<'a> {
        fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
            // The extension bit, then a bit for each member that may be absent
            let extended = reader.bit()?;
            let has_name = reader.bit()?;
            let id = reader.unsigned(0, 4095)? as u16;
            // An index among two values of the root, and none added that is known
            let kind = match reader.enumerated(2, 0, true)? {
                0 => Kind::Macro,
                _ => Kind::Small,
            };
            let name = match has_name {
                true => Some(reader.printable_string(1, 150)?),
                false => None,
            };
            let count = reader.size(0, 8)?;
            let neighbours = (0..count)
                .map(|_| reader.unsigned(0, 4095).map(|id| id as u16))
                .collect::<Result<_, _>>()?;
            // What a later version adds is skipped
            if extended {
                reader.skip_additions()?;
            }
            Ok(Self { id, kind, name, neighbours })
        }
    }

    impl Encode for Cell<'_> {
        fn encode(&self, writer: &mut Writer<'_>) -> Result<(), Error> {
            writer.bit(false);
            writer.bit(self.name.is_some());
            writer.unsigned(self.id.into(), 0, 4095)?;
            writer.index(match self.kind { Kind::Macro => 0, Kind::Small => 1 }, 2, true);
            if let Some(name) = &self.name {
                writer.printable_string(name, 1, 150)?;
            }
            writer.size(self.neighbours.len(), 0, 8)?;
            for neighbour in &self.neighbours {
                writer.unsigned((*neighbour).into(), 0, 4095)?;
            }
            Ok(())
        }
    }

    fn main() -> Result<(), Error> {
        let cell = Cell {
            id: 0x123,
            kind: Kind::Small,
            name: Some("cell-1".into()),
            neighbours: vec![0x124, 0x125],
        };
        let octets = oxirush_aper::encode(&cell)?;
        println!("{octets:02x?}");

        // The name of what is decoded is a slice of the octets
        let read: Cell = oxirush_aper::decode(&octets)?;
        println!("{read:?}");
        println!("borrowed: {}", matches!(read.name, Some(Cow::Borrowed(_))));

        // What its constraints do not hold is an error of the writer and of the reader
        let crowded = Cell { neighbours: vec![0; 9], ..read };
        println!("{}", oxirush_aper::encode(&crowded).unwrap_err());
        println!("{}", oxirush_aper::decode::<Cell>(&octets[..5]).unwrap_err());
        Ok(())
    }

    main().unwrap();
}

/// The Rust block 3 of README.md.
fn block_3() {
    use oxirush_aper::Octets;

    fn main() {
        let received = vec![0x7e, 0x00, 0x41];
        let borrowed = Octets::from(&received[..]);
        println!("{borrowed:?} borrowed: {}", borrowed.is_borrowed());

        let kept: Octets<'static> = borrowed.into_owned();
        drop(received);
        println!("{kept:?} borrowed: {}", kept.is_borrowed());
    }

    main();
}

/// The Rust block 4 of README.md.
fn block_4() {
    use oxirush_aper::{BitString, Bits};

    fn main() -> Result<(), oxirush_aper::Error> {
        // AMF Set ID ::= BIT STRING (SIZE (10)): its octets start with its first bit
        let set = Bits::new(0x155, 10)?;
        println!("{:#x} of {} bits: {:02x?}", set.value(), set.len(), set.to_vec());
        println!("{}", Bits::from_octets(&[0x55, 0x40], 10)? == set);

        // A value that its length does not hold
        println!("{}", Bits::new(0x400, 10).unwrap_err());

        // TransportLayerAddress ::= BIT STRING (SIZE (1..160, ...))
        let address = BitString::from([10, 0, 0, 1]);
        println!("{:?} of {} bits", address.as_octets(), address.len());
        Ok(())
    }

    main().unwrap();
}

/// The Rust block 5 of README.md.
fn block_5() {
    use oxirush_aper::{Reader, Writer};

    fn main() -> Result<(), oxirush_aper::Error> {
        let mut octets = Vec::new();
        let mut writer = Writer::new(&mut octets);
        let start = writer.begin_open_type();
        writer.unsigned(0x1234, 0, 65535)?;
        writer.end_open_type(start);
        println!("{octets:02x?}");

        let mut reader = Reader::new(&octets);
        let held = reader.open_type()?;
        println!("{held:?}: {:#x}", Reader::new(&held).unsigned(0, 65535)?);
        Ok(())
    }

    main().unwrap();
}

/// The Rust block 6 of README.md.
fn block_6() {
    use oxirush_aper::{Reader, Writer};

    fn main() -> Result<(), oxirush_aper::Error> {
        // SEQUENCE { a BOOLEAN, ..., b INTEGER (0..255) OPTIONAL, c BOOLEAN OPTIONAL }
        let mut octets = Vec::new();
        let mut writer = Writer::new(&mut octets);
        writer.bit(true); // additions follow
        writer.bit(true); // a
        writer.additions(&[true, false])?; // b is there, c is not
        let start = writer.begin_open_type();
        writer.unsigned(7, 0, 255)?;
        writer.end_open_type(start);

        let mut reader = Reader::new(&octets);
        let extended = reader.bit()?;
        let a = reader.bit()?;
        let mut additions = reader.additions()?;
        let b = match additions.present() {
            true => Some(Reader::new(&reader.open_type()?).unsigned(0, 255)?),
            false => None,
        };
        // What a later version of the type added is skipped
        additions.skip_rest(&mut reader)?;
        println!("{extended} {a} {b:?}");
        Ok(())
    }

    main().unwrap();
}

/// The Rust block 7 of README.md.
fn block_7() {
    use oxirush_aper::{Octets, decode, decode_strict, decode_with_faults};

    fn main() -> Result<(), oxirush_aper::Error> {
        // An OCTET STRING of one octet, whose length is in two octets
        let octets = [0x80, 0x01, 0xaa];
        println!("{:?}", decode::<Octets>(&octets)?);

        // What a tool that tests the peer wants to know
        let (_, faults) = decode_with_faults::<Octets>(&octets)?;
        println!("{}", faults[0]);

        // And a receiver that takes the one encoding alone
        println!("{}", decode_strict::<Octets>(&octets).unwrap_err());
        Ok(())
    }

    main().unwrap();
}

/// The Rust block 8 of README.md.
fn block_8() {
    // The toy protocol: a SETUP REQUEST of an Identity, a Name and a list of Items
    use toy::*;

    fn main() -> Result<(), oxirush_aper::Error> {
        // A PDU by the names of its IEs, each with the criticality that the object
        // set of the message assigns it
        let pdu = build_toy!(InitiatingMessage, Setup, SetupRequest,
            Identity(7u32),
            Name("gNB-1"),
        )?;
        let octets = pdu.encode()?;
        println!("{}", octets.iter().map(|octet| format!("{octet:02x}")).collect::<String>());

        // Its IEs, read in one pass over the octets of its message, without
        // `decode_value`: each that a field asks for is decoded, and no other
        let pdu = TOYPDU::decode(&octets)?;
        extract_toy_ies!(pdu, SetupRequest,
            req identity: u32 = Identity(id),
            opt name: Vec<u8> = Name(name) => name.0.to_vec(),
        );
        println!("{identity} {:?}", name.as_deref().map(String::from_utf8_lossy));

        // One IE by its marker type, which has its identifier
        println!("{:?}, IE {}", pdu.ie::<ie::Identity>(), <ie::Identity as oxirush_aper::protocol::Ie>::ID);
        Ok(())
    }

    main().unwrap();
}

/// The Rust block 9 of README.md.
fn block_9() {
    use toy::*;

    fn main() -> Result<(), oxirush_aper::Error> {
        // A list of single containers: each item is an IE of its own
        let items = vec![build_toy_ie!(REJECT Item(1u8))?, build_toy_ie!(REJECT Item(2u8))?];
        let pdu = build_toy!(InitiatingMessage, Setup, SetupRequest,
            Identity(7u32),
            Items(items),
        )?;
        // Each item, where it is in the octets of the PDU
        for item in pdu.items::<ie::Items>() {
            println!("item {}", item?.0);
        }

        // An IE of a message, changed in place
        let mut request: SetupRequest = pdu.decode_value()?;
        with_toy_ie_mut!(request, SetupRequest, Identity(id) = 9)?;
        println!("{:?}", request.ie::<ie::Identity>());

        // A SETUP REQUEST of two Identities: `ies::<T>()` gives each, the macros
        // take the first, and its container is checked against the object set of
        // its message without decoding a value
        let twice = build_toy!(InitiatingMessage, Setup, SetupRequest,
            Identity(7u32),
            Identity(8u32),
        )?;
        let every: Vec<_> = twice.ies::<ie::Identity>().collect::<Result<_, _>>()?;
        println!("{every:?}");
        for deviation in twice.check_structure()? {
            println!("{deviation}");
        }
        Ok(())
    }

    main().unwrap();
}

/// The Rust block 1 of src/lib.rs.
fn block_10() {
    use oxirush_aper::{Reader, Writer};

    fn main() -> Result<(), Box<dyn std::error::Error>> {
        let mut octets = Vec::new();
        let mut writer = Writer::new(&mut octets);
        // INTEGER (0..7): three bits
        writer.unsigned(5, 0, 7)?;
        // INTEGER (0..255, ...): the extension bit, then one octet, aligned
        writer.unsigned_extensible(200, 0, 255)?;
        // ENUMERATED with 6 root values and an extension marker: the fourth
        writer.index(3, 6, true);
        // SIZE (1..256) of a SEQUENCE OF: its count, then its items
        writer.size(2, 1, 256)?;
        writer.unsigned(1, 0, 1)?;
        writer.unsigned(0, 0, 1)?;
        println!("{octets:02x?}");

        let mut reader = Reader::new(&octets);
        println!("{}", reader.unsigned(0, 7)?);
        println!("{}", reader.unsigned_extensible(0, 255)?);
        println!("{}", reader.enumerated(6, 0, true)?);
        println!("{} items: {} {}", reader.size(1, 256)?, reader.bit()?, reader.bit()?);

        println!("{}", Writer::new(&mut Vec::new()).unsigned(8, 0, 7).unwrap_err());
        Ok(())
    }

    main().unwrap();
}
