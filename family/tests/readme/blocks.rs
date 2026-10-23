// The blocks of the README that use other crates of the family, each as a
// function. `tests/readme.rs` writes this file and says how: do not edit it.
#![allow(unused)]

use std::cell::RefCell;

thread_local!(static PRINTED: RefCell<String> = const { RefCell::new(String::new()) });

// What a block prints is kept, to be compared with what the README shows.
macro_rules! println {
    () => {
        PRINTED.with(|printed| printed.borrow_mut().push('\n'))
    };
    // The line is made first, where the block is: its arguments may have a
    // `?` or an `await`.
    ($($argument:tt)*) => {{
        let line = format!($($argument)*);
        PRINTED.with(|printed| {
            let mut printed = printed.borrow_mut();
            printed.push_str(&line);
            printed.push('\n');
        })
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
];

/// The block 1 of the README that uses other crates.
fn block_1() {
    use oxirush_ngap::aper::{Reader, Writer};
    use oxirush_ngap::ngap::NGAP_PDU;

    fn main() -> Result<(), Box<dyn std::error::Error>> {
        // A UE CONTEXT RELEASE REQUEST, and the octets of the value of each of its IEs
        let octets = hex::decode("002a4015000003000a0002002a005500020007000f40020500")?;
        let pdu = NGAP_PDU::decode(&octets)?;
        let ies = pdu.protocol_ies()?;
        for ie in &ies.0 {
            let mut reader = Reader::new(&ie.value);
            match ie.id.0 {
                // AMF-UE-NGAP-ID ::= INTEGER (0..1099511627775)
                10 => println!("AMF-UE-NGAP-ID {}", reader.unsigned(0, 1_099_511_627_775)?),
                // RAN-UE-NGAP-ID ::= INTEGER (0..4294967295)
                85 => println!("RAN-UE-NGAP-ID {}", reader.unsigned(0, 4_294_967_295)?),
                id => println!("IE {id}: {:02x?}", &ie.value[..]),
            }
        }

        // A Writer writes the value that the codec of the IE writes
        let mut written = Vec::new();
        Writer::new(&mut written).unsigned(42, 0, 1_099_511_627_775)?;
        println!("{}", written == *ies.0[0].value);
        Ok(())
    }
    main().unwrap();
}
