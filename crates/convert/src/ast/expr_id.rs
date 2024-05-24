//! Compact encodings for [`Expr`](crate::ast::Expr)s.

use crate::ast::ArenaError;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) struct ExprId {
    /// The high 3 bits (bits 29 to 31) contain a flag indicating what kind of expression is being
    /// referred to, while the low 29 bits contain additional information about the expression.
    ///
    /// See the comments for the various `FLAG_` constants for more information.
    id: u32,
}

impl ExprId {
    const LEN_FLAG: u8 = 3;
    const LEN_CONTENT: u8 = 29;

    const _CHECK_LEN: [(); 32] = [(); (Self::LEN_FLAG + Self::LEN_CONTENT) as usize];

    const MASK_CONTENT: u32 = u32::MAX >> Self::LEN_FLAG;
    const MASK_FLAG: u32 = !Self::MASK_CONTENT;

    const _CHECK_MASK: () = if Self::MASK_FLAG ^ Self::MASK_CONTENT != u32::MAX {
        panic!("mask constants have overlap")
    };

    /// Encodes a normal numeric (29-bit) index into the [`Arena`] ([`DecodeExprId::Index`]). This
    /// is the encoding used if all of the others cannot represent the value.
    /// - Bits 0 to 28 contain the bits of the index.
    ///
    /// [`Arena`]: crate::ast::Arena
    const FLAG_INDEX: u32 = 0b000;

    /// Encodes a [`Literal::I32`] ([`DecodeExprId::Literal`]).
    /// - Bits 0 to 26 correspond to bits 0 to 26 of the encoded value.
    /// - Bit 27 corresponds to bits 27 to 30 of the encoded value.
    /// - Bit 28 corresponds to bit 31 of the encoded value.
    ///
    /// [`Literal::I32`]: crate::ast::Literal::I32
    const FLAG_I32: u32 = 0b001;

    /// Encodes a [`Literal::I64`] ([`DecodeExprId::Literal`]).
    /// - Bits 0 to 26 correspond to bits 0 to 26 of the encoded value.
    /// - Bit 27 corresponds to bits 27 to 62 of the encoded value.
    /// - Bit 28 corresponds to bit 63 of the encoded value.
    ///
    /// [`Literal::I64`]: crate::ast::Literal::I64
    const FLAG_I64: u32 = 0b010;

    /// Encodes a [`Literal::F32`] or a [`Literal::F64`] ([`DecodeExprId::Literal`]):
    /// - Bit 28 is set by [`ExprId::ENCODE_FLOAT_IS_F64`].
    /// - If a [`Literal::F32`] is being encoded:
    ///   - Bits 0 to 27 correspond to bits 4 to 31 of the encoded value.
    ///   - This essentially "shifts" the encoded value to the right by 4 bits.
    ///   - This means that the *sign*, all 8 bits of the *exponent*, and the high 19 bits of the
    ///     *significand* are stored.
    /// - If a [`Literal::F64`] is being encoded:
    ///   - Bits 0 to 27 correspond to bits 36 to 63 of the encoded value.
    ///   - This essentially "shifts" the encoded value to the right by 36 bits.
    ///   - This means that the *sign*, all 11 bits of the *exponent*, and the high 16 bits of the
    ///     *significand* are stored.
    ///
    /// Note that this encoding includes the entirety of the encoded value's *exponent*, meaning
    /// infinities and *NaN* values (even [*canonical NaN*s]) can use this compact encoding.
    ///
    /// [`Literal::F32`]: crate::ast::Literal::F32
    /// [`Literal::F64`]: crate::ast::Literal::F64
    /// [*canonical NaN*s]: https://webassembly.github.io/spec/core/syntax/values.html#floating-point
    const FLAG_FLOAT: u32 = 0b011;

    /// Encodes a variable, which is a parameter or local in the original WebAssembly
    /// ([`DecodeExprId::Local`]), or a new temporary ([`DecodeExprId::Temporary`]).
    ///  - Bit 28 is set by [`ExprId::ENCODE_VAR_IS_LOCAL`].
    ///  - Bits 0 to 27 encode the 28-bit value for the [`TempId`] or [`LocalId`].
    ///
    /// [`TempId`]: crate::ast::TempId
    /// [`LocalId`]: crate::ast::LocalId
    const FLAG_VAR: u32 = 0b100;

    // const FLAG_V128: u32 = 0b101;

    // const FLAG_REF: u32 = 0b110; // Encode null `funcref`

    // const FLAG_RESERVED: u32 = 0b111;

    /// Gets the flag (the upper 3) bits.
    const fn flag(self) -> u32 {
        (self.id & Self::MASK_FLAG) >> Self::LEN_CONTENT
    }

    /// Gets all of the non-flag (the lower 29) bits.
    const fn contents(self) -> u32 {
        self.id & Self::MASK_CONTENT
    }

