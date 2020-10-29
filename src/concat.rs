use crate::{common::*, content::Content, deps::Deps, file::file, path::Path, span::replace_spans};
use std::collections::{HashMap, HashSet};

pub fn concat_contents(deps: &Deps, cx: &mut Context) -> Result<Content> {
    if deps.is_empty() {
        return Ok(Content::default());
    }
    let mut inners = HashMap::<_, HashSet<_>>::new();
    for path in deps.iter() {
        let mut inner = Path::default();
        for symbol in path.iter() {
            let next = inner.child(symbol);
            inners.entry(inner).or_default().insert(symbol);
            inner = next;
        }
    }
    let mut acc = Content::from(format!("mod {} ", cx.config.crate_ident));
    inside_block(&mut acc, |acc| do_concat_contents(&Path::default(), &inners, acc, cx))?;
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
    let mut replace_with = vec![];

    for child_module in file.child_modules() {
        if inners.get(path).map_or(true, |x| x.contains(&child_module.symbol())) {
            let path = child_module.path();
            let mut acc = Content::default();
            acc.push(" ");
            inside_block(&mut acc, |acc| do_concat_contents(&path, inners, acc, cx))?;
            replace_with.push((child_module.item_mod_semi_span(), Some(acc)));
        } else {
            replace_with.push((child_module.item_mod_span(), None));
            if let Some(span) = child_module.item_use_span() {
                replace_with.push((span, None));
            }
        }
    }
    replace_with.extend(file.crate_keyword_spans().map(|span| {
        let s = format!("crate::{}", cx.config.crate_ident);
        (span, Some(s.into()))
    }));
    replace_with.extend(file.doc_comment_spans().map(|span| (span, None)));
    replace_with.extend(file.test_module_spans().map(|span| (span, None)));

    replace_spans(file.content(), replace_with, acc);
    Ok(())
}

fn inside_block<F>(acc: &mut Content, f: F) -> Result<()>
where
    F: FnOnce(&mut Content) -> Result<()>,
{
    acc.push_line("{");
    f(acc)?;
    acc.push("}");
    Ok(())
}
