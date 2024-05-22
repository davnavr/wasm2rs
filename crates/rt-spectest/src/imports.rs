/// Implements the [`"spectest"`] imports used by some WebAssembly modules in the WebAssembly
/// specification test suite.
///
/// [`"spectest"`]: https://github.com/WebAssembly/spec/blob/wg-2.0.draft1/interpreter/README.md#spectest-host-module
pub struct SpecTestImports {
    //table: ,
    memory: wasm2rs_rt::memory::HeapMemory,
}

impl SpecTestImports {
    /// Allocates the default [`"memory"`] and `"table"`.
    ///
    /// # Panics
    ///
    /// Panics or aborts if a call to the global allocator failed.
    ///
    /// [`"memory"`]: SpecTestImports::memory()
    pub fn init() -> Self {
        Self {
            memory: wasm2rs_rt::memory::HeapMemory::with_limits(1, 2)
                .expect("linear memory allocation failure"),
        }
    }

    /// Provides access to the `"global_i32"` global import.
    ///
    /// # WebAssembly
    ///
    /// ## Definition
    /// ```wat
    /// (global (export "global_i32") i32)
    /// ```
    ///
    /// ## Import
    /// ```wat
    /// (global (import "spectest" "global_i32") i32)
    /// ```
    pub fn global_i32(&self) -> i32 {
        0xABBA
    }

    /// Provides access to the `"global_i64"` global import.
    ///
    /// # WebAssembly
    ///
    /// ## Definition
    /// ```wat
    /// (global (export "global_i64") i64)
    /// ```
    ///
    /// ## Import
    /// ```wat
    /// (global (import "spectest" "global_i64") i64)
    /// ```
    pub fn global_i64(&self) -> i64 {
        i64::from_le_bytes(*b"WebAssem")
    }

    /// Provides access to the `"global_f32"` global import.
    ///
    /// # WebAssembly
    ///
    /// ## Definition
    /// ```wat
    /// (global (export "global_f32") f32)
    /// ```
    ///
    /// ## Import
    /// ```wat
    /// (global (import "spectest" "global_f32") f32)
    /// ```
    pub fn global_f32(&self) -> f32 {
        core::f32::consts::E
    }

    /// Provides access to the `"global_f64"` global import.
    ///
    /// # WebAssembly
    ///
    /// ## Definition
    /// ```wat
    /// (global (export "global_f64") f64)
    /// ```
    ///
    /// ## Import
    /// ```wat
    /// (global (import "spectest" "global_f64") f64)
    /// ```
    pub fn global_f64(&self) -> f64 {
        core::f64::consts::TAU
    }

    //pub fn table

    /// Provides access to the `"memory"` linear memory import.
    ///
    /// # WebAssembly
    ///
    /// ## Definition
    /// ```wat
    /// (memory (export "memory") 1 2)
    /// ```
    ///
    /// ## Import
    /// ```wat
    /// (import "spectest" "memory" (memory 0))
    /// ```
    pub fn memory(&self) -> &wasm2rs_rt::memory::HeapMemory {
        &self.memory
    }

    /// Provides access to the `"print"` function import, which prints a single newline ('\n') character to
    /// standard output.
    ///
    /// # WebAssembly
    ///
    /// ## Definition
    /// ```wat
    /// (func (export "print"))
    /// ```
    ///
    /// ## Import
    /// ```wat
    /// (func $print (import "spectest" "print"))
    /// ```
    pub fn print(&self) -> Result<(), wasm2rs_rt::trap::TrapError> {
        println!();
        Ok(())
    }

    /// Provides access to the `"print_i32"` function import, which prints the provided [`i32`] value
    /// followed by a single newline ('\n') character to standard output.
    ///
    /// # WebAssembly
    ///
    /// ## Definition
    /// ```wat
    /// (func (export "print_i32") (param i32))
    /// ```
    ///
    /// ## Import
    /// ```wat
    /// (import "spectest" "print_i32" (func (param i32)))
    /// ```
    pub fn print_i32(&self, n: i32) -> Result<(), wasm2rs_rt::trap::TrapError> {
        println!("{n}");
        Ok(())
    }

    /// Provides access to the `"print_i64"` function import, which prints the provided [`i64`] value
    /// followed by a single newline ('\n') character to standard output.
    ///
    /// # WebAssembly
    ///
    /// ## Definition
    /// ```wat
    /// (func (export "print_i64") (param i64))
    /// ```
    ///
    /// ## Import
    /// ```wat
    /// (import "spectest" "print_i64" (func (param i64)))
    /// ```
    pub fn print_i64(&self, n: i64) -> Result<(), wasm2rs_rt::trap::TrapError> {
        println!("{n}");
        Ok(())
    }

    /// Provides access to the `"print_f32"` function import, which prints the provided [`f32`] value
    /// followed by a single newline ('\n') character to standard output.
    ///
    /// # WebAssembly
    ///
    /// ## Definition
    /// ```wat
    /// (func (export "print_f32") (param f32))
    /// ```
    ///
    /// ## Import
    /// ```wat
    /// (import "spectest" "print_f32" (func (param f32)))
    /// ```
    pub fn print_f32(&self, n: f32) -> Result<(), wasm2rs_rt::trap::TrapError> {
        println!("{n}");
        Ok(())
    }

    /// Provides access to the `"print_f64"` function import, which prints the provided [`f64`] value
    /// followed by a single newline ('\n') character to standard output.
    ///
    /// # WebAssembly
    ///
    /// ## Definition
    /// ```wat
    /// (func (export "print_f64") (param f64))
    /// ```
    ///
    /// ## Import
    /// ```wat
    /// (import "spectest" "print_f64" (func (param f64)))
    /// ```
    pub fn print_f64(&self, n: f64) -> Result<(), wasm2rs_rt::trap::TrapError> {
        println!("{n}");
        Ok(())
    }

    /// Provides access to the `"print_i32_f32"` function import, which prints its arguments
    /// followed by a single newline ('\n') character to standard output.
    ///
    /// # WebAssembly
    ///
    /// ## Definition
    /// ```wat
    /// (func (export "print_i32_f32") (param i32 f32))
    /// ```
    ///
    /// ## Import
    /// ```wat
    /// (import "spectest" "print_i32_f32" (func (param i32 f32)))
    /// ```
    pub fn print_i32_f32(&self, a: i32, b: f32) -> Result<(), wasm2rs_rt::trap::TrapError> {
        println!("{a} {b}");
        Ok(())
    }

    /// Provides access to the `"print_f64_f64"` function import, which prints its arguments
    /// followed by a single newline ('\n') character to standard output.
    ///
    /// # WebAssembly
    ///
    /// ## Definition
    /// ```wat
    /// (func (export "print_f64_f64") (param f64 f64))
    /// ```
    ///
    /// ## Import
    /// ```wat
    /// (import "spectest" "print_f64_f64" (func (param f64 f64)))
    /// ```
    pub fn print_f64_f64(&self, a: f64, b: f64) -> Result<(), wasm2rs_rt::trap::TrapError> {
        println!("{a} {b}");
        Ok(())
    }
}

impl std::fmt::Debug for SpecTestImports {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpecTestImports")
            //.field("table", &self.table)
            .field("memory", &self.memory)
            .finish()
    }
}
