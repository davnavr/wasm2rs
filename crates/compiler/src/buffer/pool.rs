/// Stores byte buffers for later use.
#[derive(Default)]
pub struct Pool {
    pool: crossbeam_queue::SegQueue<bytes::BytesMut>,
}

impl Pool {
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

    pub(crate) fn return_buffers_many<B>(&self, buffers: B)
    where
        B: IntoIterator<Item = bytes::BytesMut>,
    {
        buffers
            .into_iter()
            .for_each(|buffer| self.return_buffer(buffer));
    }
}

impl std::fmt::Debug for Pool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BufferPool").finish_non_exhaustive()
    }
}
