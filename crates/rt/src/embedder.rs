//! Default module used when embedding a WebAssembly module with no imports.

use crate::trap::{Trap, TrapCode};

pub use crate as rt;

/// The default memory implementation to use for the WebAssembly module's main memory.
#[cfg(feature = "alloc")]
pub type Memory0 = crate::memory::HeapMemory32;

/// An empty memory implementation to use for the WebAssembly module's main memory.
///
/// If the `alloc` feature is enabled, then heap allocations are used instead.
#[cfg(not(feature = "alloc"))]
pub type Memory0 = crate::memory::EmptyMemory;

/// Type used for the result of WebAssembly computations.
///
/// An `Err` indicates that a trap has occured.
pub type Result<T> = ::core::result::Result<T, crate::trap::TrapValue>;

/// By default, it is assumed a WebAssembly module has no imports.
///
/// Use the [`embedder_with_import!`] macro to define a new embedder if imports need to be
/// provided to a WebAssembly module.
///
/// [`embedder_with_import!`]: crate::embedder_with_import!
pub type Imports = ();

/// The default embedder state.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct State<I = ()> {
    imports: I,
}

impl<I> State<I> {
    /// Intiializes the embedder state with the given `imports`.
    pub fn new(imports: I) -> Self {
        Self { imports }
    }

    /// Initializes the WebAssembly module's main memory.
    pub fn initialize_mem_0<const IDX: u32, const MIN: u32, const MAX: u32>(
        &self,
    ) -> Result<Memory0> {
        #[cfg(not(feature = "alloc"))]
        return Err(self.trap(TrapCode::MemoryAllocation {
            memory: IDX,
            error: crate::memory::AllocationError::with_size(MIN),
        }));

        #[cfg(feature = "alloc")]
        return Memory0::with_limits(MIN, MAX)
            .map_err(|error| self.trap(TrapCode::MemoryAllocation { memory: IDX, error }));
    }

    /// Gets access to the module's imports.
    pub fn imports(&self) -> &I {
        &self.imports
    }
}

impl<I> Trap for State<I> {
    type Repr = crate::trap::TrapValue;

    #[inline(never)]
    fn trap(&self, code: TrapCode) -> Self::Repr {
        <Self::Repr>::new(code)
    }
}

/// Defines a new embedder module using the given type as the struct used to contain all of a
/// WebAssembly module's imports.
#[macro_export]
macro_rules! embedder_with_import {
    {
        $vis:vis mod $embedder:ident($imports:tt) $(use {
            $($import_namespace:tt as $import_alias:ident),*
        })?
    } => {
        $vis mod $embedder {
            pub use $crate::embedder::{rt, Memory0, Result};

            /// Contains the imports accessed by the WebAssembly module.
            pub type Imports = super::$imports;

            /// State for the embedder of the WebAssembly module.
            pub type State = $crate::embedder::State<Imports>;

            $($(
                #[allow(missing_docs)]
                pub type $import_alias = super::$import_namespace;
            )*)?
        }
    };
    {
        $vis:vis mod ($imports:tt) $(use {
            $($import_namespace:tt as $import_alias:ident),*
        })?
    } => {
        $crate::embedder_with_import! {
            $vis mod embedder($imports) $(use {
                $($import_namespace as $import_alias),*
            })?
        }
    };
}