    /// The largest index that can be encoded using [`ExprId::FLAG_INDEX`].
    const ENCODE_INDEX_MAX: u32 = Self::MASK_CONTENT;

    const fn new(flag: u32, contents: u32) -> Self {
        #[cfg(debug_assertions)]
        if contents & Self::MASK_FLAG != 0 {
            panic!("content bits overlap with flag");
        }

        #[cfg(debug_assertions)]
        if flag & (u32::MAX << Self::LEN_FLAG) != 0 {
            panic!("flag bits should only set lower 3");
        }

        Self {
            id: (flag << Self::LEN_CONTENT) | (contents & Self::MASK_CONTENT),
        }
    }

    /// An [`ExprId`] referring to an existing [`Expr`](crate::ast::Expr) in the [`Arena`].
    ///
    /// See [`ExprId::FLAG_INDEX`] for more details.
    ///
    /// [`Arena`]: crate::ast::Arena
    pub(in crate::ast) const fn from_index(index: usize) -> Result<Self, ArenaError> {
        if (usize::BITS > u32::BITS && index > u32::MAX as usize)
            || (index as u32) > Self::ENCODE_INDEX_MAX
        {
            Err(ArenaError::IndexTooLarge)
        } else {
            Ok(Self::new(Self::FLAG_INDEX, index as u32))
        }
    }

    const ENCODE_I32_GET_LOW_BITS: u32 = 0x07FF_FFFF;
    const ENCODE_I32_GET_MIDDLE_BITS: u32 = 0x7800_0000;
    const ENCODE_I32_GET_HIGH_BIT: u32 = 0x8000_0000;
    const ENCODE_I32_SET_MIDDLE: u32 = 1 << 27;
    const ENCODE_I32_SET_HIGH_BIT: u32 = 1 << 28;

    /// See [`ExprId::FLAG_I32`] for more details.
    pub(in crate::ast) const fn from_i32(value: i32) -> Option<Self> {
        let value = value as u32;

        // Ensure bits 27 to 30 have the same value.
        if (value & Self::ENCODE_I32_GET_MIDDLE_BITS) ^ Self::ENCODE_I32_GET_MIDDLE_BITS != 0 {
            None
        } else {
            let mut bits = value & Self::ENCODE_I32_GET_LOW_BITS;

            if value & Self::ENCODE_I32_GET_MIDDLE_BITS != 0 {
                bits |= Self::ENCODE_I32_SET_MIDDLE;
            }

            if value & Self::ENCODE_I32_GET_HIGH_BIT != 0 {
                bits |= Self::ENCODE_I32_SET_HIGH_BIT;
            }

            Some(Self::new(Self::FLAG_I32, bits))
        }
    }

    const ENCODE_I64_GET_MIDDLE_BITS: u64 = 0x7FFF_FFFF_F800_0000;
    const ENCODE_I64_GET_HIGH_BIT: u64 = 0x8000_0000_0000_0000;

    /// See [`ExprId::FLAG_I64`] for more details.
    pub(in crate::ast) const fn from_i64(value: i64) -> Option<Self> {
        let value = value as u64;

        // Ensure bits 27 to 62 have the same value.
        if (value & Self::ENCODE_I64_GET_MIDDLE_BITS) ^ Self::ENCODE_I64_GET_MIDDLE_BITS != 0 {
            None
        } else {
            let mut bits = (value as u32) & Self::ENCODE_I32_GET_LOW_BITS;

            if value & Self::ENCODE_I64_GET_MIDDLE_BITS != 0 {
                bits |= Self::ENCODE_I32_SET_MIDDLE;
            }

            if value & Self::ENCODE_I64_GET_HIGH_BIT != 0 {
                bits |= Self::ENCODE_I32_SET_HIGH_BIT;
            }

            Some(Self::new(Self::FLAG_I64, bits))
        }
    }

    /// Indicates that a [`Literal::F64`] is encoded.
    ///
    /// [`Literal::F64`]: crate::ast::Literal::F64
    const ENCODE_FLOAT_IS_F64: u32 = 0x1000_0000;

    /// The lower bits of the *significand* that are assumed to be set to zero.
    const ENCODE_FLOAT_F32_MASK_ZEROED: u32 = 0xF;
    const ENCODE_FLOAT_F32_SHIFT: u32 = Self::ENCODE_FLOAT_F32_MASK_ZEROED.trailing_ones();

    /// See [`ExprId::FLAG_FLOAT`] for more details.
    pub(in crate::ast) const fn from_f32(bits: u32) -> Option<Self> {
        // Ensure low bits of significand are all zero.
        if bits & Self::ENCODE_FLOAT_F32_MASK_ZEROED != 0 {
            None
        } else {
            Some(Self::new(
                Self::FLAG_FLOAT,
                bits >> Self::ENCODE_FLOAT_F32_SHIFT,
            ))
        }
    }

