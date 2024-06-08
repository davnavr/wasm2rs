use crate::Errno;
use wasm2rs_rt_memory_typed::{
    slice::{MutSlice, Slice},
    MutPtr, Ptr,
};

/// Result type used by functions in the `wasi_snapshot_preview1` [`Api`].
pub type Result<T> = core::result::Result<T, Errno>;

macro_rules! wasm_layout_check {
    {$($type:ty => $size:literal ^ $align:literal),+} => {$(

impl $type {
    const _SIZE_ALIGN_CHECK: () = {
        if <$type as wasm2rs_rt_memory_typed::Pointee>::SIZE != $size {
            panic!(concat!("expected WASM size to be", stringify!($size)));
        }

        if <$type as wasm2rs_rt_memory_typed::Pointee>::ALIGN.get() != $align {
            panic!(concat!("expected WASM alignment to be", stringify!($align)));
        }
    };
}

    )+};
}

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

/// An [`$inode`] is a "file serial number that is unique within its file system."
///
/// [`$inode`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L326
pub type Inode = u64;

/// A [`$device`] is an "identifier for a device containing a file system."
///
/// [`$device`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L417
pub type Device = u64;

/// A [`$linkcount`] specifies the "number of hard links to an inode."
///
/// [`$linkcount`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L456
pub type LinkCount = u64;

wasm2rs_rt_memory_typed::wasm_transparent_struct! {
    /// A [`$timestamp`] in nanoseconds.
    ///
    /// [`$timestamp`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L14
    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
    pub struct Timestamp(pub u64);
}

impl Timestamp {
    pub(crate) const fn from_i64(timestamp: i64) -> Self {
        Self(timestamp as u64)
    }

    #[allow(missing_docs)]
    pub const fn to_duration(self) -> core::time::Duration {
        core::time::Duration::from_nanos(self.0)
    }
}

impl From<Timestamp> for core::time::Duration {
    fn from(timestamp: Timestamp) -> Self {
        timestamp.to_duration()
    }
}

wasm_layout_check!(Timestamp => 8 ^ 8);

/// A [`$fd`], which represents a file descriptor handle.
///
/// [`$fd`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L277
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Fd(pub u32);

#[allow(missing_docs)]
impl Fd {
    pub const STDIN: Fd = Fd(0);
    pub const STDOUT: Fd = Fd(1);
    pub const STDERR: Fd = Fd(2);

    pub(crate) const fn from_i32(fd: i32) -> Fd {
        Fd(fd as u32)
    }
}

