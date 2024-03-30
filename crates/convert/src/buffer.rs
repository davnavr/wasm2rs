//! Manipulation of byte [`Buffer`]s.
//!
//! To allow converting a WebAssembly module in parallel, `wasm2rs` uses [`Buffer`]s to write
//! Rust source code to, then writes all of those [`Buffer`]s into the output with
//! [`write_all_vectored()`].

mod pool;
mod writer;

#[doc(no_inline)]
pub use bytes::BytesMut as Buffer;
pub use pool::Pool;
pub use writer::Writer;

/// Helper function to write all of the content from the given [`Buffer`]s.
pub fn write_all_vectored<'a>(
    output: &mut dyn std::io::Write,
    content: &'a [Buffer],
    io_buffers: &mut Vec<std::io::IoSlice<'a>>,
) -> std::io::Result<()> {
    let buffer_count = content.len();
    io_buffers.clear();
    io_buffers.extend(content.iter().map(|b| std::io::IoSlice::new(b)));
    let mut buffers = io_buffers.as_mut_slice();

    while !buffers.is_empty() {
        let mut num_written = match output.write_vectored(buffers) {
            Ok(0) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::WriteZero,
                    "could not write all buffers",
                ))
            }
            Ok(amount) => amount,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };

        while !buffers.is_empty() && buffers[0].len() <= num_written {
            num_written -= buffers[0].len();
            buffers = &mut buffers[1..];
        }

        let remaining_buffers = buffers.len();
        if let Some(new_head) = buffers.get_mut(0) {
            *new_head = std::io::IoSlice::new(&content[buffer_count - remaining_buffers]);
        }
    }

    Ok(())
}
