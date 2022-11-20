use super::{NoStarveMutex, WantGuard};
use std::sync::atomic::Ordering;

/// Binary mutex to protect critical section fairly.
///
/// # Examples
/// ```
/// use crate::rads::sync::peterson::Peterson;
/// use std::sync::atomic::Ordering;
/// use rads::sync::NoStarveMutex;
///
/// let (mut mu_a, mut mu_b) = Peterson::binary_mutex();
/// let mut val = std::sync::Arc::new(std::sync::atomic::AtomicI32::new(0));
/// let th_a = std::thread::spawn({
///     let val_a = val.clone();
///     move || {
///         let _guard_a = mu_a.lock();
///         let i = val_a.load(Ordering::Relaxed);
///         val_a.store(i + 1, Ordering::Relaxed);
///     }
/// });
/// let th_b = std::thread::spawn({
///     let val_b = val.clone();
///     move || {
///         let _guard_b = mu_b.lock();
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
pub struct PetersonAWantGuard<'a>(Option<&'a PetersonA>);
pub struct PetersonBWantGuard<'a>(Option<&'a PetersonB>);
pub struct PetersonAGuard<'a>(&'a PetersonA);
pub struct PetersonBGuard<'a>(&'a PetersonB);

impl Peterson {
    pub fn binary_mutex() -> (PetersonA, PetersonB) {
        let p = std::sync::Arc::new(Peterson::default());
        (PetersonA(p.clone()), PetersonB(p))
    }
}

impl<'a> NoStarveMutex<'a, PetersonAGuard<'a>, PetersonAWantGuard<'a>> for PetersonA {
    fn want_lock(&'a mut self) -> PetersonAWantGuard<'a> {
        // Algorithm requires no reordering of variables, hence SeqCst
        self.0.a_wants.store(true, Ordering::SeqCst);
        self.0.a_turn.store(false, Ordering::SeqCst);
        PetersonAWantGuard(Some(self))
    }
}
impl<'a> WantGuard<'a, PetersonAGuard<'a>> for PetersonAWantGuard<'a> {
    fn wait(mut self) -> PetersonAGuard<'a> {
        let p = self.0.take().unwrap();
        while p.0.b_wants.load(Ordering::SeqCst) && !p.0.a_turn.load(Ordering::SeqCst) {
            std::thread::yield_now();
        }
        PetersonAGuard(p)
    }
}
impl Drop for PetersonAWantGuard<'_> {
    fn drop(&mut self) {
        if let Some(p) = self.0 {
            PetersonAGuard(p); // reuse drop logic
        }
    }
}
impl Drop for PetersonAGuard<'_> {
    fn drop(&mut self) {
        self.0 .0.a_wants.store(false, Ordering::SeqCst)
    }
}

impl<'a> NoStarveMutex<'a, PetersonBGuard<'a>, PetersonBWantGuard<'a>> for PetersonB {
    fn want_lock(&'a mut self) -> PetersonBWantGuard<'a> {
        self.0.b_wants.store(true, Ordering::SeqCst);
        self.0.a_turn.store(true, Ordering::SeqCst);
        PetersonBWantGuard(Some(self))
    }
}
impl<'a> WantGuard<'a, PetersonBGuard<'a>> for PetersonBWantGuard<'a> {
    fn wait(mut self) -> PetersonBGuard<'a> {
        let p = self.0.take().unwrap();
        while p.0.a_wants.load(Ordering::SeqCst) && p.0.a_turn.load(Ordering::SeqCst) {
            std::thread::yield_now();
        }
        PetersonBGuard(p)
    }
}
impl Drop for PetersonBWantGuard<'_> {
    fn drop(&mut self) {
        if let Some(p) = self.0 {
            PetersonBGuard(p); // reuse drop logic
        }
    }
}
impl Drop for PetersonBGuard<'_> {
    fn drop(&mut self) {
        self.0 .0.b_wants.store(false, Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use crate::sync::{peterson::Peterson, NoStarveMutex, WantGuard};
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
        let (mut mu_a, mut mu_b) = Peterson::binary_mutex();
        let data = std::sync::Arc::new(TestData::default());
        let th_a = std::thread::spawn({
            let data = data.clone();
            move || {
                for _ in 0..WORK {
                    let _guard = mu_a.lock();
                    data.add_then_sub();
                }
            }
        });
        let th_b = std::thread::spawn({
            let data = data.clone();
            move || {
                for _ in 0..WORK {
                    let _want = mu_b.lock();
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
        let (mut mu_a, mut mu_b) = Peterson::binary_mutex();
        // 1. Let p0 acquire first, then p1 wants and blocks
        let p0_first = std::sync::Arc::new(std::sync::Barrier::new(2));
        let p1_wants = std::sync::Arc::new(std::sync::Barrier::new(2));
        // 2. p0 release and immediately acquires but blocks to p1 who's waiting
        let p0_release = std::sync::Arc::new(std::sync::Barrier::new(2));
        // 3. p1 wakes up and acquires the mutex, ie no starvation
        let p1_acquire = std::sync::Arc::new(std::sync::Barrier::new(2));
        // 4. p1 releases, allowing p0 to acquire
        let p1_release = std::sync::Arc::new(std::sync::Barrier::new(2));
        // 5. p0 wakes up and acquires the mutex
        let p0_reacquire = std::sync::Arc::new(std::sync::Barrier::new(2));

        let th0 = {
            let p0_first = p0_first.clone();
            let p0_release = p0_release.clone();
            let p0_reacquire = p0_reacquire.clone();
            std::thread::spawn(move || {
                let _guard = mu_a.lock();
                println!("p0 first");
                p0_first.wait(); // (1)
                p0_release.wait(); // (2)
                println!("p0 releasing");
                drop(_guard);
                let _guard = mu_a.lock(); // (4)
                println!("p0 reacquire");
                p0_reacquire.wait(); // (5)
            })
        };
        let th1 = std::thread::spawn({
            let p1_wants = p1_wants.clone();
            let p1_release = p1_release.clone();
            let p1_acquire = p1_acquire.clone();
            move || {
                p0_first.wait(); // (1)
                let mut _want = mu_b.want_lock();
                p1_wants.wait();
                let _guard = _want.wait(); // (3)
                println!("p1 acquires");
                p1_acquire.wait();
                p1_release.wait();
                println!("p1 releasing");
            }
        });
        p1_wants.wait(); // (1)
        p0_release.wait(); // (2)
        p1_acquire.wait(); // (3)
        std::thread::yield_now(); // (4)
        assert!(!th0.is_finished());
        assert!(!th1.is_finished());
        p1_release.wait();
        th1.join().unwrap(); // (5)
        std::thread::yield_now();
        assert!(!th0.is_finished());
        p0_reacquire.wait();
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
