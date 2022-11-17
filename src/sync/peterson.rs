use super::Mutex;
use std::sync::atomic::Ordering;

/// Binary mutex to protect critical section fairly.
///
/// 1. Mutual Exclusion - spinlocks on shared variables in the mutex to guarantee either PetersonA or PetersonB enters.
/// It is UB for multiple `acquire()` on the same mutex.
/// 2. No Starvation - assuming OS threads eventually runs, PetersonA can never cause PetersonB to fail to `acquire()`
/// regardless of `acquire()` order.
///
/// # Examples
/// ```
/// use crate::algo::sync::peterson::{Peterson, PetersonA, PetersonB};
/// use crate::algo::sync::Mutex;
/// use std::sync::atomic::Ordering;
///
/// let mu = std::sync::Arc::new(Peterson::default());
/// let mut val = std::sync::Arc::new(std::sync::atomic::AtomicI32::new(0));
/// let th_a = std::thread::spawn({
///     let mut mu_a = PetersonA::new(&mu);
///     let val_a = val.clone();
///     move || {
///         let _guard_a = mu_a.acquire();
///         let i = val_a.load(Ordering::Relaxed);
///         val_a.store(i + 1, Ordering::Relaxed);
///     }
/// });
/// let th_b = std::thread::spawn({
///     let mut mu_b = PetersonB::new(&mu);
///     let val_b = val.clone();
///     move || {
///         let _guard_b = mu_b.acquire();
///         let i = val_b.load(Ordering::Relaxed);
///         val_b.store(i + 2, Ordering::Relaxed);
///     }
/// });
/// th_a.join().unwrap();
/// th_b.join().unwrap();
/// assert_eq!(val.load(Ordering::Relaxed), 3);
/// ```
#[derive(Default)]
pub struct Peterson {
    a_wants: std::sync::atomic::AtomicBool,
    b_wants: std::sync::atomic::AtomicBool,
    a_turn: std::sync::atomic::AtomicBool,
}
pub struct PetersonA(std::sync::Arc<Peterson>);
pub struct PetersonB(std::sync::Arc<Peterson>);
pub struct PetersonAGuard<'a>(&'a PetersonA);
pub struct PetersonBGuard<'a>(&'a PetersonB);

