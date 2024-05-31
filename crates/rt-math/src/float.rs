//! Provides operations on floating-point values.

use crate::nan;

/// Implements [*NaN* propagation] for [`f32`] values.
///
/// [*NaN* propagation]: https://webassembly.github.io/spec/core/exec/numerics.html#nan-propagation
fn f32_propagate_nan(z_1: f32, z_2: f32) -> f32 {
    if nan::is_canonical_f32(z_1) && nan::is_canonical_f32(z_2) {
        f32::from_bits(nan::F32_CANONICAL)
    } else {
        // Let Rust pick a NaN value.
        z_1 + z_2
    }
}

/// Implements [*NaN* propagation] for [`64`] values.
///
/// [*NaN* propagation]: https://webassembly.github.io/spec/core/exec/numerics.html#nan-propagation
fn f64_propagate_nan(z_1: f64, z_2: f64) -> f64 {
    if nan::is_canonical_f64(z_1) && nan::is_canonical_f64(z_2) {
        f64::from_bits(nan::F64_CANONICAL)
    } else {
        // Let Rust pick a NaN value.
        z_1 + z_2
    }
}

macro_rules! zeroes_with_opposite_signs {
    ($z_1:ident, $z_2:ident) => {
        $z_1 == 0.0 && $z_2 == 0.0 && $z_1.is_sign_positive() == $z_2.is_sign_negative()
    };
}

/// Implements the [`f32.min`] WebAssembly instruction.
///
/// This corresponds to the [*fmin* operator].
///
/// [`f32.min`]: https://webassembly.github.io/spec/core/syntax/instructions.html#numeric-instructions
/// [*fmin* operator]: https://webassembly.github.io/spec/core/exec/numerics.html#op-fmin
pub fn f32_min(z_1: f32, z_2: f32) -> f32 {
    // `f32::minimum()` is currently nightly-only.
    if z_1.is_nan() || z_2.is_nan() {
        f32_propagate_nan(z_1, z_2)
    } else if zeroes_with_opposite_signs!(z_1, z_2) {
        -0.0
    } else {
        z_1.min(z_2)
    }
}

/// Implements the [`f64.min`] WebAssembly instruction.
///
/// This corresponds to the [*fmin* operator].
///
/// [`f64.min`]: https://webassembly.github.io/spec/core/syntax/instructions.html#numeric-instructions
/// [*fmin* operator]: https://webassembly.github.io/spec/core/exec/numerics.html#op-fmin
pub fn f64_min(z_1: f64, z_2: f64) -> f64 {
    // `f64::minimum()` is currently nightly-only.
    if z_1.is_nan() || z_2.is_nan() {
        f64_propagate_nan(z_1, z_2)
    } else if zeroes_with_opposite_signs!(z_1, z_2) {
        -0.0
    } else {
        z_1.min(z_2)
    }
}

/// Implements the [`f32.max`] WebAssembly instruction.
///
/// This corresponds to the [*fmax* operator].
///
/// [`f32.max`]: https://webassembly.github.io/spec/core/syntax/instructions.html#numeric-instructions
/// [*fmax* operator]: https://webassembly.github.io/spec/core/exec/numerics.html#op-fmax
pub fn f32_max(z_1: f32, z_2: f32) -> f32 {
    // `f32::fmaximum()` is currently nightly-only.
    if z_1.is_nan() || z_2.is_nan() {
        f32_propagate_nan(z_1, z_2)
    } else if zeroes_with_opposite_signs!(z_1, z_2) {
        0.0
    } else {
        z_1.max(z_2)
    }
}

/// Implements the [`f64.max`] WebAssembly instruction.
///
/// This corresponds to the [*fmax* operator].
///
/// [`f64.max`]: https://webassembly.github.io/spec/core/syntax/instructions.html#numeric-instructions
/// [*fmax* operator]: https://webassembly.github.io/spec/core/exec/numerics.html#op-fmax
pub fn f64_max(z_1: f64, z_2: f64) -> f64 {
    // `f64::maximum()` is currently nightly-only.
    if z_1.is_nan() || z_2.is_nan() {
        f64_propagate_nan(z_1, z_2)
    } else if zeroes_with_opposite_signs!(z_1, z_2) {
        0.0
    } else {
        z_1.max(z_2)
    }
}
