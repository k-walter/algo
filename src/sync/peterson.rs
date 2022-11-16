use super::{Mutex, MutexGuard};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Default)]
pub struct Peterson {
    a_wants: AtomicBool,
    b_wants: AtomicBool,
    a_turn: AtomicBool,
}
pub struct PetersonA(Arc<Peterson>);
pub struct PetersonB(Arc<Peterson>);

impl PetersonA {
    pub fn new(p: &Arc<Peterson>) -> Self {
        Self(p.clone())
    }
}
impl PetersonB {
    pub fn new(p: &Arc<Peterson>) -> Self {
        Self(p.clone())
    }
}
impl Mutex for PetersonA {
    type Output = Self;
    fn acquire(&self) -> MutexGuard<Self::Output> {
        // Algorithm requires no reordering of variables, hence SeqCst
        self.0.a_wants.store(true, Ordering::SeqCst);
        self.0.a_turn.store(false, Ordering::SeqCst);
        while self.0.b_wants.load(Ordering::SeqCst) && !self.0.a_turn.load(Ordering::SeqCst) {
            std::hint::spin_loop()
        }
        MutexGuard { mutex: self }
    }
    fn release(&self) {
        self.0.a_wants.store(false, Ordering::SeqCst)
    }
}
impl Mutex for PetersonB {
    type Output = Self;
    fn acquire(&self) -> MutexGuard<Self::Output> {
        self.0.b_wants.store(true, Ordering::SeqCst);
        self.0.a_turn.store(true, Ordering::SeqCst);
        while self.0.a_wants.load(Ordering::SeqCst) && self.0.a_turn.load(Ordering::SeqCst) {
            std::hint::spin_loop()
        }
        MutexGuard { mutex: self }
    }
    fn release(&self) {
        self.0.b_wants.store(false, Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use crate::sync::{
        peterson::{Peterson, PetersonA, PetersonB},
        Mutex,
    };
    use std::{
        sync::{
            atomic::{AtomicI32, Ordering},
            Arc,
        },
        thread::{self},
    };
    const WORK: i32 = 10_000_000 / 2;

    #[test]
    fn sequential_works() {
        let data = Arc::new(TestData::default());
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
        let data = Arc::new(TestData::default());
        let th_a = thread::spawn({
            let data = data.clone();
            move || {
                for _ in 0..WORK {
                    data.add_then_sub();
                }
            }
        });
        let th_b = thread::spawn({
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
        let mu = Arc::new(Peterson::default());
        let data = Arc::new(TestData::default());
        let th_a = thread::spawn({
            let data = data.clone();
            let mu = PetersonA::new(&mu);
            move || {
                for _ in 0..WORK {
                    let _guard = mu.acquire();
                    data.add_then_sub();
                }
            }
        });
        let th_b = thread::spawn({
            let data = data.clone();
            let mu = PetersonB::new(&mu);
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
