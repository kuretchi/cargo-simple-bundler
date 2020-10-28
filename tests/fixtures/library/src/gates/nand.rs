use super::{and_gate, not_gate};

/// The NAND gate.
pub fn nand_gate(a: bool, b: bool) -> bool {
    not_gate(and_gate(a, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(nand_gate(true, false), true);
    }
}
