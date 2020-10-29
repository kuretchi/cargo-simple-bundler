use cargo_simple_bundler::{bundle, Config};
use std::{fs, path::Path};

const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

fn expected_output(name: &str) -> String {
    let path = Path::new(MANIFEST_DIR).join(format!("tests/fixtures/output/{}.txt", name));
    fs::read_to_string(path).unwrap()
}

fn actual_output(remove_doc_comments: bool, remove_test_modules: bool) -> String {
    let config = Config {
        crate_ident: "library".to_owned(),
        crate_src_dir: Path::new(MANIFEST_DIR).join("tests/fixtures/library/src"),
        entry_file_path: Some(Path::new(MANIFEST_DIR).join("tests/fixtures/entry_file.rs")),
        remove_doc_comments,
        remove_test_modules,
        indent_spaces: 4,
    };
    let mut buf = vec![];
    bundle(config, &mut buf).unwrap();
    remove_empty_lines(&String::from_utf8(buf).unwrap())
}

fn remove_empty_lines(s: &str) -> String {
    s.lines().filter(|line| !line.trim_start().is_empty()).fold(String::new(), |mut acc, line| {
        acc.push_str(line);
        acc.push('\n');
        acc
    })
}

#[test]
fn no_options() {
    let expected = expected_output("no-options");
    let actual = actual_output(false, false);
    assert_eq!(actual, expected);
}

#[test]
fn remove_doc_comments() {
    let expected = expected_output("remove-doc-comments");
    let actual = actual_output(true, false);
    assert_eq!(actual, expected);
}

#[test]
fn remove_test_modules() {
    let expected = expected_output("remove-test-modules");
    let actual = actual_output(false, true);
    assert_eq!(actual, expected);
}

#[test]
fn remove_doc_comments_remove_test_modules() {
    let expected = expected_output("remove-doc-comments-remove-test-modules");
    let actual = actual_output(true, true);
    assert_eq!(actual, expected);
}
