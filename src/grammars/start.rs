use std::{cmp::Ordering, fmt};

use pest::{error::Error, Parser};
use pest_derive::Parser;

use crate::namespace::{DepthSensitiveMap, Namespace};

#[derive(Parser)]
#[grammar = "grammars/start.pest"]
struct StartTagParser;

#[derive(Default)]
struct StartTag {
    name: String,
    attributes: Vec<Attribute>,
}

impl fmt::Display for StartTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { name, attributes } = self;
        if attributes.is_empty() {
            write!(f, "{name}")
        } else {
            write!(
                f,
                "{name} {}",
                attributes
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        }
    }
}

#[derive(Default, PartialEq, Eq)]
struct Attribute {
    name: String,
    value: String,
}

impl fmt::Display for Attribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { name, value } = self;
        write!(f, r#"{name}="{value}""#)
    }
}

impl Attribute {
    fn cmp_with_namespaces(
        &self,
        other: &Self,
        registered_namespaces: &mut DepthSensitiveMap<String, Namespace>,
    ) -> std::cmp::Ordering {
        /*
         * · The default namespace declaration xmlns="...", if any, comes first.
         * · Namespace declarations, sorted by prefix (the part after "xmlns:"). So xmlns:a="http://www.w3.org" comes before xmlns:b="http://www.ietf.org".
         * · Unqualified attributes, sorted by name. So attr="..." comes before attr2="...".
         * · Qualified attributes, sorted by namespace URI then name. So b:attr="..." comes before a:attr="...", because we read this as http://www.ietf.org:attr="..." comes before http://www.w3.org:attr="...". And a:attr="..." comes before a:attr2="..."
         */
        if self.name == "xmlns" {
            return Ordering::Less;
        }

        if self.name.starts_with("xmlns:") {
            if other.name.starts_with("xmlns:") {
                // Check local part
                return self.name.cmp(&other.name);
            }
            return Ordering::Less;
        }

        if self.name.contains(':') {
            // Qualified
            if other.name.contains(':') {
                // Other qualified
                // Sort by assocated addresses, THEN local part
                let namespaces = registered_namespaces.to_map();

                let mut this_split = self.name.split(':');
                let this_prefix = this_split.next().unwrap().to_string();
                let this_namespace = namespaces.get(&this_prefix).unwrap();
                let this_local = this_split.next().unwrap();

                let mut other_split = other.name.split(':');
                let other_prefix = other_split.next().unwrap().to_string();
                let other_namespace = namespaces.get(&other_prefix).unwrap();
                let other_local = other_split.next().unwrap();

                match this_namespace.url.cmp(&other_namespace.url) {
                    Ordering::Equal => this_local.cmp(other_local),
                    ord => ord,
                }
            } else {
                Ordering::Greater
            }
        } else {
            // Unqualified
            if other.name.contains(':') {
                // Other qualified
                Ordering::Less
            } else {
                // Check local part
                self.name.cmp(&other.name)
            }
        }
    }
}

pub fn canonicalize_start_tag(
    start_tag: &str,
    depth: usize,
    registered_namespaces: &mut DepthSensitiveMap<String, Namespace>,
) -> Result<String, Box<Error<Rule>>> {
    let mut tag = StartTag::default();

    let start_tag = StartTagParser::parse(Rule::StartTag, start_tag)?
        .next()
        .unwrap();
    tracing::trace!("start tag parsed as: {start_tag}");

    let mut start_tag_inner = start_tag.into_inner();

    let qname = start_tag_inner.next().unwrap();
    assert_eq!(qname.as_rule(), Rule::QName);
    qname.as_str().clone_into(&mut tag.name);

    for attr in start_tag_inner {
        if attr.as_rule() == Rule::EOI {
            break;
        }
        assert_eq!(attr.as_rule(), Rule::Attribute);

        let mut attr_inner = attr.into_inner();
        let name = attr_inner.next().unwrap();
        assert_eq!(name.as_rule(), Rule::QName);

        let value = attr_inner.next().unwrap().into_inner().next().unwrap();
        assert!([
            Rule::AttValueInnerNoSpaces,
            Rule::AttValueInnerDoubleQuote,
            Rule::AttValueInnerSingleQuote
        ]
        .contains(&value.as_rule()));

        let attribute = Attribute {
            name: name.as_str().to_owned(),
            value: super::character_refs::canonicalize_character_references(
                &value.as_str().replace('\r', "").replace('\n', " "),
                &super::character_refs::Situation::Attribute,
            )
            .unwrap(),
        };

        let mut add_attribute = true;

        // Detect namespaces and register
        if attribute.name == "xmlns" {
            let url = attribute.value.clone();

            #[allow(clippy::unnecessary_to_owned)]
            if registered_namespaces
                .to_map()
                .get(&"_".to_string())
                .map(|ns| ns.url.clone())
                == Some(url.clone())
            {
                add_attribute = false;
            } else {
                tracing::debug!("Registering new default namespace at depth {depth}: {url}");
                registered_namespaces.insert_at_depth(depth, "_", Namespace { url });
            }
        }
        if attribute.name.starts_with("xmlns:") {
            let name = attribute.name[6..].to_string();
            let url = attribute.value.clone();

            if registered_namespaces
                .to_map()
                .get(&name)
                .map(|ns| ns.url.clone())
                == Some(url.clone())
            {
                add_attribute = false;
            } else {
                tracing::debug!("Registering new {name} namespace at depth {depth}: {url}");
                registered_namespaces.insert_at_depth(depth, name, Namespace { url });
            }
        }

        if add_attribute {
            tag.attributes.push(attribute);
        }
    }

    // Sort attributes
    tag.attributes
        .sort_by(|a, b| a.cmp_with_namespaces(b, registered_namespaces));

    // TODO Determine if this element or any attributes reference namespaces

    Ok(tag.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_test::traced_test;

    #[test]
    fn test_formatting() {
        let a = StartTag {
            name: "Test".to_string(),
            attributes: vec![],
        };
        assert_eq!(a.to_string(), "Test");

        let a = StartTag {
            name: "Test".to_string(),
            attributes: vec![Attribute {
                name: "attr".to_string(),
                value: "abcdef".to_string(),
            }],
        };
        assert_eq!(a.to_string(), r#"Test attr="abcdef""#);
    }

    #[traced_test]
    #[test]
    fn test_canonicalize_start_tag() {
        let input = r#"a:e3   a:name = "elem3&asd;"   id=elem3   "#;
        let output = r#"a:e3 id="elem3" a:name="elem3&asd;""#;
        assert_eq!(
            output,
            canonicalize_start_tag(input, 0, &mut DepthSensitiveMap::new()).unwrap()
        );
    }

    #[traced_test]
    #[test]
    fn test_attribute_order() {
        let input = r#"e5 a:attr="out" b:attr="sorted" attr2="all" attr="I'm" xmlns:b="http://www.ietf.org" xmlns:a="http://www.w3.org" xmlns="http://example.org""#;
        let output = r#"e5 xmlns="http://example.org" xmlns:a="http://www.w3.org" xmlns:b="http://www.ietf.org" attr="I'm" attr2="all" b:attr="sorted" a:attr="out""#;
        assert_eq!(
            output,
            canonicalize_start_tag(input, 0, &mut DepthSensitiveMap::new()).unwrap()
        );
    }
}
