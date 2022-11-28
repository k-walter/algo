use crate::order::{HasEvents, LogicalClock, OrdProcess};
use std::cmp::Ordering;

#[derive(Clone, Default)]
#[cfg_attr(test, derive(Debug))]
pub struct MatrixClock {}

impl PartialOrd for MatrixClock {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        todo!()
    }
}

impl PartialEq<Self> for MatrixClock {
    fn eq(&self, other: &Self) -> bool {
        todo!()
    }
}

impl LogicalClock for MatrixClock {
    fn new(i: usize, n_procs: usize) -> Self {
        todo!()
    }

    fn extend(&self) -> Self {
        todo!()
    }

    fn merge(&self, other: &Self) -> Self {
        todo!()
    }
}

pub struct GCProcess {
    events: Vec<MatrixClock>,
}

impl GCProcess {
    pub fn new(i: usize, n_procs: usize) -> Self {
        Self {
            events: Vec::from([MatrixClock::new(i, n_procs)]),
        }
    }

    pub fn gc(&mut self) -> Vec<MatrixClock> {
        todo!()
    }
}

impl HasEvents<MatrixClock> for GCProcess {
    fn events(&self) -> &[MatrixClock] {
        &self.events
    }

    fn events_mut(&mut self) -> &mut Vec<MatrixClock> {
        &mut self.events
    }
}

impl OrdProcess<MatrixClock> for GCProcess {}

#[cfg(test)]
mod tests {
    use crate::order::matrix_clock::{GCProcess, MatrixClock};
    use crate::order::OrdProcess;
    use rand::Rng;

    #[test]
    fn it_works() {
        let mut rng = rand::thread_rng();
        let n_procs = rng.gen_range(2..=200);
        let mut ps: Vec<_> = (0..n_procs).map(|i| GCProcess::new(i, n_procs)).collect();

        // 0 does some work
        let n_events = rng.gen_range(1..=200);
        for _ in 0..n_events {
            ps[0].exec(|| {});
        }

        // Send from 0->1->2->...->n-1
        for (i, j) in (0..).zip(1..n_procs) {
            let mut e = None;
            ps[i].send(|ev| {
                e = Some(ev);
            });
            ps[j].recv(|| e.unwrap());
            assert_eq!(ps[0].gc(), vec![]);
        }

        // Send from n-1->0, can GC n_events + send event
        let mut e = None;
        ps.last_mut().unwrap().send(|ev| {
            e = Some(ev);
        });
        ps[0].recv(|| e.unwrap());
        assert_eq!(!ps[0].gc().len(), n_events + 1);
    }
}
