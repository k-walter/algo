use crate::order::{pairwise_max, GCClock, HasEvents, LogicalClock, OrdProcess};
use std::cmp::Ordering;
use std::collections::VecDeque;

#[derive(Clone, Default)]
#[cfg_attr(test, derive(Debug))]
pub struct MatrixClock {
    i: usize,
    clk: Vec<Vec<usize>>,
}

impl GCClock for MatrixClock {
    fn gc(&self, latest: &Self) -> bool {
        let seq = self.clk[self.i][self.i];
        // have all have seen my seq?
        latest.clk.iter().all(|vi| vi[self.i] >= seq)
    }
}

impl LogicalClock for MatrixClock {
    fn new(i: usize, n_procs: usize) -> Self {
        Self {
            i,
            clk: (0..n_procs)
                .map(|j| {
                    if i == j {
                        // So that p0 is not comparable to p1
                        (0..n_procs).map(|j| usize::from(i == j)).collect()
                    } else {
                        // i don't know anything about other processes
                        vec![0; n_procs]
                    }
                })
                .collect(),
        }
    }

    fn extend(&self) -> Self {
        let mut c = self.clone();
        c.clk[self.i][self.i] += 1;
        c
    }

    fn merge(&self, other: &Self) -> Self {
        let mut c = Self {
            i: self.i,
            clk: self
                .clk
                .iter()
                .zip(&other.clk)
                // Take max of what everyone has seen
                .map(|(u, v)| pairwise_max(u.iter(), v.iter()).collect())
                .collect(),
        };
        // I have seen max of what everyone has seen
        c.clk[self.i] = (0..self.clk.len())
            .map(|col| c.clk.iter().fold(0, |acc, vi| vi[col].max(acc)))
            .collect();
        // Receive event > previous event
        c.clk[self.i][self.i] += 1;
        c
    }
}

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

pub struct GCProcess {
    i: usize,
    n_procs: usize,
    events: VecDeque<MatrixClock>,
}

impl GCProcess {
    pub fn new(i: usize, n_procs: usize) -> Self {
        Self {
            i,
            n_procs,
            events: VecDeque::new(),
        }
    }
    pub fn gc(&mut self) -> Vec<MatrixClock> {
        let Some(latest) = self.events.back() else {
            return Vec::new()
        };
        let i = self.events.partition_point(|c| c.gc(latest));
        self.events.drain(..i).collect()
    }
}

impl OrdProcess<MatrixClock> for GCProcess {
    fn snapshot(&self) -> &[MatrixClock] {
        let (front, _back) = self.events.as_slices();
        front
    }
}

impl HasEvents<MatrixClock> for GCProcess {
    fn last_event(&self) -> Option<&MatrixClock> {
        self.events.back()
    }
    fn push_event(&mut self, e: MatrixClock) {
        self.events.push_back(e);
    }
    fn pid(&self) -> usize {
        self.i
    }
    fn n_procs(&self) -> usize {
        self.n_procs
    }
}

#[cfg(test)]
mod tests {
    use crate::order::matrix_clock::{GCProcess, MatrixClock};
    use crate::order::{LogicalClock, OrdProcess};
    use rand::Rng;

    #[test]
    fn gc_after_all_seen() {
        let mut rng = rand::thread_rng();
        let n_procs = rng.gen_range(2..=200);
        let mut ps: Vec<_> = (0..n_procs).map(|i| GCProcess::new(i, n_procs)).collect();

        // 0 does some work
        let n_events = rng.gen_range(1..=200);
        for _ in 0..n_events {
            ps[0].exec(|| {});
        }
        assert_eq!(ps[0].gc(), vec![]);
        assert_eq!(ps[0].snapshot().len(), n_events);

        // Send from 0->1->2->...->n-1
        for (i, j) in (0..).zip(1..n_procs) {
            let mut e = None;
            ps[i].send(|ev| {
                e = Some(ev);
            });
            ps[j].recv(|| e.unwrap());
            assert_eq!(ps[0].gc(), vec![]);
            assert_eq!(ps[0].snapshot().len(), n_events + 1); // + send event
        }

        // Send from n-1->0, can GC n_events + send event
        let mut e = None;
        ps.last_mut().unwrap().send(|ev| {
            e = Some(ev);
        });
        ps[0].recv(|| e.unwrap());
        let gc = ps[0].gc();
        assert_eq!(gc.len(), n_events + 1, "Got {gc:?}");
        assert_eq!(ps[0].snapshot().len(), 1); // recv event only
    }
}
