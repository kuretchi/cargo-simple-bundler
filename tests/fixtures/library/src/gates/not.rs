/// The NOT gate.
pub fn not_gate(a: bool) -> bool {
    !a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(not_gate(true), false);
    }
}
