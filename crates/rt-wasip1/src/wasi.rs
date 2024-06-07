use crate::Api;
use wasm2rs_rt_memory_typed::Ptr;

/// Allows WebAssembly modules translated by `wasm2rs` to use a imported WASI functions provided by
/// an [`Api`] implementation.
#[derive(Clone, Debug)]
pub struct Wasi<A: Api> {
    memory: A::Memory,
    api: A,
}

/// Error code returned when an [`Api`] call is successful.
const SUCCESS: i32 = 0;

fn result_to_error_code(result: crate::Result<()>) -> i32 {
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

    fn arg_sizes_get_impl(&self, argc: Ptr<u32>, argv_buf_size: Ptr<u32>) -> crate::Result<()> {
        let crate::DataSizes { count, buf_size } = self.api.args_sizes_get()?;
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
        environ_count: Ptr<u32>,
        environ_buf_size: Ptr<u32>,
    ) -> crate::Result<()> {
        let crate::DataSizes { count, buf_size } = self.api.args_sizes_get()?;
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
        resolution: Ptr<crate::Timestamp>,
    ) -> crate::Result<()> {
        // `inval` used if clock is not supported, so it's used here when `clock_id` is garbage.
        let id = crate::ClockId::try_from(clock_id)?;
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
        precision: crate::Timestamp,
        time: Ptr<crate::Timestamp>,
    ) -> crate::Result<()> {
        let id = crate::ClockId::try_from(clock_id)?;
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
            precision as u64,
            time.into(),
        )))
    }

    fn fd_advise_impl(&self, fd: u32, offset: u64, len: u64, advice: u32) -> crate::Result<()> {
        let advice =
            crate::Advice::try_from(u8::try_from(advice).map_err(|_| crate::Errno::_inval)?)?;

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
            fd as u32,
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
            fd as u32,
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
        Ok(result_to_error_code(self.api.fd_close(fd as u32)))
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
        Ok(result_to_error_code(self.api.fd_datasync(fd as u32)))
    }
}
