//! Implementation for [WebAssembly global] variables.
//!
//! [WebAssembly global]: https://webassembly.github.io/spec/core/syntax/modules.html#globals

/// Trait for types of value that can be stored in a [`Global`].
pub trait GlobalValue {
    /// The default value for globals of this type.
    const ZERO: Self;
}

impl GlobalValue for i32 {
    const ZERO: i32 = 0;
}

impl GlobalValue for i64 {
    const ZERO: i64 = 0;
}

impl GlobalValue for f32 {
    const ZERO: f32 = 0.0;
}

impl GlobalValue for f64 {
    const ZERO: f64 = 0.0;
}

/// Represents a [WebAssembly global] variable.
///
/// [WebAssembly global]: https://webassembly.github.io/spec/core/syntax/modules.html#globals
#[repr(transparent)]
pub struct Global<T: GlobalValue> {
    contents: core::cell::Cell<T>,
}

impl<T: GlobalValue> Default for Global<T> {
    fn default() -> Self {
        Self::new(T::ZERO)
    }
}

impl<T: GlobalValue> Global<T> {
    /// Creates a new global variable with the specified initial value.
    pub const fn new(value: T) -> Self {
        Self {
            contents: core::cell::Cell::new(value),
        }
    }

    /// Sets the value of the global variable.
    ///
    /// This implements the [`global.set`] instruction.
    ///
    /// [`global.set`]: https://webassembly.github.io/spec/core/syntax/instructions.html#variable-instructions
    pub fn set(&self, value: T) {
        self.contents.set(value)
    }

    fn borrow_with<F, R>(&self, f: F) -> R
    where
        F: for<'a> FnOnce(&'a T) -> R,
    {
        struct Fixup<'a, T: GlobalValue> {
            cell: &'a core::cell::Cell<T>,
            contents: Option<T>,
        }

        impl<T: GlobalValue> Drop for Fixup<'_, T> {
            /// If the closure panics, this ensures the original value is put back in the
            /// [`Global`].
            fn drop(&mut self) {
                // Forgetting `ZERO` shouldn't have an effect.
                core::mem::forget(self.cell.replace(self.contents.take().unwrap()));
            }
        }

        let fixup = Fixup {
            cell: &self.contents,
            contents: Some(self.contents.replace(T::ZERO)),
        };

        f(fixup.contents.as_ref().unwrap())
    }

    /// Gets the value of the global variable, using the given closure to clone it.
    ///
    /// This implements the [`global.get`] instruction for non-[`Copy`] types.
    ///
    /// The closure **should not** modify or observe the contents of the global, as it may observe
    /// nonsensical results.
    ///
    /// [`global.get`]: https://webassembly.github.io/spec/core/syntax/instructions.html#variable-instructions
    pub fn get_with<F>(&self, f: F) -> T
    where
        F: for<'a> FnOnce(&'a T) -> T,
    {
        self.borrow_with(f)
    }

    /// Gets the value of the global variable.
    ///
    /// This implements the [`global.get`] instruction for [`Copy`] types.
    ///
    /// [`global.get`]: https://webassembly.github.io/spec/core/syntax/instructions.html#variable-instructions
    pub fn get(&self) -> T
    where
        T: Copy,
    {
        self.contents.get()
    }
}

impl<T: GlobalValue + Clone> Clone for Global<T> {
    fn clone(&self) -> Self {
        Self::new(self.get_with(Clone::clone))
    }
}

impl<T: GlobalValue + PartialEq> PartialEq for Global<T> {
    fn eq(&self, other: &Self) -> bool {
        self.borrow_with(|x| other.borrow_with(|y| x.eq(y)))
    }
}

impl<T: GlobalValue + Eq> Eq for Global<T> {}

impl<T: GlobalValue, U> PartialEq<&U> for Global<T>
where
    T: PartialEq<U>,
{
    fn eq(&self, other: &&U) -> bool {
        self.borrow_with(|this| this.eq(*other))
    }
}

impl<T: GlobalValue + core::fmt::Debug> core::fmt::Debug for Global<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.borrow_with(|contents| core::fmt::Debug::fmt(contents, f))
    }
}