impl PetersonA {
    pub fn new(p: &std::sync::Arc<Peterson>) -> Self {
        Self(p.clone())
    }
}
impl PetersonB {
    pub fn new(p: &std::sync::Arc<Peterson>) -> Self {
        Self(p.clone())
    }
}
impl<'a> Mutex<'a, PetersonAGuard<'a>> for PetersonA {
    fn acquire(&'a mut self) -> PetersonAGuard<'a> {
        // Algorithm requires no reordering of variables, hence SeqCst
        self.0.a_wants.store(true, Ordering::SeqCst);
        self.0.a_turn.store(false, Ordering::SeqCst);
        while self.0.b_wants.load(Ordering::SeqCst) && !self.0.a_turn.load(Ordering::SeqCst) {
            std::hint::spin_loop()
        }
        PetersonAGuard(self)
    }
}
impl Drop for PetersonAGuard<'_> {
    fn drop(&mut self) {
        self.0 .0.a_wants.store(false, Ordering::SeqCst)
    }
}
impl<'a> Mutex<'a, PetersonBGuard<'a>> for PetersonB {
    fn acquire(&'a mut self) -> PetersonBGuard<'a> {
        self.0.b_wants.store(true, Ordering::SeqCst);
        self.0.a_turn.store(true, Ordering::SeqCst);
        while self.0.a_wants.load(Ordering::SeqCst) && self.0.a_turn.load(Ordering::SeqCst) {
            std::hint::spin_loop()
        }
        PetersonBGuard(self)
    }
}
impl Drop for PetersonBGuard<'_> {
    fn drop(&mut self) {
        self.0 .0.b_wants.store(false, Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use crate::sync::{
        peterson::{Peterson, PetersonA, PetersonB},
        Mutex,
    };
    use std::sync::atomic::Ordering;
    const WORK: i32 = 10_000_000 / 2;

    #[test]
    fn sequential_works() {
        let data = std::sync::Arc::new(TestData::default());
        for _ in 0..WORK {
            data.add_then_sub();
        }
        for _ in 0..WORK {
            data.sub_then_add();
        }
        assert_eq!(data.0.load(Ordering::Relaxed), WORK * 2);
        assert_eq!(data.1.load(Ordering::Relaxed), -WORK * 2);
    }

    #[test]
    fn race_conditions() {
        let data = std::sync::Arc::new(TestData::default());
        let th_a = std::thread::spawn({
            let data = data.clone();
            move || {
                for _ in 0..WORK {
                    data.add_then_sub();
                }
            }
        });
        let th_b = std::thread::spawn({
            let data = data.clone();
            move || {
                for _ in 0..WORK {
                    data.sub_then_add();
                }
            }
        });
        th_a.join().unwrap();
        th_b.join().unwrap();
        assert!(data.0.load(Ordering::Relaxed) > WORK);
        assert!(data.0.load(Ordering::Relaxed) < WORK * 2);
        assert!(data.1.load(Ordering::Relaxed) < -WORK);
        assert!(data.1.load(Ordering::Relaxed) > -WORK * 2);
    }

    #[test]
    fn mutual_exclusion() {
        let mu = std::sync::Arc::new(Peterson::default());
        let data = std::sync::Arc::new(TestData::default());
        let th_a = std::thread::spawn({
            let data = data.clone();
            let mut mu = PetersonA::new(&mu);
            move || {
                for _ in 0..WORK {
                    let _guard = mu.acquire();
                    data.add_then_sub();
                }
            }
        });
        let th_b = std::thread::spawn({
            let data = data.clone();
            let mut mu = PetersonB::new(&mu);
            move || {
                for _ in 0..WORK {
                    let _guard = mu.acquire();
                    data.sub_then_add();
                }
            }
        });
        th_a.join().unwrap();
        th_b.join().unwrap();
        assert_eq!(data.0.load(Ordering::Relaxed), WORK * 2);
        assert_eq!(data.1.load(Ordering::Relaxed), -WORK * 2);
    }

    #[test]
    fn no_starvation() {
        let mu = std::sync::Arc::new(Peterson::default());

        // A acquires
        let mut mu_a = PetersonA::new(&mu);
        let _guard_a = mu_a.acquire();

        // B blocks
        let th_b = std::thread::spawn({
            let mut mu_b = PetersonB::new(&mu);
            move || {
                let _guard_b = mu_b.acquire();
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        });
        std::thread::sleep(std::time::Duration::from_millis(500));
        assert!(!th_b.is_finished());

        // A release and acquire, then blocks so that B doesn't starve
        drop(_guard_a);
        let th_a = std::thread::spawn({
            move || {
                let _guard_a = mu_a.acquire();
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        });

        // B done, A still blocking
        th_b.join().unwrap();
        assert!(!th_a.is_finished());
        th_a.join().unwrap();
    }

    #[derive(Default)]
    struct TestData(std::sync::atomic::AtomicI32, std::sync::atomic::AtomicI32);
    impl TestData {
        // Relaxed since tests only require order within the same variable
        //
        // Memory Ordering Rules:
        // 1. Thread executes in program order
        // 2. All threads agree on each var's modification order (generated code)
        // 3. Different variables can be modified independently, unless SeqCst
        // 4. Threads may observe modification of different variables in different order
        fn add_then_sub(&self) {
            let i = self.0.load(Ordering::Relaxed);
            let d = self.1.load(Ordering::Relaxed);
            self.0.store(i + 1, Ordering::Relaxed);
            self.1.store(d - 1, Ordering::Relaxed);
        }
        fn sub_then_add(&self) {
            let d = self.1.load(Ordering::Relaxed);
            let i = self.0.load(Ordering::Relaxed);
            self.1.store(d - 1, Ordering::Relaxed);
            self.0.store(i + 1, Ordering::Relaxed);
        }
    }
}
