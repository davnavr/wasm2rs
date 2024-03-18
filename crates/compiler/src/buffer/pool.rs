/// Stores byte buffers for later use.
#[derive(Default)]
pub struct BufferPool {
    pool: Box<crossbeam_queue::SegQueue<bytes::BytesMut>>,
}

impl BufferPool {
    /// Gets an new empty buffer.
    ///
    /// If no buffers are currently in the pool, a new one is returned with the specified capacity.
    pub fn take_buffer(&self, new_buffer_capacity: usize) -> bytes::BytesMut {
        match self.pool.pop() {
            Some(buf) => buf,
            None => bytes::BytesMut::with_capacity(new_buffer_capacity),
        }
    }

    /// Attempts to move a buffer back into the pool.
    pub fn return_buffer(&self, mut buffer: bytes::BytesMut) {
        buffer.clear();
        self.pool.push(buffer)
    }
}

impl std::fmt::Debug for BufferPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BufferPool").finish_non_exhaustive()
    }
}
