# `xml-canonicalization`

A pure Rust [XML Canonicalisation] implementation.

## Unsupported Features

Certain XML features (that are rarely used in practice) are unsupported by this
crate:

- Entity references
- Default attributes via DTD
- Document Subset Expressions

Otherwise, all XML canonicalisation features are supported, and are validated
against examples from the W3C specification.

## Usage

```rust
use std::io::Cursor;
use xml_canonicalization::Canonicalizer;

let input = r#"<?xml version="1.0" encoding="ISO-8859-1"?><doc>&#169;</doc>"#;
let canonical = r#"<doc>©</doc>"#;

let mut result = vec![];
Canonicalizer::read_from_str(input)
    .write_to_writer(Cursor::new(&mut result))
    .canonicalize(true)
    .expect("canonicalization shouldn't fail");
assert_eq!(String::from_utf8_lossy(&result), canonical);
```

[XML Canonicalisation]: https://en.wikipedia.org/wiki/Canonical_XML