macro_rules! int_enum {
    {$(
        $(#[$meta:meta])*
        enum $name:ident($int:ty) = {
            $($case:ident = $num:literal,)*
        }
    )*} => {$(

$(#[$meta])*
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr($int)]
#[non_exhaustive]
pub enum $name {
    $(
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
#[allow(missing_docs)]
enum ClockId(u32) = {
    Realtime = 0,
    Monotonic = 1,
    ProcessCpuTimeId = 2,
    ThreadCpuTimeId = 3,
}

/// An [`$advice`] provides file access advisory information.
///
/// [`$advice`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L365C1-L380C2
#[allow(missing_docs)]
enum Advice(u8) = {
    Normal = 0,
    Sequential = 1,
    Random = 2,
    WillNeed = 3,
    DontNeed = 4,
    NoReuse = 5,
}

/// A [`$filetype`] indicates the "type of a file descriptor or file."
///
/// [`$filetype`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L329C1-L348C2
#[allow(missing_docs)]
enum FileType(u8) = {
    Unknown = 0,
    BlockDevice = 1,
    CharacterDevice = 2,
    Directory = 3,
    RegularFile = 4,
    SocketDgram = 5,
    SocketStream = 6,
    SymbolicLink = 7,
}

}

macro_rules! int_flags {
    {$(
        $(#[$meta:meta])*
        struct $name:ident($int:ty) = {
            $(
                $(#[$case_meta_name:ident $($case_meta_args:tt)*])*
                $case:ident = $num:literal,
            )*
        }
    )*} => {$(

bitflags::bitflags! {

$(#[$meta])*
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct $name : $int {
    $(
        $(#[$case_meta_name $($case_meta_args)*])*
        const $case = 1 << $num;
    )*
}

}

impl $name {
    #[allow(missing_docs)]
    pub const fn validate(self) -> Result<Self> {
        if Self::all().bits() | self.bits() == Self::all().bits() {
            Ok(self)
        } else {
            Err(Errno::_inval)
        }
    }
}

    )*};
}

int_flags! {

/// Corresponds to [`$fdflags`], "file descriptor flags."
///
/// [`$fdflags`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L329C1-L348C2
struct FdFlags(u16) = {
    #[allow(missing_docs)]
    APPEND = 0,
    #[allow(missing_docs)]
    DSYNC = 1,
    #[allow(missing_docs)]
    NONBLOCK = 2,
    #[allow(missing_docs)]
    RSYNC = 3,
    #[allow(missing_docs)]
    SYNC = 4,
}

/// A [`$rights`] flag specifies "file descriptor rights, determining which actions may be performed."
///
/// [`$rights`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L198C1-L274C2
struct Rights(u64) = {
    /// The right to invoke [`fd_datasync`](Api::fd_datasync).
    DATASYNC = 0,
    #[allow(missing_docs)]
    READ = 1,
    #[allow(missing_docs)]
    SEEK = 2,
    #[allow(missing_docs)]
    FDSTAT_SET_FLAGS = 3,
    #[allow(missing_docs)]
    FD_SYNC = 4,
    // TODO: More Rights!
}

/// Corresponds to [`$fstflags`], indicating "which file time attributes to adjust."
///
/// [`$fstflags`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L420C1-L431C2
struct FstFlags(u16) = {
    #[allow(missing_docs)]
    ATIM = 0,
    #[allow(missing_docs)]
    ATIM_NOW = 1,
    #[allow(missing_docs)]
    MTIM = 2,
    #[allow(missing_docs)]
    MTIM_NOW = 3,
}

}

wasm2rs_rt_memory_typed::wasm_struct! {
    /// A [`$fdstat`] contains "file descriptor attributes".
    ///
    /// [`$fdstat`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L401C1-L413C2
    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    #[allow(missing_docs)]
    pub struct FdStat {
        /// A [`FileType`].
        pub file_type: u8,
        /// An [`FdFlags`] value.
        pub flags: u16,
        /// A [`Rights`] value.
        pub rights_base: u64,
        /// A [`Rights`] value.
        pub rights_inheriting: u64,
    }

    /// A [`$filestat`] contains "file attributes".
    ///
    /// [`$filestat`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L459C1-L478C2
    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    #[allow(missing_docs)]
    pub struct FileStat {
        pub dev: Device,
        pub ino: Inode,
        /// A [`FileType`].
        pub filetype: u8,
        pub nlink: LinkCount,
        pub size: u32,
        pub atim: Timestamp,
        pub mtim: Timestamp,
        pub ctim: Timestamp,
    }

    /// An [`$iovec`] defines "a region of memory for scatter/gather reads."
    ///
    /// [`$iovec`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L280C1-L287C2
    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    #[allow(missing_docs)]
    pub struct IoVec {
        pub buf: MutPtr<u8>,
        pub buf_len: u32,
    }

    /// A [`$prestat_dir`] contains "the contents of a [`$prestat`] when (the) type is
    /// [`preopentype::dir`]."
    ///
    /// [`$prestat_dir`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L734C1-L739C2
    /// [`$prestat`]: PreStat
    /// [`preopentype::dir`]: PreOpenType::Dir
    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    pub struct PreStatDir {
        /// "The length of the directory name for use with [`fd_prestat_dir_name`]."
        ///
        /// [`fd_prestat_dir_name`]: Api::fd_prestat_dir_name()
        pub pr_name_len: u32,
    }
}

impl FdStat {
    #[allow(missing_docs)]
    pub const fn new(
        file_type: FileType,
        flags: FdFlags,
        rights_base: Rights,
        rights_inheriting: Rights,
    ) -> Self {
        Self {
            file_type: file_type as u8,
            flags: flags.bits(),
            rights_base: rights_base.bits(),
            rights_inheriting: rights_inheriting.bits(),
        }
    }
}

wasm_layout_check! {
    FdStat => 24 ^ 8,
    FileStat => 64 ^ 8,
    PreStatDir => 4 ^ 4
}

wasm2rs_rt_memory_typed::wasm_union! {
    /// A [`$prestat`] contains "information about a pre-opened capability."
    ///
    /// [`$prestat`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L742C1-L746C2
    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    #[allow(clippy::exhaustive_enums)]
    pub enum PreStat : u8 {
        #[allow(missing_docs)]
        Dir(PreStatDir) = 0,
    }
}

wasm_layout_check! {
    PreStat => 8 ^ 4
}

/// An array of [`IoVec`]s, used in [`fd_pread`](Api::fd_pread()).
pub type IoVecArray = Slice<IoVec>;

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
    fn fd_prestat_dir_name(&self, mem: &Self::Memory, fd: Fd, path: MutSlice<u8>) -> Result<()> {
        let _ = (mem, fd, path);
        Err(Errno::_nosys)
    }
}
