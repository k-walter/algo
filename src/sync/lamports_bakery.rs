use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc,
};

use super::Mutex;

pub struct Bakery {
    getting_no: Vec<AtomicBool>,
    q_no: Vec<AtomicU32>,
}
impl Bakery {
    pub fn new(n: usize) -> Self {
        Self {
            getting_no: (0..n).map(|_| AtomicBool::new(false)).collect(),
            q_no: (0..n).map(|_| AtomicU32::new(0)).collect(),
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
        self.bakery.getting_no[self.n].store(true, Ordering::SeqCst);
        let q_no = 1 + self
            .bakery
            .q_no
            .iter()
            .fold(0, |acc, i| i.load(Ordering::SeqCst).max(acc));
        self.bakery.q_no[self.n].store(q_no, Ordering::SeqCst);
        self.bakery.getting_no[self.n].store(false, Ordering::SeqCst);

        for i in 0..self.bakery.q_no.len() {
            while self.bakery.getting_no[i].load(Ordering::SeqCst) {
                std::hint::spin_loop()
            }
            while self.bakery.q_no[i].load(Ordering::SeqCst) != 0
                && (q_no > self.bakery.q_no[i].load(Ordering::SeqCst)
                    || (q_no == self.bakery.q_no[i].load(Ordering::SeqCst) && i < self.n))
            {
                std::hint::spin_loop()
            }
        }

        super::MutexGuard { mutex: self }
    }
    fn release(&self) {
        self.bakery.q_no[self.n].store(0, Ordering::SeqCst);
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
        todo!()
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
