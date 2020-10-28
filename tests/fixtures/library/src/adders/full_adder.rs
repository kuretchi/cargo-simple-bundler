use super::half_adder;
use crate::gates::or_gate;

/// The full adder.
pub fn full_adder(a: bool, b: bool, c: bool) -> (bool, bool) {
    let (s, c1) = half_adder(a, b);
    let (s, c2) = half_adder(s, c);
    let c = or_gate(c1, c2);
    (s, c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(full_adder(true, false, true), (false, true))
    }
}
