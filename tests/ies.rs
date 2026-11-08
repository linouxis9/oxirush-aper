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

//! The walks of the IEs of a message and of the items of a list, on the protocol of
//! `toy/mod.rs`, end, and give each IE and each item once. They run before
//! `tests/protocol.rs`, whose readers loop for good on a walk that does not move, and each
//! on a thread of its own: a walk that never ends fails its test instead of holding the
//! run.

include!("toy/mod.rs");

use std::sync::mpsc::{RecvTimeoutError, channel};
use std::time::Duration;
use toy::*;

/// Runs `test` on a thread of its own, for 5 s at most.
fn bounded(test: impl FnOnce() + Send + 'static) {
    let (done, finished) = channel();
    let thread = std::thread::spawn(move || {
        test();
        let _ = done.send(());
    });
    if let Err(RecvTimeoutError::Timeout) = finished.recv_timeout(Duration::from_secs(5)) {
        panic!("the walk still goes after 5 s");
    }
    if let Err(panic) = thread.join() {
        std::panic::resume_unwind(panic);
    }
}

/// The occurrences of an IE in a message, past an IE of another identifier, each once.
#[test]
fn the_ies_of_a_message_are_walked_once_each() {
    bounded(|| {
        let identity = |id: u32| build_toy_ie!(SetupRequest, REJECT Identity(id)).unwrap();
        let name = build_toy_ie!(SetupRequest, Name("toy")).unwrap();
        let message = SetupRequest::new(ProtocolIEContainer(vec![name, identity(1), identity(2)]));
        let every: Vec<_> = message.ies::<ie::Identity>().take(4).collect();
        assert_eq!(every, [Ok(Identity(1)), Ok(Identity(2))]);
    });
}

/// The items of a list, each once.
#[test]
fn the_items_of_a_list_are_walked_once_each() {
    bounded(|| {
        let first = build_toy_ie!(REJECT Item(1u8)).unwrap();
        let second = build_toy_ie!(REJECT Item(2u8)).unwrap();
        let pdu = build_toy!(
            InitiatingMessage,
            Setup,
            SetupRequest,
            Identity(7u32),
            Items(vec![first, second]),
        )
        .unwrap();
        let items: Vec<_> = pdu.items::<ie::Items>().take(4).collect();
        assert_eq!(items, [Ok(Item(1)), Ok(Item(2))]);
    });
}
