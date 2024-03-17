use crate::rust::AnyIdent;

/// Represents a Rust [path].
///
/// [path]: https://doc.rust-lang.org/reference/paths.html
#[derive(Clone, Copy)]
pub struct Path<'a, N>
where
    N: Clone + IntoIterator<Item = AnyIdent<'a>>,
{
    global: bool,
    namespace: N,
    name: AnyIdent<'a>,
}

#[allow(missing_docs)]
impl<'a, N> Path<'a, N>
where
    N: Clone + IntoIterator<Item = AnyIdent<'a>>,
{
    /// Creates a new path.
    pub const fn new(global: bool, namespace: N, name: AnyIdent<'a>) -> Self {
        Self {
            global,
            namespace,
            name,
        }
    }

    pub fn namespace(&self) -> &N {
        &self.namespace
    }

    pub fn name(&self) -> &AnyIdent<'a> {
        &self.name
    }

    pub fn is_global(&self) -> bool {
        self.global
    }
}

impl<'a, N> std::fmt::Display for Path<'a, N>
where
    N: Clone + IntoIterator<Item = AnyIdent<'a>>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.global {
            f.write_str("::")?;
        }

        for (i, name) in self.namespace.clone().into_iter().enumerate() {
            if i > 0 {
                f.write_str("::")?;
            }

            write!(f, "{name}")?;
        }

        write!(f, "{}", self.name)
    }
}

impl<'a, N> std::fmt::Debug for Path<'a, N>
where
    N: Clone + IntoIterator<Item = AnyIdent<'a>>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}
