use crate::gates::{and_gate, xor_gate};

/// The half adder.
pub fn half_adder(a: bool, b: bool) -> (bool, bool) {
    let s = xor_gate(a, b);
    let c = and_gate(a, b);
    (s, c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(half_adder(true, true), (false, true));
    }
}
