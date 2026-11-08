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

//! The walks of the IEs of a message and of the items of a list end, and give each IE
//! and each item once. They are a test of their own, each on a thread of its own: the
//! readers of `tests/readers.rs` loop for good on a walk that does not move, and a walk
//! that never ends fails its test here instead of holding the run.

use oxirush_toy::toy::*;
use oxirush_toy::{Ie, build_toy, ie};
use std::sync::mpsc::{RecvTimeoutError, channel};
use std::time::Duration;

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
        let ran = |id: u32| ie::UE_RAN_ID::field(Criticality::Reject, id).unwrap();
        let core = ie::UE_Core_ID::field(Criticality::Reject, 42u64).unwrap();
        let message = ReleaseRequest {
            protocol_ies: ProtocolIEContainer(vec![core, ran(1), ran(2)]),
        };
        let every: Vec<_> = message.ies::<ie::UE_RAN_ID>().take(4).collect();
        assert_eq!(every, [Ok(UERANID(1)), Ok(UERANID(2))]);
    });
}

/// The items of a list, each once.
#[test]
fn the_items_of_a_list_are_walked_once_each() {
    bounded(|| {
        let item = |id: u8| Item {
            item_id: id,
            cause: None,
            ie_extensions: None,
        };
        let first = ie::Item::field(Criticality::Reject, item(1)).unwrap();
        let second = ie::Item::field(Criticality::Reject, item(2)).unwrap();
        let area = AreaItem {
            tac: TAC([0, 0, 1]),
            broadcast_plmns: BroadcastPLMNs(vec![PLMNIdentity([0x02, 0xf8, 0x39])]),
            ie_extensions: None,
        };
        let items = Items(vec![first, second]);
        let pdu = build_toy!(SetupRequest, Areas(Areas(vec![area])), Items(items)).unwrap();
        let read: Vec<_> = pdu.items::<ie::Items>().take(4).collect();
        assert_eq!(read, [Ok(item(1)), Ok(item(2))]);
    });
}
