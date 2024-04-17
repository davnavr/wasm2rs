/// Stores byte buffers for later use.
pub struct Pool {
    pool: crate::pool::Pool<bytes::BytesMut>,
}

impl Default for Pool {
    fn default() -> Self {
        Self::new()
    }
}

impl Pool {
    pub(crate) const fn new() -> Self {
        Self {
            pool: crate::pool::Pool::new(),
        }
    }

    /// Gets an new empty buffer.
    ///
    /// If no buffers are currently in the pool, a new one is returned with the specified capacity.
    ///
    /// Buffers originating from the pool are guaranteed to not have a capacity of zero.
    pub fn take_buffer(&self, new_buffer_capacity: usize) -> bytes::BytesMut {
        match self.pool.pop() {
            Some(buf) => buf,
            None => bytes::BytesMut::with_capacity(new_buffer_capacity),
        }
    }

    /// Attempts to move a buffer back into the pool.
    pub fn return_buffer(&self, mut buffer: bytes::BytesMut) {
        if buffer.capacity() > 0 {
            buffer.clear();
            self.pool.push(buffer)
        }
    }
}

impl std::fmt::Debug for Pool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BufferPool").finish_non_exhaustive()
    }
}
