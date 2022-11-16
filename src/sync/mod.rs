pub mod peterson;

pub trait Mutex
{
    type Output: Mutex; // usually Self
    fn acquire(&self) -> MutexGuard<Self::Output>;
    fn release(&self);
}

pub struct MutexGuard<'a, T> where T: Mutex {
    mutex: &'a T,
}
impl<T> Drop for MutexGuard<'_, T> where T: Mutex {
    fn drop(&mut self) {
        self.mutex.release()
    }
}
