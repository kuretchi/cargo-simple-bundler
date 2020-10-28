use super::{and_gate, not_gate, or_gate};

/// The XOR gate.
pub fn xor_gate(a: bool, b: bool) -> bool {
    or_gate(and_gate(a, not_gate(b)), and_gate(not_gate(a), b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(xor_gate(true, false), true);
    }
}
