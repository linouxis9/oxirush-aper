//! The output that the README shows under its first sample is what the crate says.

use oxirush_aper::{AsnType, Decode, Error, Octets, Reader};

#[derive(Debug)]
struct Sample<'a> {
    _id: u16,
    _payload: Option<Octets<'a>>,
}

impl AsnType for Sample<'_> {
    const NAME: &'static str = "Sample";
}

impl<'a> Decode<'a> for Sample<'a> {
    fn decode(reader: &mut Reader<'a>) -> Result<Self, Error> {
        let extended = reader.bit()?;
        let has_payload = reader.bit()?;
        let _id = reader.unsigned(0, 65535)? as u16;
        let _payload = match has_payload {
            true => Some(reader.octet_string(0, oxirush_aper::UNBOUNDED)?),
            false => None,
        };
        if extended {
            reader.skip_additions()?;
        }
        Ok(Self { _id, _payload })
    }
}

#[test]
fn the_first_sample_prints_what_the_readme_shows() {
    let readme = include_str!("../README.md");
    let (_, after) = readme.split_once("```text\n").expect("an output block");
    let (shown, _) = after.split_once("\n```").expect("the end of the block");

    let octets = [0x40, 0x12, 0x34, 0x02, 0xca, 0xfe];
    let short = oxirush_aper::decode::<Sample>(&octets[..5]).unwrap_err();
    assert_eq!(short.to_string(), shown);
}
