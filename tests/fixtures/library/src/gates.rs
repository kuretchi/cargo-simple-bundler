//! Logic gates.

pub use and::and_gate;
pub use nand::*;
pub use not::not_gate;
pub use or::*;
pub use xor::xor_gate;

mod and;
mod nand;
mod not;
mod or;
mod xor;
