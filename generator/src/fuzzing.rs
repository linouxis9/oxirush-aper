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

//! The dictionaries of the fuzz targets of a protocol, written beside its bindings from
//! its procedures, its IEs and its table of the types: `fuzz/octets.dict` for the
//! targets that read the octets of a PDU, `fuzz/names.dict` for those that write trees
//! and paths.

use std::collections::BTreeSet;
use std::fmt::Write;

use anyhow::Result;

use crate::registry::Registry;

/// The octet of a criticality in aligned PER, which writes it in the two high bits.
fn criticality(name: &str) -> u8 {
    match name {
        "reject" => 0x00,
        "notify" => 0x80,
        _ => 0x40,
    }
}

/// The header of each message as aligned PER writes it, under the name of the message:
/// the alternative of the PDU, the procedure code and the criticality of the procedure;
/// then each IE of the message, its identifier and its criticality. Each value once,
/// under the first name that has it.
pub(crate) fn octets(registry: &Registry) -> Result<String> {
    let mut out = String::new();
    let mut seen = BTreeSet::new();
    let mut entry = |out: &mut String, name: &str, octets: [u8; 3]| -> Result<()> {
        if seen.insert(octets) {
            let [a, b, c] = octets;
            writeln!(out, "{name}=\"\\x{a:02x}\\x{b:02x}\\x{c:02x}\"")?;
        }
        Ok(())
    };
    for (code, procedure) in &registry.procedures {
        // The class of an elementary procedure has `ignore` by default.
        let of_procedure = criticality(procedure.criticality.as_deref().unwrap_or("ignore"));
        for (alternative, message) in [0x00, 0x20, 0x40].into_iter().zip(&procedure.messages) {
            let Some(message) = message else {
                continue;
            };
            entry(&mut out, &message.name, [alternative, *code, of_procedure])?;
            for (id, of_ie, _) in message.ies.iter().flatten() {
                let [high, low] = id.to_be_bytes();
                let name = format!("{}_{id}", message.name);
                entry(&mut out, &name, [high, low, criticality(of_ie)])?;
            }
        }
    }
    Ok(out)
}

/// The names that a tree and a path write: of the messages, of the IEs, and `names`, of
/// the members, the alternatives and the values of the types. Each once, in the order
/// of the names.
pub(crate) fn names(registry: &Registry, names: &BTreeSet<String>) -> Result<String> {
    let messages = registry.procedures.values().flat_map(|procedure| {
        let messages = procedure.messages.iter().flatten();
        messages.map(|message| message.name.clone())
    });
    let ies = registry.ies.values().map(|ie| ie.name.clone());
    let all: BTreeSet<String> = messages.chain(ies).chain(names.iter().cloned()).collect();
    let mut out = String::new();
    for name in &all {
        writeln!(out, "{name:?}")?;
    }
    Ok(out)
}
