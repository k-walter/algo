use std::sync::{
    atomic::{AtomicI32, Ordering},
    Arc,
};

use super::Mutex;

pub struct Bakery {
    q_nos: Vec<AtomicI32>,
}
impl Bakery {
    const ENTER: i32 = -1;
    const FREE: i32 = 0;
    pub fn new(size: usize) -> Self {
        Self {
            q_nos: (0..size).map(|_| AtomicI32::new(Bakery::FREE)).collect(),
        }
    }
}

pub struct BakeryN {
    n: usize,
    bakery: Arc<Bakery>,
}
impl BakeryN {
    pub fn new(n: usize, bakery: &Arc<Bakery>) -> Self {
        Self {
            n,
            bakery: bakery.clone(),
        }
    }
}
impl Mutex for BakeryN {
    type Output = Self;
    fn acquire(&self) -> super::MutexGuard<Self::Output> {
        // Optimization: entering the queue is -1, which is fine since -1 < all +ve queue numbers
        self.bakery.q_nos[self.n].store(Bakery::ENTER, Ordering::SeqCst);
        let q_no = 1 + self
            .bakery
            .q_nos
            .iter()
            .fold(0, |acc, i| i.load(Ordering::SeqCst).max(acc));
        self.bakery.q_nos[self.n].store(q_no, Ordering::SeqCst);

        for (i, other_q_no) in self.bakery.q_nos.iter().enumerate() {
            while other_q_no.load(Ordering::SeqCst) != Bakery::FREE
                && (other_q_no.load(Ordering::SeqCst), i) < (q_no, self.n)
            {
                std::hint::spin_loop()
            }
        }

        super::MutexGuard { mutex: self }
    }
    fn release(&self) {
        self.bakery.q_nos[self.n].store(Bakery::FREE, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use crate::sync::{
        lamports_bakery::{Bakery, BakeryN},
        Mutex,
    };
    use std::sync::{
        atomic::{AtomicI32, Ordering},
        Arc,
    };
    const N_THREADS: i32 = 4;
    const WORK: i32 = 1_000_000 / N_THREADS;

    #[test]
    fn sequential_works() {
        let data = Arc::new(TestData::default());
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
        let data = Arc::new(TestData::default());
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
        let data = Arc::new(TestData::default());
        let mu = Arc::new(Bakery::new(N_THREADS as usize));
        let ths = (0..N_THREADS as usize)
            .map(|n| {
                let data = data.clone();
                let mu = BakeryN::new(n, &mu);
                std::thread::spawn(move || {
                    for _ in 0..WORK {
                        let _guard = mu.acquire();
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
        let mu = Arc::new(Bakery::new(N_THREADS as usize));

        // 0 to n-2 acquire
        let ths = (0..N_THREADS as usize - 1)
            .map(|n| {
                let mu = BakeryN::new(n, &mu);
                std::thread::spawn(move || {
                    let _guard = mu.acquire();
                    std::thread::sleep(std::time::Duration::from_millis(100));
                })
            })
            .collect::<Vec<_>>();

        // n-1 th blocks
        let th = std::thread::spawn({
            let mu = BakeryN::new(N_THREADS as usize - 1, &mu);
            move || {
                let _guard_b = mu.acquire();
                std::thread::sleep(std::time::Duration::from_millis(1000));
            }
        });
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(!th.is_finished());

        // 0 to n-2 release and acquire but block, because n-1 acquires
        let ths = ths
            .into_iter()
            .enumerate()
            .map(|(n, th)| {
                let mu = BakeryN::new(n, &mu);
                th.join().unwrap();
                std::thread::spawn(move || {
                    let _guard = mu.acquire();
                    std::thread::sleep(std::time::Duration::from_millis(100));
                })
            })
            .collect::<Vec<_>>();
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(ths.iter().all(|th| !th.is_finished()));

        // n-1 releases, then rest acquire
        th.join().unwrap();
        assert!(ths.iter().all(|th| !th.is_finished()));
        ths.into_iter().for_each(|th| th.join().unwrap());
    }

    #[derive(Default)]
    struct TestData(AtomicI32, AtomicI32);
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
