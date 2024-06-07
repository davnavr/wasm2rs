use crate::Errno;
use wasm2rs_rt_memory_typed::Ptr;

/// Result type used by functions in the `wasi_snapshot_preview1` [`Api`].
pub type Result<T> = core::result::Result<T, Errno>;

/// Specifies the counts and sizes for CLI argument or environment variable data returned by
/// [`args_sizes_get`] and [`environ_sizes_get`].
///
/// [`args_sizes_get`]: Api::args_sizes_get()
/// [`environ_sizes_get`]: Api::environ_sizes_get()
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DataSizes {
    /// Specifies the number of CLI arguments or environment variables.
    pub count: u32,
    /// Specifies the size of the buffer needed to hold all of the CLI argument or environment
    /// variable data.
    pub buf_size: u32,
}

/// A [`$filesize`], measuring the length of a file or a region into a file.
///
/// [`$timestamp`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L11
pub type FileSize = u64;

/// A [`$timestamp`] in nanoseconds.
///
/// [`$timestamp`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L14
pub type Timestamp = u64;

/// A [`$fd`], which represents a file descriptor handle.
///
/// [`$fd`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L277
pub type Fd = u32;

macro_rules! int_enum {
    {$(
        $(#[$meta:meta])*
        $name:ident($int:ty) = {
            $($case:ident = $num:literal,)*
        }
    )*} => {$(

$(#[$meta])*
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr($int)]
#[non_exhaustive]
pub enum $name {
    $(
        #[allow(missing_docs)]
        $case = $num
    ),*
}

impl TryFrom<$int> for $name {
    type Error = Errno;

    fn try_from(value: $int) -> Result<Self> {
        match value {
            $($num => Ok(Self::$case),)*
            _ => Err(Errno::_inval),
        }
    }
}

    )*};
}

int_enum! {

/// A [`$clockid`] identifies a clock.
///
/// [`$clockid`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L17C1-L32C2
ClockId(u32) = {
    Realtime = 0,
    Monotonic = 1,
    ProcessCpuTimeId = 2,
    ThreadCpuTimeId = 3,
}

/// An [`$advice`] provides file access advisory information.
///
/// [`$advice`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L365C1-L380C2
Advice(u8) = {
    Normal = 0,
    Sequential = 1,
    Random = 2,
    WillNeed = 3,
    DontNeed = 4,
    NoReuse = 5,
}

}

/// Provides the implementation of the [`wasi_snapshot_preview1`] API.
///
/// For most methods, the default implementation simply returns [`Err(Errno::_nosys)`].
///
/// [`wasi_snapshot_preview1`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx
/// [`Err(Errno::_nosys)`]: Errno::_nosys
pub trait Api {
    /// The linear memory that is accessed by WASI functions.
    type Memory: crate::memory::Memory;

    /// Type used to report WebAssembly [`Trap`]s. Rather than trapping, WASI functions are
    /// expected to return an [`Errno`].
    ///
    /// [`Trap`]: wasm2rs_rt_core::trap
    type Trap: wasm2rs_rt_core::trap::TrapInfo;

    /// Reads command-line argument data, writing null-terminated strings and pointers to them into
    /// `argv_buf` and `argv` respectively. The length of the `argv` array and the size of
    /// `argv_buf` should match the [`DataSizes`] returned by [`args_sizes_get`].
    ///
    /// The first argument is expected to be the "name" of the program.
    ///
    /// # See Also
    ///
    /// - [`Wasi::args_get()`](crate::Wasi::args_get()).
    /// - `"args_get"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`args_sizes_get`]: Api::args_sizes_get()
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L17C3-L21C4
    fn args_get(&self, mem: &Self::Memory, argv: Ptr<Ptr<u8>>, argv_buf: Ptr<u8>) -> Result<()> {
        let _ = (mem, argv, argv_buf);
        Err(Errno::_nosys)
    }

    /// Returns the number of command-line arguments and the size of the buffer needed to contain
    /// them.
    ///
    /// # See Also
    ///
    /// - [`Wasi::args_sizes_get()`](crate::Wasi::args_sizes_get()).
    /// - `"args_sizes_get"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L23C3-L29C4
    fn args_sizes_get(&self) -> Result<DataSizes> {
        Err(Errno::_nosys)
    }

    /// Reads environment variable data, writing key/value pairs separated by `=` as
    /// null-terminated strings and pointers to those strings into `environ_buf` and `environ`
    /// respectively. The length of the `environ` array and the size of `environ_buf` should match
    /// the [`DataSizes`] returned by [`environ_sizes_get`].
    ///
    /// # See Also
    ///
    /// - [`Wasi::environ_get()`](crate::Wasi::environ_get()).
    /// - `"environ_get"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`environ_sizes_get`]: Api::environ_sizes_get()
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L33C3-L37C4
    fn environ_get(
        &self,
        mem: &Self::Memory,
        environ: Ptr<Ptr<u8>>,
        environ_buf: Ptr<u8>,
    ) -> Result<()> {
        let _ = (mem, environ, environ_buf);
        Err(Errno::_nosys)
    }

    /// Returns the number of environment variables and the size of the buffer needed to contain
    /// them.
    ///
    /// # See Also
    ///
    /// - [`Wasi::environ_sizes_get()`](crate::Wasi::environ_sizes_get()).
    /// - `"environ_sizes_get"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L39C3-L45C4
    fn environ_sizes_get(&self) -> Result<DataSizes> {
        Err(Errno::_nosys)
    }

    /// Returns the resolution of the given clock.
    ///
    /// # Errors
    ///
    /// If the given clock is unsupported, [`Errno::_inval`] is returned.
    ///
    /// # See Also
    ///
    /// - [`Wasi::clock_res_get()`](crate::Wasi::clock_res_get()).
    /// - `"clock_res_get"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L51C3-L57C4
    fn clock_res_get(&self, id: ClockId) -> Result<Timestamp> {
        let _ = id;
        Err(Errno::_inval)
    }

    /// Returns the time value for the given clock.
    ///
    /// # See Also
    ///
    /// - [`Wasi::clock_time_get()`](crate::Wasi::clock_time_get()).
    /// - `"clock_time_get"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L60C3-L68C4
    fn clock_time_get(&self, id: ClockId, precision: Timestamp) -> Result<Timestamp> {
        let _ = (id, precision);
        Err(Errno::_inval) // `inval` also used by `clock_res_get`
    }

    /// Used by the application to indicate how it the given file descriptor will be used.
    ///
    /// # See Also
    ///
    /// - [`Wasi::fd_advise()`](crate::Wasi::fd_advise()).
    /// - `"fd_advise"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L72C3-L81C4
    fn fd_advise(&self, fd: Fd, offset: FileSize, len: FileSize, advice: Advice) -> Result<()> {
        let _ = (fd, offset, len, advice);
        Err(Errno::_nosys)
    }

    /// Allocates additional space in a file.
    ///
    /// # See Also
    ///
    /// - [`Wasi::fd_allocate()`](crate::Wasi::fd_allocate()).
    /// - `"fd_allocate"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L85C3-L92C4
    fn fd_allocate(&self, fd: Fd, offset: FileSize, len: FileSize) -> Result<()> {
        let _ = (fd, offset, len);
        Err(Errno::_nosys)
    }

    /// Closes a file descriptor.
    ///
    /// # See Also
    ///
    /// - [`Wasi::fd_close()`](crate::Wasi::fd_close()).
    /// - `"fd_close"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L96C3-L99C4
    fn fd_close(&self, fd: Fd) -> Result<()> {
        let _ = fd;
        Err(Errno::_nosys)
    }

    /// "Synchronizes the data of a file to disk."
    ///
    /// # See Also
    ///
    /// - [`Wasi::fd_datasync()`](crate::Wasi::fd_datasync()).
    /// - `"fd_datasync"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: fd_datasync
    fn fd_datasync(&self, fd: Fd) -> Result<()> {
        let _ = fd;
        Err(Errno::_nosys)
    }

    // fn fd_fdstat_get
}
