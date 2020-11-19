use crate::{common::*, content::Content, deps::Deps, file::file, path::Path, span::take_spans};
use std::collections::{HashMap, HashSet};

pub fn concat_contents(deps: &Deps, cx: &mut Context) -> Result<Content> {
    if deps.is_empty() {
        return Ok(Content::default());
    }
    let mut inners = HashMap::<_, HashSet<_>>::new();
    for path in
        deps.iter().filter(|path| path.strict_ancestors().all(|ancestor| !deps.contains(&ancestor)))
    {
        let mut inner = Path::default();
        for symbol in path.iter() {
            let next = inner.child(symbol);
            inners.entry(inner).or_default().insert(symbol);
            inner = next;
        }
    }
    let mut acc = Content::from(format!("mod {} ", cx.config.crate_ident));
    inside_block(&mut acc, cx.config.indent_spaces, |acc| {
        do_concat_contents(&Path::default(), &inners, acc, cx)
    })?;
    acc.push_line("");
    Ok(acc)
}

fn do_concat_contents(
    path: &Path,
    inners: &HashMap<Path, HashSet<Symbol>>,
    acc: &mut Content,
    cx: &mut Context,
) -> Result<()> {
    let file = file(path, cx)?;
    let mut target_spans = file.target_spans().clone();
    let mut replace_with = vec![];

    for child_module in file.child_modules() {
        if inners.get(path).map_or(true, |x| x.contains(&child_module.symbol())) {
            let path = child_module.path();
            let mut acc = Content::default();
            acc.push(" ");
            inside_block(&mut acc, cx.config.indent_spaces, |acc| {
                do_concat_contents(&path, inners, acc, cx)
            })?;
            replace_with.push((child_module.item_mod_semi_span(), acc));
        } else {
            target_spans.remove(child_module.item_mod_span());
            if let Some(span) = child_module.item_use_span() {
                target_spans.remove(span);
            }
        }
    }
    replace_with.extend(file.crate_keyword_spans().map(|span| {
        let s = format!("crate::{}", cx.config.crate_ident);
        (span, s.into())
    }));

    replace_with.sort_unstable_by_key(|&(span, _)| span);
    let mut replace_with = replace_with.into_iter().peekable();

    for chunk in take_spans(file.content(), &target_spans) {
        for _ in 0..chunk.line_offset {
            acc.push_line("");
        }
        for _ in 0..chunk.column_offset {
            acc.push(" ");
        }

        replace_with
            .peeking_take_while(|&(span, _)| span.start < chunk.span.start)
            .inspect(|(span, _)| assert!(span.end <= chunk.span.start))
            .for_each(drop);

        let mut rest = chunk.content;
        let mut offset = chunk.span.start.column;

        for (span, s) in replace_with.peeking_take_while(|&(span, _)| span.start < chunk.span.end) {
            assert!(span.end <= chunk.span.end);
            acc.push(&rest[..span.start.column - offset]);
            acc.append(s);
            rest = &rest[span.end.column - offset..];
            offset = span.end.column;
        }
        acc.push(rest);
    }
    Ok(())
}

fn inside_block<F>(acc: &mut Content, indent_spaces: usize, f: F) -> Result<()>
where
    F: FnOnce(&mut Content) -> Result<()>,
{
    let mut s = Content::default();
    f(&mut s)?;
    s.indent(indent_spaces);

    acc.push_line("{");
    acc.append(s);
    acc.push_line("");
    acc.push("}");
    Ok(())
}
