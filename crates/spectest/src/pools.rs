#[derive(Default)]
pub struct StringPool {
    pool: crossbeam_queue::SegQueue<String>,
}

impl StringPool {
    pub fn take_buffer(&self, capacity: usize) -> String {
        let mut buf = self.pool.pop().unwrap_or_default();
        buf.reserve(capacity);
        buf
    }

    pub fn return_buffer(&self, mut buf: String) {
        if buf.capacity() > 0 {
            buf.clear();
            self.pool.push(buf);
        }
    }
}

pub struct Pools<'a> {
    pub strings: &'a StringPool,
    pub buffers: &'a wasm2rs::buffer::Pool,
}
