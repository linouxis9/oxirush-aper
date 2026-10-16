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

//! What the generators of oxirush-ngap and oxirush-s1ap share: the bindings of a
//! protocol on `oxirush-aper`, from its ASN.1.
//!
//! rasn-compiler parses the modules of a specification and resolves their references,
//! their parameters and their constraints, and writes what it resolved as bindings
//! for rasn. That text is the description of the protocol here:
//!
//! - [`containers`] gives the protocol IE and extension containers one type each;
//! - [`bindings`] writes each type with its codec on `oxirush-aper`, its conversions
//!   and its JSON form, and names each field by the rule of [`names`];
//! - [`rasn_fix`] corrects the bindings for rasn, which are the oracle of the tests:
//!   the same types with the codec of another implementation;
//! - [`registry`] reads the procedures and the IEs from the ASN.1, and writes the list
//!   that the macros of a protocol crate expand;
//! - [`inspection`] writes what the `inspect` feature needs beside it;
//! - [`asn1`] reads the modules, each checked against the digest that is recorded
//!   for it.
//!
//! The generator of a protocol, in the `build/` of its crate, runs rasn-compiler, calls
//! these in that order and adds what has the names of the protocol.

pub mod asn1;
pub mod bindings;
pub mod containers;
pub mod inspection;
pub mod names;
pub mod rasn_fix;
pub mod registry;
