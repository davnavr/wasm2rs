use crate::ast::ExprId;

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
    branch_targets: Vec<crate::ast::BranchTarget>,
    //uncommon: Option<Box<ArenaUncommon>>, // contains stuff not used often
}

impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

impl Arena {
    pub(crate) const fn new() -> Self {
        Self {
            arena: Vec::new(),
            branch_targets: Vec::new(),
        }
    }

    fn allocate_inner(&mut self, expr: crate::ast::Expr) -> Result<ExprId, ArenaError> {
        use crate::ast::{Expr, Literal};

        macro_rules! try_encode {
            ($encode:expr) => {
                if let Some(encoded) = $encode {
                    return Ok(encoded);
                }
            };
        }

        // Check if a compact encoding is available.
        match &expr {
            Expr::Literal(literal) => match literal {
                Literal::I32(value) => try_encode!(ExprId::from_i32(*value)),
                Literal::I64(value) => try_encode!(ExprId::from_i64(*value)),
                Literal::F32(bits) => try_encode!(ExprId::from_f32(*bits)),
                Literal::F64(bits) => try_encode!(ExprId::from_f64(*bits)),
            },
            // Expr::BinaryOperator { kind, c_1, c_2 } => try_encode!(ExprId::from_bin_op(*kind, *c_1, *c_2)),
            Expr::Temporary(temporary) => try_encode!(ExprId::from_temporary(*temporary)),
            Expr::GetLocal(local) => try_encode!(ExprId::from_local(*local)),
            _ => (),
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
        use crate::ast::DecodeExprId;

        match DecodeExprId::from(id) {
            DecodeExprId::Index(index) => self.arena[index],
            DecodeExprId::Literal(literal) => literal.into(),
            DecodeExprId::Local(local) => crate::ast::Expr::GetLocal(local),
            DecodeExprId::Temporary(temporary) => crate::ast::Expr::Temporary(temporary),
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

    pub(crate) fn allocate_branch_targets<E: From<ArenaError>>(
        &mut self,
        targets: impl Iterator<Item = Result<crate::ast::BranchTarget, E>>,
    ) -> Result<crate::ast::BranchTargetList, E> {
        let start_len = self.branch_targets.len();
        let start_index = u32::try_from(start_len).map_err(|_| ArenaError::IndexTooLarge)?;
        self.branch_targets.reserve(targets.size_hint().0);
        for result in targets {
            self.branch_targets.push(result?);
        }
        let calculated_len = self.branch_targets.len() - start_len;

        Ok(crate::ast::BranchTargetList {
            index: start_index,
            count: u32::try_from(calculated_len).map_err(|_| ArenaError::IndexTooLarge)?,
        })
    }

    pub(crate) fn get_branch_targets(
        &self,
        targets: crate::ast::BranchTargetList,
    ) -> &[crate::ast::BranchTarget] {
        &self.branch_targets[targets.index as usize..][..targets.count as usize]
    }
}
