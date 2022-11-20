use super::NoStarveMutex;
use crate::sync::WantGuard;
use std::sync::atomic::Ordering;

/// N-ary mutex to protect critical section fairly.
///
/// # Examples
/// ```
/// use crate::rads::sync::lamports_bakery::{Bakery, BakeryN};
/// use std::sync::atomic::Ordering;
/// use rads::sync::NoStarveMutex;
///
/// let data = std::sync::Arc::new(std::sync::atomic::AtomicI32::new(0));
/// let mu = std::sync::Arc::new(Bakery::new(4));
/// let ths = (0..4)
///     .map(|n| {
///         let data = data.clone();
///         let mut mu = BakeryN::new(n, &mu);
///         let n = n as i32 + 1;
///         std::thread::spawn(move || {
///             for _ in 0..10_000 {
///                 let _guard = mu.lock();
///                 let i = data.load(Ordering::Relaxed);
///                 data.store(i + n, Ordering::Relaxed);
///             }
///         })
///     })
///     .collect::<Vec<_>>();
/// ths.into_iter().for_each(|th| th.join().unwrap());
/// assert_eq!(data.load(Ordering::Relaxed), 10_000 * (1 + 2 + 3 + 4));
/// ```
pub struct Bakery {
    q_nos: Vec<std::sync::atomic::AtomicI32>,
}
impl Bakery {
    const ENTER: i32 = -1;
    const FREE: i32 = 0;
    pub fn new(size: usize) -> Self {
        assert!(size > 1, "Do you really need a mutex of size {size}?");
        Self {
            q_nos: (0..size)
                .map(|_| std::sync::atomic::AtomicI32::new(Bakery::FREE))
                .collect(),
        }
    }
}

pub struct BakeryN {
    n: usize,
    bakery: std::sync::Arc<Bakery>,
}
pub struct BakeryWant<'a>(Option<&'a BakeryN>);
pub struct BakeryGuard<'a>(&'a BakeryN);
impl BakeryN {
    // Does not check if index is taken.
    pub fn new(n: usize, bakery: &std::sync::Arc<Bakery>) -> Self {
        let size = bakery.q_nos.len();
        assert!(n < size, "0-based user index {n} >= bakery of size={size}");
        Self {
            n,
            bakery: bakery.clone(),
        }
    }
}
impl<'a> NoStarveMutex<'a, BakeryGuard<'a>, BakeryWant<'a>> for BakeryN {
    fn want_lock(&'a mut self) -> BakeryWant<'a> {
        // Optimization: entering the queue is -1, which is fine since -1 < all +ve queue numbers
        self.bakery.q_nos[self.n].store(Bakery::ENTER, Ordering::SeqCst);
        let q_no = 1 + self
            .bakery
            .q_nos
            .iter()
            .fold(0, |acc, i| i.load(Ordering::SeqCst).max(acc));
        self.bakery.q_nos[self.n].store(q_no, Ordering::SeqCst);
        BakeryWant(Some(self))
    }
}

impl<'a> WantGuard<'a, BakeryGuard<'a>> for BakeryWant<'a> {
    fn wait(mut self) -> BakeryGuard<'a> {
        let b = self.0.take().unwrap();
        let q_no = b.bakery.q_nos[b.n].load(Ordering::SeqCst);
        for (i, other_q_no) in b.bakery.q_nos.iter().enumerate() {
            while other_q_no.load(Ordering::SeqCst) != Bakery::FREE
                && (other_q_no.load(Ordering::SeqCst), i) < (q_no, b.n)
            {
                std::thread::yield_now();
            }
        }
        BakeryGuard(b)
    }
}
impl<'a> Drop for BakeryWant<'a> {
    fn drop(&mut self) {
        if let Some(b) = self.0 {
            BakeryGuard(b); // reuse drop logic
        }
    }
}

impl Drop for BakeryGuard<'_> {
    fn drop(&mut self) {
        self.0.bakery.q_nos[self.0.n].store(Bakery::FREE, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use crate::sync::{
        lamports_bakery::{Bakery, BakeryN},
        NoStarveMutex, WantGuard,
    };
    use std::sync::atomic::Ordering;
    const N_THREADS: i32 = 4;
    const WORK: i32 = 1_000_000 / N_THREADS;

    #[test]
    fn sequential_works() {
        let data = std::sync::Arc::new(TestData::default());
        for i in 0..N_THREADS {
            for _ in 0..WORK {
                if i % 2 == 0 {
                    data.add_then_sub();
                } else {
                    data.sub_then_add();
                }
            }
        }
        assert_eq!(data.0.load(Ordering::Relaxed), WORK * N_THREADS);
        assert_eq!(data.1.load(Ordering::Relaxed), -WORK * N_THREADS);
    }

    #[test]
    fn race_conditions() {
        let data = std::sync::Arc::new(TestData::default());
        let ths = (0..N_THREADS)
            .map(|i| {
                let data = data.clone();
                std::thread::spawn(move || {
                    for _ in 0..WORK {
                        if i % 2 == 0 {
                            data.add_then_sub();
                        } else {
                            data.sub_then_add();
                        }
                    }
                })
            })
            .collect::<Vec<_>>();
        ths.into_iter().for_each(|th| th.join().unwrap());
        assert!(data.0.load(Ordering::Relaxed) > WORK);
        assert!(data.0.load(Ordering::Relaxed) < WORK * N_THREADS);
        assert!(data.1.load(Ordering::Relaxed) < -WORK);
        assert!(data.1.load(Ordering::Relaxed) > -WORK * N_THREADS);
    }

    #[test]
    fn mutual_exclusion() {
        let data = std::sync::Arc::new(TestData::default());
        let mu = std::sync::Arc::new(Bakery::new(N_THREADS as usize));
        let ths = (0..N_THREADS as usize)
            .map(|n| {
                let data = data.clone();
                let mut mu = BakeryN::new(n, &mu);
                std::thread::spawn(move || {
                    for _ in 0..WORK {
                        let _guard = mu.lock();
                        if n % 2 == 0 {
                            data.add_then_sub();
                        } else {
                            data.sub_then_add();
                        }
                    }
                })
            })
            .collect::<Vec<_>>();
        ths.into_iter().for_each(|th| th.join().unwrap());
        assert_eq!(data.0.load(Ordering::Relaxed), WORK * N_THREADS);
        assert_eq!(data.1.load(Ordering::Relaxed), -WORK * N_THREADS);
    }

    #[test]
    fn no_starvation() {
        let mu = std::sync::Arc::new(Bakery::new(2));
        // 1. Let p0 locks first, then p1 wants and blocks
        let p0_first = std::sync::Arc::new(std::sync::Barrier::new(2));
        let p1_wants = std::sync::Arc::new(std::sync::Barrier::new(2));
        // 2. p0 release and immediately locks but blocks to p1 who's waiting
        let p0_release = std::sync::Arc::new(std::sync::Barrier::new(2));
        // 3. p1 wakes up and locks the mutex, ie no starvation
        let p1_acquire = std::sync::Arc::new(std::sync::Barrier::new(2));
        // 4. p1 releases, allowing p0 to lock
        let p1_release = std::sync::Arc::new(std::sync::Barrier::new(2));
        // 5. p0 wakes up and locks the mutex
        let p0_reacquire = std::sync::Arc::new(std::sync::Barrier::new(2));

        let th0 = {
            let mut mu_a = BakeryN::new(0, &mu);
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
            let mut mu_b = BakeryN::new(1, &mu);
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
