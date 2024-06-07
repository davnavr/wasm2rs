use crate::memory::{Address, BoundsCheck, Memory};

/// Represents an [`Address`] to some struct `T` within a linear [`Memory<I>`].
#[derive(Clone, Copy, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Ptr<T: Pointee<I>, I: Address = u32> {
    address: I,
    _marker: core::marker::PhantomData<fn() -> T>,
}

impl<T: Pointee<I>, I: Address> Ptr<T, I> {
    /// Gets a pointer to [`Address`] `0`, which typically means `NULL` in many programming
    /// languages (e.g. C).
    pub const ZERO: Self = Self::from_address(I::ZERO);

    /// Creates a new [`Ptr`] to `T` from the given [`Address`].
    pub const fn from_address(address: I) -> Self {
        Self {
            address,
            _marker: core::marker::PhantomData,
        }
    }

    /// Gets the underlying numeric [`Address`] into the linear [`Memory`].
    pub const fn to_address(self) -> I {
        self.address
    }

    /// Calls [`Pointee::load_from()`].
    pub fn load<M: Memory<I> + ?Sized>(self, mem: &M) -> BoundsCheck<T> {
        T::load_from(mem, self.address)
    }

    /// Calls [`Pointee::store_into()`].
    pub fn store<M: Memory<I> + ?Sized>(self, mem: &M, value: T) -> BoundsCheck<()> {
        T::store_into(mem, self.address, value)
    }
}

impl<T: Pointee> From<i32> for Ptr<T> {
    fn from(address: i32) -> Self {
        Self::from_address(address as u32)
    }
}

impl<T: Pointee<I>, I: Address> core::fmt::Pointer for Ptr<T, I> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:#0width$x}",
            self.address,
            width = core::mem::size_of::<I>() * 2
        )
    }
}

impl<T: Pointee<I>, I: Address> core::fmt::Debug for Ptr<T, I> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Pointer::fmt(self, f)
    }
}

const fn alignment(align: usize) -> core::num::NonZeroUsize {
    match core::num::NonZeroUsize::new(align) {
        Some(ok) => ok,
        None => panic!("zero value"),
    }
}

/// Trait for types that support reading from and writing into linear [`Memory<I>`].
pub trait Pointee<I: Address = u32>: Sized {
    /// The size, in bytes, of the value.
    const SIZE: usize;

    /// What [`Ptr<Self>`] should be a multiple of for aligned accesses.
    const ALIGN: core::num::NonZeroUsize;

    // These methods could also be in the `MemoryExt` trait.

    /// Loads the value from linear [`Memory`].
    fn load_from<M: Memory<I> + ?Sized>(mem: &M, address: I) -> BoundsCheck<Self>;

    /// Stores the value into linear [`Memory`].
    fn store_into<M: Memory<I> + ?Sized>(mem: &M, address: I, value: Self) -> BoundsCheck<()>;
}

impl<T: Pointee<I>, I: Address + Pointee<I>> Pointee<I> for Ptr<T, I> {
    const SIZE: usize = core::mem::size_of::<I>();
    const ALIGN: core::num::NonZeroUsize = alignment(Self::SIZE);

    fn load_from<M: Memory<I> + ?Sized>(mem: &M, address: I) -> BoundsCheck<Self> {
        I::load_from(mem, address).map(Ptr::from_address)
    }

    fn store_into<M: Memory<I> + ?Sized>(mem: &M, address: I, value: Self) -> BoundsCheck<()> {
        I::store_into(mem, address, value.to_address())
    }
}

macro_rules! integer_pointee {
    ($($load:ident / $store:ident => $int:ty $(as $convert:ty)?;)*) => {$(

impl<I: Address> Pointee<I> for $int {
    const SIZE: usize = core::mem::size_of::<$int>();
    const ALIGN: core::num::NonZeroUsize = alignment(core::mem::size_of::<$int>());

    fn load_from<M: Memory<I> + ?Sized>(mem: &M, address: I) -> BoundsCheck<$int> {
        M::$load(mem, address) $(.map(|i: $convert| i as $int))?
    }

    fn store_into<M: Memory<I> + ?Sized>(mem: &M, address: I, value: $int) -> BoundsCheck<()> {
        M::$store(mem, address, value $(as $convert)?)
    }
}

    )*};
}

integer_pointee! {
    i8_load / i8_store => u8 as i8;
    i8_load / i8_store => i8;
    i16_load / i16_store => u16 as i16;
    i16_load / i16_store => i16;
    i32_load / i32_store => u32 as i32;
    i32_load / i32_store => i32;
    i64_load / i64_store => u64 as i64;
    i64_load / i64_store => i64;
}
