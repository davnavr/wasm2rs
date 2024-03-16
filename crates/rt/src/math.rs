//! Runtime support functions for simple math operations.

#[cold]
#[inline(never)]
fn trap_division_by_zero<TR>(trap: &TR) -> TR::Repr
where
    TR: crate::trap::Trap + ?Sized,
{
    trap.trap(crate::trap::TrapCode::DivisionByZero)
}

/// Implementation for the `i32.rem_u` instruction.
///
/// Calculates `num % denom`, trapping on division by zero.
#[inline(always)]
pub fn i32_rem_u<TR>(num: i32, denom: i32, trap: &TR) -> Result<i32, TR::Repr>
where
    TR: crate::trap::Trap + ?Sized,
{
    if let Some(rem) = u32::checked_rem(num as u32, denom as u32) {
        Ok(rem as i32)
    } else {
        Err(trap_division_by_zero(trap))
    }
}
