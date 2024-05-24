//! Compact encodings for [`Expr`](crate::ast::Expr)s.

use crate::ast::ArenaError;

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
    ///     temporary).
    ///     - Bit 3 is set if the variable corresponds to a WebAssembly parameter or local.
    ///     - Bits 4 to 31 store the index.
    ///   - `101` encodes a translated data segment. The index is stored in bits 3 to 31.
    ///   - `110` encodes a translated element segment. The index is stored in bits 3 to 31.
    ///   - `111` encodes a function reference. The function index is stored in bits 3 to 31,
    ///     except when bits 3 to 31 are all set, indicating a `null` function reference.
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
    const ENCODE_VARIABLE: u32 = 0b100;

    /// Gets all of the non-flag bits.
    const fn contents(self) -> u32 {
        (self.id & Self::CONTENT_MASK) >> Self::FLAG_LEN
    }

    const ENCODE_INDEX_MAX: u32 = Self::CONTENT_MASK >> Self::FLAG_LEN;

    /// An [`ExprId`] referring to an existing [`Expr`](crate::ast::Expr) in the [`Arena`].
    pub(in crate::ast) const fn from_index(index: usize) -> Result<Self, ArenaError> {
        let index = index as u32;
        if index <= Self::ENCODE_INDEX_MAX {
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

    pub(in crate::ast) const fn from_i32(value: u32) -> Option<Self> {
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

    const ENCODE_VARIABLE_MAX_INDEX: u32 = Self::CONTENT_MASK >> (Self::FLAG_LEN + 1);

    pub(in crate::ast) const fn from_temporary(temporary: crate::ast::TempId) -> Option<Self> {
        if temporary.0 <= Self::ENCODE_VARIABLE_MAX_INDEX {
            Some(Self {
                id: (temporary.0 << (Self::FLAG_LEN + 1)) | Self::ENCODE_VARIABLE,
            })
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum DecodeExprId {
    Index(usize),
    I32(i32),
    Temporary(crate::ast::TempId),
    Local(crate::ast::LocalId),
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
            ExprId::ENCODE_VARIABLE => {
                let index = id.contents() >> 1;
                if id.contents() & 1 == 0 {
                    Self::Temporary(crate::ast::TempId(index))
                } else {
                    Self::Local(crate::ast::LocalId(index))
                }
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
