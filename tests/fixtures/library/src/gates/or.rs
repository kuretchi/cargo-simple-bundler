/// The OR gate.
pub fn or_gate(a: bool, b: bool) -> bool {
    a | b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(or_gate(true, false), true);
    }
}
