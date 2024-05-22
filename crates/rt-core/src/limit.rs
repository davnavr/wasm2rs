//! Provides functions for validating [WebAssembly limits].
//!
//! [WebAssembly limits]: https://webassembly.github.io/spec/core/syntax/types.html#limits

/// Error type used when WebAssembly *limits* do not [match].
///
/// See the documentation for the [`limits::check()`] function for more information.
///
/// [match]: https://webassembly.github.io/spec/core/valid/types.html#match-limits
/// [`limits::check()`]: check()
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[allow(clippy::exhaustive_enums)]
pub enum LimitsCheckError {
    /// The minimum is greater than the maximum.
    Invalid,
    /// The minimum is less than expected.
    MinimumTooSmall,
    /// The maximum is greater than expected.
    MaximumTooLarge,
}

/// Determines if the given WebAssembly *limits* [match].
///
/// # Errors
///
/// Returns an error if:
/// - The `minimum` is greater than the `maximum`.
/// - The `minimum` is less than the `expected_minimum`.
/// - The `maximum` is greater than the `expected_maximum`.
///
/// [match]: https://webassembly.github.io/spec/core/valid/types.html#match-limits
pub fn check<I>(
    minimum: I,
    maximum: I,
    expected_minimum: I,
    expected_maximum: I,
) -> Result<(), LimitsCheckError>
where
    I: PartialOrd,
{
    if minimum > maximum {
        Err(LimitsCheckError::Invalid)
    } else if minimum < expected_minimum {
        Err(LimitsCheckError::MinimumTooSmall)
    } else if maximum > expected_maximum {
        Err(LimitsCheckError::MaximumTooLarge)
    } else {
        Ok(())
    }
}
