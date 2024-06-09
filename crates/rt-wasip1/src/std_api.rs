use crate::api;
use alloc::vec::Vec;
use core::cell::RefCell;
use wasm2rs_rt_core::trap;
use wasm2rs_rt_memory::Memory;

#[derive(Debug)]
enum Handle {
    Empty(std::io::Empty),
    Stdin(std::io::Stdin),
    StdoutRaw(std::io::Stdout),
    StderrRaw(std::io::Stderr),
    //File
}

struct HandleSet {
    handles: alloc::boxed::Box<[Option<Handle>]>,
    /// The lowest free [`api::Fd`] that will be used the next time [`allocate()`] is called.
    ///
    /// It is an invariant that:
    /// - Either `next_fd == handles.len()` or `handles[next_fd].is_none()`.
    /// - All of the [`handle`]s in the range `0..lowest` are [`Some`].
    ///
    /// Additionally, to avoid any bugs in users of the [`StdApi`], `lowest` will always be greater
    /// than `2`, to avoid reusing any [`api::Fd`]s typically associated with the standard streams.
    ///
    /// [`allocate()`]: HandleSet::allocate()
    /// [`handle`]: HandleSet::handles
    lowest: usize,
}

/* impl HandleSet {
    const TOO_MANY_OPEN_FILES: api::Errno = api::Errno::_mfile;

    fn get(&self, fd: api::Fd) -> api::Result<&Handle> {
        usize::try_from(fd.0)
            .ok()
            .and_then(|i| self.handles.get(i).map(Option::as_ref))
            .flatten()
            .ok_or(api::Errno::_badf)
    }

    fn allocate(&mut self, maximum: usize) -> api::Result<api::Fd> {
        debug_assert!(self.lowest <= maximum);

        if self.lowest == maximum {
            return Err(Self::TOO_MANY_OPEN_FILES);
        }

        let fd = u32::try_from(self.lowest).map_err(|_| Self::TOO_MANY_OPEN_FILES).map(api::Fd)?;

        if self.lowest == self.handles.len() {
            struct DropGuard<'a> {
                source: &'a mut Box<[Option<Handle>]>,
                borrowed: Vec<Option<Handle>>,
            }

            // Reallocate the `handles` array.
            let mut handles = DropGuard {
                borrowed: core::mem::take(&mut self.handles).into_vec(),
                source: &mut self.handles,
            };

            // 1.5 growth factor for fun!
            handles.borrowed.try_reserve_exact((handles.borrowed.capacity() + 1) / 2)
                .map_err(|_| Self::TOO_MANY_OPEN_FILES)?;

            impl Drop for DropGuard<'_> {
                fn drop(&mut self) {
                    self.borrowed.resize_with(self.borrowed.capacity(), || None);
                    *self.source = core::mem::take(&mut self.borrowed).into_boxed_slice();
                }
            }
        }

        todo!()
    }
} */

type StringData = alloc::borrow::Cow<'static, str>;

/// An implementation of the WASI [`Api`] backed by the [Rust standard library] and the [`cap_std`]
/// crate.
///
/// [`Api`]: api::Api
/// [Rust standard library]: std
pub struct StdApi<M: Memory, E: trap::TrapInfo> {
    handles: RefCell<HandleSet>,
    max_handle_count: usize,
    program_name: StringData,
    //arguments: Vec<>,
    //arguments_len: u32,
    _marker: core::marker::PhantomData<fn(M) -> E>,
}

impl<M: Memory, E: trap::TrapInfo> StdApi<M, E> {
    /// Gets the maximum number of open [`Fd`]s that can exist at any given time.
    ///
    /// [`Fd`]: api::Fd
    pub fn max_opened_fds(&self) -> usize {
        self.max_handle_count
    }
}

/// Helper struct used to configure and create a [`StdApi`] instance.
#[derive(Debug)]
pub struct StdApiBuilder {
    standard_streams: [Handle; 3],
    max_handle_count: usize,
    program_name: Option<StringData>,
    // arguments: Vec<StringData>, // TODO: Check for null terminators
    // /// The total length, in bytes, of all CLI arguments, **including** any added null
    // /// terminators.
    // ///
    // /// The number of added null terminators is equal to `arguments.len()`.
    // arguments_len: usize,
}

impl Default for StdApiBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl StdApiBuilder {
    /// Creates a [`StdApiBuilder`] with the default parameters, which are:
    ///
    /// - No inherited standard streams.
    /// - A maximum of 512 open [`Fd`]s at any given time.
    /// - A program name taken from [`std::env::current_exe()`], or if the latter cannot be
    ///   obtained or is not valid UTF-8, `"(unknown).wasm"`.
    ///
    /// [`Fd`]: api::Fd
    pub fn new() -> Self {
        Self {
            standard_streams: core::array::from_fn(|_| Handle::Empty(std::io::empty())),
            max_handle_count: 512,
            program_name: None,
        }
    }

