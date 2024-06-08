use crate::api::{self, Api, Fd, MutPtr, Ptr};

/// Allows WebAssembly modules translated by `wasm2rs` to use a imported WASI functions provided by
/// an [`Api`] implementation.
#[derive(Clone, Debug)]
pub struct Wasi<A: Api> {
    memory: A::Memory,
    api: A,
}

/// Error code returned when an [`Api`] call is successful.
const SUCCESS: i32 = 0;

fn result_to_error_code(result: api::Result<()>) -> i32 {
    match result {
        Ok(()) => SUCCESS,
        Err(errno) => errno as i32,
    }
}

type Result<A> = core::result::Result<i32, <A as Api>::Trap>;

impl<A: Api> Wasi<A> {
    /// Creates a new source of [`Wasi`] API imports from the given linear [`Memory`] and [`Api`]
    /// implementation.
    ///
    /// [`Memory`]: wasm2rs_rt_memory::Memory
    pub fn new(memory: A::Memory, api: A) -> Self {
        Self { memory, api }
    }

    /// Calls [`Api::args_get()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "args_get" (func
    ///     (param $argv i32)
    ///     (param $argv_buf i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn args_get(&self, argv: i32, argv_buf: i32) -> Result<A> {
        Ok(result_to_error_code(self.api.args_get(
            &self.memory,
            argv.into(),
            argv_buf.into(),
        )))
    }

    fn arg_sizes_get_impl(&self, argc: MutPtr<u32>, argv_buf_size: MutPtr<u32>) -> api::Result<()> {
        let api::DataSizes { count, buf_size } = self.api.args_sizes_get()?;
        argc.store(&self.memory, count)?;
        argv_buf_size.store(&self.memory, buf_size)?;
        Ok(())
    }

    /// Calls [`Api::args_sizes_get()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "args_sizes_get" (func
    ///     (param $argc i32)
    ///     (param $argv_buf_size i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn args_sizes_get(&self, argc: i32, argv_buf_size: i32) -> Result<A> {
        Ok(result_to_error_code(
            self.arg_sizes_get_impl(argc.into(), argv_buf_size.into()),
        ))
    }

    /// Calls [`Api::environ_get()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "environ_get" (func
    ///     (param $environ i32)
    ///     (param $environ_buf i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn environ_get(&self, environ: i32, environ_buf: i32) -> Result<A> {
        Ok(result_to_error_code(self.api.environ_get(
            &self.memory,
            environ.into(),
            environ_buf.into(),
        )))
    }

    fn environ_sizes_get_impl(
        &self,
        environ_count: MutPtr<u32>,
        environ_buf_size: MutPtr<u32>,
    ) -> api::Result<()> {
        let api::DataSizes { count, buf_size } = self.api.args_sizes_get()?;
        environ_count.store(&self.memory, count)?;
        environ_buf_size.store(&self.memory, buf_size)?;
        Ok(())
    }

    /// Calls [`Api::environ_sizes_get()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "environ_sizes_get" (func
    ///     (param $environ_count i32)
    ///     (param $environ_buf_size i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn environ_sizes_get(&self, environ_count: i32, environ_buf_size: i32) -> Result<A> {
        Ok(result_to_error_code(self.environ_sizes_get_impl(
            environ_count.into(),
            environ_buf_size.into(),
        )))
    }

    fn clock_res_get_impl(
        &self,
        clock_id: u32,
        resolution: MutPtr<api::Timestamp>,
    ) -> api::Result<()> {
        // `inval` used if clock is not supported, so it's used here when `clock_id` is garbage.
        let id = api::ClockId::try_from(clock_id)?;
        resolution.store(&self.memory, self.api.clock_res_get(id)?)?;
        Ok(())
    }

    /// Calls [`Api::clock_res_get()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "clock_res_get" (func
    ///     (param $clock_id i32)
    ///     (param $resolution i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn clock_res_get(&self, clock_id: i32, resolution: i32) -> Result<A> {
        Ok(result_to_error_code(
            self.clock_res_get_impl(clock_id as u32, resolution.into()),
        ))
    }

    fn clock_time_get_impl(
        &self,
        clock_id: u32,
        precision: api::Timestamp,
        time: MutPtr<api::Timestamp>,
    ) -> api::Result<()> {
        let id = api::ClockId::try_from(clock_id)?;
        time.store(&self.memory, self.api.clock_time_get(id, precision)?)?;
        Ok(())
    }

    /// Calls [`Api::clock_time_get()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "clock_time_get" (func
    ///     (param $clock_id i32)
    ///     (param $precision i64)
    ///     (param $time i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn clock_time_get(&self, clock_id: i32, precision: i64, time: i32) -> Result<A> {
        Ok(result_to_error_code(self.clock_time_get_impl(
            clock_id as u32,
            api::Timestamp::from_i64(precision),
            time.into(),
        )))
    }

    fn fd_advise_impl(&self, fd: Fd, offset: u64, len: u64, advice: u32) -> api::Result<()> {
        let advice = api::Advice::try_from(u8::try_from(advice).map_err(|_| api::Errno::_inval)?)?;

        self.api.fd_advise(fd, offset, len, advice)
    }

    /// Calls [`Api::fd_advise()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "fd_advise" (func
    ///     (param $fd i32)
    ///     (param $offset i64)
    ///     (param $len i64)
    ///     (param $advice i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn fd_advise(&self, fd: i32, offset: i64, len: i64, advice: i32) -> Result<A> {
        Ok(result_to_error_code(self.fd_advise_impl(
            Fd::from_i32(fd),
            offset as u64,
            len as u64,
            advice as u32,
        )))
    }

    /// Calls [`Api::fd_allocate()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "fd_allocate" (func
    ///     (param $fd i32)
    ///     (param $offset i64)
    ///     (param $len i64)
    ///     (result i32)
    /// ))
    /// ```
    pub fn fd_allocate(&self, fd: i32, offset: i64, len: i64) -> Result<A> {
        Ok(result_to_error_code(self.api.fd_allocate(
            Fd::from_i32(fd),
            offset as u64,
            len as u64,
        )))
    }

    /// Calls [`Api::fd_close()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "fd_close" (func
    ///     (param $fd i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn fd_close(&self, fd: i32) -> Result<A> {
        Ok(result_to_error_code(self.api.fd_close(Fd::from_i32(fd))))
    }

    /// Calls [`Api::fd_datasync()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "fd_datasync" (func
    ///     (param $fd i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn fd_datasync(&self, fd: i32) -> Result<A> {
        Ok(result_to_error_code(self.api.fd_datasync(Fd::from_i32(fd))))
    }

    fn fd_fdstat_get_impl(&self, fd: Fd, buf_ptr: MutPtr<api::FdStat>) -> api::Result<()> {
        buf_ptr.store(&self.memory, self.api.fd_fdstat_get(fd)?)?;
        Ok(())
    }

    /// Calls [`Api::fd_fdstat_get()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "fd_fdstat_get" (func
    ///     (param $fd i32)
    ///     (param $buf_ptr i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn fd_fdstat_get(&self, fd: i32, buf_ptr: i32) -> Result<A> {
        Ok(result_to_error_code(
            self.fd_fdstat_get_impl(Fd::from_i32(fd), buf_ptr.into()),
        ))
    }

    fn fd_fdstat_set_flags_impl(&self, fd: Fd, flags: u32) -> api::Result<()> {
        let flags =
            api::FdFlags::from_bits_retain(u16::try_from(flags).map_err(|_| api::Errno::_inval)?);

        self.api.fd_fdstat_set_flags(fd, flags)
    }

    /// Calls [`Api::fd_fdstat_set_flags()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "fd_fdstat_set_flags" (func
    ///     (param $fd i32)
    ///     (param $flags i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn fd_fdstat_set_flags(&self, fd: i32, flags: i32) -> Result<A> {
        Ok(result_to_error_code(
            self.fd_fdstat_set_flags_impl(Fd::from_i32(fd), flags as u32),
        ))
    }

    /// Calls [`Api::fd_fdstat_set_rights()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "fd_fdstat_set_rights" (func
    ///     (param $fd i32)
    ///     (param $fs_rights_base i64)
    ///     (param $fs_rights_inheriting i64)
    ///     (result i32)
    /// ))
    /// ```
    pub fn fd_fdstat_set_rights(
        &self,
        fd: i32,
        fs_rights_base: i64,
        fs_rights_inheriting: i64,
    ) -> Result<A> {
        Ok(result_to_error_code(self.api.fd_fdstat_set_rights(
            Fd::from_i32(fd),
            api::Rights::from_bits_retain(fs_rights_base as u64),
            api::Rights::from_bits_retain(fs_rights_inheriting as u64),
        )))
    }

    fn fd_filestat_get_impl(&self, fd: Fd, buf: MutPtr<api::FileStat>) -> api::Result<()> {
        buf.store(&self.memory, self.api.fd_filestat_get(fd)?)?;
        Ok(())
    }

    /// Calls [`Api::fd_filestat_get()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "fd_filestat_get" (func
    ///     (param $fd i32)
    ///     (param $buf i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn fd_filestat_get(&self, fd: i32, buf: i32) -> Result<A> {
        Ok(result_to_error_code(
            self.fd_filestat_get_impl(Fd::from_i32(fd), buf.into()),
        ))
    }

    /// Calls [`Api::fd_filestat_set_size()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "fd_filestat_set_size" (func
    ///     (param $fd i32)
    ///     (param $st_size i64)
    ///     (result i32)
    /// ))
    /// ```
    pub fn fd_filestat_set_size(&self, fd: i32, st_size: i64) -> Result<A> {
        Ok(result_to_error_code(
            self.api
                .fd_filestat_set_size(Fd::from_i32(fd), st_size as u64),
        ))
    }

    fn fd_filestat_set_times_impl(
        &self,
        fd: Fd,
        atim: api::Timestamp,
        mtim: api::Timestamp,
        fst_flags: u32,
    ) -> api::Result<()> {
        let flags = api::FstFlags::from_bits_retain(
            u16::try_from(fst_flags).map_err(|_| api::Errno::_inval)?,
        );

        self.api.fd_filestat_set_times(fd, atim, mtim, flags)
    }

    /// Calls [`Api::fd_filestat_set_times()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "fd_filestat_set_times" (func
    ///     (param $fd i32)
    ///     (param $atim i64)
    ///     (param $mtim i64)
    ///     (param $fst_flags i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn fd_filestat_set_times(
        &self,
        fd: i32,
        atim: i64,
        mtim: i64,
        fst_flags: i32,
    ) -> Result<A> {
        Ok(result_to_error_code(self.fd_filestat_set_times_impl(
            Fd::from_i32(fd),
            api::Timestamp::from_i64(atim),
            api::Timestamp::from_i64(mtim),
            fst_flags as u32,
        )))
    }

    fn fd_pread_impl(
        &self,
        fd: Fd,
        iovs: Ptr<api::IoVec>,
        iovs_len: u32,
        offset: api::FileSize,
        nread: MutPtr<u32>,
    ) -> api::Result<()> {
        let iovs = api::IoVecArray {
            items: iovs,
            count: iovs_len,
        };

        nread.store(
            &self.memory,
            self.api.fd_pread(&self.memory, fd, iovs, offset)?,
        )?;

        Ok(())
    }

    /// Calls [`Api::fd_pread()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "fd_pread" (func
    ///     (param $fd i32)
    ///     (param $iovs i32)
    ///     (param $iovs_len i32)
    ///     (param $offset i64)
    ///     (param $nread i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn fd_pread(
        &self,
        fd: i32,
        iovs: i32,
        iovs_len: i32,
        offset: i64,
        nread: i32,
    ) -> Result<A> {
        Ok(result_to_error_code(self.fd_pread_impl(
            Fd::from_i32(fd),
            iovs.into(),
            iovs_len as u32,
            offset as u64,
            nread.into(),
        )))
    }

    fn fd_prestat_get_impl(&self, fd: Fd, buf: MutPtr<api::PreStat>) -> api::Result<()> {
        buf.store(&self.memory, self.api.fd_prestat_get(fd)?)?;
        Ok(())
    }

    /// Calls [`Api::fd_prestat_get()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "fd_prestat_get" (func
    ///     (param $fd i32)
    ///     (param $buf i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn fd_prestat_get(&self, fd: i32, buf: i32) -> Result<A> {
        Ok(result_to_error_code(
            self.fd_prestat_get_impl(Fd::from_i32(fd), buf.into()),
        ))
    }

    /// Calls [`Api::fd_prestat_dir_name()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "fd_prestat_dir_name" (func
    ///     (param $fd i32)
    ///     (param $path i32)
    ///     (param $path_len i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn fd_prestat_dir_name(&self, fd: i32, path: i32, path_len: i32) -> api::Result<()> {
        self.api.fd_prestat_dir_name(
            &self.memory,
            Fd::from_i32(fd),
            wasm2rs_rt_memory_typed::slice::MutSlice {
                items: path.into(),
                count: path_len as u32,
            },
        )
    }

    fn fd_pwrite_impl(
        &self,
        fd: Fd,
        iovs: Ptr<api::CIoVec>,
        iovs_len: u32,
        offset: api::FileSize,
        nwritten: MutPtr<u32>,
    ) -> api::Result<()> {
        let iovs = api::CIoVecArray {
            items: iovs,
            count: iovs_len,
        };

        nwritten.store(
            &self.memory,
            self.api.fd_pwrite(&self.memory, fd, iovs, offset)?,
        )?;

        Ok(())
    }

    /// Calls [`Api::fd_pwrite()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "fd_pwrite" (func
    ///     (param $fd i32)
    ///     (param $iovs i32)
    ///     (param $iovs_len i32)
    ///     (param $offset i64)
    ///     (param $nwritten i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn fd_pwrite(
        &self,
        fd: i32,
        iovs: i32,
        iovs_len: i32,
        offset: i64,
        nwritten: i32,
    ) -> Result<A> {
        Ok(result_to_error_code(self.fd_pwrite_impl(
            Fd::from_i32(fd),
            iovs.into(),
            iovs_len as u32,
            offset as u64,
            nwritten.into(),
        )))
    }

    fn fd_read_impl(
        &self,
        fd: Fd,
        iovs: Ptr<api::IoVec>,
        iovs_len: u32,
        nread: MutPtr<u32>,
    ) -> api::Result<()> {
        let iovs = api::IoVecArray {
            items: iovs,
            count: iovs_len,
        };

        nread.store(&self.memory, self.api.fd_read(&self.memory, fd, iovs)?)?;

        Ok(())
    }

    /// Calls [`Api::fd_read()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "fd_read" (func
    ///     (param $fd i32)
    ///     (param $iovs i32)
    ///     (param $iovs_len i32)
    ///     (param $nread i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn fd_read(&self, fd: i32, iovs: i32, iovs_len: i32, nread: i32) -> Result<A> {
        Ok(result_to_error_code(self.fd_read_impl(
            Fd::from_i32(fd),
            iovs.into(),
            iovs_len as u32,
            nread.into(),
        )))
    }

    fn fd_readdir_impl(
        &self,
        fd: api::Fd,
        buf: api::MutPtr<u8>,
        buf_len: u32,
        cookie: api::DirCookie,
        buf_used: api::MutPtr<u32>,
    ) -> api::Result<()> {
        let buf = wasm2rs_rt_memory_typed::slice::MutSlice {
            items: buf,
            count: buf_len,
        };

        buf_used.store(
            &self.memory,
            self.api.fd_readdir(&self.memory, fd, buf, cookie)?,
        )?;

        Ok(())
    }

    /// Calls [`Api::fd_readdir()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "fd_readdir" (func
    ///     (param $fd i32)
    ///     (param $buf i32)
    ///     (param $buf_len i32)
    ///     (param $cookie i64)
    ///     (param $buf_used i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn fd_readdir(
        &self,
        fd: i32,
        buf: i32,
        buf_len: i32,
        cookie: i64,
        buf_used: i32,
    ) -> Result<A> {
        Ok(result_to_error_code(self.fd_readdir_impl(
            Fd::from_i32(fd),
            buf.into(),
            buf_len as u32,
            api::DirCookie(cookie as u64),
            buf_used.into(),
        )))
    }

    /// Calls [`Api::fd_renumber()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "fd_renumber" (func
    ///     (param $fd i32)
    ///     (param $to i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn fd_renumber(&self, fd: i32, to: i32) -> Result<A> {
        Ok(result_to_error_code(
            self.api.fd_renumber(Fd::from_i32(fd), Fd::from_i32(to)),
        ))
    }

    fn fd_seek_impl(
        &self,
        fd: api::Fd,
        offset: api::FileDelta,
        whence: u32,
        new_offset: MutPtr<u64>,
    ) -> api::Result<()> {
        let whence = u8::try_from(whence)
            .map_err(|_| api::Errno::_inval)
            .and_then(api::Whence::try_from)?;

        new_offset.store(&self.memory, self.api.fd_seek(fd, offset, whence)?)?;
        Ok(())
    }

    /// Calls [`Api::fd_seek()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "fd_seek" (func
    ///     (param $fd i32)
    ///     (param $offset i64)
    ///     (param $whence i32)
    ///     (param $new_offset i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn fd_seek(&self, fd: i32, offset: i64, whence: i32, new_offset: i32) -> Result<A> {
        Ok(result_to_error_code(self.fd_seek_impl(
            Fd::from_i32(fd),
            offset,
            whence as u32,
            new_offset.into(),
        )))
    }

    /// Calls [`Api::fd_sync()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "fd_sync" (func
    ///     (param $fd i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn fd_sync(&self, fd: i32) -> Result<A> {
        Ok(result_to_error_code(
            self.api.fd_sync(api::Fd::from_i32(fd)),
        ))
    }

    fn fd_tell_impl(&self, fd: api::Fd, offset: MutPtr<u64>) -> api::Result<()> {
        offset.store(&self.memory, self.api.fd_tell(fd)?)?;
        Ok(())
    }

    /// Calls [`Api::fd_tell()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "fd_tell" (func
    ///     (param $fd i32)
    ///     (param $offset i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn fd_tell(&self, fd: i32, offset: i32) -> Result<A> {
        Ok(result_to_error_code(
            self.fd_tell_impl(api::Fd::from_i32(fd), offset.into()),
        ))
    }

    fn fd_write_impl(
        &self,
        fd: api::Fd,
        iovs: Ptr<api::CIoVec>,
        iovs_len: u32,
        nwritten: MutPtr<u64>,
    ) -> api::Result<()> {
        let iovs = api::CIoVecArray {
            items: iovs,
            count: iovs_len,
        };

        nwritten.store(&self.memory, self.api.fd_write(&self.memory, fd, iovs)?)?;

        Ok(())
    }

    /// Calls [`Api::fd_write()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "fd_write" (func
    ///     (param $fd i32)
    ///     (param $iovs i32)
    ///     (param $iovs_len i32)
    ///     (param $nwritten i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn fd_write(&self, fd: i32, iovs: i32, iovs_len: i32, nwritten: i32) -> Result<A> {
        Ok(result_to_error_code(self.fd_write_impl(
            Fd::from_i32(fd),
            iovs.into(),
            iovs_len as u32,
            nwritten.into(),
        )))
    }

    fn path_create_directory_impl(
        &self,
        fd: api::Fd,
        path: Ptr<u8>,
        path_len: u32,
    ) -> api::Result<()> {
        let path = api::Path {
            items: path,
            count: path_len,
        };

        self.api.path_create_directory(&self.memory, fd, path)
    }

    /// Calls [`Api::path_create_directory()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "path_create_directory" (func
    ///     (param $fd i32)
    ///     (param $path i32)
    ///     (param $path_len i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn path_create_directory(&self, fd: i32, path: i32, path_len: i32) -> Result<A> {
        Ok(result_to_error_code(self.path_create_directory_impl(
            api::Fd::from_i32(fd),
            path.into(),
            path_len as u32,
        )))
    }

    fn path_filestat_get_impl(
        &self,
        fd: api::Fd,
        flags: u32,
        path: Ptr<u8>,
        path_len: u32,
        buf: MutPtr<api::FileStat>,
    ) -> api::Result<()> {
        let path = api::Path {
            items: path,
            count: path_len,
        };

        buf.store(
            &self.memory,
            self.api.path_filestat_get(
                &self.memory,
                fd,
                api::LookupFlags::from_bits_retain(flags),
                path,
            )?,
        )?;

        Ok(())
    }

    /// Calls [`Api::path_filestat_get()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "path_filestat_get" (func
    ///     (param $fd i32)
    ///     (param $flags i32)
    ///     (param $path i32)
    ///     (param $path_len i32)
    ///     (param $buf i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn path_filestat_get(
        &self,
        fd: i32,
        flags: i32,
        path: i32,
        path_len: i32,
        buf: i32,
    ) -> Result<A> {
        Ok(result_to_error_code(self.path_filestat_get_impl(
            api::Fd::from_i32(fd),
            flags as u32,
            path.into(),
            path_len as u32,
            buf.into(),
        )))
    }

    #[allow(clippy::too_many_arguments)]
    fn path_filestat_set_times_impl(
        &self,
        fd: api::Fd,
        flags: u32,
        path: Ptr<u8>,
        path_len: u32,
        atim: api::Timestamp,
        mtim: api::Timestamp,
        fst_flags: u32,
    ) -> api::Result<()> {
        let fst_flags = api::FstFlags::from_bits_retain(
            u16::try_from(fst_flags).map_err(|_| api::Errno::_inval)?,
        );
        let path = api::Path {
            items: path,
            count: path_len,
        };

        self.api.path_filestat_set_times(
            &self.memory,
            fd,
            api::LookupFlags::from_bits_retain(flags),
            path,
            atim,
            mtim,
            fst_flags,
        )?;
        Ok(())
    }

    /// Calls [`Api::path_filestat_set_times()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "path_filestat_set_times" (func
    ///     (param $fd i32)
    ///     (param $flags i32)
    ///     (param $path i32)
    ///     (param $path_len i32)
    ///     (param $atim i64)
    ///     (param $mtim i64)
    ///     (param $fst_flags i32)
    ///     (result i32)
    /// ))
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn path_filestat_set_times(
        &self,
        fd: i32,
        flags: i32,
        path: i32,
        path_len: i32,
        atim: i64,
        mtim: i64,
        fst_flags: i32,
    ) -> Result<A> {
        Ok(result_to_error_code(self.path_filestat_set_times_impl(
            api::Fd::from_i32(fd),
            flags as u32,
            path.into(),
            path_len as u32,
            api::Timestamp::from_i64(atim),
            api::Timestamp::from_i64(mtim),
            fst_flags as u32,
        )))
    }

    #[allow(clippy::too_many_arguments)]
    fn path_link_impl(
        &self,
        old_fd: api::Fd,
        old_flags: u32,
        old_path: Ptr<u8>,
        old_path_len: u32,
        new_fd: api::Fd,
        new_path: Ptr<u8>,
        new_path_len: u32,
    ) -> api::Result<()> {
        let old_path = api::Path {
            items: old_path,
            count: old_path_len,
        };
        let new_path = api::Path {
            items: new_path,
            count: new_path_len,
        };

        self.api.path_link(
            &self.memory,
            old_fd,
            api::LookupFlags::from_bits_retain(old_flags),
            old_path,
            new_fd,
            new_path,
        )?;

        Ok(())
    }

    /// Calls [`Api::path_link()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "path_link" (func
    ///     (param $old_fd i32)
    ///     (param $old_flags i32)
    ///     (param $old_path i32)
    ///     (param $old_path_len i32)
    ///     (param $new_fd i32)
    ///     (param $new_path i32)
    ///     (param $new_path_len i32)
    ///     (result i32)
    /// ))
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn path_link(
        &self,
        old_fd: i32,
        old_flags: i32,
        old_path: i32,
        old_path_len: i32,
        new_fd: i32,
        new_path: i32,
        new_path_len: i32,
    ) -> Result<A> {
        Ok(result_to_error_code(self.path_link_impl(
            api::Fd::from_i32(old_fd),
            old_flags as u32,
            old_path.into(),
            old_path_len as u32,
            api::Fd::from_i32(new_fd),
            new_path.into(),
            new_path_len as u32,
        )))
    }

    #[allow(clippy::too_many_arguments)]
    fn path_open_impl(
        &self,
        dir_fd: api::Fd,
        dir_flags: u32,
        path: Ptr<u8>,
        path_len: u32,
        o_flags: u32,
        rights_base: api::Rights,
        rights_inheriting: api::Rights,
        fd_flags: u32,
        opened_fd: MutPtr<Fd>,
    ) -> api::Result<()> {
        let path = api::Path {
            items: path,
            count: path_len,
        };
        let o_flags =
            api::OFlags::from_bits_retain(u16::try_from(o_flags).map_err(|_| api::Errno::_inval)?);
        let fd_flags = api::FdFlags::from_bits_retain(
            u16::try_from(fd_flags).map_err(|_| api::Errno::_inval)?,
        );

        opened_fd.store(
            &self.memory,
            self.api.path_open(
                &self.memory,
                dir_fd,
                api::LookupFlags::from_bits_retain(dir_flags),
                path,
                o_flags,
                rights_base,
                rights_inheriting,
                fd_flags,
            )?,
        )?;

        Ok(())
    }

    /// Calls [`Api::path_open()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "path_open" (func
    ///     (param $dir_fd i32)
    ///     (param $dir_flags i32)
    ///     (param $path i32)
    ///     (param $path_len i32)
    ///     (param $fs_rights_base i64)
    ///     (param $fs_rights_inheriting i64)
    ///     (param $fs_flags i32)
    ///     (param $fd i32)
    ///     (result i32)
    /// ))
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn path_open(
        &self,
        dir_fd: i32,
        dir_flags: i32,
        path: i32,
        path_len: i32,
        o_flags: i32,
        fs_rights_base: i64,
        fs_rights_inheriting: i64,
        fs_flags: i32,
        opened_fd: i32,
    ) -> Result<A> {
        Ok(result_to_error_code(self.path_open_impl(
            api::Fd::from_i32(dir_fd),
            dir_flags as u32,
            path.into(),
            path_len as u32,
            o_flags as u32,
            api::Rights::from_bits_retain(fs_rights_base as u64),
            api::Rights::from_bits_retain(fs_rights_inheriting as u64),
            fs_flags as u32,
            opened_fd.into(),
        )))
    }

    fn path_readlink_impl(
        &self,
        dir_fd: api::Fd,
        path: Ptr<u8>,
        path_len: u32,
        buf: MutPtr<u8>,
        buf_len: u32,
        buf_used: MutPtr<u32>,
    ) -> api::Result<()> {
        let path = api::Path {
            items: path,
            count: path_len,
        };
        let buf = wasm2rs_rt_memory_typed::slice::MutSlice {
            items: buf,
            count: buf_len,
        };

        buf_used.store(
            &self.memory,
            self.api.path_readlink(&self.memory, dir_fd, path, buf)?,
        )?;
        Ok(())
    }

    /// Calls [`Api::path_readlink()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "path_readlink" (func
    ///     (param $dir_fd i32)
    ///     (param $path i32)
    ///     (param $path_len i32)
    ///     (param $buf i32)
    ///     (param $buf_len i32)
    ///     (param $buf_used i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn path_readlink(
        &self,
        dir_fd: i32,
        path: i32,
        path_len: i32,
        buf: i32,
        buf_len: i32,
        buf_used: i32,
    ) -> Result<A> {
        Ok(result_to_error_code(self.path_readlink_impl(
            api::Fd::from_i32(dir_fd),
            path.into(),
            path_len as u32,
            buf.into(),
            buf_len as u32,
            buf_used.into(),
        )))
    }

    fn path_remove_directory_impl(
        &self,
        fd: api::Fd,
        path: Ptr<u8>,
        path_len: u32,
    ) -> api::Result<()> {
        let path = api::Path {
            items: path,
            count: path_len,
        };

        self.api.path_remove_directory(&self.memory, fd, path)?;
        Ok(())
    }

    /// Calls [`Api::path_remove_directory()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "path_remove_directory" (func
    ///     (param $fd i32)
    ///     (param $path i32)
    ///     (param $path_len i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn path_remove_directory(&self, fd: i32, path: i32, path_len: i32) -> Result<A> {
        Ok(result_to_error_code(self.path_remove_directory_impl(
            api::Fd::from_i32(fd),
            path.into(),
            path_len as u32,
        )))
    }

    fn path_rename_impl(
        &self,
        old_fd: api::Fd,
        old_path: Ptr<u8>,
        old_path_len: u32,
        new_fd: api::Fd,
        new_path: Ptr<u8>,
        new_path_len: u32,
    ) -> api::Result<()> {
        let old_path = api::Path {
            items: old_path,
            count: old_path_len,
        };
        let new_path = api::Path {
            items: new_path,
            count: new_path_len,
        };

        self.api
            .path_rename(&self.memory, old_fd, old_path, new_fd, new_path)?;

        Ok(())
    }

    /// Calls [`Api::path_rename()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "path_rename" (func
    ///     (param $old_fd i32)
    ///     (param $old_path i32)
    ///     (param $old_path_len i32)
    ///     (param $new_fd i32)
    ///     (param $new_path i32)
    ///     (param $new_path_len i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn path_rename(
        &self,
        old_fd: i32,
        old_path: i32,
        old_path_len: i32,
        new_fd: i32,
        new_path: i32,
        new_path_len: i32,
    ) -> Result<A> {
        Ok(result_to_error_code(self.path_rename_impl(
            api::Fd::from_i32(old_fd),
            old_path.into(),
            old_path_len as u32,
            api::Fd::from_i32(new_fd),
            new_path.into(),
            new_path_len as u32,
        )))
    }

    fn path_symlink_impl(
        &self,
        old_path: Ptr<u8>,
        old_path_len: u32,
        fd: api::Fd,
        new_path: Ptr<u8>,
        new_path_len: u32,
    ) -> api::Result<()> {
        let old_path = api::Path {
            items: old_path,
            count: old_path_len,
        };
        let new_path = api::Path {
            items: new_path,
            count: new_path_len,
        };

        self.api
            .path_symlink(&self.memory, old_path, fd, new_path)?;

        Ok(())
    }

    /// Calls [`Api::path_symlink()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "path_symlink" (func
    ///     (param $old_path i32)
    ///     (param $old_path_len i32)
    ///     (param $fd i32)
    ///     (param $new_path i32)
    ///     (param $new_path_len i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn path_symlink(
        &self,
        old_path: i32,
        old_path_len: i32,
        fd: i32,
        new_path: i32,
        new_path_len: i32,
    ) -> Result<A> {
        Ok(result_to_error_code(self.path_symlink_impl(
            old_path.into(),
            old_path_len as u32,
            api::Fd::from_i32(fd),
            new_path.into(),
            new_path_len as u32,
        )))
    }

    fn path_unlink_file_impl(&self, fd: api::Fd, path: Ptr<u8>, path_len: u32) -> api::Result<()> {
        let path = api::Path {
            items: path,
            count: path_len,
        };

        self.api.path_unlink_file(&self.memory, fd, path)?;
        Ok(())
    }

    /// Calls [`Api::path_unlink_file()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "path_unlink_file" (func
    ///     (param $fd i32)
    ///     (param $path i32)
    ///     (param $path_len i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn path_unlink_file(&self, fd: i32, path: i32, path_len: i32) -> Result<A> {
        Ok(result_to_error_code(self.path_unlink_file_impl(
            api::Fd::from_i32(fd),
            path.into(),
            path_len as u32,
        )))
    }

    fn poll_oneoff_impl(
        &self,
        r#in: Ptr<api::Subscription>,
        out: MutPtr<api::Event>,
        nsubscriptions: u32,
        nevents: MutPtr<u32>,
    ) -> api::Result<()> {
        nevents.store(
            &self.memory,
            self.api.poll_oneoff(
                &self.memory,
                api::EventPoll {
                    subscriptions: r#in,
                    events: out,
                    count: nsubscriptions,
                },
            )?,
        )?;

        Ok(())
    }

    /// Calls [`Api::poll_oneoff()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "poll_oneoff" (func
    ///     (param $in i32)
    ///     (param $out i32)
    ///     (param $nsubscriptions i32)
    ///     (param $nevents i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn poll_oneoff(&self, r#in: i32, out: i32, nsubscriptions: i32, nevents: i32) -> Result<A> {
        Ok(result_to_error_code(self.poll_oneoff_impl(
            r#in.into(),
            out.into(),
            nsubscriptions as u32,
            nevents.into(),
        )))
    }

    /// Calls [`Api::proc_exit()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "proc_exit" (func
    ///     (param $rval i32)
    ///     ;; (@witx noreturn)
    /// ))
    /// ```
    pub fn proc_exit(&self, rval: i32) -> core::result::Result<core::convert::Infallible, A::Trap> {
        Err(self.api.proc_exit(api::ExitCode(rval as u32)))
    }

    fn proc_raise_impl(&self, sig: u32) -> core::result::Result<api::Result<()>, A::Trap> {
        let signal = u8::try_from(sig)
            .map_err(|_| api::Errno::_inval)
            .and_then(api::Signal::try_from);

        match signal {
            Ok(sig) => self.api.proc_raise(sig),
            Err(errno) => Ok(Err(errno)),
        }
    }

    /// Calls [`Api::proc_raise()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "proc_raise" (func
    ///     (param $sig i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn proc_raise(&self, sig: i32) -> Result<A> {
        match self.proc_raise_impl(sig as u32) {
            Err(trap) => Err(trap),
            Ok(Err(errno)) => Ok(errno as i32),
            Ok(Ok(())) => Ok(SUCCESS),
        }
    }

    /// Calls [`Api::sched_yield()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "sched_yield" (func (result i32)))
    /// ```
    pub fn sched_yield(&self) -> Result<A> {
        Ok(result_to_error_code(self.api.sched_yield()))
    }

    fn random_get_impl(&self, buf: MutPtr<u8>, buf_len: u32) -> api::Result<()> {
        self.api.random_get(
            &self.memory,
            wasm2rs_rt_memory_typed::slice::MutSlice {
                items: buf,
                count: buf_len,
            },
        )?;

        Ok(())
    }

    /// Calls [`Api::random_get()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "random_get" (func
    ///     (param $buf i32)
    ///     (param $buf_len i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn random_get(&self, buf: i32, buf_len: i32) -> Result<A> {
        Ok(result_to_error_code(
            self.random_get_impl(buf.into(), buf_len as u32),
        ))
    }

    fn sock_accept_impl(&self, sock: api::Fd, flags: u32, ro_fd: MutPtr<Fd>) -> api::Result<()> {
        let flags =
            api::FdFlags::from_bits_retain(u16::try_from(flags).map_err(|_| api::Errno::_inval)?);

        ro_fd.store(&self.memory, self.api.sock_accept(sock, flags)?)?;
        Ok(())
    }

    /// Calls [`Api::sock_accept()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "sock_accept" (func
    ///     (param $sock i32)
    ///     (param $flags i32)
    ///     (param $ro_fd i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn sock_accept(&self, sock: i32, flags: i32, ro_fd: i32) -> Result<A> {
        Ok(result_to_error_code(self.sock_accept_impl(
            api::Fd::from_i32(sock),
            flags as u32,
            ro_fd.into(),
        )))
    }

    fn sock_recv_impl(
        &self,
        sock: api::Fd,
        ri_data: Ptr<api::IoVec>,
        ri_data_len: u32,
        ri_flags: u32,
        ro_data_len: MutPtr<u32>,
        ro_flags: MutPtr<api::RoFlags>,
    ) -> api::Result<()> {
        let ri_data = api::IoVecArray {
            items: ri_data,
            count: ri_data_len,
        };
        let ri_flags = api::RiFlags::from_bits_retain(
            u16::try_from(ri_flags).map_err(|_| api::Errno::_inval)?,
        );

        let (out_data_len, out_flags) =
            self.api.sock_recv(&self.memory, sock, ri_data, ri_flags)?;

        ro_data_len.store(&self.memory, out_data_len)?;
        ro_flags.store(&self.memory, out_flags)?;

        Ok(())
    }

    /// Calls [`Api::sock_recv()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "sock_recv" (func
    ///     (param $sock i32)
    ///     (param $ri_data i32)
    ///     (param $ri_data_len i32)
    ///     (param $ri_flags i32)
    ///     (param $ro_data_len i32)
    ///     (param $ro_flags i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn sock_recv(
        &self,
        sock: i32,
        ri_data: i32,
        ri_data_len: i32,
        ri_flags: i32,
        ro_data_len: i32,
        ro_flags: i32,
    ) -> Result<A> {
        Ok(result_to_error_code(self.sock_recv_impl(
            api::Fd::from_i32(sock),
            ri_data.into(),
            ri_data_len as u32,
            ri_flags as u32,
            ro_data_len.into(),
            ro_flags.into(),
        )))
    }

    fn sock_send_impl(
        &self,
        sock: api::Fd,
        si_data: Ptr<api::CIoVec>,
        si_data_len: u32,
        si_flags: u32,
        so_data_len: MutPtr<u32>,
    ) -> api::Result<()> {
        let si_data = api::CIoVecArray {
            items: si_data,
            count: si_data_len,
        };
        let si_flags = api::SiFlags::from_bits_retain(
            u16::try_from(si_flags).map_err(|_| api::Errno::_inval)?,
        );

        so_data_len.store(
            &self.memory,
            self.api.sock_send(&self.memory, sock, si_data, si_flags)?,
        )?;
        Ok(())
    }

    /// Calls [`Api::sock_send()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "sock_send" (func
    ///     (param $sock i32)
    ///     (param $ri_data i32)
    ///     (param $ri_data_len i32)
    ///     (param $ri_flags i32)
    ///     (param $ro_data_len i32)
    ///     (param $ro_flags i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn sock_send(
        &self,
        sock: i32,
        si_data: i32,
        si_data_len: i32,
        si_flags: i32,
        so_data_len: i32,
    ) -> Result<A> {
        Ok(result_to_error_code(self.sock_send_impl(
            api::Fd::from_i32(sock),
            si_data.into(),
            si_data_len as u32,
            si_flags as u32,
            so_data_len.into(),
        )))
    }

    fn sock_shutdown_impl(&self, sock: api::Fd, how: u32) -> api::Result<()> {
        let how =
            api::SdFlags::from_bits_retain(u8::try_from(how).map_err(|_| api::Errno::_inval)?);
        self.api.sock_shutdown(sock, how)
    }

    /// Calls [`Api::sock_shutdown()`].
    ///
    /// # Signature
    ///
    /// ```wat
    /// (import "wasi_snapshot_preview1" "sock_shutdown" (func
    ///     (param $sock i32)
    ///     (param $how i32)
    ///     (result i32)
    /// ))
    /// ```
    pub fn sock_shutdown(&self, sock: i32, how: i32) -> Result<A> {
        Ok(result_to_error_code(
            self.sock_shutdown_impl(api::Fd::from_i32(sock), how as u32),
        ))
    }
}
