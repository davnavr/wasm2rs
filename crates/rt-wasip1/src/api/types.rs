use crate::api::{Errno, MutPtr, Ptr};
use wasm2rs_rt_memory_typed::slice;

/// Result type used by functions in the `wasi_snapshot_preview1` [`Api`].
///
/// [`Api`]: crate::api::Api
pub type Result<T> = core::result::Result<T, Errno>;

/// A string type representing paths to files or directories in the WASI [`Api`].
///
/// The contents of the string are typically assumed to be UTF-8.
///
/// [`Api`]: crate::api::Api
pub type Path = slice::Slice<u8>;

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
/// [`args_sizes_get`]: crate::api::Api::args_sizes_get()
/// [`environ_sizes_get`]: crate::api::Api::environ_sizes_get()
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

/// A [`$filedelta`] is a "relative offset within a file."
///
/// [`$filedelta`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L303
pub type FileDelta = i64;

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

/// A [`$userdata`] value contains a "user-provided value that may be attached to objects that is
/// retained when extracted from the implementation." See the documentation for the
/// [`Subscription`] struct for more information.
///
/// [`$userdata`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L482
pub type UserData = u64;

wasm2rs_rt_memory_typed::wasm_transparent_struct! {

/// A [`$fd`], which represents a file descriptor handle.
///
/// [`$fd`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L277
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct Fd(pub u32);

/// A [`$timestamp`] in nanoseconds.
///
/// [`$timestamp`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L14
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct Timestamp(pub u64);

/// A [`$dircookie`] is "a reference to the offset of a directory entry."
///
/// [`$dircookie`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L320
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct DirCookie(pub u64);

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

impl DirCookie {
    /// "Signifies the start of the directory"
    pub const START: Self = Self(0);
}

wasm_layout_check!(Timestamp => 8 ^ 8);
wasm_layout_check!(DirCookie => 8 ^ 8);

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

/// Corresponds to [`$whence`], which specifies "the position relative to which to set the offset
/// of the file descriptor."
///
/// [`$whence`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L306C1-L315C2
#[allow(missing_docs)]
enum Whence(u8) = {
    Set = 0,
    Cur = 1,
    End = 2,
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

/// An [`$eventtype`] indicates the "type of a subscription to an event or its occurrence."
///
/// [`$eventtype`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L485C1-L497C2
#[allow(missing_docs)]
enum EventType(u8) = {
    Clock = 0,
    FdRead = 1,
    FdWrite = 2,
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

impl<I: wasm2rs_rt_memory::Address> wasm2rs_rt_memory_typed::Pointee<I> for $name {
    const SIZE: usize = <$int as wasm2rs_rt_memory_typed::Pointee<I>>::SIZE;

    const ALIGN: core::num::NonZeroUsize = <$int as wasm2rs_rt_memory_typed::Pointee<I>>::ALIGN;

    fn load_from<M>(mem: &M, address: I) -> wasm2rs_rt_memory::BoundsCheck<Self>
    where
        M: wasm2rs_rt_memory::Memory<I> + ?Sized,
    {
        <$int as wasm2rs_rt_memory_typed::Pointee<I>>::load_from(mem, address).map(Self::from_bits_retain)
    }

    fn store_into<M>(mem: &M, address: I, value: Self) -> wasm2rs_rt_memory::BoundsCheck<()>
    where
        M: wasm2rs_rt_memory::Memory<I> + ?Sized,
    {
        <$int as wasm2rs_rt_memory_typed::Pointee<I>>::store_into(mem, address, value.bits())
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

/// Corresponds to [`$lookupflags`], which are used for "determining the method of how paths are
/// resolved."
///
/// [`$lookupflags`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L434C1-L439C2
struct LookupFlags(u32) = {
    /// "As long as the resolved path corresponds to a symbolic link, it is expanded."
    SYMLINK_FOLLOW = 0,
}

/// Corresponds to [`$oflags`], used with [`path_open`](crate::api::Api::path_open).
///
/// [`$oflags`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L434C1-L439C2
struct OFlags(u16) = {
    /// "Create file if it does not exist."
    CREAT = 0,
    /// "Fail if not a directory."
    DIRECTORY = 1,
    /// "Fail if file already exists."
    EXCL = 2,
    /// "Truncate file to size 0."
    TRUNC = 3,
}

/// An [`$eventrwflags`] value indicates "the state of the file descriptor subscribed to with
/// [`EventType::FdRead`] or [`EventType::FdWrite`]."
///
/// [`$eventrwflags`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L501C1-L506C2
struct EventRwFlags(u16) = {
    /// "The peer of this socket has closed or disconnected."
    FdReadWriteHangup = 0,
}

/// A [`$subclockflags`] value contains "flags determining how to interpret the timestamp provided
/// in [`SubscriptionClock::timeout`]."
///
/// [`$subclockflags`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L536C1-L545C2
struct SubClockFlags(u16) = {
    #[allow(missing_docs)]
    ABSTIME = 0,
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

    /// An [`$ciovec`] defines a read-only" region of memory for scatter/gather reads."
    ///
    /// [`$ciovec`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L290C1-L297C2
    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    #[allow(missing_docs)]
    pub struct CIoVec {
        pub buf: Ptr<u8>,
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

    /// An [`$event_fd_readwrite`] structure contains "the contents of an [`Event`] when type is
    /// [`EventType::FdRead`] or [`EventType::FdWrite`]."
    ///
    /// [`$event_fd_readwrite`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L510C1-L517C2
    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    pub struct EventFdReadWrite {
        /// "The number of bytes available for reading or writing."
        pub nbytes: FileSize,
        /// "The state of the file descriptor."
        pub flags: EventRwFlags,
    }

    /// An [`$event`] structure describes "an event that occurred."
    ///
    /// [`$event`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L520C1-L532C2
    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    pub struct Event {
        /// "User-provided value that got attached to" [`Subscription::user_data`].
        pub user_data: UserData,
        /// Indicates if "an error that occurred while processing the subscription request."
        ///
        /// The [`Errno`] value can be obtained by calling [`Errno::try_from_raw()`].
        pub error: u16,
        /// An [`EventType`] indicating "the type of event that occurred."
        pub r#type: u8,
        /// "The contents of the event, if it is an [`EventType::FdRead`] or
        /// [`EventType::FdWrite`]. [`EventType::Clock`] events ignore this field."
        pub fd_readwrite: EventFdReadWrite,
    }

    /// A [`$subscription_clock`] structure contains "the contents of a [`Subscription`] when (the)
    /// type is [`EventType::Clock`]."
    ///
    /// [`$subscription_clock`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L548C1-L560C2
    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    pub struct SubscriptionClock {
        /// "The clock against which to compare the timestamp."
        ///
        /// The acutal [`ClockId`] can be obtained by calling `ClockId::try_from()`.
        pub id: u32,
        /// "The absolute or relative timestamp."
        pub timeout: Timestamp,
        /// "The amount of time that the implementation may wait additionally to coalesce with
        /// other events."
        pub precision: Timestamp,
        /// "Flags specifying whether the timeout is absolute or relative".
        pub flags: SubClockFlags,
    }

    /// A [`$subscription_fd_readwrite`] structure contains "the contents of a [`Subscription`]
    /// when (the) type is type is [`EventType::FdRead`] or [`EventType::FdWrite`]."
    ///
    /// [`$subscription_fd_readwrite`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L564C12-L564C37
    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    pub struct SubscriptionFdReadWrite {
        /// "The file descriptor on which to wait for it to become ready for reading or writing."
        pub file_description: Fd,
    }

    /// A [`$subscription`] describes a "subscription to an event." It is used with the
    /// [`poll_oneoff`](crate::api::Api::poll_oneoff) function.
    ///
    /// [`$subscription`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L581C1-L589C2
    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    pub struct Subscription {
        #[allow(missing_docs)]
        pub user_data: UserData,
        /// "The type of the event to which to subscribe, and its contents".
        pub contents: SubscriptionU,
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
    IoVec => 8 ^ 4,
    CIoVec => 8 ^ 4,
    PreStatDir => 4 ^ 4,
    EventFdReadWrite => 16 ^ 8,
    Event => 32 ^ 8,
    SubscriptionClock => 32 ^ 8,
    SubscriptionFdReadWrite => 4 ^ 4,
    Subscription => 48 ^ 8
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

    /// A [`$subscription_u`] contains "the contents of a" [`Subscription`].
    ///
    /// [`$subscription_u`]: https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/typenames.witx#L572C1-L578C2
    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    #[allow(clippy::exhaustive_enums)]
    #[allow(missing_docs)]
    pub enum SubscriptionU : u8 {
        Clock(SubscriptionClock) = 0,
        FdRead(SubscriptionFdReadWrite) = 1,
        FdWrite(SubscriptionFdReadWrite) = 2,
    }
}

wasm_layout_check! {
    PreStat => 8 ^ 4,
    SubscriptionU => 40 ^ 8
}

/// An array of [`IoVec`]s, used in functions like [`fd_read`] or [`fd_pread`].
///
/// [`fd_read`]: crate::api::Api::fd_read()
/// [`fd_pread`]: crate::api::Api::fd_pread()
pub type IoVecArray = slice::Slice<IoVec>;

/// An array of [`CIoVec`]s, used in functions like [`fd_write`] or [`fd_pwrite`].
///
/// [`fd_write`]: crate::api::Api::fd_write()
/// [`fd_pwrite`]: crate::api::Api::fd_pwrite()
pub type CIoVecArray = slice::Slice<CIoVec>;

/// Contains the arguments to [`poll_oneoff`].
///
/// [`poll_oneoff`]: crate::api::Api::poll_oneoff()
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EventPoll {
    /// "The events to which to subscribe."
    pub subscriptions: Ptr<Subscription>,
    /// Stores "the events that have occurred" after the [`poll_oneoff`] call.
    ///
    /// [`poll_oneoff`]: crate::api::Api::poll_oneoff()
    pub events: MutPtr<Event>,
    /// Stores the total "number of [`subscriptions`] and [`events`]."
    ///
    /// [`subscriptions`]: EventPoll::subscriptions
    /// [`events`]: EventPoll::events
    pub count: u32,
}

impl EventPoll {
    /// Gets a slice of the [`subscriptions`](EventPoll::subscriptions).
    pub fn subscriptions_slice(&self) -> slice::Slice<Subscription> {
        slice::Slice {
            items: self.subscriptions,
            count: self.count,
        }
    }
}