    /// Inherits the stadard input stream, and the standard output and error streams **without**
    /// sanitizing escape sequences.
    pub fn inherit_standard_streams_without_sanitation(mut self) -> Self {
        self.standard_streams[api::Fd::STDIN.0 as usize] = Handle::Stdin(std::io::stdin());
        self.standard_streams[api::Fd::STDOUT.0 as usize] = Handle::StdoutRaw(std::io::stdout());
        self.standard_streams[api::Fd::STDERR.0 as usize] = Handle::StderrRaw(std::io::stderr());
        self
    }

    /// Sets the name of the program. This is provided as the first CLI argument string.
    pub fn program_name<S>(mut self, name: S) -> Self
    where
        S: Into<StringData>,
    {
        self.program_name = Some(name.into());
        self
    }

    fn env_program_name() -> Option<alloc::string::String> {
        std::env::current_exe()
            .ok()?
            .file_name()?
            .to_str()
            .map(Into::into)
    }

    /// Creates a new [`StdApi`] from the given configuration.
    ///
    /// # Panics
    ///
    /// Panics when:
    /// - The length of the program name plus a null terminator exceeds [`u32::MAX`].
    /// - The total number of CLI arguments (excluding the program name) is greater than or equal
    ///   to [`u32::MAX`].
    /// - The total size, in bytes, of the program name and all CLI arguments with appended null
    ///   terminators exceeds [`u32::MAX`].
    pub fn build<M: Memory, E: trap::TrapInfo>(self) -> StdApi<M, E> {
        let lowest_fd = self.standard_streams.len() + 1;
        let mut handles = Vec::with_capacity(5);
        handles.extend(self.standard_streams.into_iter().map(Some));
        handles.resize_with(handles.capacity(), || None);

        let program_name = if let Some(program_name) = self.program_name {
            program_name
        } else if let Some(process_name) = Self::env_program_name() {
            process_name.into()
        } else {
            "(unknown).wasm".into()
        };

        match u32::try_from(program_name.len()) {
            Ok(program_name_len) if program_name_len < u32::MAX => (),
            bad => panic!(
                "program name length ({}) is too long: {bad:?}",
                program_name.len()
            ),
        }

        StdApi {
            handles: RefCell::new(HandleSet {
                handles: handles.into_boxed_slice(),
                lowest: lowest_fd,
            }),
            max_handle_count: self.max_handle_count,
            program_name,
            _marker: core::marker::PhantomData,
        }
    }
}

impl<M: Memory, E: trap::TrapInfo> api::Api for StdApi<M, E> {
    type Memory = M;
    type Trap = crate::Trap<E>;

    fn args_get(
        &self,
        mem: &Self::Memory,
        argv: api::MutPtr<api::MutPtr<u8>>,
        argv_buf: api::MutPtr<u8>,
    ) -> api::Result<()> {
        // TODO: Write more CLI arguments
        // TODO: Helper to write slice into MutPtr<u8>.
        mem.copy_from_slice(argv_buf.to_address(), self.program_name.as_bytes())?;
        // Store null terminator of program name.
        mem.i8_store(argv_buf.to_address() + (self.program_name.len() as u32), 0)?;
        argv.store(mem, argv_buf)?;
        Ok(())
    }

    fn args_sizes_get(&self) -> api::Result<api::DataSizes> {
        // TODO: Include more CLI arguments
        Ok(api::DataSizes {
            count: 1,
            buf_size: self.program_name.len() as u32 + 1,
        })
    }

    fn fd_write(
        &self,
        mem: &Self::Memory,
        fd: api::Fd,
        iovs: api::CIoVecArray,
    ) -> api::Result<api::FileSize> {
        let _ = (mem, fd, iovs); // TODO: Helpers for reading from (C)IoVecArray
        todo!("Hello Mundo!");
    }

    fn proc_exit(&self, rval: api::ExitCode) -> Self::Trap {
        Self::Trap::ProcExit(rval)
    }

    fn proc_raise(&self, sig: api::Signal) -> core::result::Result<api::Result<()>, Self::Trap> {
        use api::Signal;

        match sig {
            Signal::None | Signal::Pipe | Signal::Chld | Signal::Urg | Signal::Winch => Ok(Ok(())),
            _ => Err(Self::Trap::ProcRaise(sig)),
        }
    }
}

impl<M: Memory, E: trap::TrapInfo> core::fmt::Debug for StdApi<M, E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        struct CliArguments<'a> {
            program_name: &'a StringData,
            arguments: &'a [StringData],
        }

        impl core::fmt::Debug for CliArguments<'_> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.debug_list()
                    .entry(self.program_name)
                    .entries(self.arguments)
                    .finish()
            }
        }

        let handles = self.handles.borrow();

        f.debug_struct("StdApi")
            .field("max_opened_fds", &self.max_handle_count as _)
            .field(
                "cli_arguments",
                &CliArguments {
                    program_name: &self.program_name,
                    arguments: &[],
                },
            )
            .field("lowest_fd", &handles.lowest as _)
            .field("handles", &handles.handles.as_ref())
            .finish()
    }
}
