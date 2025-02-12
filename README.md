# `xml-c14n`

[XML Canonicalisation] for Rust.

## Unsupported Features

Certain XML features (that are rarely used in practice) are unsupported by this
crate:

- Entity references
- Default attributes via DTD
- Document Subset Expressions

Otherwise, all XML canonicalisation features are supported, and are validated
against examples from the W3C specification.

[XML Canonicalisation]: https://en.wikipedia.org/wiki/Canonical_XML
