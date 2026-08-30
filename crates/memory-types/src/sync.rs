//! Lock-poisoning recovery.
//!
//! Workspace policy (v3.1 / Phase 54-06): never panic the daemon on a poisoned
//! `std::sync` mutex/rwlock. Recover the inner guard and count the event so it
//! is observable.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::LockResult;

/// Number of times a poisoned lock was recovered process-wide.
pub static LOCK_POISON_RECOVERIES: AtomicU64 = AtomicU64::new(0);

/// Recover from a poisoned `Mutex`/`RwLock` by taking the inner guard.
///
/// Poisoning means a previous holder panicked. The data may be inconsistent,
/// but aborting the daemon is worse than continuing with a metric bump.
#[inline]
pub fn recover_lock<T>(result: LockResult<T>) -> T {
    result.unwrap_or_else(|poisoned| {
        LOCK_POISON_RECOVERIES.fetch_add(1, Ordering::Relaxed);
        poisoned.into_inner()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::thread;

    #[test]
    fn recover_lock_returns_guard_on_success() {
        let m = Mutex::new(7);
        let g = recover_lock(m.lock());
        assert_eq!(*g, 7);
    }

    #[test]
    fn recover_lock_recovers_from_poison() {
        let m = Arc::new(Mutex::new(0));
        let m2 = Arc::clone(&m);
        let _ = thread::spawn(move || {
            let _g = m2.lock().unwrap();
            panic!("poison the mutex");
        })
        .join();

        let before = LOCK_POISON_RECOVERIES.load(Ordering::Relaxed);
        let mut g = recover_lock(m.lock());
        *g = 42;
        drop(g);
        assert!(LOCK_POISON_RECOVERIES.load(Ordering::Relaxed) > before);
        assert_eq!(*recover_lock(m.lock()), 42);
    }
}
