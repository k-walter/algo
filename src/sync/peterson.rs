use super::{Mutex, MutexGuard};
use std::sync::atomic::AtomicBool;

#[derive(Default)]
pub struct Peterson {
    is_acquired: AtomicBool,
}

impl Peterson {
    pub fn new() -> Self {
        Self::default()
    }
}
impl Mutex for Peterson {
    type Output = Self;
    fn acquire(&self) -> MutexGuard<Self::Output> {
        MutexGuard { mutex: self }
    }

    fn release(&self) {
        
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, thread::spawn};

    use crate::sync::Mutex;

    #[test]
    fn test() {
        let num = 100;
        let mutex = Arc::new(super::Peterson::new());
        let threads: Vec<_> = (0..num)
            .map(|_| {
                let mutex = mutex.clone();
                spawn(move || {
                    let _ = mutex.acquire();
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
    }
}
