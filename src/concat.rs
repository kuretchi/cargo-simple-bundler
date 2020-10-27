use crate::{common::*, deps::Deps, file::file, path::Path, span::replace_spanned_strs};
use either::Either;
use std::{
    collections::{HashMap, HashSet},
    io::prelude::*,
    iter,
};

pub fn concat_contents<W>(deps: &Deps, writer: &mut W, cx: &mut Context) -> Result<()>
where
    W: ?Sized + Write,
{
    if deps.is_empty() {
        return Ok(());
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
    write!(writer, "mod {} ", cx.config.crate_ident)?;
    inside_block(writer, |writer| do_concat_contents(&Path::default(), &inners, writer, cx))
}

fn do_concat_contents<W>(
    path: &Path,
    inners: &HashMap<Path, HashSet<Symbol>>,
    writer: &mut W,
    cx: &mut Context,
) -> Result<()>
where
    W: ?Sized + Write,
{
    let file = file(&path, cx)?;

    let modules_replacer = file.child_modules().flat_map(|child_module| {
        if inners.get(path).map_or(true, |x| x.contains(&child_module.symbol())) {
            let path = child_module.path();
            let f = move |w: &mut W, cx: &mut _| {
                write!(w, " ")?;
                inside_block(w, |w| do_concat_contents(&path, inners, w, cx))
            };
            let f = Box::new(f) as Box<dyn (FnOnce(&mut _, &mut _) -> _)>;
            Either::Left(iter::once((child_module.item_mod_semi_span(), Some(f))))
        } else {
            let spans =
                iter::once(child_module.item_mod_span()).chain(child_module.item_use_span());
            Either::Right(spans.map(move |span| (span, None)))
        }
    });
    let crate_keywords_replacer = file.crate_keyword_spans().map(|span| {
        let f = |w: &mut W, cx: &mut Context| {
            write!(w, "crate::{}", cx.config.crate_ident)?;
            Ok(())
        };
        let f = Box::new(f) as _;
        (span, Some(f))
    });
    let doc_comments_remover = file.doc_comment_spans().map(|span| (span, None));
    let test_modules_remover = file.test_module_spans().map(|span| (span, None));

    let replacers = modules_replacer
        .chain(crate_keywords_replacer)
        .chain(doc_comments_remover)
        .chain(test_modules_remover);
    replace_spanned_strs(&file.content(), replacers, writer, cx)
}

fn inside_block<W, F>(writer: &mut W, f: F) -> Result<()>
where
    W: ?Sized + Write,
    F: FnOnce(&mut W) -> Result<()>,
{
    writeln!(writer, "{{")?;
    f(writer)?;
    writeln!(writer)?;
    writeln!(writer, "}}")?;
    Ok(())
}
