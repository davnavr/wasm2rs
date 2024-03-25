//! Runtime support functions for simple math operations.

#[cold]
#[inline(never)]
fn integer_division_by_zero<E>(trap: &E) -> E::Repr
where
    E: crate::trap::Trap + ?Sized,
{
    trap.trap(crate::trap::TrapCode::IntegerDivisionByZero)
}

#[cold]
#[inline(never)]
fn integer_overflow<E>(trap: &E) -> E::Repr
where
    E: crate::trap::Trap + ?Sized,
{
    trap.trap(crate::trap::TrapCode::IntegerOverflow)
}

#[cold]
#[inline(never)]
fn conversion_to_integer<E>(trap: &E) -> E::Repr
where
    E: crate::trap::Trap + ?Sized,
{
    trap.trap(crate::trap::TrapCode::ConversionToInteger)
}

macro_rules! int_div {
    {$(
        $signed:ty => $div:ident = $div_name:literal $(as $unsigned:ty)?;
    )*} => {$(
        #[doc = concat!(
            "Implementation for the [`", $div_name, "`] instruction.\n\nCalculates `num / denom`,",
            " trapping on division by zero.\n\n",
            $(
                "The `num` and `denom` are interpreted as an [`", stringify!($unsigned), "`] ",
                "value, and the resulting [`", stringify!($unsigned), "`] quotient is ",
                "reinterpreted as an [`", stringify!($signed), "`] value.\n\n",
            )?
            "[`", $div_name, "`]: ",
            "https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-numeric"
        )]
        #[inline(always)]
        pub fn $div<E>(num: $signed, denom: $signed, trap: &E) -> Result<$signed, E::Repr>
        where
            E: crate::trap::Trap + ?Sized,
        {
            match (num $(as $unsigned)?).checked_div(denom $(as $unsigned)?) {
                Some(quot) => Ok(quot as $signed),
                _ if denom == 0 => Err(integer_division_by_zero(trap)),
                _ => Err(integer_overflow(trap)),
            }
        }
    )*};
}

int_div! {
    i32 => i32_div_s = "i32.div_s";
    i32 => i32_div_u = "i32.div_u" as u32;
    i64 => i64_div_s = "i64.div_s";
    i64 => i64_div_u = "i64.div_u" as u64;
}

macro_rules! int_rem {
    {$(
        $signed:ty => $rem:ident = $rem_name:literal $(as $unsigned:ty)?;
    )*} => {$(
        #[doc = concat!(
            "Implementation for the [`", $rem_name, "`] instruction.\n\nCalculates `num % denom`,",
            " trapping on division by zero.\n\n",
            $(
                "The `num` and `denom` are interpreted as an [`", stringify!($unsigned), "`] ",
                "value, and the resulting [`", stringify!($unsigned), "`] remainder is ",
                "reinterpreted as an [`", stringify!($signed), "`] value.\n\n",
            )?
            "[`", $rem_name, "`]: ",
            "https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-numeric"
        )]
        #[inline(always)]
        pub fn $rem<E>(num: $signed, denom: $signed, trap: &E) -> Result<$signed, E::Repr>
        where
            E: crate::trap::Trap + ?Sized,
        {
            if let Some(rem) = (num $(as $unsigned)?).checked_rem(denom $(as $unsigned)?) {
                Ok(rem as $signed)
            } else {
                Err(integer_division_by_zero(trap))
            }
        }
    )*};
}

int_rem! {
    i32 => i32_rem_s = "i32.rem_s";
    i32 => i32_rem_u = "i32.rem_u" as u32;
    i64 => i64_rem_s = "i64.rem_s";
    i64 => i64_rem_u = "i64.rem_u" as u64;
}

macro_rules! prefer_right {
    ($left: ty | $right: ty) => {
        $right
    };
    ($left: ty) => {
        $left
    };
}

macro_rules! iXX_trunc_fXX {
    {$(
        $float:ty => $trunc:ident = $trunc_name:literal -> $int:ty $(as $reinterpret:ty)?;
    )*} => {$(
        #[doc = concat!(
            "Implementation for the [`", $trunc_name, "`] instruction.\n\n",
            "Casts a [`", stringify!($float), "`] value to an [`", stringify!($int), "`], ",
            "[trapping] on [`", stringify!($float), "::NAN`], [`", stringify!($float),
            "::INFINITY`], [`",  stringify!($float), "::NEG_INFINITY`], and if the [`",
            stringify!($float), "`] value is too large to fit into an [`", stringify!($int),
            "`].\n\n",
            $(
                "The result is then reinterpreted as an [`", stringify!($reinterpret), "`] value.",
                "\n\n",
            )?
            "[trapping]: crate::trap::TrapCode::ConversionToInteger\n",
            "[`", $trunc_name,
            "`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-numeric"
        )]
        #[inline(always)]
        pub fn $trunc<E>(value: $float, trap: &E) -> Result<prefer_right!($int $(| $reinterpret)?), E::Repr>
        where
            E: crate::trap::Trap + ?Sized,
        {
            match <$int as num_traits::cast::NumCast>::from(value) {
                Some(n) => Ok(n $(as $reinterpret)?),
                None => Err(conversion_to_integer(trap)),
            }
        }
    )*};
}

iXX_trunc_fXX! {
    f32 => i32_trunc_f32_s = "i32.trunc_f32_s" -> i32;
    f64 => i32_trunc_f64_s = "i32.trunc_f64_s" -> i32;
    f32 => i32_trunc_f32_u = "i32.trunc_f32_u" -> u32 as i32;
    f64 => i32_trunc_f64_u = "i32.trunc_f64_u" -> u32 as i32;
    f32 => i64_trunc_f32_s = "i64.trunc_f32_s" -> i64;
    f64 => i64_trunc_f64_s = "i64.trunc_f64_s" -> i64;
    f32 => i64_trunc_f32_u = "i64.trunc_f32_u" -> u64 as i64;
    f64 => i64_trunc_f64_u = "i64.trunc_f64_u" -> u64 as i64;
}
