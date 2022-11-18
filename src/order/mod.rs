pub mod vector_clock;

// PartialOrd because not all clocks are comparable
pub trait LogicalClock: PartialOrd + Clone {
    fn new(i: usize, n_procs: usize) -> Self;
    fn extend(&self) -> Self;
    fn merge(&self, other: &Self) -> Self;
}

pub struct Process<Event: LogicalClock> {
    events: Vec<Event>,
}
impl<Event> Process<Event>
where
    Event: LogicalClock,
{
    pub fn new(i: usize, n_procs: usize) -> Self {
        Self {
            events: Vec::from([Event::new(i, n_procs)]),
        }
    }
    // Provide a clock for event before executing
    pub fn exec<F: FnOnce()>(&mut self, f: F) {
        let e = self.events.last().unwrap().extend();
        self.events.push(e);
        f();
    }
    // Sends new clock to receiving party
    pub fn send<F: FnOnce(Event)>(&mut self, send_fn: F) {
        let e = self.events.last().unwrap().extend();
        self.events.push(e.clone());
        send_fn(e);
    }
    // Receives clock from sending party and updates own clock
    pub fn recv<F: FnOnce() -> Event>(&mut self, recv_fn: F) {
        let e_recv = recv_fn();
        let e = self.events.last().unwrap().merge(&e_recv);
        self.events.push(e);
    }
    // Snapshot of event clocks occurred on process
    pub fn snapshot(&self) -> Vec<Event> {
        self.events[1..].to_owned()
    }
}
