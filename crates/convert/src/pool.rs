//! Allows reusing objects.

// TODO: Use Mutex<Vec<T>> instead, have thread_local::ThreadLocal<Vec<T>> alongside
#[cfg(feature = "crossbeam-queue")]
pub(crate) type Pool<T> = crossbeam_queue::SegQueue<T>;

#[cfg(not(feature = "crossbeam-queue"))]
pub(crate) struct Pool<T> {
    pool: std::cell::RefCell<Vec<T>>,
}

#[cfg(not(feature = "crossbeam-queue"))]
impl<T> Pool<T> {
    pub(crate) const fn new() -> Self {
        Self {
            pool: std::cell::RefCell::new(Vec::new()),
        }
    }

    pub(crate) fn pop(&self) -> Option<T> {
        self.pool.borrow_mut().pop()
    }

    pub(crate) fn push(&self, value: T) {
        self.pool.borrow_mut().push(value)
    }
}

#[cfg(not(feature = "crossbeam-queue"))]
impl<T> Default for Pool<T> {
    fn default() -> Self {
        Self::new()
    }
}
