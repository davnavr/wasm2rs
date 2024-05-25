use crate::{Address, Memory};

/// Prints the contents of a linear [`Memory`].
#[derive(Clone, Copy, Debug)]
pub struct HexDump<'a, I: Address, M: Memory<I>> {
    memory: &'a M,
    _marker: core::marker::PhantomData<fn(I)>,
}

impl<'a, I: Address, M: Memory<I>> HexDump<'a, I, M> {
    /// Constructs a hex dump of the given [`Memory`].
    pub const fn new(memory: &'a M) -> Self {
        Self {
            memory,
            _marker: core::marker::PhantomData,
        }
    }
}

impl<'a, I: Address, M: Memory<I>> From<&'a M> for HexDump<'a, I, M> {
    fn from(memory: &'a M) -> Self {
        Self::new(memory)
    }
}

impl<'a, I: Address, M: Memory<I>> core::fmt::UpperHex for HexDump<'a, I, M> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let sixteen: I = <I as From<u8>>::from(16u8);
        let size: I = self.memory.size() * I::cast_from_u32(crate::PAGE_SIZE);
        let width = size.checked_ilog(sixteen).unwrap_or_default().max(7) as usize;

        write!(f, "{: <width$}", "Address")?;
        writeln!(
            f,
            "  00 01 02 03  04 05 06 07  08 09 0A 0B  0C 0D 0E 0F  ASCII"
        )?;

        let mut bytes = [0u8; 16];
        let mut address = I::ZERO;
        while address < size {
            let remaining = size - address;
            let bytes = &mut bytes[..16usize.min(remaining.as_())];
            if self.memory.copy_to_slice(address, bytes).is_err() {
                break;
            }

            write!(f, "{address:0width$X}")?;

            for (i, b) in bytes.iter().enumerate() {
                if i % 4 == 0 {
                    f.write_str("  ")?;
                }

                write!(f, "{b:02X}")?;
            }

            writeln!(f)?;

            address += sixteen.min(remaining);
        }

        Ok(())
    }
}

impl<'a, I: Address, M: Memory<I>> core::fmt::Display for HexDump<'a, I, M> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::UpperHex::fmt(self, f)
    }
}
