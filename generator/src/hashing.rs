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

//! The perfect hash of the names of a table, written beside it: the displacement of
//! each bucket and the entry of each slot, which `oxirush_aper::inspect::Hashed` reads
//! with the functions of the same file.

use std::cmp::Reverse;
use std::fmt::Write;

use anyhow::{Result, ensure};

// The functions that place a name, which the lookup of the crate shares.
#[allow(dead_code)]
#[path = "../../src/inspect/hashed.rs"]
mod hashed;

/// A static of `Hashed` named `name`, after the documentation `doc`, that finds each of
/// `keys`, which it holds: the slot of a key holds its index. Of two keys that are the
/// same, the first is found.
pub(crate) fn write(out: &mut String, doc: &str, name: &str, keys: &[String]) -> Result<()> {
    ensure!(
        keys.len() < usize::from(hashed::EMPTY),
        "more names than a slot holds"
    );
    let mut first = Vec::new();
    for (at, key) in keys.iter().enumerate() {
        if !keys[..at].contains(key) {
            first.push(at);
        }
    }
    let hashes: Vec<u64> = first
        .iter()
        .map(|at| hashed::hash(keys[*at].as_bytes()))
        .collect();
    let (displacements, slots) = place(&hashes)?;
    let slots: Vec<u16> = slots
        .iter()
        .map(|slot| match slot {
            Some(key) => first[*key] as u16,
            None => hashed::EMPTY,
        })
        .collect();
    let list = |numbers: &[u16]| {
        let numbers: Vec<String> = numbers.iter().map(u16::to_string).collect();
        numbers.join(", ")
    };
    writeln!(out, "/// {doc}")?;
    writeln!(out, "#[rustfmt::skip]")?;
    writeln!(
        out,
        "pub(crate) static {name}: oxirush_aper::inspect::Hashed = oxirush_aper::inspect::Hashed {{\n    \
         keys: &{keys:?},\n    displacements: &[{}],\n    slots: &[{}],\n}};",
        list(&displacements),
        list(&slots)
    )?;
    Ok(())
}

/// The displacement of each bucket and the key in each slot, for keys of distinct
/// hashes: four keys for a bucket, and a fifth more slots than keys, or more where no
/// displacement places the keys of a bucket.
fn place(hashes: &[u64]) -> Result<(Vec<u16>, Vec<Option<usize>>)> {
    let keys = hashes.len();
    let buckets = keys.div_ceil(4).max(1);
    let mut members = vec![Vec::new(); buckets];
    for (key, hash) in hashes.iter().enumerate() {
        members[hashed::bucket(*hash, buckets)].push(key);
    }
    // The fullest buckets first, which have the most free slots to choose from.
    let mut order: Vec<usize> = (0..buckets).collect();
    order.sort_by_key(|bucket| Reverse(members[*bucket].len()));
    let mut size = (keys + keys / 5).max(1);
    'size: loop {
        ensure!(size < 4 * keys + 64, "no displacement places the names");
        let mut slots = vec![None; size];
        let mut displacements = vec![0; buckets];
        'bucket: for bucket in &order {
            let keys = &members[*bucket];
            for displacement in 0..=u16::MAX {
                let mut places: Vec<usize> = keys
                    .iter()
                    .map(|key| hashed::slot(hashes[*key], displacement, size))
                    .collect();
                if places.iter().any(|place| slots[*place].is_some()) {
                    continue;
                }
                places.sort_unstable();
                places.dedup();
                if places.len() < keys.len() {
                    continue;
                }
                for key in keys {
                    slots[hashed::slot(hashes[*key], displacement, size)] = Some(*key);
                }
                displacements[*bucket] = displacement;
                continue 'bucket;
            }
            size += size / 8 + 1;
            continue 'size;
        }
        return Ok((displacements, slots));
    }
}
