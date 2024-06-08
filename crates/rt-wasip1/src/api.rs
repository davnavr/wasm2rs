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
    Advice, CIoVec, CIoVecArray, ClockId, DataSizes, Device, DirCookie, Event, EventFdReadWrite,
    EventPoll, EventRwFlags, EventType, Fd, FdFlags, FdStat, FileDelta, FileSize, FileStat,
    FileType, FstFlags, Inode, IoVec, IoVecArray, LinkCount, LookupFlags, OFlags, Path, PreStat,
    PreStatDir, Result, Rights, SubClockFlags, Subscription, SubscriptionClock,
    SubscriptionFdReadWrite, SubscriptionU, Timestamp, UserData, Whence,
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

    /// "Read from a file descriptor."
    ///
    /// # See Also
    ///
    /// - [`Wasi::fd_read()`](crate::Wasi::fd_read()).
    /// - `"fd_read"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L211C3-L218C4
    fn fd_read(&self, mem: &Self::Memory, fd: Fd, iovs: IoVecArray) -> Result<u32> {
        let _ = (mem, fd, iovs);
        Err(Errno::_nosys)
    }

    /// "Read directory entries from a directory."
    ///
    /// # See Also
    ///
    /// - [`Wasi::fd_readdir()`](crate::Wasi::fd_readdir()).
    /// - `"fd_readdir"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L230C3-L240C4
    fn fd_readdir(
        &self,
        mem: &Self::Memory,
        fd: Fd,
        buf: slice::MutSlice<u8>,
        cookie: DirCookie,
    ) -> Result<u32> {
        // TODO: Make a newtype helper for writing DirEntry + names into `buf`.
        let _ = (mem, fd, buf, cookie);
        Err(Errno::_nosys)
    }

    /// "Atomically replace a file descriptor by renumbering another file descriptor."
    ///
    /// # See Also
    ///
    /// - [`Wasi::fd_renumber()`](crate::Wasi::fd_renumber()).
    /// - `"fd_renumber"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L252C3-L257C4
    fn fd_renumber(&self, fd: Fd, to: Fd) -> Result<()> {
        let _ = (fd, to);
        Err(Errno::_nosys)
    }

    /// "Move the offset of a file descriptor."
    ///
    /// # See Also
    ///
    /// - [`Wasi::fd_seek()`](crate::Wasi::fd_seek()).
    /// - `"fd_seek"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L261C3-L270C4
    fn fd_seek(&self, fd: Fd, offset: FileDelta, whence: Whence) -> Result<FileSize> {
        let _ = (fd, offset, whence);
        Err(Errno::_nosys)
    }

    /// "Synchronize the data and metadata of a file to disk."
    ///
    /// # See Also
    ///
    /// - [`Wasi::fd_sync()`](crate::Wasi::fd_sync()).
    /// - `"fd_sync"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]:https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L274C3-L277C4
    fn fd_sync(&self, fd: Fd) -> Result<()> {
        let _ = fd;
        Err(Errno::_nosys)
    }

    /// "Return the current offset of a file descriptor."
    ///
    /// # See Also
    ///
    /// - [`Wasi::fd_tell()`](crate::Wasi::fd_tell()).
    /// - `"fd_tell"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L281C3-L286C4
    fn fd_tell(&self, fd: Fd) -> Result<FileSize> {
        let _ = fd;
        Err(Errno::_nosys)
    }

    /// "Write to a file descriptor."
    ///
    /// # See Also
    ///
    /// - [`Wasi::fd_write()`](crate::Wasi::fd_write()).
    /// - `"fd_write"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L290C3-L297C4
    fn fd_write(&self, mem: &Self::Memory, fd: Fd, iovs: CIoVecArray) -> Result<FileSize> {
        let _ = (fd, mem, iovs);
        Err(Errno::_nosys)
    }

    /// "Create a directory."
    ///
    /// # See Also
    ///
    /// - [`Wasi::path_create_directory()`](crate::Wasi::path_create_directory()).
    /// - `"path_create_directory"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L301C3-L306C4
    fn path_create_directory(&self, mem: &Self::Memory, fd: Fd, path: Path) -> Result<()> {
        let _ = (mem, fd, path);
        Err(Errno::_nosys)
    }

    /// "Create a directory."
    ///
    /// # See Also
    ///
    /// - [`Wasi::path_filestat_get()`](crate::Wasi::path_filestat_get()).
    /// - `"path_filestat_get"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L310C3-L319C4
    fn path_filestat_get(
        &self,
        mem: &Self::Memory,
        fd: Fd,
        flags: LookupFlags,
        path: Path,
    ) -> Result<FileStat> {
        let _ = (mem, fd, flags, path);
        Err(Errno::_nosys)
    }

    /// "Adjust the timestamps of a file or directory."
    ///
    /// # See Also
    ///
    /// - [`Wasi::path_filestat_set_times()`](crate::Wasi::path_filestat_set_times()).
    /// - `"path_filestat_set_times"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L323C3-L336C4
    #[allow(clippy::too_many_arguments)]
    fn path_filestat_set_times(
        &self,
        mem: &Self::Memory,
        fd: Fd,
        flags: LookupFlags,
        path: Path,
        atim: Timestamp,
        mtim: Timestamp,
        fst_flags: FstFlags,
    ) -> Result<()> {
        let _ = (mem, fd, flags, path, atim, mtim, fst_flags);
        Err(Errno::_nosys)
    }

    /// "Create a hard link."
    ///
    /// # See Also
    ///
    /// - [`Wasi::path_link()`](crate::Wasi::path_link()).
    /// - `"path_link"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L340C3-L351C4
    fn path_link(
        &self,
        mem: &Self::Memory,
        old_fd: Fd,
        old_flags: LookupFlags,
        old_path: Path,
        new_fd: Fd,
        new_path: Path,
    ) -> Result<()> {
        let _ = (mem, old_fd, old_flags, old_path, new_fd, new_path);
        Err(Errno::_nosys)
    }

    /// "Open a file or directory."
    ///
    /// # See Also
    ///
    /// - [`Wasi::path_open()`](crate::Wasi::path_open()).
    /// - `"path_open"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L362C3-L385C4
    #[allow(clippy::too_many_arguments)]
    fn path_open(
        &self,
        mem: &Self::Memory,
        dir_fd: Fd,
        dir_flags: LookupFlags,
        path: Path,
        o_flags: OFlags,
        rights_base: Rights,
        rights_inheriting: Rights,
        fd_flags: FdFlags,
    ) -> Result<Fd> {
        let _ = mem;
        let _ = dir_fd;
        let _ = dir_flags;
        let _ = path;
        let _ = o_flags;
        let _ = rights_base;
        let _ = rights_inheriting;
        let _ = fd_flags;
        Err(Errno::_nosys)
    }

    /// "Read the contents of a symbolic link."
    ///
    /// # See Also
    ///
    /// - [`Wasi::path_readlink()`](crate::Wasi::path_readlink()).
    /// - `"path_readlink"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L389C3-L399C4
    fn path_readlink(
        &self,
        mem: &Self::Memory,
        dir_fd: Fd,
        path: Path,
        buf: slice::MutSlice<u8>,
    ) -> Result<u32> {
        let _ = (mem, dir_fd, path, buf);
        Err(Errno::_nosys)
    }

    /// "Remove a directory."
    ///
    /// # Errors
    ///
    /// If the directory is not empty, [`Errno::_notempty`] is returned.
    ///
    /// # See Also
    ///
    /// - [`Wasi::path_remove_directory()`](crate::Wasi::path_remove_directory()).
    /// - `"path_remove_directory"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L404C3-L409C4
    fn path_remove_directory(&self, mem: &Self::Memory, fd: Fd, path: Path) -> Result<u32> {
        let _ = (mem, fd, path);
        Err(Errno::_nosys)
    }

    /// "Rename a file or directory."
    ///
    /// # See Also
    ///
    /// - [`Wasi::path_rename()`](crate::Wasi::path_rename()).
    /// - `"path_rename"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L413C3-L422C4
    fn path_rename(
        &self,
        mem: &Self::Memory,
        old_fd: Fd,
        old_path: Path,
        new_fd: Fd,
        new_path: Path,
    ) -> Result<()> {
        let _ = (mem, old_fd, old_path, new_fd, new_path);
        Err(Errno::_nosys)
    }

    /// "Create a symbolic link."
    ///
    /// # See Also
    ///
    /// - [`Wasi::path_symlink()`](crate::Wasi::path_symlink()).
    /// - `"path_symlink"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L426C3-L433C4
    fn path_symlink(
        &self,
        mem: &Self::Memory,
        old_path: Path,
        fd: Fd,
        new_path: Path,
    ) -> Result<()> {
        let _ = (mem, old_path, fd, new_path);
        Err(Errno::_nosys)
    }

    /// "Unlink a file."
    ///
    /// # Errors
    ///
    /// Returns [`Errno::_isdir`] "if the path refers to a directory."
    ///
    /// # See Also
    ///
    /// - [`Wasi::path_unlink_file()`](crate::Wasi::path_unlink_file()).
    /// - `"path_unlink_file"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L439C3-L444C4
    fn path_unlink_file(&self, mem: &Self::Memory, fd: Fd, path: Path) -> Result<()> {
        let _ = (mem, fd, path);
        Err(Errno::_nosys)
    }

    /// "Concurrently poll for the occurrence of a set of events."
    ///
    /// # See Also
    ///
    /// - [`Wasi::poll_oneoff()`](crate::Wasi::poll_oneoff()).
    /// - `"poll_oneoff"` in [`wasi_snapshot_preview1.witx`]
    ///
    /// [`wasi_snapshot_preview1.witx`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx#L447C3-L457C4
    fn poll_oneoff(&self, mem: &Self::Memory, events: EventPoll) -> Result<u32> {
        let _ = (mem, events);
        Err(Errno::_nosys)
    }
}
