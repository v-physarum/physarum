//! Target facts that change how assembly text is read.
//!
//! The host target is fixed when this crate is built. A triple passed to
//! [`crate::TargetAssert`] is classified once per compile.

/// Object format and instruction set of one compiled probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Target {
    pub(crate) format: ObjectFormat,
    pub(crate) isa: Isa,
}

impl Target {
    /// The target `rustc` builds for when no `--target` is given.
    pub(crate) const HOST: Self = Self { format: ObjectFormat::HOST, isa: Isa::HOST };

    /// Target for `triple`. `None` is the host.
    pub(crate) fn for_triple(triple: Option<&str>) -> Self {
        match triple {
            None => Self::HOST,
            Some(triple) => {
                Self { format: ObjectFormat::for_triple(triple), isa: Isa::for_triple(triple) }
            }
        }
    }
}

/// Object file format of the emitted assembly.
///
/// The format decides how a function starts, how its symbol is spelled, and
/// how local labels look.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ObjectFormat {
    /// ELF: `.type name,@function` opens each function. The symbol is the Rust name.
    Elf,
    /// Mach-O: a symbol label inside `__TEXT,__text` opens each function.
    /// The symbol carries a leading `_`.
    MachO,
}

/// Prefix Mach-O adds to every C-level symbol.
pub(crate) const MACH_O_SYMBOL_PREFIX: char = '_';

/// Target vendor whose triples emit Mach-O.
const APPLE_VENDOR: &str = "-apple-";

impl ObjectFormat {
    const HOST: Self = if cfg!(target_vendor = "apple") {
        Self::MachO
    } else {
        Self::Elf
    };

    fn for_triple(triple: &str) -> Self {
        if triple.contains(APPLE_VENDOR) {
            Self::MachO
        } else {
            Self::Elf
        }
    }
}

/// Instruction set of the emitted assembly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Isa {
    /// `x86_64` in AT&T syntax. `#` starts a comment.
    X86_64,
    /// `AArch64`. `#` marks an immediate. `;` (Mach-O) or `//` (ELF) starts a comment.
    Aarch64,
}

/// Triple architectures that are [`Isa::Aarch64`].
const AARCH64_ARCHES: [&str; 3] = ["aarch64", "arm64", "arm64e"];
const TRIPLE_SEPARATOR: char = '-';
const X86_COMMENT: char = '#';
const AARCH64_MACH_O_COMMENT: char = ';';
const AARCH64_ELF_COMMENT: &str = "//";

impl Isa {
    const HOST: Self = if cfg!(target_arch = "aarch64") {
        Self::Aarch64
    } else {
        Self::X86_64
    };

    fn for_triple(triple: &str) -> Self {
        let arch = triple.split(TRIPLE_SEPARATOR).next().unwrap_or(triple);
        if AARCH64_ARCHES.contains(&arch) {
            Self::Aarch64
        } else {
            Self::X86_64
        }
    }

    /// `line` without its trailing comment.
    pub(crate) fn strip_comment(self, line: &str) -> &str {
        let end = match self {
            Self::X86_64 => line.find(X86_COMMENT),
            Self::Aarch64 => {
                let semicolon = line.find(AARCH64_MACH_O_COMMENT);
                let slashes = line.find(AARCH64_ELF_COMMENT);
                match (semicolon, slashes) {
                    (Some(left), Some(right)) => Some(left.min(right)),
                    (found, None) | (None, found) => found,
                }
            }
        };
        end.map_or(line, |at| &line[..at])
    }
}
