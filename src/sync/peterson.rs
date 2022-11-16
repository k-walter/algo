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

    #[test]
    fn race_condition() {
        // Concurrent
        const WORK: i32 = 10_000_000 / 2;
        let incr = Arc::new(AtomicI32::new(0));
        let decr = Arc::new(AtomicI32::new(0));
        let th_a = thread::spawn({
            let incr = incr.clone();
            let decr = decr.clone();
            move || {
                for _ in 0..WORK {
                    let i = incr.load(Ordering::Acquire);
                    let d = decr.load(Ordering::Acquire);
                    incr.store(i + 1, Ordering::Release);
                    decr.store(d - 1, Ordering::Release);
                }
            }
        });
        let th_b = thread::spawn({
            let incr = incr.clone();
            let decr = decr.clone();
            move || {
                for _ in 0..WORK {
                    let d = decr.load(Ordering::Acquire);
                    let i = incr.load(Ordering::Acquire);
                    decr.store(d - 1, Ordering::Release);
                    incr.store(i + 1, Ordering::Release);
                }
            }
        });
        th_a.join().unwrap();
        th_b.join().unwrap();
        assert!(incr.load(Ordering::Relaxed) > WORK);
        assert_ne!(incr.load(Ordering::Relaxed), WORK * 2);
        assert!(decr.load(Ordering::Relaxed) < -WORK);
        assert_ne!(decr.load(Ordering::Relaxed), -WORK * 2);

        // Sequential
        let incr = Arc::new(AtomicI32::new(0));
        let decr = Arc::new(AtomicI32::new(0));
        for _ in 0..WORK {
            let i = incr.load(Ordering::Acquire);
            let d = decr.load(Ordering::Acquire);
            incr.store(i + 1, Ordering::Release);
            decr.store(d - 1, Ordering::Release);
        }
        for _ in 0..WORK {
            let d = decr.load(Ordering::Acquire);
            let i = incr.load(Ordering::Acquire);
            decr.store(d - 1, Ordering::Release);
            incr.store(i + 1, Ordering::Release);
        }
        assert_eq!(incr.load(Ordering::Relaxed), WORK * 2);
        assert_eq!(decr.load(Ordering::Relaxed), -WORK * 2);
    }

    #[test]
    fn mutual_exclusion() {
        const WORK: i32 = 10_000_000 / 2;
        let mu = Arc::new(Peterson::default());
        let incr = Arc::new(AtomicI32::new(0));
        let decr = Arc::new(AtomicI32::new(0));
        let th_a = thread::spawn({
            let mu = PetersonA::new(&mu);
            let incr = incr.clone();
            let decr = decr.clone();
            move || {
                for _ in 0..WORK {
                    let _guard = mu.acquire();
                    let i = incr.load(Ordering::Acquire);
                    let d = decr.load(Ordering::Acquire);
                    incr.store(i + 1, Ordering::Release);
                    decr.store(d - 1, Ordering::Release);
                }
            }
        });
        let th_b = thread::spawn({
            let mu = PetersonB::new(&mu);
            let incr = incr.clone();
            let decr = decr.clone();
            move || {
                for _ in 0..WORK {
                    let _guard = mu.acquire();
                    let d = decr.load(Ordering::Acquire);
                    let i = incr.load(Ordering::Acquire);
                    decr.store(d - 1, Ordering::Release);
                    incr.store(i + 1, Ordering::Release);
                }
            }
        });
        th_a.join().unwrap();
        th_b.join().unwrap();
        assert_eq!(incr.load(Ordering::Acquire), WORK * 2);
        assert_eq!(decr.load(Ordering::Acquire), -WORK * 2);
    }
}
