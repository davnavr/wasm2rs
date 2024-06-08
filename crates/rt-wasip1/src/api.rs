//! Contains the [`Api`] trait as well as types modelling those described in
//! [`wasi_snapshot_preview1.witx`].
//!
//! [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx

#[doc(no_inline)]
pub use wasm2rs_rt_memory_typed::{MutPtr, Ptr};

mod errno;
mod types;

pub use errno::Errno;
pub use types::{
    Advice, CIoVec, CIoVecArray, ClockId, DataSizes, Device, Fd, FdFlags, FdStat, FileSize,
    FileStat, FileType, FstFlags, Inode, IoVec, IoVecArray, LinkCount, PreStat, PreStatDir, Result,
    Rights, Timestamp,
};

use wasm2rs_rt_memory_typed::slice;

/// Provides the implementation of the [`wasi_snapshot_preview1`] API.
///
/// For all methods, the default implementation simply returns [`Err(Errno::_nosys)`].
///
/// [`wasi_snapshot_preview1`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx
/// [`Err(Errno::_nosys)`]: Errno::_nosys
pub trait Api {
    /// The linear memory that is accessed by WASI functions.
    type Memory: wasm2rs_rt_memory::Memory;

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
    fn args_get(
        &self,
        mem: &Self::Memory,
        argv: MutPtr<MutPtr<u8>>,
        argv_buf: MutPtr<u8>,
    ) -> Result<()> {
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
        environ: MutPtr<MutPtr<u8>>,
        environ_buf: MutPtr<u8>,
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
        Err(Errno::_nosys)
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
        Err(Errno::_nosys)
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
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L103C3-L106C4
    fn fd_datasync(&self, fd: Fd) -> Result<()> {
        let _ = fd;
        Err(Errno::_nosys)
    }

    /// "Get the attributes of a file descriptor."
    ///
    /// # See Also
    ///
    /// - [`Wasi::fd_fdstat_get()`](crate::Wasi::fd_fdstat_get()).
    /// - `"fd_fdstat_get"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L110C3-L115C4
    fn fd_fdstat_get(&self, fd: Fd) -> Result<FdStat> {
        let _ = fd;
        Err(Errno::_nosys)
    }

    /// "Adjust the flags associated with a file descriptor."
    ///
    /// # See Also
    ///
    /// - [`Wasi::fd_fdstat_set_flags()`](crate::Wasi::fd_fdstat_set_flags()).
    /// - `"fd_fdstat_set_flags"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L117C3-L124C4
    fn fd_fdstat_set_flags(&self, fd: Fd, flags: FdFlags) -> Result<()> {
        let _ = (fd, flags);
        Err(Errno::_nosys)
    }

    /// "Adjust the rights associated with a file descriptor."
    ///
    /// # See Also
    ///
    /// - [`Wasi::fd_fdstat_set_rights()`](crate::Wasi::fd_fdstat_set_rights()).
    /// - `"fd_fdstat_set_rights"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L128C3-L134C4
    fn fd_fdstat_set_rights(
        &self,
        fd: Fd,
        rights_base: Rights,
        rights_inheriting: Rights,
    ) -> Result<()> {
        let _ = (fd, rights_base, rights_inheriting);
        Err(Errno::_nosys)
    }

    /// "Return the attributes of an open file."
    ///
    /// # See Also
    ///
    /// - [`Wasi::fd_filestat_get()`](crate::Wasi::fd_filestat_get()).
    /// - `"fd_filestat_get"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L137C3-L142C4
    fn fd_filestat_get(&self, fd: Fd) -> Result<FileStat> {
        let _ = fd;
        Err(Errno::_nosys)
    }

    /// "Adjust the size of an open file. If this increases the file's size, the extra bytes are
    /// filled with zeros."
    ///
    /// # See Also
    ///
    /// - [`Wasi::fd_filestat_set_size()`](crate::Wasi::fd_filestat_set_size()).
    /// - `"fd_filestat_set_size"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L146C3-L151C4
    fn fd_filestat_set_size(&self, fd: Fd, size: FileSize) -> Result<()> {
        let _ = (fd, size);
        Err(Errno::_nosys)
    }

    /// "Adjust the timestamps of an open file or directory."
    ///
    /// # See Also
    ///
    /// - [`Wasi::fd_filestat_set_times()`](crate::Wasi::fd_filestat_set_times()).
    /// - `"fd_filestat_set_times"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L155C3-L164C4
    fn fd_filestat_set_times(
        &self,
        fd: Fd,
        atim: Timestamp,
        mtim: Timestamp,
        fst_flags: FstFlags,
    ) -> Result<()> {
        let _ = (fd, atim, mtim, fst_flags);
        Err(Errno::_nosys)
    }

    /// "Read from a file descriptor, without using and updating the file descriptor's offset."
    ///
    /// # See Also
    ///
    /// - [`Wasi::fd_pread()`](crate::Wasi::fd_pread()).
    /// - `"fd_pread"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L168C3-L177C4
    fn fd_pread(
        &self,
        mem: &Self::Memory,
        fd: Fd,
        iovs: IoVecArray,
        offset: FileSize,
    ) -> Result<u32> {
        let _ = (mem, fd, iovs, offset);
        Err(Errno::_nosys)
    }

    /// "Return a description of the given preopened file descriptor."
    ///
    /// # See Also
    ///
    /// - [`Wasi::fd_prestat_get()`](crate::Wasi::fd_prestat_get()).
    /// - `"fd_prestat_get"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L180C3-L185C4
    fn fd_prestat_get(&self, fd: Fd) -> Result<PreStat> {
        let _ = fd;
        Err(Errno::_nosys)
    }

    /// "Return a description of the given preopened file descriptor."
    ///
    /// # See Also
    ///
    /// - [`Wasi::fd_prestat_dir_name()`](crate::Wasi::fd_prestat_dir_name()).
    /// - `"fd_prestat_dir_name"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L188C3-L194C4
    fn fd_prestat_dir_name(
        &self,
        mem: &Self::Memory,
        fd: Fd,
        path: slice::MutSlice<u8>,
    ) -> Result<()> {
        let _ = (mem, fd, path);
        Err(Errno::_nosys)
    }

    /// "Write to a file descriptor, without using and updating the file descriptor's offset."
    ///
    /// # See Also
    ///
    /// - [`Wasi::fd_pwrite()`](crate::Wasi::fd_pwrite()).
    /// - `"fd_pwrite"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L198C3-L207C4
    fn fd_pwrite(
        &self,
        mem: &Self::Memory,
        fd: Fd,
        iovs: CIoVecArray,
        offset: FileSize,
    ) -> Result<u32> {
        let _ = (mem, fd, iovs, offset);
        Err(Errno::_nosys)
    }
}
