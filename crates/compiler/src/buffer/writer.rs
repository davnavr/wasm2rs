/// Struct for writing bytes into a buffer.
#[must_use = "call `Writer::finish()`"]
pub struct Writer<'a> {
    output: Vec<bytes::Bytes>,
    buffer: bytes::BytesMut,
    pool: &'a crate::buffer::Pool,
}

impl<'a> Writer<'a> {
    const DEFAULT_BUFFER_CAPACITY: usize = 512;

    /// Creates a new buffer that bytes can be written to, taking new buffers from the given `pool`.
    pub fn new(pool: &'a crate::buffer::Pool) -> Self {
        Self {
            output: Vec::new(),
            buffer: pool.take_buffer(0),
            pool,
        }
    }

    /// Gets the pool that new buffers are taken from.
    pub fn pool(&self) -> &'a crate::buffer::Pool {
        self.pool
    }

    /// Returns a vector containing all of the bytes that were written.
    ///
    /// The writers current buffer is also returned to the pool.
    pub fn finish(mut self) -> Vec<bytes::Bytes> {
        let to_append = self.buffer.split_to(self.buffer.len());
        if !to_append.is_empty() {
            self.output.reserve_exact(1);
            self.output.push(to_append.freeze());
        }

        self.pool.return_buffer(self.buffer);
        self.output
    }

    /// Ensures that the `buffer` is not empty, taking or allocating a new buffer as necessary and
    /// commiting any remaining bytes in the old buffer that need to be output.
    ///
    /// Returns the number of bytes that can be written into before requiring another `reserve()`
    /// call.
    ///
    /// The `more` parameter is used as the minimum capacity when a new buffer is required.
    fn reserve(&mut self, more: usize) -> usize {
        if self.buffer.len() == self.buffer.capacity() {
            let new_capacity;
            if !self.buffer.is_empty() {
                new_capacity = self.buffer.capacity().saturating_mul(2);

                // Buffer contains data that needs to be output
                self.output.push(std::mem::take(&mut self.buffer).freeze());
            } else {
                new_capacity = Self::DEFAULT_BUFFER_CAPACITY;
            }

            self.buffer = self.pool.take_buffer(new_capacity.max(more));
            debug_assert!(self.buffer.is_empty());
        }

        debug_assert_ne!(self.buffer.capacity(), 0);

        self.buffer.capacity()
    }

    /// Writes all of the `bytes` into this buffer.
    pub fn write(&mut self, mut bytes: &[u8]) {
        while !bytes.is_empty() {
            let to_write = self.reserve(bytes.len()).min(bytes.len());
            self.buffer.extend_from_slice(&bytes[..to_write]);
            bytes = &bytes[to_write..];
        }
    }

    /// Writes all of a string's bytes into this buffer.
    pub fn write_str(&mut self, s: &str) {
        self.write(s.as_bytes());
    }
}

impl std::fmt::Write for Writer<'_> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.write_str(s);
        Ok(())
    }
}

impl std::fmt::Debug for Writer<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(&self.output)
            .entry(&self.buffer)
            .finish()
    }
}
