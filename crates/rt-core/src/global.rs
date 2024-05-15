//! Implementation for [WebAssembly global] variables.
//!
//! [WebAssembly global]: https://webassembly.github.io/spec/core/syntax/modules.html#globals

// TODO: This could be moved to the `rt-umbrella` crate so `rt-simd` doesn't have to depend on `rt-core`.

mod private {
    #[derive(Debug)]
    #[non_exhaustive]
    pub struct Private;
}

/// Trait for types of value that can be stored in a [`Global`].
pub trait GlobalValue: Clone {
    /// The default value for globals of this type.
    const ZERO: Self;

    /// Workaround for a lack of specialization in stable Rust.
    ///
    /// Allows [`Copy`]ing the value from a [`Cell<Self>`](core::cell::Cell).
    #[inline]
    fn try_copy(cell: &core::cell::Cell<Self>, _: private::Private) -> Option<Self> {
        let _ = cell;
        None
    }
}

impl GlobalValue for i32 {
    const ZERO: i32 = 0;

    #[inline(always)]
    fn try_copy(cell: &core::cell::Cell<Self>, _: private::Private) -> Option<Self> {
        Some(cell.get())
    }
}

impl GlobalValue for i64 {
    const ZERO: i64 = 0;

    #[inline(always)]
    fn try_copy(cell: &core::cell::Cell<Self>, _: private::Private) -> Option<Self> {
        Some(cell.get())
    }
}

impl GlobalValue for f32 {
    const ZERO: f32 = 0.0;

    #[inline(always)]
    fn try_copy(cell: &core::cell::Cell<Self>, _: private::Private) -> Option<Self> {
        Some(cell.get())
    }
}

impl GlobalValue for f64 {
    const ZERO: f64 = 0.0;

    #[inline(always)]
    fn try_copy(cell: &core::cell::Cell<Self>, _: private::Private) -> Option<Self> {
        Some(cell.get())
    }
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
        Self::ZERO
    }
}

impl<T: GlobalValue> Global<T> {
    /// The global variable with the default value.
    pub const ZERO: Self = Self::new(T::ZERO);

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
        if let Some(copy) = T::try_copy(&self.contents, private::Private) {
            // Specialized path for well-known `Copy` types.
            f(&copy)
        } else {
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
    }

    /// Gets the value of the global variable by [`Copy`]ing it.
    ///
    /// This exists as a separate method due to a lack of specialization in stable Rust. If you
    /// know `T: Copy`, then call this method rather than [`Global::get()`].
    pub fn get_copied(&self) -> T
    where
        T: Copy,
    {
        self.contents.get()
    }

    /// Gets the value of the global variable.
    ///
    /// This implements the [`global.get`] instruction. Internally, this uses a workaround for a
    /// lack of specialization in stable Rust to simply [`Copy`] the value if `T` is an [`i32`],
    /// [`i64`], [`f32`], or an [`f64`]. If you know `T: Copy`, you can call
    /// [`Global::get_copied()`] instead.
    ///
    /// [`global.get`]: https://webassembly.github.io/spec/core/syntax/instructions.html#variable-instructions
    pub fn get(&self) -> T {
        if let Some(copy) = T::try_copy(&self.contents, private::Private) {
            copy
        } else {
            // This branch should get optimized away if `T` is copied.
            self.borrow_with(Clone::clone)
        }
    }

    /// Gets the underlying value.
    pub fn into_inner(self) -> T {
        self.contents.into_inner()
    }

    /// Gets a mutable reference to the underlying value.
    pub fn get_mut(&mut self) -> &mut T {
        self.contents.get_mut()
    }
}

impl<T: GlobalValue> From<T> for Global<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T: GlobalValue> Clone for Global<T> {
    fn clone(&self) -> Self {
        Self::new(self.get())
    }
}

impl<T: GlobalValue> PartialEq for Global<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.borrow_with(|this| other.borrow_with(|other| this.eq(other)))
    }
}

impl<T: GlobalValue + Eq> Eq for Global<T> {}

impl<T: GlobalValue + core::cmp::PartialOrd> core::cmp::PartialOrd for Global<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.borrow_with(|this| other.borrow_with(|other| this.partial_cmp(other)))
    }
}

impl<T: GlobalValue + core::cmp::Ord> core::cmp::Ord for Global<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.borrow_with(|this| other.borrow_with(|other| this.cmp(other)))
    }
}

impl<T: GlobalValue + core::hash::Hash> core::hash::Hash for Global<T> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.borrow_with(|this| this.hash(state))
    }
}

macro_rules! fmt_traits {
    ($($fmt:path,)+) => {$(
        impl<T: GlobalValue + $fmt> $fmt for Global<T> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.borrow_with(|contents| <T as $fmt>::fmt(contents, f))
            }
        }
    )+};
}

fmt_traits! {
    core::fmt::Debug,
    core::fmt::Display,
    core::fmt::Binary,
    core::fmt::Octal,
    core::fmt::LowerHex,
    core::fmt::UpperHex,
    core::fmt::Pointer,
    core::fmt::LowerExp,
    core::fmt::UpperExp,
}
