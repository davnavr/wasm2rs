/// Writes bytes into [`Buffer`]s.
///
/// [`Buffer`]: crate::buffer::Buffer
#[must_use = "call `Writer::finish()`"]
pub struct Writer<'a> {
    output: Vec<bytes::BytesMut>,
    buffer: bytes::BytesMut,
    pool: &'a crate::buffer::Pool,
}

impl<'a> Writer<'a> {
    const DEFAULT_BUFFER_CAPACITY: usize = 512;

    /// Creates a new [`Writer`] to write bytes into, taking new [`Buffer`]s from the given `pool`.
    ///
    /// [`Buffer`]: crate::buffer::Buffer
    pub fn new(pool: &'a crate::buffer::Pool) -> Self {
        Self {
            output: Vec::new(),
            buffer: pool.take_buffer(0),
            pool,
        }
    }

    /// Gets the pool that new [`Buffer`]s are taken from.
    ///
    /// [`Buffer`]: crate::buffer::Buffer
    pub fn pool(&self) -> &'a crate::buffer::Pool {
        self.pool
    }

    /// Returns a vector containing all of the bytes that were written.
    ///
    /// The writer's current [`Buffer`] is also returned to the pool, allowing any unused space in
    /// the [`Buffer`] to be used by a later [`Writer`].
    ///
    /// [`Buffer`]: crate::buffer::Buffer
    pub fn finish(mut self) -> Vec<bytes::BytesMut> {
        let to_append = self.buffer.split_to(self.buffer.len());
        if !to_append.is_empty() {
            self.output.reserve_exact(1);
            self.output.push(to_append);
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
                self.output.push(std::mem::take(&mut self.buffer));
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

    /// Allows using a [`Writer`] with the [`write!`] macro.
    ///
    /// # Panics
    ///
    /// Panics if some underlying write operation (e.g. [`Display::fmt()`]) returned an [`Error`].
    ///
    /// [`Display::fmt()`]: std::fmt::Display::fmt()
    /// [`Error`]: std::fmt::Error
    pub fn write_fmt(&mut self, args: std::fmt::Arguments) {
        <Self as std::fmt::Write>::write_fmt(self, args).unwrap();
    }
}

impl std::fmt::Write for Writer<'_> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.write_str(s);
        Ok(())
    }
}

impl crate::write::Write for Writer<'_> {
    fn write_str(&mut self, s: &str) {
        <Self>::write_str(self, s);
    }

    fn write_fmt(&mut self, args: std::fmt::Arguments) {
        <Self>::write_fmt(self, args)
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
