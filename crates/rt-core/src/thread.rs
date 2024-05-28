//! Provides the [`panicking()`] function.

/// If the `std` feature is enabled, calls [`std::thread::panicking()`]. Otherwise, returns `false`.
pub fn panicking() -> bool {
    #[cfg(feature = "std")]
    return std::thread::panicking();

    #[cfg(not(feature = "std"))]
    return false;
}
