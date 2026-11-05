//! The calibration canister on PocketIC, the replica's local test instance: each call's reply carries the
//! instructions the replica counted between the canister's two reads of its counter.

use std::path::PathBuf;

use candid::Principal;
use pocket_ic::{PocketIc, PocketIcBuilder};

use crate::meter::args;

pub struct Replica {
    pic: PocketIc,
    canister: Principal,
}

impl Replica {
    /// A PocketIC instance from `server` (its binary) with one application subnet, and the canister on it.
    pub fn new(server: PathBuf, canister: &[u8]) -> Replica {
        let pic = PocketIcBuilder::new().with_server_binary(server).with_application_subnet().build();
        let c = pic.create_canister();
        pic.add_cycles(c, 1_000_000_000_000_000);
        pic.install_canister(c, canister.to_vec(), vec![], None);
        Replica { pic, canister: c }
    }

    /// Runs workload `sel`: the replica's instructions between the canister's two counter reads, and the
    /// workload's result.
    pub fn run(&self, sel: u32, words: [u64; 6]) -> Result<(u64, u64), String> {
        self.run_reply(sel, words).map(|(a, b, _)| (a, b))
    }

    /// `run`, with the reply's bytes.
    pub fn run_reply(&self, sel: u32, words: [u64; 6]) -> Result<(u64, u64, Vec<u8>), String> {
        let reply = self
            .pic
            .update_call(self.canister, Principal::anonymous(), "run", args(sel, words))
            .map_err(|e| format!("{e:?}"))?;
        let word = |i: usize| reply.get(i..i + 8).and_then(|s| s.try_into().ok()).map(u64::from_le_bytes);
        match (word(0), word(8)) {
            (Some(a), Some(b)) => Ok((a, b, reply)),
            _ => Err(format!("a short reply of {} bytes", reply.len())),
        }
    }
}
