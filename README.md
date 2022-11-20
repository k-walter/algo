# Rust Algorithms And Data Structures (RADS)
Goals
- Implement some cool (mostly parallel) algorithms and data structures
- Learn rust
- Practice TDD


## Parallel RADS
The implementation of these algorithms require your platform's atomic support. `O(n)` refers to the number of processes

### Synchronization
Mutex protects a critical section from concurrent access. If you must guarantee entering the critical section eventually...  
- [x] [NoStarveMutex Trait](src/sync/mod.rs) locks in a bounded time (i.e. realtime) of `O(n)`
- [x] [Peterson's Algorithm](src/sync/peterson.rs) for starvation-free binary mutual exclusion
- [x] [Lamport's Bakery](src/sync/lamports_bakery.rs) for starvation-free n-ary mutual exclusion (with `O(n)` time and space)

### Causal Ordering
Physical Clocks are hard (impossible?) to synchronize without errors. If you must know whether event `s` "causes" /
"happens before" event `t`...
- [x] [Logical Clock Trait](src/order/mod.rs) relax constraints enough to agree on the order of causal events
- [x] [Vector Clock](src/order/vector_clock.rs) compares iff event `s` "happens before" event `t` (with `O(n)` time and space)

## TODO
### CS4231 Parallel & Distributed Algorithms
#### Causal Ordering
- [ ] Matrix Clock
- [ ] Chandy & Lamport's Protocol (Consistent Global Snapshot) 
- [ ] Causal Order Delivery
- [ ] Skeen's Algorithm (Total Order Broadcast)

#### Distributed Consensus
No node/link failure
- [ ] Skeen's Algorithm (Total Order Broadcast)
- [ ] Chang-Roberts Algorithm (Leader Election on Ring)
- [ ] Distributed Spanning Tree

Crash Failure, Reliable Channel, Synchronous
- [ ] F + 1 Round Protocol

No Failure, Unreliable Channel, Synchronous
- [ ] P(fail) = 1 / R Randomized Algorithm

Crash Failure, Reliable Channel, Asynchronous (FLP Impossibility Theorem)

Byzantine Failure, Reliable Channel, Synchronous
- [ ] N >= 4F + 1 Coordinator Protocol

#### Self-Stabilizing
- [ ] Self-Stabilizing Spanning Tree

### CS3223
### CS4224

### Hash Tables
- [ ] Cuckoo Hashing
- [ ] Robin Hood Hashing
- [ ] Sliding Bloom Filter
- [ ] https://programming.guide/hash-tables.html
