/// The AND gate.
pub fn and_gate(a: bool, b: bool) -> bool {
    a & b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(and_gate(true, false), false);
    }
}
