pub mod matrix_clock;
pub mod vector_clock;

// PartialOrd because not all clocks are comparable
pub trait LogicalClock: PartialOrd + Clone {
    fn new(i: usize, n_procs: usize) -> Self;
    fn extend(&self) -> Self;
    fn merge(&self, other: &Self) -> Self;
}

pub trait GCClock: LogicalClock {
    fn gc(i: usize, clk: &Self) -> bool;
}

pub trait HasEvents<Event: LogicalClock> {
    fn events(&self) -> &[Event];
    fn events_mut(&mut self) -> &mut Vec<Event>;
}

pub trait OrdProcess<Event>: HasEvents<Event>
where
    Event: LogicalClock,
{
    // Provide a clock for event before executing
    fn exec<F: FnOnce()>(&mut self, f: F) {
        let e = self.events().last().unwrap().extend();
        self.events_mut().push(e);
        f();
    }
    // Sends new clock to receiving party
    fn send<F: FnOnce(Event)>(&mut self, send_fn: F) {
        let e = self.events().last().unwrap().extend();
        self.events_mut().push(e.clone());
        send_fn(e);
    }
    // Receives clock from sending party and updates own clock
    fn recv<F: FnOnce() -> Event>(&mut self, recv_fn: F) {
        let e_recv = recv_fn();
        let e = self.events().last().unwrap().merge(&e_recv);
        self.events_mut().push(e);
    }
    // Snapshot of event clocks occurred on process
    fn snapshot(&self) -> Vec<Event> {
        self.events()[1..].to_owned()
    }
}
