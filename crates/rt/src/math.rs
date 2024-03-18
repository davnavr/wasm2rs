//! Runtime support functions for simple math operations.

#[cold]
#[inline(never)]
fn integer_division_by_zero<TR>(trap: &TR) -> TR::Repr
where
    TR: crate::trap::Trap + ?Sized,
{
    trap.trap(crate::trap::TrapCode::IntegerDivisionByZero)
}

#[cold]
#[inline(never)]
fn integer_overflow<TR>(trap: &TR) -> TR::Repr
where
    TR: crate::trap::Trap + ?Sized,
{
    trap.trap(crate::trap::TrapCode::IntegerOverflow)
}

macro_rules! int_ops {
    {$(
        $signed:ty | $unsigned:ty {
            $div_s:ident = $div_s_name:literal;
            $div_u:ident = $div_u_name:literal;
            $rem_s:ident = $rem_s_name:literal;
            $rem_u:ident = $rem_u_name:literal;
        }
    )*} => {$(
        #[doc = "Implementation for the [`"]
        #[doc = $div_s_name]
        #[doc = "`] instruction.\n\nCalculates `num / denom`, trapping on division by zero.\n\n"]
        #[doc = "[`"]
        #[doc = $div_s_name]
        #[doc = "`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-numeric"]
        #[inline(always)]
        #[doc(alias = $div_s_name)]
        pub fn $div_s<TR>(num: $signed, denom: $signed, trap: &TR) -> Result<$signed, TR::Repr>
        where
            TR: crate::trap::Trap + ?Sized,
        {
            match <$signed>::checked_div(num, denom) {
                Some(quot) => Ok(quot),
                _ if denom == 0 => Err(integer_division_by_zero(trap)),
                _ => Err(integer_overflow(trap)),
            }
        }

        #[doc = "Implementation for the [`"]
        #[doc = $div_u_name]
        #[doc = "`] instruction.\n\nInterprets parameters as an unsigned integer, then calculates"]
        #[doc = " `num / denom`, trapping on division by zero.\n\n [`"]
        #[doc = $div_u_name]
        #[doc = "`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-numeric"]
        #[inline(always)]
        #[doc(alias = $div_u_name)]
        pub fn $div_u<TR>(num: $signed, denom: $signed, trap: &TR) -> Result<$signed, TR::Repr>
        where
            TR: crate::trap::Trap + ?Sized,
        {
            if let Some(quot) = <$unsigned>::checked_div(num as $unsigned, denom as $unsigned) {
                Ok(quot as $signed)
            } else {
                Err(integer_division_by_zero(trap))
            }
        }

        #[doc = "Implementation for the [`"]
        #[doc = $rem_s_name]
        #[doc = "`] instruction.\n\nCalculates `num % denom`, trapping on division by zero.\n\n [`"]
        #[doc = $rem_s_name]
        #[doc = "`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-numeric"]
        #[inline(always)]
        #[doc(alias = $rem_u_name)]
        pub fn $rem_s<TR>(num: $signed, denom: $signed, trap: &TR) -> Result<$signed, TR::Repr>
        where
            TR: crate::trap::Trap + ?Sized,
        {
            if let Some(rem) = <$signed>::checked_rem(num, denom) {
                Ok(rem)
            } else {
                Err(integer_division_by_zero(trap))
            }
        }

        #[doc = "Implementation for the [`"]
        #[doc = $rem_u_name]
        #[doc = "`] instruction.\n\nInterprets parameters as an unsigned integer, then calculates"]
        #[doc = " `num % denom`, trapping on division by zero.\n\n [`"]
        #[doc = $rem_u_name]
        #[doc = "`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-numeric"]
        #[inline(always)]
        #[doc(alias = $rem_u_name)]
        pub fn $rem_u<TR>(num: $signed, denom: $signed, trap: &TR) -> Result<$signed, TR::Repr>
        where
            TR: crate::trap::Trap + ?Sized,
        {
            if let Some(rem) = <$unsigned>::checked_rem(num as $unsigned, denom as $unsigned) {
                Ok(rem as $signed)
            } else {
                Err(integer_division_by_zero(trap))
            }
        }
    )*};
}

int_ops! {
    i32 | u32 {
        i32_div_s = "i32.div_s";
        i32_div_u = "i32.div_u";
        i32_rem_s = "i32.rem_s";
        i32_rem_u = "i32.rem_u";
    }

    i64 | u64 {
        i64_div_s = "i64.div_s";
        i64_div_u = "i64.div_u";
        i64_rem_s = "i64.rem_s";
        i64_rem_u = "i64.rem_u";
    }
}
