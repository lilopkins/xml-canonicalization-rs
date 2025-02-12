use std::io::Cursor;

use tracing_test::traced_test;
use xml_c14n::Canonicalizer;

#[test]
#[traced_test]
fn from_to_reader() {
    let input = r#"<test attr="valid"></test>"#;
    let mut target = vec![];
    Canonicalizer::read_from_reader(Cursor::new(input))
        .write_to_writer(Cursor::new(&mut target))
        .canonicalize(false)
        .expect("canonicalization shouldn't fail");
}

#[test]
#[traced_test]
fn from_to_str() {
    let input = r#"<test attr="valid"></test>"#;
    let mut output = vec![];
    Canonicalizer::read_from_str(input)
        .write_to_writer(Cursor::new(&mut output))
        .canonicalize(false)
        .expect("canonicalization shouldn't fail");
}

#[test]
#[traced_test]
fn from_to_file() {
    Canonicalizer::read_from_file("./tests/test-1.xml")
        .expect("failed to read from file")
        .write_to_file("./target/test-1.canonical.xml")
        .expect("failed to target file")
        .canonicalize(false)
        .expect("canonicalization shouldn't fail");
}
