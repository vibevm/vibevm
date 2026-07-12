//! The designated unsafe audit crate for process-environment
//! mutation — the AUD-0016 posture, redesigned 2026-06-12.
//!
//! `std::env::set_var` / `remove_var` are `unsafe` in edition 2024:
//! on POSIX a concurrent `getenv` during a `setenv` is undefined
//! behaviour, so the compiler can no longer pretend mutation is
//! benign. The house posture (unsafe-gate rule, ENGINE-CONFORM §4):
//! all `unsafe` lives in a designated audit crate behind a safe API,
//! or carries fn-grain `#[spec(deviates, reason)]` testimony. This
//! crate is that audit home for env mutation. Every other mutation
//! site in the workspace is a conform finding by construction.
//!
//! Scope discipline: this is **test infrastructure** — consume it as
//! a dev-dependency only. Production startup promotion (vibe-cli's
//! `promote_user_config_env`) and FFI boundaries keep their unsafe in
//! place under recorded testimony instead; a safe mutate-anytime API
//! for production would advertise soundness this crate cannot prove.

specmark::scope!("spec://org.vibevm.ai-native.core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules");

use std::ffi::OsString;
use std::sync::{Mutex, MutexGuard, PoisonError};

/// Serializes every environment-mutating guard in the process. One
/// global lock — not per-variable — so two tests never interleave
/// mutations of different variables that one consumer reads together.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// A serialized, restoring environment guard — the safe API over the
/// audited `unsafe` env calls.
///
/// Acquiring the guard takes the process-wide env lock; dropping it
/// restores every touched variable to its pre-touch value (first
/// touch wins) and releases the lock. One guard handles any number of
/// variables, so a test never holds two guards (a second `lock()` on
/// the same thread would deadlock — the price of real serialization,
/// stated here so the failure mode is named).
///
/// ```
/// let mut env = rust_ai_native_env_audit::EnvGuard::lock();
/// env.set("ENV_AUDIT_DOCTEST", "on");
/// env.unset("ENV_AUDIT_DOCTEST_GONE");
/// assert_eq!(std::env::var("ENV_AUDIT_DOCTEST").as_deref(), Ok("on"));
/// drop(env);
/// assert!(std::env::var_os("ENV_AUDIT_DOCTEST").is_none());
/// ```
pub struct EnvGuard {
    /// Pre-touch values, captured once per key on first touch and
    /// restored in reverse order on drop.
    saved: Vec<(String, Option<OsString>)>,
    /// Held for the guard's lifetime so the restore writes in `drop`
    /// happen while mutation is still serialized.
    _lock: MutexGuard<'static, ()>,
}

impl EnvGuard {
    /// Acquire the process-wide env lock. No mutation happens yet.
    ///
    /// A poisoned lock is recovered, not propagated: a panicking test
    /// already restored its variables in `drop`, so the environment
    /// is consistent and the next test may proceed.
    pub fn lock() -> Self {
        EnvGuard {
            saved: Vec::new(),
            _lock: ENV_LOCK.lock().unwrap_or_else(PoisonError::into_inner),
        }
    }

    /// Set `key` to `value`, capturing the pre-touch value for
    /// restore-on-drop.
    pub fn set(&mut self, key: &str, value: &str) -> &mut Self {
        self.touch(key);
        // SAFETY: mutation is serialized process-wide by ENV_LOCK
        // (held by self), and every mutation site in this workspace
        // goes through this guard — the unsafe-gate conform rule
        // turns any other site into a finding. Concurrent readers of
        // vibevm-private variable names hold the same guard by house
        // pattern; child processes spawned by tests receive a copied
        // environment and cannot race this one.
        unsafe { std::env::set_var(key, value) };
        self
    }

    /// Remove `key`, capturing the pre-touch value for
    /// restore-on-drop.
    pub fn unset(&mut self, key: &str) -> &mut Self {
        self.touch(key);
        // SAFETY: same serialization argument as `set`.
        unsafe { std::env::remove_var(key) };
        self
    }

    /// Capture `key`'s current value the first time it is touched.
    fn touch(&mut self, key: &str) {
        if !self.saved.iter().any(|(k, _)| k == key) {
            self.saved.push((key.to_string(), std::env::var_os(key)));
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        // Runs before `_lock` is released — restore is serialized.
        for (key, prev) in self.saved.drain(..).rev() {
            match prev {
                // SAFETY: same serialization argument as `set`.
                Some(value) => unsafe { std::env::set_var(&key, &value) },
                // SAFETY: same serialization argument as `set`.
                None => unsafe { std::env::remove_var(&key) },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_then_drop_restores_absent_variable() {
        let key = "ENV_AUDIT_TEST_ABSENT";
        {
            let mut env = EnvGuard::lock();
            env.set(key, "temp");
            assert_eq!(std::env::var(key).as_deref(), Ok("temp"));
        }
        assert!(std::env::var_os(key).is_none());
    }

    #[test]
    fn set_then_drop_restores_previous_value() {
        let key = "ENV_AUDIT_TEST_PREV";
        let mut outer = EnvGuard::lock();
        outer.set(key, "original");
        {
            // No inner guard — it would deadlock on the global lock.
            // The outer guard keeps mutating; first-touch capture
            // means the restore still lands on the pre-guard state.
            outer.set(key, "shadow");
            assert_eq!(std::env::var(key).as_deref(), Ok("shadow"));
        }
        drop(outer);
        assert!(std::env::var_os(key).is_none());
    }

    #[test]
    fn unset_then_drop_restores_value_set_before_first_touch() {
        let key = "ENV_AUDIT_TEST_UNSET";
        let mut env = EnvGuard::lock();
        env.set(key, "kept");
        env.unset(key);
        assert!(std::env::var_os(key).is_none());
        env.set(key, "again");
        drop(env);
        // First touch captured the pre-guard state: absent.
        assert!(std::env::var_os(key).is_none());
    }

    #[test]
    fn one_guard_carries_many_variables() {
        let (a, b) = ("ENV_AUDIT_TEST_A", "ENV_AUDIT_TEST_B");
        {
            let mut env = EnvGuard::lock();
            env.set(a, "1").set(b, "2");
            assert_eq!(std::env::var(a).as_deref(), Ok("1"));
            assert_eq!(std::env::var(b).as_deref(), Ok("2"));
        }
        assert!(std::env::var_os(a).is_none());
        assert!(std::env::var_os(b).is_none());
    }

    #[test]
    fn guards_serialize_across_threads() {
        // Two threads contend for the same variable; the lock means
        // each observes only its own value while its guard lives.
        let key = "ENV_AUDIT_TEST_RACE";
        let writer = |val: &'static str| {
            move || {
                let mut env = EnvGuard::lock();
                env.set(key, val);
                for _ in 0..50 {
                    assert_eq!(std::env::var(key).as_deref(), Ok(val));
                    std::thread::yield_now();
                }
            }
        };
        let t1 = std::thread::spawn(writer("one"));
        let t2 = std::thread::spawn(writer("two"));
        t1.join().expect("thread one");
        t2.join().expect("thread two");
        assert!(std::env::var_os(key).is_none());
    }
}
