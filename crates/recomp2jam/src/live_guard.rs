//! Serializes the live-gateway tests behind one rig account.
//!
//! The local Memoryer rig hands out one usable session per account, so
//! two concurrent walks kick each other off (`AlreadyConnected`): every
//! guarded live test holds this lock for its whole walk, and skips
//! still return fast (the guard drops on the early return).

use std::sync::{Mutex, MutexGuard};

/// Hold the rig for one live walk; recovers from a poisoned lock so
/// one failed walk never reds the rest of the suite.
pub fn hold_live_gateway() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}
