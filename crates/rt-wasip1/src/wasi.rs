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
}