    /// The lower bits of the *significand* that are assumed to be set to zero.
    const ENCODE_FLOAT_F64_MASK_ZEROED: u64 = 0xFFFFFFFFF;
    const ENCODE_FLOAT_F64_SHIFT: u32 = Self::ENCODE_FLOAT_F64_MASK_ZEROED.trailing_ones();

    /// See [`ExprId::FLAG_FLOAT`] for more details.
    pub(in crate::ast) const fn from_f64(bits: u64) -> Option<Self> {
        // Ensure low bits of significand are all zero.
        if bits & Self::ENCODE_FLOAT_F64_MASK_ZEROED != 0 {
            None
        } else {
            Some(Self::new(
                Self::FLAG_FLOAT,
                (bits >> Self::ENCODE_FLOAT_F64_SHIFT) as u32,
            ))
        }
    }

    /// The largest index to a [`TempId`] or [`LocalId`] that can be encoded using
    /// [`ExprId::FLAG_VAR`].
    ///
    /// [`TempId`]: crate::ast::TempId
    /// [`LocalId`]: crate::ast::LocalId
    const ENCODE_VAR_MAX_INDEX: u32 = 0x0FFF_FFFF;

    /// Indicates that a [`DecodeExprId::Local`] is encoded.
    const ENCODE_VAR_IS_LOCAL: u32 = 1 << 28;

    /// Encodes a [`DecodeExprId::Temporary`].
    ///
    /// See [`ExprId::FLAG_VAR`] for more details.
    pub(in crate::ast) fn from_temporary(temporary: crate::ast::TempId) -> Option<Self> {
        if temporary.0 > Self::ENCODE_VAR_MAX_INDEX {
            None
        } else {
            Some(Self::new(Self::FLAG_VAR, temporary.0))
        }
    }

    /// Encodes a [`DecodeExprId::Local`].
    ///
    /// See [`ExprId::FLAG_VAR`] for more details.
    pub(in crate::ast) fn from_local(local: crate::ast::LocalId) -> Option<Self> {
        if local.0 > Self::ENCODE_VAR_MAX_INDEX {
            None
        } else {
            Some(Self::new(
                Self::FLAG_VAR,
                local.0 | Self::ENCODE_VAR_IS_LOCAL,
            ))
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum DecodeExprId {
    Index(usize),
    Literal(crate::ast::Literal),
    Temporary(crate::ast::TempId),
    Local(crate::ast::LocalId),
}

impl From<ExprId> for DecodeExprId {
    fn from(id: ExprId) -> Self {
        use crate::ast::Literal;

        let encoded = id.contents();
        match id.flag() {
            ExprId::FLAG_INDEX => {
                // Won't overflow, original index was also an `usize`.
                Self::Index(encoded as usize)
            }
            ExprId::FLAG_I32 => {
                let mut value = encoded & ExprId::ENCODE_I32_GET_LOW_BITS;

                if encoded & ExprId::ENCODE_I32_SET_HIGH_BIT != 0 {
                    value |= ExprId::ENCODE_I32_GET_HIGH_BIT;
                }

                if encoded & ExprId::ENCODE_I32_SET_MIDDLE != 0 {
                    value |= ExprId::ENCODE_I32_GET_MIDDLE_BITS;
                }

                Self::Literal(Literal::I32(value as i32))
            }
            ExprId::FLAG_I64 => {
                let mut value = (encoded & ExprId::ENCODE_I32_GET_LOW_BITS) as u64;

                if encoded & ExprId::ENCODE_I32_SET_HIGH_BIT != 0 {
                    value |= ExprId::ENCODE_I64_GET_HIGH_BIT;
                }

                if encoded & ExprId::ENCODE_I32_SET_MIDDLE != 0 {
                    value |= ExprId::ENCODE_I64_GET_MIDDLE_BITS;
                }

                Self::Literal(Literal::I64(value as i64))
            }
            ExprId::FLAG_FLOAT => {
                if encoded & ExprId::ENCODE_FLOAT_IS_F64 != 0 {
                    Self::Literal(Literal::F64(
                        (encoded as u64) << ExprId::ENCODE_FLOAT_F64_SHIFT,
                    ))
                } else {
                    Self::Literal(Literal::F32(encoded << ExprId::ENCODE_FLOAT_F32_SHIFT))
                }
            }
            ExprId::FLAG_VAR => {
                let index = encoded & ExprId::ENCODE_VAR_MAX_INDEX;
                if encoded & ExprId::ENCODE_VAR_IS_LOCAL != 0 {
                    Self::Local(crate::ast::LocalId(index))
                } else {
                    Self::Temporary(crate::ast::TempId(index))
                }
            }
            // ExprId::FLAG_V128 => todo!(),
            // ExprId::FLAG_REF => todo!(),
            unknown => unreachable!("encountered unknown ID type ({unknown:#03b})"),
        }
    }
}

impl std::fmt::Debug for ExprId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&DecodeExprId::from(*self), f)
    }
}
