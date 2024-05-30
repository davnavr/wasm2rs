/// Implements [`externref`]s as integers for the purposes of the WebAssembly specification tests.
///
/// [`externref`]: https://webassembly.github.io/spec/core/syntax/types.html#reference-types
#[derive(Clone, Copy, Default, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct HostRef(pub Option<usize>);

impl std::fmt::Debug for HostRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            None => f.write_str("Null"),
            Some(n) => write!(f, "{n:#X}"),
        }
    }
}

impl wasm2rs_rt::table::TableElement for HostRef {}

impl wasm2rs_rt::table::NullableTableElement for HostRef {
    const NULL: Self = Self(None);

    fn is_null(&self) -> bool {
        self.0.is_none()
    }
}
