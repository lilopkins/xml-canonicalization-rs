use pest::{error::Error, iterators::Pair, Parser};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammars/character_refs.pest"]
struct CharacterRefParser;

pub enum Situation {
    Attribute,
    Content,
}

pub fn canonicalize_character_reference(input: &str) -> Result<String, Box<Error<Rule>>> {
    let mut output = String::new();

    let input_string = CharacterRefParser::parse(Rule::InputRef, input)?
        .next()
        .unwrap();
    tracing::trace!("input ref parsed as: {input_string}");

    let inner = input_string.into_inner().next().unwrap();
    assert_eq!(inner.as_rule(), Rule::InnerCharacterReference);

    let inner = inner.into_inner().next().unwrap();
    assert!([Rule::WellKnownRef, Rule::DecimalRef, Rule::HexadecimalRef].contains(&inner.as_rule()));

    inner_character_reference(&inner, &Situation::Content, &mut output);
    Ok(output)
}

pub fn canonicalize_character_references(
    input: &str,
    situation: &Situation,
) -> Result<String, Box<Error<Rule>>> {
    let mut output = String::new();

    let input_string = CharacterRefParser::parse(Rule::InputString, input)?
        .next()
        .unwrap();
    tracing::trace!("input string parsed as: {input_string}");

    for pair in input_string.into_inner() {
        match pair.as_rule() {
            Rule::EOI => break,
            Rule::Character => {
                output.push_str(pair.as_str());
            }
            Rule::CharacterThatMightNeedChanging => match pair.as_str() {
                ">" => match situation {
                    Situation::Attribute => output.push('>'),
                    Situation::Content => output.push_str("&gt;"),
                },
                "\"" => match situation {
                    Situation::Attribute => output.push_str("&quot;"),
                    Situation::Content => output.push('"'),
                },
                "\x0d" => output.push_str("&#xD"),
                "\x09" => match situation {
                    Situation::Attribute => output.push_str("&#x9;"),
                    Situation::Content => output.push('\x09'),
                },
                "\x0a" => match situation {
                    Situation::Attribute => output.push_str("&#xA;"),
                    Situation::Content => output.push('\x0a'),
                },
                _ => unreachable!(),
            },
            Rule::CharacterReference => {
                let inner = pair.into_inner().next().unwrap();
                let inner_ref = inner.into_inner().next().unwrap();
                inner_character_reference(&inner_ref, situation, &mut output);
            }
            _ => unreachable!(),
        }
    }

    Ok(output)
}

fn inner_character_reference(inner: &Pair<'_, Rule>, situation: &Situation, output: &mut String) {
    match inner.as_rule() {
        Rule::WellKnownRef => match inner.as_str() {
            "amp" => output.push_str("&amp;"),
            "lt" => output.push_str("&lt;"),
            "apos" => output.push('\''),

            "gt" => match situation {
                Situation::Attribute => output.push('>'),
                Situation::Content => output.push_str("&gt;"),
            },
            "quot" => match situation {
                Situation::Attribute => output.push_str("&quot;"),
                Situation::Content => output.push('"'),
            },
            _ => unreachable!(),
        },
        Rule::DecimalRef => {
            let val = inner.as_str();
            if let Ok(code) = val.parse::<u32>() {
                match code {
                    0xd => output.push_str("&#xD;"),
                    0xa => match situation {
                        Situation::Attribute => output.push_str("&#xA;"),
                        Situation::Content => {
                            output.push(char::from_u32(code).unwrap());
                        }
                    },
                    0x9 => match situation {
                        Situation::Attribute => output.push_str("&#x9;"),
                        Situation::Content => {
                            output.push(char::from_u32(code).unwrap());
                        }
                    },
                    _ => output.push(char::from_u32(code).unwrap()),
                }
            }
        }
        Rule::HexadecimalRef => {
            let val = inner.as_str();
            if let Ok(code) = u32::from_str_radix(val, 16) {
                match code {
                    0xd => output.push_str("&#xD;"),
                    0xa => match situation {
                        Situation::Attribute => output.push_str("&#xA;"),
                        Situation::Content => {
                            output.push(char::from_u32(code).unwrap());
                        }
                    },
                    0x9 => match situation {
                        Situation::Attribute => output.push_str("&#x9;"),
                        Situation::Content => {
                            output.push(char::from_u32(code).unwrap());
                        }
                    },
                    _ => output.push(char::from_u32(code).unwrap()),
                }
            }
        }
        _ => unreachable!(),
    }
}
