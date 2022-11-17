pub mod lamports_bakery;
pub mod peterson;

// Mutex lives longer than guard, because guard releases mutex
pub trait Mutex<'a, Guard: 'a>
where
    // Only allow releasing after acquiring guard
    // drop(&mut) guarantees release once at compile time
    Guard: Drop,
{
    // &mut guarantees acquire once within the same or narrower scope at compile time
    fn acquire(&'a mut self) -> Guard;
}
