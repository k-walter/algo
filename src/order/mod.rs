pub mod matrix_clock;
pub mod vector_clock;

// PartialOrd because not all clocks are comparable
pub trait LogicalClock: PartialOrd + Clone {
    fn new(i: usize, n_procs: usize) -> Self;
    fn extend(&self) -> Self;
    fn merge(&self, other: &Self) -> Self;
}

pub trait GCClock: LogicalClock {
    fn gc(&self, latest: &Self) -> bool;
}

pub trait HasEvents<Event: LogicalClock> {
    fn last_event(&self) -> Option<&Event>;
    fn push_event(&mut self, e: Event);
    fn pid(&self) -> usize;
    fn n_procs(&self) -> usize;
}

pub trait OrdProcess<Event>: HasEvents<Event>
where
    Event: LogicalClock,
{
    // Provide a clock for event before executing
    fn exec<F: FnOnce()>(&mut self, f: F) {
        let e = self
            .last_event()
            .unwrap_or(&Event::new(self.pid(), self.n_procs()))
            .extend();
        self.push_event(e);
        f();
    }
    // Sends new clock to receiving party
    fn send<F: FnOnce(Event)>(&mut self, send_fn: F) {
        let e = self
            .last_event()
            .unwrap_or(&Event::new(self.pid(), self.n_procs()))
            .extend();
        self.push_event(e.clone());
        send_fn(e);
    }
    // Receives clock from sending party and updates own clock
    fn recv<F: FnOnce() -> Event>(&mut self, recv_fn: F) {
        let e_recv = recv_fn();
        let e = self
            .last_event()
            .unwrap_or(&Event::new(self.pid(), self.n_procs()))
            .merge(&e_recv);
        self.push_event(e);
    }
    // Snapshot of event clocks occurred on process
    fn snapshot(&self) -> &[Event];
}

// Helper function
fn pairwise_max<'a, I>(a: I, b: I) -> impl Iterator<Item = usize> + 'a
where
    I: Iterator<Item = &'a usize> + 'a,
{
    a.zip(b).map(|(i, j)| *i.max(j))
}
