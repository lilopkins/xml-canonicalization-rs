#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
    sync::Arc,
};

use namespace::{DepthSensitiveMap, Namespace};
use quick_xml::{
    events::{BytesPI, BytesStart, BytesText, Event},
    Reader, Writer,
};
use regex::Regex;

mod grammars;
mod namespace;

/// [`Canonicalizer`]s take XML and return the canonicalised form of that XML.
pub struct Canonicalizer<R, W> {
    reader: Reader<R>,
    writer: Option<Writer<W>>,
}

impl<R, W> Canonicalizer<R, W> {
    /// Initialize a new [`Canonicalizer`] that reads from the provided
    /// `reader`.
    #[must_use]
    pub fn read_from_reader(reader: R) -> Self {
        tracing::trace!("Canonicalizer initialising with reader");
        Self {
            reader: Reader::from_reader(reader),
            writer: None,
        }
    }
}

impl<'a, W> Canonicalizer<&'a [u8], W> {
    /// Initialize a new [`Canonicalizer`] that reads from the provided
    /// `str`.
    #[must_use]
    pub fn read_from_str(str: &'a str) -> Self {
        tracing::trace!("Canonicalizer initialising from &str");
        Self {
            reader: Reader::from_str(str),
            writer: None,
        }
    }
}

impl<W> Canonicalizer<BufReader<File>, W> {
    /// Initialize a new [`Canonicalizer`] that reads from the provided
    /// `str`.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the file couldn't be opened and/or read.
    pub fn read_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Arc<std::io::Error>> {
        tracing::trace!("Canonicalizer initialising from file");
        Ok(Self {
            reader: Reader::from_file(path).map_err(|e| match e {
                quick_xml::Error::Io(e) => e,
                _ => unreachable!("no other errors are raised by quick_xml at this point"),
            })?,
            writer: None,
        })
    }
}

impl<R, W: Write> Canonicalizer<R, W> {
    /// Set the writer for this Canonicalizer to a writer.
    #[must_use]
    pub fn write_to_writer(mut self, writer: W) -> Self {
        self.writer = Some(Writer::new(writer));
        self
    }
}

impl<R> Canonicalizer<R, BufWriter<File>> {
    /// Set the writer for this Canonicalizer to a file.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the file couldn't be opened and/or read.
    pub fn write_to_file<P: AsRef<Path>>(mut self, path: P) -> Result<Self, std::io::Error> {
        self.writer = Some(Writer::new(BufWriter::new(File::create(path)?)));
        Ok(self)
    }
}

impl<R: BufRead, W: Write> Canonicalizer<R, W> {
    /// Start canonicalizing the document to the writer.
    ///
    /// # Errors
    ///
    /// Returns an XML error if the XML parsed is invalid.
    ///
    /// # Panics
    ///
    /// This will panic is a writer has not been initialised with
    /// `write_to_string`, `write_to_file`, or `write_to_writer`.
    #[allow(clippy::too_many_lines)]
    pub fn canonicalize(mut self, retain_comments: bool) -> Result<(), quick_xml::Error> {
        tracing::debug!("Canonicalisation starting…");

        // Set reader config
        self.reader.config_mut().allow_unmatched_ends = false;
        self.reader.config_mut().check_end_names = true;
        self.reader.config_mut().expand_empty_elements = true;

        let mut registered_namespaces = DepthSensitiveMap::new();
        // Add a default blank namespace.
        registered_namespaces.insert_at_depth(0, "_", Namespace { url: String::new() });
        let mut writer = self
            .writer
            .expect("trying to canonicalize without a writer initialised");
        let mut buf = vec![];
        let mut depth = 0;
        let mut hit_pi_rule = false;
        let mut text_buf = String::new();
        let whitespace_duplicate_regex = Regex::new(r"\n\n*").unwrap();
        let pi_tidyup_regex = Regex::new(r"^(\S+)(?:\s*( .*)|\s*$)").unwrap();
        loop {
            let e = self.reader.read_event_into(&mut buf);
            tracing::trace!("Event: {e:?}");
            match e {
                // The XML declaration and document type declaration (DTD) are removed.
                Ok(Event::Decl(_) | Event::DocType(_)) => (),
                // Empty should be unreachable as we've instructed the reader to always expand
                Ok(Event::Empty(_)) => unreachable!(),
                Ok(Event::PI(p)) => {
                    hit_pi_rule = true;
                    let p = p.into_inner();
                    let p = String::from_utf8_lossy(&p).into_owned();
                    let p = pi_tidyup_regex.replace_all(&p, "$1$2").into_owned();
                    if !text_buf.is_empty() {
                        writer.write_event(Event::Text(BytesText::from_escaped(&text_buf)))?;
                        text_buf.clear();
                    }
                    writer.write_event(Event::PI(BytesPI::new(p)))?;
                }

                // Remove all comments if needed
                Ok(Event::Comment(c)) => {
                    if retain_comments {
                        if !text_buf.is_empty() {
                            writer.write_event(Event::Text(BytesText::from_escaped(&text_buf)))?;
                            text_buf.clear();
                        }
                        writer.write_event(Event::Comment(c))?;
                    }
                }

                Ok(Event::Text(t)) => {
                    let text = t.into_inner();
                    let text = String::from_utf8_lossy(&text);
                    // Normalise whitespace
                    let text = text.replace("\r\n", "\n");
                    text_buf.push_str(
                        &grammars::character_refs::canonicalize_character_references(
                            &text,
                            &grammars::character_refs::Situation::Content,
                        )
                        .unwrap(),
                    );
                    // Remove whitespace outside of the root tag
                    if depth == 0 {
                        if hit_pi_rule {
                            // Remove duplicates only
                            text_buf = whitespace_duplicate_regex
                                .replace_all(&text_buf, "\n")
                                .into_owned();
                        } else {
                            text_buf = text_buf.replace('\n', "");
                        }
                    }
                }
                Ok(Event::CData(c)) => {
                    let c = c.into_inner();
                    let c = String::from_utf8_lossy(&c);
                    let c = c
                        .replace('&', "&amp;")
                        .replace('<', "&lt;")
                        .replace('>', "&gt;");
                    text_buf.push_str(&c);
                }
                Ok(Event::GeneralRef(b)) => {
                    let b = b.into_inner();
                    let b = String::from_utf8_lossy(&b);
                    text_buf.push_str(
                        &grammars::character_refs::canonicalize_character_reference(&b).unwrap(),
                    );
                }

                Ok(Event::Start(s)) => {
                    depth += 1;
                    if !text_buf.is_empty() {
                        writer.write_event(Event::Text(BytesText::from_escaped(&text_buf)))?;
                        text_buf.clear();
                    }
                    let s = String::from_utf8_lossy(&s);
                    let s = grammars::start::canonicalize_start_tag(
                        &s,
                        depth,
                        &mut registered_namespaces,
                    )
                    .unwrap();
                    writer.write_event(Event::Start(BytesStart::new(s.trim())))?;
                }
                Ok(Event::End(e)) => {
                    // Drop all known namespaces at base depth greater than new current
                    registered_namespaces.remove_depth(depth);

                    depth -= 1;

                    if !text_buf.is_empty() {
                        writer.write_event(Event::Text(BytesText::from_escaped(&text_buf)))?;
                        text_buf.clear();
                    }
                    writer.write_event(Event::End(e))?;
                }

                Ok(Event::Eof) => break,
                Err(e) => return Err(e),
            }
        }

        Ok(())
    }
}
