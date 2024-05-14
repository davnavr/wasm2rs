// See `rt-core/src/symbol.rs`

/*
/// Maps the addresses of Rust functions generated by `wasm2rs` to a [`WasmSymbol`].
#[derive(Clone)]
pub struct WasmSymbolTable {
    lookup: fn(*const ()) -> Option<&'static WasmSymbol>,
    addresses: &'static [*const ()],
}

/// Iterates over the addresses and [`WasmSymbol`]s of a [`WasmSymbolTable`].
///
/// See the documentation for [`WasmSymbolTable::iter()`] for more information.
#[derive(Clone)]
pub struct WasmSymbolTableIter {
    lookup: fn(*const ()) -> Option<&'static WasmSymbol>,
    addresses: core::slice::Iter<'static, *const ()>,
}

impl core::iter::Iterator for WasmSymbolTableIter {
    type Item = (*const (), &'static WasmSymbol);

    fn next(&mut self) -> Option<Self::Item> {
        let address = *self.addresses.next()?;
        let symbol = (self.lookup)(address)?;
        Some((address, symbol))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.addresses.len(), Some(self.addresses.len()))
    }
}

impl core::iter::DoubleEndedIterator for WasmSymbolTableIter {
    fn next_back(&mut self) -> Option<Self::Item> {
        let address = *self.addresses.next_back()?;
        let symbol = (self.lookup)(address)?;
        Some((address, symbol))
    }
}

impl core::fmt::Debug for WasmSymbolTableIter {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_map().entries(self.clone()).finish()
    }
}

impl WasmSymbolTable {
    /// Creates a new table for looking up [`WasmSymbol`]s.
    ///
    /// The [`lookup`] should return `Some` for every pointer in the list of [`addresses`].
    ///
    /// The [`addresses`] are expected to be ordered such that the 0th element refers to the WASM
    /// function at index 0, the 1st element refers to the 1st WASM function, and so on.
    ///
    /// [`lookup`]: WasmSymbolTable::lookup()
    /// [`addresses`]: WasmSymbolTable::addresses()
    pub const fn new(
        lookup: fn(*const ()) -> Option<&'static WasmSymbol>,
        addresses: &'static [*const ()],
    ) -> Self {
        Self { lookup, addresses }
    }

    /// Returns a [`WasmSymbol`] corresponding to the address of a Rust function; or `None` if
    /// one could not be found.
    ///
    /// # Warning
    ///
    /// Note that due to current issues regarding [comparison of function pointers] in Rust, this
    /// function may return inconsistent results if optimizations merge two different functions with
    /// the same body.
    ///
    /// [comparison of function pointers]: https://github.com/rust-lang/rust/issues/70861
    pub fn lookup(&self, address: *const ()) -> Option<&'static WasmSymbol> {
        (self.lookup)(address)
    }

    /// Returns all of the addresses that have an associated [`WasmSymbol`].
    pub fn addresses(&self) -> &'static [*const ()] {
        self.addresses
    }

    /// Returns an iterator over each address and their associated [`WasmSymbol`].
    pub fn iter(&self) -> WasmSymbolTableIter {
        WasmSymbolTableIter {
            lookup: self.lookup,
            addresses: self.addresses.iter(),
        }
    }
}

impl core::fmt::Debug for WasmSymbolTable {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}
*/