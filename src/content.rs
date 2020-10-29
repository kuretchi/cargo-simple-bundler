use std::fmt;

pub struct Content {
    lines: Vec<String>,
    new_line: bool,
}

impl Default for Content {
    fn default() -> Self {
        Content { lines: vec![], new_line: true }
    }
}

impl From<String> for Content {
    fn from(s: String) -> Self {
        Content { lines: vec![s], new_line: false }
    }
}

impl fmt::Display for Content {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (last, lines) = match self.lines.split_last() {
            Some(x) => x,
            None => return Ok(()),
        };
        for line in lines {
            writeln!(f, "{}", line)?;
        }
        write!(f, "{}", last)?;
        if self.new_line {
            writeln!(f)?;
        }
        Ok(())
    }
}

impl Content {
    fn push_inner(&mut self, s: &str) {
        if self.new_line {
            self.lines.push(s.to_owned());
        } else {
            self.lines.last_mut().unwrap().push_str(s);
        }
    }

    pub fn push(&mut self, s: &str) {
        self.push_inner(s);
        self.new_line = false;
    }

    pub fn push_line(&mut self, s: &str) {
        self.push_inner(s);
        self.new_line = true;
    }

    pub fn append(&mut self, other: Content) {
        let mut lines = other.lines.into_iter();
        if let Some(first) = lines.next() {
            if self.new_line {
                self.lines.push(first);
            } else {
                self.push_line(&first);
            }
            for line in lines {
                self.lines.push(line);
            }
        }
        self.new_line = other.new_line;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push() {
        let mut s = Content::default();
        s.push("a");
        s.push_line("b");
        s.push("c");

        assert_eq!(s.to_string(), "ab\nc");
    }

    #[test]
    fn push_new_line() {
        let mut s = Content::default();
        s.push("a");
        s.push_line("b");
        s.push_line("c");

        assert_eq!(s.to_string(), "ab\nc\n");
    }

    #[test]
    fn append() {
        let mut s = Content::default();
        s.push("a");
        let mut t = Content::default();
        t.push("b");
        s.append(t);

        assert_eq!(s.to_string(), "ab");
    }

    #[test]
    fn append_self_new_line() {
        let mut s = Content::default();
        s.push_line("a");
        let mut t = Content::default();
        t.push("b");
        s.append(t);

        assert_eq!(s.to_string(), "a\nb");
    }

    #[test]
    fn append_other_new_line() {
        let mut s = Content::default();
        s.push("a");
        let mut t = Content::default();
        t.push_line("b");
        s.append(t);

        assert_eq!(s.to_string(), "ab\n");
    }

    #[test]
    fn append_self_new_line_other_new_line() {
        let mut s = Content::default();
        s.push_line("a");
        let mut t = Content::default();
        t.push_line("b");
        s.append(t);

        assert_eq!(s.to_string(), "a\nb\n");
    }
}
