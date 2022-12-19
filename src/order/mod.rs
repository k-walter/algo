pub mod chandy_lamport;
pub mod matrix_clock;
pub mod vector_clock;

// PartialOrd because not all clocks are comparable
pub trait CausalOrd: PartialOrd {}

pub trait LogicalClock: Clone {
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
    fn events(&self) -> &[Event];
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
    // Expects a function that sends clock to receiving processes in a lossless FIFO channel
    fn send<F: FnOnce(Event)>(&mut self, send_fn: F) {
        let e = self
            .last_event()
            .unwrap_or(&Event::new(self.pid(), self.n_procs()))
            .extend();
        self.push_event(e.clone());
        send_fn(e);
    }
    // Receives clock from sending party and updates own clock
    // Expects a function that receives clocks from any other process in a lossless FIFO channel
    fn recv<F: FnOnce() -> Event>(&mut self, recv_fn: F) {
        let e_recv = recv_fn();
        let e = self
            .last_event()
            .unwrap_or(&Event::new(self.pid(), self.n_procs()))
            .merge(&e_recv);
        self.push_event(e);
    }
}

// Helper function
fn pairwise_max<'a, I>(a: I, b: I) -> impl Iterator<Item = usize> + 'a
where
    I: Iterator<Item = &'a usize> + 'a,
{
    a.zip(b).map(|(i, j)| *i.max(j))
}
