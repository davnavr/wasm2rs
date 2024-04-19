#[derive(Debug)]
pub(crate) enum ArenaError {
    IndexTooLarge,
    ListLengthOverflow,
}

impl std::fmt::Display for ArenaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::IndexTooLarge => "index exceeded maximum",
            Self::ListLengthOverflow => "expression list length exceeded maximum",
        })
    }
}

impl std::error::Error for ArenaError {}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) struct ExprId {
    /// - Bits 0 to 2 contain a flag indicating what kind of expression is being referred to.
    ///   - `000` encodes a normal numeric index into the [`Arena`]. The index is stored in bits
    ///     3 to 31.
    ///   - `001` encodes a [`Literal::I32`].
    ///     - Bits 3..29 correspond to bits 0..26 of the encoded value.
    ///     - Bit 30 corresponds to bits 27..30 of the encoded value.
    ///     - Bit 31 corresponds to bit 31 of the encoded value.
    ///   - `010` encodes a [`Literal::I64`].
    ///     - Bits 3..29 correspond to bits 0..26 of the encoded value.
    ///     - Bit 30 corresponds to bits 27..62 of the encoded value.
    ///     - Bit 31 corresponds to bit 63 of the encoded value.
    ///   - `011` encodes a [`Literal::F32`] or a [`Literal::F64`]:
    ///     - Bit 31 corresponds to the sign bit of the encoded value.
    ///     - Bit 3 is set only if a [`Literal::F64`] is being encoded:
    ///       - If a [`Literal::F32`] is being encoded. then:
    ///         - Bits 30 to 23 correspond to the *exponent*.
    ///         - Bits 4 to 22 correspond to the high 19 bits of the `fraction`.
    ///       - If a [`Literal::F64`] is being encoded, then:
    ///         - Bits 30 to 20 correspond to the *exponent*.
    ///         - Bits 4 to 20 correspond to the high 17 bits of the `fraction`.
    ///   - `100` encodes a variable (a parameter or local in the original WebAssembly, or a new
    ///     temporary). The index is stored in bits 3..31.
    ///   - `101` encodes a translated data segment. The index is stored in bits 3..31.
    ///   - `110` encodes a translated element segment. The index is stored in bits 3..31.
    ///   - `111` encodes a function reference. The function index is stored in bits 3..31, except
    ///     when bits 3..31 are all set, indicating a `null` function reference.
    ///
    /// [`Literal::I32`]: crate::ast::Literal::I32
    /// [`Literal::I64`]: crate::ast::Literal::I64
    /// [`Literal::F32`]: crate::ast::Literal::F32
    /// [`Literal::F64`]: crate::ast::Literal::F64
    id: u32,
}

impl ExprId {
    const FLAG_LEN: u8 = 3;
    const CONTENT_MASK: u32 = u32::MAX << Self::FLAG_LEN;
    const FLAG_MASK: u32 = !Self::CONTENT_MASK;

    const ENCODE_INDEX: u32 = 0b000;
    const ENCODE_I32: u32 = 0b001;

    /// Gets all of the non-flag bits.
    const fn contents(self) -> u32 {
        (self.id & Self::CONTENT_MASK) >> Self::FLAG_LEN
    }

    const MAX_INDEX: u32 = Self::CONTENT_MASK >> Self::FLAG_LEN;

    const fn from_index(index: usize) -> Result<Self, ArenaError> {
        let index = index as u32;
        if index <= Self::MAX_INDEX {
            Ok(Self {
                id: index << Self::FLAG_LEN,
            })
        } else {
            Err(ArenaError::IndexTooLarge)
        }
    }

    const ENCODE_I32_LOW_MASK: u32 = 0x07FF_FFFF;
    const ENCODE_I32_HIGH_BIT: u32 = 0x8000_0000;
    const ENCODE_I32_REMAINING_MASK: u32 = !(Self::ENCODE_I32_LOW_MASK | Self::ENCODE_I32_HIGH_BIT);

    const fn from_i32(value: u32) -> Option<Self> {
        // Check if all of the specified bits have the same value
        if matches!(
            value & Self::ENCODE_I32_REMAINING_MASK,
            0 | Self::ENCODE_I32_REMAINING_MASK
        ) {
            Some(Self {
                // TODO: Need to add REMAINING_MASK bit
                id: Self::ENCODE_I32
                    | ((value & Self::ENCODE_I32_LOW_MASK) << Self::FLAG_LEN)
                    | (value & Self::ENCODE_I32_HIGH_BIT),
            })
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum DecodeExprId {
    Index(usize),
    I32(i32),
}

impl From<ExprId> for DecodeExprId {
    fn from(id: ExprId) -> Self {
        match id.id & ExprId::FLAG_MASK {
            ExprId::ENCODE_INDEX => Self::Index(id.contents() as usize),
            ExprId::ENCODE_I32 => {
                const REMAINING_BITS_FLAG: u32 = 0x4000_0000;

                let mut value = (id.contents() & ExprId::ENCODE_I32_LOW_MASK)
                    | (id.id & ExprId::ENCODE_I32_HIGH_BIT);

                if id.id & REMAINING_BITS_FLAG == REMAINING_BITS_FLAG {
                    value |= ExprId::ENCODE_I32_REMAINING_MASK;
                }

                Self::I32(value as i32)
            }
            unknown => unreachable!("encountered unknown ID type ({unknown:#03b})"),
        }
    }
}

impl std::fmt::Debug for ExprId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&DecodeExprId::from(*self), f)
    }
}

