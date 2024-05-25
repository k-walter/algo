use crate::order::{HasEvents, LogicalClock, OrdProcess};
use std::collections::HashMap;

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct ChandyLamportClock {
    i: usize,
    clk: usize,
    is_snapshot: bool,
}
impl LogicalClock for ChandyLamportClock {
    fn new(i: usize, _n_procs: usize) -> Self {
        Self {
            i,
            clk: 0,
            is_snapshot: false,
        }
    }
    fn extend(&self) -> Self {
        Self {
            i: self.i,
            clk: self.clk + 1,
            is_snapshot: false,
        }
    }
    fn merge(&self, _other: &Self) -> Self {
        self.extend()
    }
}

pub struct ChandyLamportProc {
    i: usize,
    n: usize,
    events: Vec<ChandyLamportClock>,
    snapshots: HashMap<ChandyLamportClock, usize>,
}
impl ChandyLamportProc {
    pub fn snapshots(&self) -> Vec<(ChandyLamportClock, &[ChandyLamportClock])> {
        self.snapshots
            .iter()
            .map(|(k, v)| (k.clone(), &self.events[..*v]))
            .collect()
    }
}

impl ChandyLamportProc {
    fn new(i: usize, n: usize) -> Self {
        Self {
            i,
            n,
            events: Vec::new(),
            snapshots: HashMap::new(),
        }
    }
    // Expects to take a function that sends clock to all other processes in a lossless FIFO channel
    pub fn global_snapshot<F: Fn(ChandyLamportClock)>(&self, send_fn: F) {
        let e = self
            .last_event()
            .unwrap_or(&ChandyLamportClock::new(self.i, self.n))
            .clone();
        send_fn(e);
    }
}

impl HasEvents<ChandyLamportClock> for ChandyLamportProc {
    fn last_event(&self) -> Option<&ChandyLamportClock> {
        self.events.last()
    }
    fn push_event(&mut self, e: ChandyLamportClock) {
        self.events.push(e)
    }
    fn pid(&self) -> usize {
        self.i
    }
    fn n_procs(&self) -> usize {
        self.n
    }
    fn events(&self) -> &[ChandyLamportClock] {
        self.events.as_slice()
    }
}

impl OrdProcess<ChandyLamportClock> for ChandyLamportProc {
    fn recv<F: FnOnce() -> ChandyLamportClock>(&mut self, recv_fn: F) {
        let e_recv = recv_fn();
        if !e_recv.is_snapshot {
            let e = self
                .last_event()
                .unwrap_or(&ChandyLamportClock::new(self.pid(), self.n_procs()))
                .merge(&e_recv);
            self.push_event(e);
        } else if let std::collections::hash_map::Entry::Vacant(e) = self.snapshots.entry(e_recv) {
            e.insert(self.events.len());
            // TODO broadcast to all
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::order::chandy_lamport::ChandyLamportProc;
    use crate::order::{HasEvents, OrdProcess};

    #[test]
    fn wont_snapshot_before_send_after_recv() {
        let (tx1, rx1) = std::sync::mpsc::channel();
        let (tx2, rx2) = std::sync::mpsc::channel();
        let tx = Vec::from([tx1, tx2]);

        let p1 = std::thread::spawn({
            let tx = tx.clone();
            move || {
                let mut p = ChandyLamportProc::new(0, 2);
                assert!(p.snapshots().is_empty());

                p.global_snapshot(|e| tx.iter().for_each(|tx| tx.send(e.clone()).unwrap())); // snapshots on start
                assert_eq!(p.snapshots().len(), 1);
                assert!(p.snapshots().last().unwrap().1.is_empty()); // hence snapshot is empty

                p.send(|e| tx[0].send(e).unwrap());
                assert_eq!(p.snapshots().len(), 1); // no new snapshot
                assert!(p.snapshots().last().unwrap().1.is_empty()); // old snapshot
            }
        });

        let p2 = std::thread::spawn({
            move || {
                let mut p = ChandyLamportProc::new(1, 2);
                assert!(p.snapshots().is_empty());

                p.recv(|| rx2.recv().unwrap()); // should be snapshot
                assert_eq!(p.snapshots().len(), 1);
                assert_eq!(p.snapshots().last().unwrap().1.len(), 1); // immediately snapshot 1st recv

                p.recv(|| rx2.recv().unwrap()); // should be recv
                assert_eq!(p.snapshots().len(), 1); // no new snapshot
                assert_eq!(p.snapshots().last().unwrap().1.len(), 1); // old snapshot
            }
        });

        p1.join().unwrap();
        p2.join().unwrap();
    }

    #[test]
    fn snapshot_after_send_before_recv() {
        // 1 snapshot

        // 2 send
        // 2 recv (snapshot)

        // 3 recv (snapshot)
        // 3 recv

        // in flight?
    }

    #[test]
    fn snapshot_after_send_recv() {
        // 2 send
        // 2 recv (snapshot)

        // 1 snapshot

        // 3 recv (snapshot)
        // 3 recv
    }

    #[test]
    fn union_snapshots() {
        // send 1 - recv 1 - send 1 - recv 1
        // send n - recv n
        // send 1 - recv 1 - send 1 - recv 1
        // send n - recv n

        // all program order
        // all recv with send
    }

    #[test]
    fn intersection_snapshots() {
        // all program order
        // all recv with send
    }
}
