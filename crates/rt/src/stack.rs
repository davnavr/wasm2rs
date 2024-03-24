//! Functions for checking stack overflow conditions.

use crate::trap::Trap;

/// Checks if there is enough space on the stack for approximately `amount` bytes worth of
/// variables.
///
/// This function will not exactly detect all stack overflows. It may indicate that a stack
/// overflow would occur even if there was stack space remaining. It may also fail to detect a
/// stack overflow. Typical usage with `wasm2rs` will often overestimate the amount of stack space
/// used for functions.
///
/// If the `stack-overflow-detection` feature is enabled, it uses the [`stacker::remaining_stack()`]
/// function to estimate the remaining stack space. In situations where the remaining stack space
/// cannot be queried, or if the feature is not enabled, this function does nothing.
///
/// # Errors
///
/// If stack overflow detection is enabled and a stack overflow could occur, a [`Trap`] is produced.
#[inline]
pub fn check_for_overflow<T>(amount: usize, trap: &T) -> Result<(), T::Repr>
where
    T: Trap + ?Sized,
{
    #[cfg(not(feature = "stack-overflow-detection"))]
    return {
        let _ = amount;
        let _ = trap;
        Ok(())
    };

    #[cfg(feature = "stack-overflow-detection")]
    return {
        /// Constant amount to add to the `amount` to check for.
        ///
        /// In most ABIs, a function call pushes a return address and does other alignment stuff.
        /// This arbitrary value mostly accounts for extra stuff that may be pushed onto the stack.
        ///
        /// This also ensures extra space is reserved for calling [`Trap::trap_stack_overflow()`].
        const CALL_OVERHEAD: usize = 512;

        match stacker::remaining_stack() {
            None => Ok(()),
            Some(remaining) if remaining >= amount.saturating_add(CALL_OVERHEAD) => Ok(()),
            Some(_) => Err(trap.trap_stack_overflow()),
        }
    };
}
