pub mod lamports_bakery;
pub mod peterson;

/// Starvation Free Mutex allows for realtime or bounded wait.
///
/// The requirements for that are
/// 1. Mutual Exclusion - spinlocks on shared variables in the mutex to guarantee only one enters the critical section.
/// 2. No Starvation - assuming OS threads eventually runs, mutexN can never cause mutexM (N!=M) to fail to `wait()`
/// after `want_lock()`.
pub trait NoStarveMutex<'a, Guard: 'a, Want: 'a>
where
    // Only allow releasing after acquiring guard
    Guard: Drop,
    Want: WantGuard<'a, Guard>,
{
    // &mut guarantees no double acquire within the same scope, at compile time
    fn lock(&'a mut self) -> Guard {
        self.want_lock().wait()
    }
    fn want_lock(&'a mut self) -> Want;
}

pub trait WantGuard<'a, Guard: 'a>: Drop
where
    Guard: Drop,
{
    // WARNING: because Want drop()s after moving into Guard, implementation must book-keep to tell drop() not to release again
    fn wait(self) -> Guard;
}
