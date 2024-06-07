//! Provides types and traits to allow for safer and easier accesses of linear memory in embedders
//! for `wasm2rs` code.

#![no_std]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)] // Unsafe code present in dependencies
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(unreachable_pub)]
#![deny(clippy::alloc_instead_of_core)]
#![deny(clippy::std_instead_of_alloc)]
#![deny(clippy::cast_possible_truncation)]
#![deny(clippy::exhaustive_enums)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

pub use wasm2rs_rt_memory as memory;

mod pointer;

pub use pointer::{MutPtr, Pointee, Ptr};

pub mod slice;

/// Implements the [`Pointee`] trait for `repr(C)`-like struct passed to WebAssembly.
#[macro_export]
macro_rules! wasm_struct {
    {$(
        $(#[$struct_meta:meta])*
        $struct_vis:vis struct $struct_name:ident {
            $(
                $(#[$field_meta:meta])*
                $field_vis:vis $field_name:ident: $field_type:ty,
            )*
        }
    )*} => {$(

$(#[$struct_meta])*
$struct_vis struct $struct_name {
    $(
        $(#[$field_meta])*
        $field_vis $field_name: $field_type,
    )*
}

impl<I: $crate::memory::Address> $crate::Pointee<I> for $struct_name {
    const SIZE: usize = {
        let mut size = 0;

        $(
            if size % <$field_type as $crate::Pointee<I>>::ALIGN.get() != 0 {
                // Need to align for next field
                size += size % <$field_type as $crate::Pointee<I>>::ALIGN.get();
            }

            size += <$field_type as $crate::Pointee<I>>::SIZE;
        )*

        size
    };

    const ALIGN: ::core::num::NonZeroUsize = {
        let mut align = 1;

        $(
            if align < <$field_type as $crate::Pointee<I>>::ALIGN.get() {
                align = <$field_type as $crate::Pointee<I>>::ALIGN.get();
            }
        )*

        match ::core::num::NonZeroUsize::new(align) {
            Some(ok) => ok,
            None => panic!("alignment of zero"),
        }
    };

    fn load_from<M>(mem: &M, address: I) -> $crate::memory::BoundsCheck<Self>
    where
        M: $crate::memory::Memory<I> + ?Sized,
    {
       let mut offset = address;

       $(
            let realign = offset % I::cast_from_usize(<$field_type as $crate::Pointee<I>>::ALIGN.get());
            if realign != I::ZERO {
                // Need to align for next field
                offset += realign;
            }

            let $field_name = <$field_type as $crate::Pointee<I>>::load_from(
                mem,
                $crate::memory::EffectiveAddress::with_offset(offset, address).calculate()?
            )?;
       )*

       Ok(Self { $($field_name),* })
    }

    fn store_into<M>(mem: &M, address: I, value: Self) -> $crate::memory::BoundsCheck<()>
    where
        M: $crate::memory::Memory<I> + ?Sized,
    {
        let mut offset = address;

        $(
            let realign = offset % I::cast_from_usize(<$field_type as $crate::Pointee<I>>::ALIGN.get());
            if realign != I::ZERO {
                // Need to align for next field
                offset += realign;
            }

            <$field_type as $crate::Pointee<I>>::store_into(
                mem,
                $crate::memory::EffectiveAddress::with_offset(offset, address).calculate()?,
                value.$field_name,
            )?;
       )*

        Ok(())
    }
}

    )*};
}

/// Implements the [`Pointee`] trait for a `repr(transparent)` struct wrapping a type already
/// implementing [`Pointee`].
#[macro_export]
macro_rules! wasm_transparent_struct {
    {
        $(#[$struct_meta:meta])*
        $struct_vis:vis struct $struct_name:ident($field_vis:vis $field_type:ty);
    } => {

$(#[$struct_meta])*
#[repr(transparent)]
$struct_vis struct $struct_name($field_vis $field_type);

impl<I: $crate::memory::Address> $crate::Pointee<I> for $struct_name
where
    $field_type: $crate::Pointee<I>,
{
    const SIZE: usize = <$field_type as $crate::Pointee<I>>::SIZE;
    const ALIGN: ::core::num::NonZeroUsize = <$field_type as $crate::Pointee<I>>::ALIGN;

    fn load_from<M>(mem: &M, address: I) -> $crate::memory::BoundsCheck<Self>
    where
        M: $crate::memory::Memory<I> + ?Sized,
    {
        <$field_type as $crate::Pointee<I>>::load_from(mem, address).map(Self)
    }

    fn store_into<M>(mem: &M, address: I, value: Self) -> $crate::memory::BoundsCheck<()>
    where
        M: $crate::memory::Memory<I> + ?Sized,
    {
        <$field_type as $crate::Pointee<I>>::store_into(mem, address, value.0)
    }
}

    };
}
