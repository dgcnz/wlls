// source: https://github.com/zoni/obsidian-export
pub mod references;
pub mod walker;

pub use walker::{vault_contents, WalkOptions};

use pulldown_cmark::{CowStr, Event, Options, Parser as MdParser, Tag, TagEnd};
use std::path::PathBuf;

use crate::references::{ObsidianNoteReference, RefParser, RefParserState, RefType};
use snafu::Snafu;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Snafu)]
#[allow(clippy::exhaustive_enums)]
pub enum ExportError {
    #[snafu(display("Encountered an error while trying to walk '{}'", path.display()))]
    WalkDirError { path: PathBuf, source: ignore::Error },
}

/// Get the full path for the given filename when it's contained in `vault_contents`, taking into
/// account:
///
/// 1. Standard Obsidian note references not including a .md extension.
/// 2. Case-insensitive matching
/// 3. Unicode normalization rules using normalization form C (<https://www.w3.org/TR/charmod-norm/#unicodeNormalization>)
pub fn lookup_filename_in_vault<'a>(
    filename: &str,
    vault_contents: &'a [PathBuf],
) -> Option<&'a PathBuf> {
    let filename = PathBuf::from(filename);
    let filename_normalized = filename.to_string_lossy().nfc().collect::<String>();

    vault_contents.iter().find(|path| {
        let path_normalized_str = path.to_string_lossy().nfc().collect::<String>();
        let path_normalized = PathBuf::from(&path_normalized_str);
        let path_normalized_lowered = PathBuf::from(&path_normalized_str.to_lowercase());

        path_normalized.ends_with(&filename_normalized)
            || path_normalized.ends_with(filename_normalized.clone() + ".md")
            || path_normalized_lowered.ends_with(filename_normalized.to_lowercase())
            || path_normalized_lowered.ends_with(filename_normalized.to_lowercase() + ".md")
    })
}

pub fn collect_references(content: &str) -> Vec<String> {
    let parser_options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_MATH
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_GFM;

    let mut frontmatter = String::new();
    let mut ref_parser = RefParser::new();
    let mut refs = Vec::new();
    // Most of the time, a reference triggers 5 events: [ or ![, [, <text>, ], ]
    let mut buffer = Vec::with_capacity(5);

    let mut parser = MdParser::new_ext(content, parser_options);
    'outer: while let Some(event) = parser.next() {
        // Collect frontmatter exactly like obsidian-export, but we don't use it.
        if matches!(event, Event::Start(Tag::MetadataBlock(_))) {
            for event in parser.by_ref() {
                match event {
                    Event::Text(cowstr) => frontmatter.push_str(&cowstr),
                    Event::End(TagEnd::MetadataBlock(_)) => {
                        continue 'outer;
                    }
                    _ => panic!(
                        "Unexpected event while processing frontmatter: {:?}",
                        event
                    ),
                }
            }
        }
        if ref_parser.state == RefParserState::Resetting {
            buffer.clear();
            ref_parser.reset();
        }
        buffer.push(event.clone());
        match ref_parser.state {
            RefParserState::NoState => match event {
                Event::Text(CowStr::Borrowed("![")) => {
                    ref_parser.ref_type = Some(RefType::Embed);
                    ref_parser.transition(RefParserState::ExpectSecondOpenBracket);
                }
                Event::Text(CowStr::Borrowed("[")) => {
                    ref_parser.ref_type = Some(RefType::Link);
                    ref_parser.transition(RefParserState::ExpectSecondOpenBracket);
                }
                _ => {
                    buffer.clear();
                }
            },
            RefParserState::ExpectSecondOpenBracket => match event {
                Event::Text(CowStr::Borrowed("[")) => {
                    ref_parser.transition(RefParserState::ExpectRefText);
                }
                _ => {
                    ref_parser.transition(RefParserState::Resetting);
                }
            },
            RefParserState::ExpectRefText => match event {
                Event::Text(CowStr::Borrowed("]")) => {
                    ref_parser.transition(RefParserState::Resetting);
                }
                Event::Text(text) => {
                    ref_parser.ref_text.push_str(&text);
                    ref_parser.transition(RefParserState::ExpectRefTextOrCloseBracket);
                }
                Event::Start(Tag::Emphasis) | Event::End(TagEnd::Emphasis) => {
                    ref_parser.ref_text.push('*');
                    ref_parser.transition(RefParserState::ExpectRefTextOrCloseBracket);
                }
                Event::Start(Tag::Strong) | Event::End(TagEnd::Strong) => {
                    ref_parser.ref_text.push_str("**");
                    ref_parser.transition(RefParserState::ExpectRefTextOrCloseBracket);
                }
                Event::Start(Tag::Strikethrough) | Event::End(TagEnd::Strikethrough) => {
                    ref_parser.ref_text.push_str("~~");
                    ref_parser.transition(RefParserState::ExpectRefTextOrCloseBracket);
                }
                _ => {
                    ref_parser.transition(RefParserState::Resetting);
                }
            },
            RefParserState::ExpectRefTextOrCloseBracket => match event {
                Event::Text(CowStr::Borrowed("]")) => {
                    ref_parser.transition(RefParserState::ExpectFinalCloseBracket);
                }
                Event::Text(text) => {
                    ref_parser.ref_text.push_str(&text);
                }
                Event::Start(Tag::Emphasis) | Event::End(TagEnd::Emphasis) => {
                    ref_parser.ref_text.push('*');
                }
                Event::Start(Tag::Strong) | Event::End(TagEnd::Strong) => {
                    ref_parser.ref_text.push_str("**");
                }
                Event::Start(Tag::Strikethrough) | Event::End(TagEnd::Strikethrough) => {
                    ref_parser.ref_text.push_str("~~");
                }
                _ => {
                    ref_parser.transition(RefParserState::Resetting);
                }
            },
            RefParserState::ExpectFinalCloseBracket => match event {
                Event::Text(CowStr::Borrowed("]")) => match ref_parser.ref_type {
                    Some(RefType::Link) | Some(RefType::Embed) => {
                        let raw = ref_parser.ref_text.clone();
                        let note_ref = ObsidianNoteReference::from_str(raw.as_ref());
                        if let Some(file) = note_ref.file {
                            refs.push(file.to_string());
                        }
                        buffer.clear();
                        ref_parser.transition(RefParserState::Resetting);
                    }
                    None => panic!("In ExpectFinalCloseBracket but ref_type is None"),
                },
                _ => {
                    ref_parser.transition(RefParserState::Resetting);
                }
            },
            RefParserState::Resetting => {
                buffer.clear();
                ref_parser.reset();
            }
        }
    }

    refs
}
