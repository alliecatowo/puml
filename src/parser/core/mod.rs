// parser/core: parse-loop helpers split into focused submodules.
// All items are re-exported into the parent parser namespace via `use core::*;`.

// Import everything from the parent parser module so submodules can access via super::*.
use super::*;

pub(crate) mod blocks;
pub(crate) mod families;

pub(crate) use blocks::*;
pub(crate) use families::*;
