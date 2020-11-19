use std::collections::{btree_map, BTreeMap};

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

#[derive(Clone, Default, Debug)]
pub struct SpanUnion(BTreeMap<LineColumn, LineColumn>);

fn into_span((&start, &end): (&LineColumn, &LineColumn)) -> Span {
    Span { start, end }
}

impl SpanUnion {
    pub fn iter(&self) -> impl Iterator<Item = Span> + '_ {
        self.0.iter().map(into_span)
    }

    pub fn insert(&mut self, mut span: Span) {
        if let Some(l_boundary) = self.0.range(..span.start).next_back().map(into_span) {
            if span.start <= l_boundary.end {
                // l_boundary: [--------]
                //       span:       [-----]
                span.start = l_boundary.start;
                // l_boundary: [--------]
                //       span: [<----|-----]
            }
        }
        if let Some(r_boundary) = self.0.range(..=span.end).next_back().map(into_span) {
            // r_boundary:    [--------]
            //       span: [-----]
            span.end = span.end.max(r_boundary.end);
            // r_boundary:    [--------]
            //       span: [-----------]
        }
        while let Some((&inner_start, _)) = self.0.range(span.start..=span.end).next() {
            self.0.remove(&inner_start);
        }
        self.0.insert(span.start, span.end);
    }

    pub fn remove(&mut self, span: Span) {
        if let Some(l_boundary) = self.0.range(..span.start).next_back().map(into_span) {
            if span.start < l_boundary.end {
                // l_boundary: [--------]
                //       span:       [-----]
                *self.0.get_mut(&l_boundary.start).unwrap() = span.start;
                // l_boundary: [-----]<-|
                //       span:       [-----]
                if span.end < l_boundary.end {
                    // l_boundary: [-----]<----------|
                    //       span:       [-----]
                    self.0.insert(span.end, l_boundary.end);
                    // l_boundary: [-----]     [-----]
                    //       span:       [-----]
                    return;
                }
            }
        }
        if let Some(r_boundary) = self.0.range(..span.end).next_back().map(into_span) {
            if span.end < r_boundary.end {
                // r_boundary:    [--------]
                //       span: [-----]
                self.0.remove(&r_boundary.start);
                self.0.insert(span.end, r_boundary.end);
                // r_boundary:       [-----]
                //       span: [-----]
            }
        }
        while let Some((&inner_start, _)) = self.0.range(span.start..span.end).next() {
            self.0.remove(&inner_start);
        }
    }
}

pub fn take_spans<'a>(content: &'a str, spans: &'a SpanUnion) -> TakeSpans<'a> {
    TakeSpans { prev_end: LineColumn { line: 1, column: 0 }, rest: content, spans: spans.0.iter() }
}

#[derive(Debug)]
pub struct TakeSpans<'a> {
    prev_end: LineColumn,
    rest: &'a str,
    spans: btree_map::Iter<'a, LineColumn, LineColumn>,
}

impl<'a> TakeSpans<'a> {
    fn nth_line_start(&self, n: usize) -> usize {
        let mut start = 0;
        for _ in 0..n {
            start += self.rest[start..].find('\n').unwrap() + 1;
        }
        start
    }
}

#[derive(Debug)]
pub struct TakeSpansItem<'a> {
    pub span: Span,
    pub line_offset: usize,
    pub column_offset: usize,
    pub content: &'a str,
}

impl<'a> Iterator for TakeSpans<'a> {
    type Item = TakeSpansItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let span = into_span(self.spans.next()?);

        let line_offset = span.start.line - self.prev_end.line;
        let column_offset =
            span.start.column - if line_offset == 0 { self.prev_end.column } else { 0 };

        let start_idx = if line_offset == 0 {
            span.start.column - self.prev_end.column
        } else {
            self.nth_line_start(line_offset) + span.start.column
        };
        self.rest = &self.rest[start_idx..];

        let end_idx = if span.start.line == span.end.line {
            span.end.column - span.start.column
        } else {
            self.nth_line_start(span.end.line - span.start.line) + span.end.column
        };
        let (content, rest) = self.rest.split_at(end_idx);
        self.rest = rest;
        self.prev_end = span.end;

        Some(TakeSpansItem { span, line_offset, column_offset, content })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_union_test() {
        let mut span_union = SpanUnion::default();

        // span0: [-----]
        // span1:            [-----]
        // span2:    [-------]
        // span3:    [----------]

        let span0 = Span {
            start: LineColumn { line: 1, column: 2 },
            end: LineColumn { line: 1, column: 4 },
        };
        span_union.insert(span0);
        assert_eq!(span_union.iter().collect::<Vec<_>>(), [span0]);

        let span1 = Span {
            start: LineColumn { line: 2, column: 1 },
            end: LineColumn { line: 2, column: 3 },
        };
        span_union.insert(span1);
        assert_eq!(span_union.iter().collect::<Vec<_>>(), [span0, span1]);

        let span2 = Span {
            start: LineColumn { line: 1, column: 3 },
            end: LineColumn { line: 2, column: 1 },
        };
        span_union.insert(span2);
        assert_eq!(
            span_union.iter().collect::<Vec<_>>(),
            [Span { start: span0.start, end: span1.end }]
        );

        let span3 = Span {
            start: LineColumn { line: 1, column: 3 },
            end: LineColumn { line: 2, column: 2 },
        };
        span_union.remove(span3);
        assert_eq!(
            span_union.iter().collect::<Vec<_>>(),
            [
                Span { start: span0.start, end: span3.start },
                Span { start: span3.end, end: span1.end },
            ]
        );
    }

    #[test]
    fn take_spans_test() {
        let mut span_union = SpanUnion::default();
        let spans = vec![
            Span {
                start: LineColumn { line: 1, column: 3 },
                end: LineColumn { line: 2, column: 2 },
            },
            Span {
                start: LineColumn { line: 2, column: 4 },
                end: LineColumn { line: 2, column: 5 },
            },
            Span {
                start: LineColumn { line: 3, column: 1 },
                end: LineColumn { line: 3, column: 6 },
            },
        ];
        for &span in &spans {
            span_union.insert(span);
        }

        let content = "012345\n012345\n012345";
        let mut iter = take_spans(content, &span_union);

        {
            let item = iter.next().unwrap();
            assert_eq!(item.span, spans[0]);
            assert_eq!(item.line_offset, 0);
            assert_eq!(item.column_offset, 3);
            assert_eq!(item.content, "345\n01");
        }
        {
            let item = iter.next().unwrap();
            assert_eq!(item.span, spans[1]);
            assert_eq!(item.line_offset, 0);
            assert_eq!(item.column_offset, 2);
            assert_eq!(item.content, "4");
        }
        {
            let item = iter.next().unwrap();
            assert_eq!(item.span, spans[2]);
            assert_eq!(item.line_offset, 1);
            assert_eq!(item.column_offset, 1);
            assert_eq!(item.content, "12345");
        }

        assert!(iter.next().is_none());
    }
}
