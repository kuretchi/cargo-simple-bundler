use crate::common::*;
use std::{io::prelude::*, mem};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct LineColumn {
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct Span {
    pub start: LineColumn, // inclusive
    pub end: LineColumn,   // exclusive
}

impl From<proc_macro2::Span> for Span {
    fn from(span: proc_macro2::Span) -> Span {
        let start = span.start();
        let end = span.end();
        Span {
            start: LineColumn { line: start.line, column: start.column },
            end: LineColumn { line: end.line, column: end.column },
        }
    }
}

pub fn replace_spanned_strs<W, I, F>(
    content: &str,
    replacers: I,
    writer: &mut W,
    cx: &mut Context,
) -> Result<()>
where
    W: ?Sized + Write,
    F: FnOnce(&mut W, &mut Context) -> Result<()>,
    I: IntoIterator<Item = (Span, Option<F>)>,
{
    let replacers = {
        let mut vec = replacers.into_iter().collect_vec();
        vec.sort_unstable_by_key(|&(span, _)| span);
        vec.into_iter().coalesce(|x, y| {
            if y.0.start <= x.0.end {
                assert!(y.0.end <= x.0.end); // x includes y
                assert!(x.1.is_none());
                Ok((x.0, None))
            } else {
                Err((x, y))
            }
        })
    };

    let mut lines = (1..).zip(content.lines()).peekable();
    let mut start_col = 0;

    for (span, f) in replacers {
        for (_, line) in lines.peeking_take_while(|&(i, _)| i != span.start.line) {
            writeln!(writer, "{}", &line[mem::replace(&mut start_col, 0)..])?;
        }
        let &(_, start_line) = lines.peek().unwrap();
        write!(writer, "{}", &start_line[..span.start.column])?;
        if let Some(f) = f {
            f(writer, cx)?;
        }
        lines.peeking_take_while(|&(i, _)| i != span.end.line).for_each(drop);
        start_col = span.end.column;
    }
    for (_, line) in lines {
        writeln!(writer, "{}", &line[mem::replace(&mut start_col, 0)..])?;
    }

    Ok(())
}