/// Used to refer to zero or more related [`Expr`]essions.
///
/// This is usually used for comma-separated lists of [`Expr`]essions, such as in function
/// arguments or result values.
///
/// [`Expr`]: crate::ast::Expr
#[derive(Clone, Copy)]
pub(crate) struct ExprListId {
    /// The index to the first expression in the list is the low 20 bits, while the `len` is
    /// calculated by adding 1 to the value in the high 12 bits.
    ///
    /// If all bits are set, then the list is empty.
    id: u32,
}

impl ExprListId {
    const INDEX_WIDTH: u8 = 20;
    const LEN_MASK: u32 = u32::MAX << Self::INDEX_WIDTH;
    const INDEX_MASK: u32 = !Self::LEN_MASK;

    pub(crate) const EMPTY: Self = Self { id: u32::MAX };
    pub(crate) const MAX_INDEX: u32 = Self::INDEX_MASK;
    pub(crate) const MAX_LEN: u32 = (Self::LEN_MASK >> Self::INDEX_WIDTH) - 1;

    pub(crate) fn new(index: u32, len: usize) -> crate::Result<Self, ArenaError> {
        let len = u32::try_from(len)
            .ok()
            .filter(|len| *len <= Self::MAX_LEN)
            .ok_or(ArenaError::ListLengthOverflow)?;

        if len == 0 {
            Ok(Self::EMPTY)
        } else if index <= Self::MAX_INDEX {
            let encoded = Self {
                id: index | ((len - 1) << Self::INDEX_WIDTH),
            };

            debug_assert!(!encoded.is_empty());

            Ok(encoded)
        } else {
            Err(ArenaError::IndexTooLarge)
        }
    }

    pub(crate) const fn is_empty(self) -> bool {
        self.id == Self::EMPTY.id
    }

    pub(crate) const fn len(self) -> u32 {
        if self.is_empty() {
            0
        } else {
            (self.id >> Self::INDEX_WIDTH) + 1
        }
    }

    pub(crate) const fn first(self) -> Option<u32> {
        if self.is_empty() {
            None
        } else {
            Some(self.id & Self::INDEX_MASK)
        }
    }
}

impl std::fmt::Debug for ExprListId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(first) = self.first() {
            std::fmt::Debug::fmt(&(first..first + self.len()), f)
        } else {
            f.debug_list().finish()
        }
    }
}

/// An arena used to contain [`Expr`]essions.
///
/// [`Expr`]: crate::ast::Expr
#[derive(Debug)]
pub(crate) struct Arena {
    arena: Vec<crate::ast::Expr>,
}

impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

impl Arena {
    pub(crate) const fn new() -> Self {
        Self { arena: Vec::new() }
    }

    fn allocate_inner(&mut self, expr: crate::ast::Expr) -> Result<ExprId, ArenaError> {
        // Literals may have a compact encoding available.
        if let crate::ast::Expr::Literal(literal) = &expr {
            use crate::ast::Literal;

            match literal {
                Literal::I32(i) => {
                    if let Some(encoded) = ExprId::from_i32(*i as u32) {
                        return Ok(encoded);
                    }
                }
                _ => (), // TODO: Implement encoding of other types of literals.
            }
        }

        let id = ExprId::from_index(self.arena.len())?;
        self.arena.push(expr);
        Ok(id)
    }

    pub(crate) fn allocate(
        &mut self,
        expr: impl Into<crate::ast::Expr>,
    ) -> Result<ExprId, ArenaError> {
        self.allocate_inner(expr.into())
    }

    pub(crate) fn get(&self, id: ExprId) -> crate::ast::Expr {
        use crate::ast::Literal;

        match DecodeExprId::from(id) {
            DecodeExprId::Index(index) => self.arena[index],
            DecodeExprId::I32(i32) => Literal::I32(i32).into(),
        }
    }

    pub(crate) fn get_list(&self, list: ExprListId) -> &[crate::ast::Expr] {
        if let Some(first) = list.first() {
            &self.arena[(first as usize)..][..list.len() as usize]
        } else {
            &[]
        }
    }

    pub(crate) fn allocate_many<E>(&mut self, expressions: E) -> Result<ExprListId, ArenaError>
    where
        E: IntoIterator<Item = ExprId>,
    {
        let start_len = self.arena.len();
        let start_index = u32::try_from(start_len).map_err(|_| ArenaError::IndexTooLarge)?;

        let expressions = expressions.into_iter();
        self.arena.reserve(expressions.size_hint().0);
        for id in expressions {
            self.arena.push(self.get(id));
        }

        ExprListId::new(start_index, self.arena.len() - start_len)
    }
}
