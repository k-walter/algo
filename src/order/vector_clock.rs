use super::LogicalClock;

#[derive(Clone)]
pub struct VectorClock {
    i: usize,
    clk: Vec<usize>,
}

impl LogicalClock for VectorClock {
    fn new(i: usize, n_procs: usize) -> Self {
        assert!(
            i < n_procs,
            "Expect 0-based index of process {i} < n_procs={n_procs}"
        );
        Self {
            i,
            clk: (0..n_procs).map(|j| usize::from(i == j)).collect(),
        }
    }
    fn extend(&self) -> Self {
        let mut e = self.clone();
        e.clk[e.i] += 1;
        e
    }
    fn merge(&self, other: &Self) -> Self {
        assert!(
            self.clk.len() == other.clk.len(),
            "Cannot merge with process that is aware of differing processes"
        );
        let mut e = self.clone();
        for (s, t) in e.clk.iter_mut().zip(&other.clk) {
            *s = (*s).max(*t);
        }
        e.clk[e.i] += 1;
        e
    }
}

impl PartialOrd for VectorClock {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.clk.len() != other.clk.len() {
            return None;
        }
        self.clk
            .iter()
            .zip(&other.clk)
            .try_fold(std::cmp::Ordering::Equal, |acc, (s, t)| match s.cmp(t) {
                std::cmp::Ordering::Greater => None,
                std::cmp::Ordering::Less => Some(std::cmp::Ordering::Less),
                std::cmp::Ordering::Equal => Some(acc),
            })
    }
}

impl PartialEq for VectorClock {
    fn eq(&self, other: &Self) -> bool {
        self.clk == other.clk
    }
}

mod test {
    use crate::order::{vector_clock::VectorClock, LogicalClock, Process};
    use rand::Rng;

    #[test]
    fn partial_ord() {
        let e1 = VectorClock::new(0, 2);
        assert_eq!(e1.partial_cmp(&e1), Some(std::cmp::Ordering::Equal));
        let e2 = e1.extend();
        assert_eq!(e1.partial_cmp(&e2), Some(std::cmp::Ordering::Less));
        assert_eq!(e2.partial_cmp(&e2), Some(std::cmp::Ordering::Equal));

        let f1 = VectorClock::new(1, 2);
        assert_eq!(e1.partial_cmp(&f1), None);
        assert_eq!(e2.partial_cmp(&f1), None);
        assert_eq!(f1.partial_cmp(&f1), Some(std::cmp::Ordering::Equal));
        let f2 = f1.merge(&e1);
        assert_eq!(e1.partial_cmp(&f2), Some(std::cmp::Ordering::Less));
        assert_eq!(e2.partial_cmp(&f2), None);
        assert_eq!(f1.partial_cmp(&f2), Some(std::cmp::Ordering::Less));
        assert_eq!(f2.partial_cmp(&f2), Some(std::cmp::Ordering::Equal));
    }

    #[test]
    fn mock_scheduler() {
        let (tx3_2, rx3_2) = std::sync::mpsc::channel::<VectorClock>();
        let (tx1_2, rx1_2) = std::sync::mpsc::channel::<VectorClock>();
        let (tx3, rx3) = std::sync::mpsc::channel::<VectorClock>();

        let th1 = std::thread::spawn(move || {
            let mut p = Process::new(0, 3);
            p.exec(rand_timeout);
            p.send(|e| tx1_2.send(e).unwrap());
            p.exec(rand_timeout);
            p
        });
        let th2 = std::thread::spawn(move || {
            let mut p = Process::new(1, 3);
            p.exec(rand_timeout);
            p.recv(|| rx3_2.recv().unwrap());
            p.recv(|| rx1_2.recv().unwrap());
            p.send(|e| tx3.send(e).unwrap());
            p
        });
        let th3 = std::thread::spawn(move || {
            let mut p = Process::new(2, 3);
            p.exec(rand_timeout);
            p.send(|e| tx3_2.send(e).unwrap());
            p.exec(rand_timeout);
            p.recv(|| rx3.recv().unwrap());
            p
        });

        let p1 = th1.join().unwrap().snapshot();
        let p2 = th2.join().unwrap().snapshot();
        let p3 = th3.join().unwrap().snapshot();

        // Number of events
        assert_eq!(p1.len(), 3);
        assert_eq!(p2.len(), 4);
        assert_eq!(p3.len(), 4);

        // Program order --> s<t
        assert!(p1.iter().zip(&p1[1..]).all(|(s, t)| s < t));
        assert!(p2.iter().zip(&p2[1..]).all(|(s, t)| s < t));
        assert!(p3.iter().zip(&p3[1..]).all(|(s, t)| s < t));

        // Send-receive | transitive order --> s<t
        assert!(p3[..2].iter().all(|s| s < &p2[1])); // from p3 to p2
        assert!(p1[..2].iter().all(|s| s < &p2[2])); // from p1 to p2
        assert!(p3[..2].iter().all(|s| s < &p2[2]));
        assert!(p2.iter().all(|s| s < &p3[3])); // from p2 to p3
        assert!(p1[..2].iter().all(|s| s < &p3[3]));
    }

    fn rand_timeout() {
        let mut rng = rand::thread_rng();
        let t = rng.gen_range(0..=200);
        std::thread::sleep(std::time::Duration::from_millis(t));
    }
}
