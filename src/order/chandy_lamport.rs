use crate::order::{HasEvents, LogicalClock, OrdProcess};

#[derive(Clone, Default)]
pub struct ChandyLamportClock {
    is_snapshot: bool,
}
impl LogicalClock for ChandyLamportClock {
    fn new(i: usize, n_procs: usize) -> Self {
        todo!()
    }
    fn extend(&self) -> Self {
        Self::default()
    }
    fn merge(&self, other: &Self) -> Self {
        // if other is snapshot
        // if have not snapshot for origin
        // snapshot
        todo!()
    }
}

pub struct ChandyLamportProc {
    i: usize,
    n: usize,
    events: Vec<ChandyLamportClock>,
    snapshots: Vec<usize>,
}
impl ChandyLamportProc {
    pub fn snapshots(&self) -> Vec<&[ChandyLamportClock]> {
        self.snapshots.iter().map(|&i| &self.events[..i]).collect()
    }
}

impl ChandyLamportProc {
    fn new(i: usize, n: usize) -> Self {
        Self {
            i,
            n,
            events: Vec::new(),
            snapshots: Vec::new(),
        }
    }
    // Expects to take a function that sends clock to all other processes in a lossless FIFO channel
    pub fn global_snapshot<F: Fn(ChandyLamportClock)>(&self, f: F) {
        todo!()
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

impl OrdProcess<ChandyLamportClock> for ChandyLamportProc {}

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
                assert!(p.snapshots().last().unwrap().is_empty()); // hence snapshot is empty

                p.send(|e| tx[0].send(e).unwrap());
                assert_eq!(p.snapshots().len(), 1); // no new snapshot
                assert!(p.snapshots().last().unwrap().is_empty()); // old snapshot
            }
        });

        let p2 = std::thread::spawn({
            move || {
                let mut p = ChandyLamportProc::new(1, 2);
                assert!(p.snapshots().is_empty());

                p.recv(|| rx2.recv().unwrap()); // should be snapshot
                assert_eq!(p.snapshots().len(), 1);
                assert_eq!(p.snapshots().last().unwrap().len(), 1); // immediately snapshot 1st recv

                p.recv(|| rx2.recv().unwrap()); // should be recv
                assert_eq!(p.snapshots().len(), 1); // no new snapshot
                assert_eq!(p.snapshots().last().unwrap().len(), 1); // old snapshot
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
