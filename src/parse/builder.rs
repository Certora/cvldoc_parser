use std::ops::Not;

use super::lexed::{Ast, CodeChunk, Style, Token};
use super::terminated_line::{TerminatedLine, Terminator};
use crate::parse::terminated_line::JoinToString;
use crate::util::span_to_range::{RangeConverter, Span};
use crate::{AssociatedElement, CvlDoc, DocData, DocumentationTag, Tag};
use chumsky::primitive::todo;
use color_eyre::eyre::{bail, ensure, eyre};
use color_eyre::Result;
use itertools::Itertools;
use ropey::Rope;

#[derive(Debug, Clone)]
pub enum CvlDocBuilder {
    FreeFormComment {
        text: String,
        span: Span,
    },
    Documentation {
        spanned_body: Vec<(TerminatedLine, Span)>,
        associated: Option<AssociatedElement>,
        span: Span,
    },
    DocumentationTwo {
        doc_tags: Vec<DocumentationTag>,
        span: Span,
    },
    AssociatedElement(AssociatedElement),
    CommentedOutBlock,
    CommentedOutLine,
    ParseError,
}

impl Ast {
    // pub(super) fn from_token(token: Token, span: Span, converter: RangeConverter) -> CvlDocBuilderReborn {
    //     match token {
    //         Token::FreeFormSlashed(input) => {
    //             let text = ContentLines::new(&input, span.clone(), &['/'])
    //                 .into_iter()
    //                 .map(|(terminated_line, _)| terminated_line.to_string())
    //                 .collect();

    //             CvlDocBuilderReborn::FreeFormComment(text)
    //         }
    //         Token::FreeFormStarred(input) => {
    //             let text = ContentLines::new(&input, span.clone(), &['/', '*', ' ', '\t'])
    //                 .into_iter()
    //                 .map(|(terminated_line, _)| terminated_line.to_string())
    //                 .collect();

    //             CvlDocBuilderReborn::FreeFormComment(text)
    //         }
    //         Token::CvlDocSlashed(input) => {
    //             let spanned_body = ContentLines::new(&input, span.clone(), &['/']);
    //             let doc_tags = CvlDocBuilder::process_doc_body2(spanned_body, converter);
    //             CvlDocBuilderReborn::Documentation(doc_tags)
    //         }
    //         Token::CvlDocStarred(input) => {
    //             let spanned_body = ContentLines::new(&input, span.clone(), &['/', '*', ' ', '\t']);
    //             let doc_tags = CvlDocBuilder::process_doc_body2(spanned_body, converter);
    //             CvlDocBuilderReborn::Documentation(doc_tags)
    //         },
    //         _ => panic!(),
    //     }
    // }

    // pub(super) fn from_spanned_raw_string(data: RawString, span: Span, converter: RangeConverter) -> CvlDocBuilderReborn {
    //     match data {
    //         RawString::FreeFormSlashed(input) => {
    //             let text = ContentLines::new(&input, span.clone(), &['/'])
    //                 .into_iter()
    //                 .map(|(terminated_line, _)| terminated_line.to_string())
    //                 .collect();

    //             CvlDocBuilderReborn::FreeFormComment(text)
    //         }
    //         RawString::FreeFormStarred(input) => {
    //             let text = ContentLines::new(&input, span.clone(), &['/', '*', ' ', '\t'])
    //                 .into_iter()
    //                 .map(|(terminated_line, _)| terminated_line.to_string())
    //                 .collect();

    //             CvlDocBuilderReborn::FreeFormComment(text)
    //         }
    //         RawString::CvlDocSlashed(input) => {
    //             let spanned_body = ContentLines::new(&input, span.clone(), &['/']);
    //             let doc_tags = CvlDocBuilder::process_doc_body2(spanned_body, converter);
    //             CvlDocBuilderReborn::Documentation(doc_tags)
    //         }
    //         RawString::CvlDocStarred(input) => {
    //             let spanned_body = ContentLines::new(&input, span.clone(), &['/', '*', ' ', '\t']);
    //             let doc_tags = CvlDocBuilder::process_doc_body2(spanned_body, converter);
    //             CvlDocBuilderReborn::Documentation(doc_tags)
    //         },
    //     }
    // }

    fn chars_to_trim<'a>(style: Style) -> &'a [char] {
        match style {
            Style::Slashed => &['/'],
            Style::Starred => &['/', '*', ' ', '\t'],
        }
    }

    pub(super) fn process(self, converter: RangeConverter, src: &str) -> CvlDocBuilder {
        // let chunk = |code_chunk: &CodeChunk| {
        //     let span = code_chunk.0.clone();
        //     converter.byte_slice(span).map(String::from)
        // };

        let chunk = |code_chunk: &CodeChunk| code_chunk.to_str(src).map(String::from);

        match self {
            Ast::FreeFormComment(style, span) => {
                let input = src.get(span.clone()).expect("validated by parser");

                let text = ContentLines::new(input, span.clone(), Ast::chars_to_trim(style))
                    .into_iter()
                    .map(|(terminated_line, _)| terminated_line.to_string())
                    .collect();

                CvlDocBuilder::FreeFormComment { text, span }
            }
            Ast::Documentation(style, span) => {
                let input = src.get(span.clone()).expect("validated by parser");

                let spanned_body =
                    ContentLines::new(input, span.clone(), Ast::chars_to_trim(style));
                let doc_tags = CvlDocBuilder::process_doc_body2(spanned_body, converter.clone());
                CvlDocBuilder::DocumentationTwo { doc_tags, span }
            }
            Ast::Methods { block } => {
                let block = chunk(&block).expect("validated by parser");
                let assoc = AssociatedElement::Methods { block };
                CvlDocBuilder::AssociatedElement(assoc)
            }
            Ast::Function {
                name,
                params,
                returns,
                block,
            } => {
                let block = chunk(&block).expect("validated by parser");
                let assoc = AssociatedElement::Function {
                    name,
                    params,
                    returns,
                    block,
                };

                CvlDocBuilder::AssociatedElement(assoc)
            }
            Ast::ParseError => CvlDocBuilder::ParseError,
            Ast::GhostMapping {
                mapping,
                name,
                block,
            } => {
                let block = chunk(&block);
                let assoc = AssociatedElement::GhostMapping {
                    name,
                    mapping,
                    block,
                };

                CvlDocBuilder::AssociatedElement(assoc)
            }
            Ast::Ghost {
                name,
                ty_list,
                returns,
                block,
            } => {
                let block = block.as_ref().and_then(chunk);
                let assoc = AssociatedElement::Ghost {
                    name,
                    ty_list,
                    returns,
                    block,
                };

                CvlDocBuilder::AssociatedElement(assoc)
            }
            Ast::Rule {
                name,
                params,
                filters,
                block,
            } => {
                let block = chunk(&block).expect("validated by parser");
                let filters = filters.as_ref().and_then(chunk);
                let assoc = AssociatedElement::Rule {
                    name,
                    params,
                    filters,
                    block,
                };

                CvlDocBuilder::AssociatedElement(assoc)
            }
            Ast::Definition {
                name,
                params,
                returns,
                definition,
            } => {
                let definition = definition
                    .trim_start()
                    .trim_end_matches(|c: char| c.is_ascii_whitespace() || c == ';')
                    .to_string();
                let assoc = AssociatedElement::Definition {
                    name,
                    params,
                    returns,
                    definition,
                };

                CvlDocBuilder::AssociatedElement(assoc)
            }
            Ast::InvariantSimplified {
                name,
                params,
                spanned_tail,
            } => {
                // //the invariant tail is split into these three parts, in order:
                // // (1) the invariant expression (mandatory)
                // // (2) param filters block (optional)
                // // (3) the invariant proof (optional)

                // let filters_start = spanned_tail
                //     .iter()
                //     .position(|tok| matches!(tok, Token::Filtered));
                // let proof_start = spanned_tail
                //     .iter()
                //     .position(|tok| matches!(tok, Token::Preserved));

                // let filters = {
                //     let mut balance = 0;

                //     let filter_block = spanned_tail
                //         .iter()
                //         .map(|(tok, _span)| tok)
                //         .skip_while(|tok| !matches!(tok, Token::Filtered))
                //         .take_while(|tok| {
                //             match tok {
                //                 Token::CurlyOpen => balance += 1,
                //                 Token::CurlyClose => balance -= 1,
                //                 _ => (),
                //             };
                //             balance > 0
                //         })
                //         .join(" ");

                //     if !filter_block.is_empty() {
                //         Some(filter_block)
                //     } else {
                //         None
                //     }
                // };

                // let assoc = AssociatedElement::Invariant {
                //     name,
                //     params,
                //     invariant: Default::default(),
                //     filters,
                //     block: Default::default(),
                // };

                // CvlDocBuilder::AssociatedElement(assoc)

                todo!()
            }
            Ast::Invariant {
                name,
                params,
                invariant,
                filters,
                proof,
            } => {
                let invariant = chunk(&invariant).unwrap_or_else(|| format!("{invariant:?}"));
                let filters = filters.as_ref().and_then(chunk);
                let proof = proof.as_ref().and_then(chunk);

                let assoc = AssociatedElement::Invariant {
                    name,
                    params,
                    invariant,
                    filters,
                    block: proof,
                };

                CvlDocBuilder::AssociatedElement(assoc)
            }
        }
    }
}

impl CvlDocBuilder {
    // pub(super) fn from_spanned_raw_string(data: RawString, span: Span, converter: RangeConverter) -> CvlDocBuilder {
    //     match data {
    //         RawString::FreeFormSlashed(input) => {
    //             let text = ContentLines::new(&input, span.clone(), &['/'])
    //                 .into_iter()
    //                 .map(|(terminated_line, _)| terminated_line.to_string())
    //                 .collect();

    //             CvlDocBuilder::FreeFormComment { text, span }
    //         }
    //         RawString::FreeFormStarred(input) => {
    //             let text = ContentLines::new(&input, span.clone(), &['/', '*', ' ', '\t'])
    //                 .into_iter()
    //                 .map(|(terminated_line, _)| terminated_line.to_string())
    //                 .collect();

    //             CvlDocBuilder::FreeFormComment { text, span }
    //         }
    //         RawString::CvlDocSlashed(input) => {
    //             let spanned_body = ContentLines::new(&input, span.clone(), &['/']);
    //             CvlDocBuilder::process_doc_body2(spanned_body, converter)
    //         }
    //         RawString::CvlDocStarred(input) => todo!(),
    //     }
    // }

    fn raw_data(src: Rope, span: Span) -> Result<String> {
        src.get_slice(span)
            .map(String::from)
            .ok_or_else(|| eyre!("span is outside of file bounds"))
    }

    pub fn build(self, converter: RangeConverter, src: Rope) -> Result<CvlDoc> {
        match self {
            CvlDocBuilder::FreeFormComment { text, span } => {
                let cvl_doc = CvlDoc {
                    raw: CvlDocBuilder::raw_data(src, span.clone())?,
                    range: converter.to_range(span),
                    data: DocData::FreeForm(text),
                };
                Ok(cvl_doc)
            }
            CvlDocBuilder::Documentation {
                spanned_body,
                associated,
                span,
            } => {
                ensure!(!spanned_body.is_empty(), "documentation has no body");
                let tags = CvlDocBuilder::process_doc_body(spanned_body, converter.clone());

                let cvl_doc = CvlDoc {
                    raw: CvlDocBuilder::raw_data(src, span.clone())?,
                    range: converter.to_range(span),
                    data: DocData::Documentation { tags, associated },
                };
                Ok(cvl_doc)
            }
            CvlDocBuilder::CommentedOutBlock | CvlDocBuilder::CommentedOutLine => {
                bail!("currently commented out code is not parsed")
            }
            CvlDocBuilder::ParseError => bail!("parse errors can not be converted"),
            CvlDocBuilder::DocumentationTwo { .. } => todo!(),
            CvlDocBuilder::AssociatedElement(_) => todo!(),
        }
    }

    fn tag_from_content(content: &[char]) -> Option<Tag> {
        let mut content_chars = content.iter().copied();

        match content_chars.next() {
            Some('@') => {
                let tag_literal: String = content_chars
                    .take_while(|c| !c.is_ascii_whitespace())
                    .collect();
                Some(tag_literal.into())
            }
            _ => None,
        }
    }

    fn tag_from_content2(content: &str) -> Option<Tag> {
        if content.starts_with('@') {
            let tag_end = content
                .find(|c: char| c.is_ascii_whitespace())
                .unwrap_or(content.len());

            let tag = content[..tag_end].into();
            Some(tag)
        } else {
            None
        }
    }

    fn process_doc_body(
        spanned_body: Vec<(TerminatedLine, Span)>,
        converter: RangeConverter,
    ) -> Vec<DocumentationTag> {
        let mut tags = Vec::new();

        let mut cur_tag = Tag::default();
        let mut cur_desc: Vec<TerminatedLine> = Vec::new();
        let mut cur_span = None;

        static PADDING: &[char] = &[' ', '\t'];

        for (mut line, line_span) in spanned_body.into_iter() {
            if let Some(new_tag) = CvlDocBuilder::tag_from_content(&line.content) {
                if !cur_desc.is_empty() {
                    let desc = std::mem::take(&mut cur_desc);
                    let doc_tag = DocumentationTag::new(cur_tag, desc.join_to_string(), cur_span);

                    tags.push(doc_tag);
                }

                line.content.drain(..new_tag.len() + 1);
                cur_tag = new_tag;
                cur_span = {
                    let start = line_span.start;
                    let span = start..start + cur_tag.len();
                    Some(converter.to_range(span))
                };
            }

            line = line.trim(PADDING);
            cur_desc.push(line);
        }

        // this check deals with the cases where the body was empty,
        // or contained only whitespace lines.
        // otherwise we are guaranteed to have an in-progress tag that should be pushed.
        if !cur_desc.is_empty() {
            let doc_tag = DocumentationTag::new(cur_tag, cur_desc.join_to_string(), cur_span);
            tags.push(doc_tag);
        }

        tags
    }

    fn process_doc_body2<'a>(
        spanned_input: impl IntoIterator<Item = (TerminatedStr<'a>, Span)>,
        converter: RangeConverter,
    ) -> Vec<DocumentationTag> {
        let mut tags = Vec::new();

        let mut cur_tag = Tag::default();
        let mut cur_desc: Vec<TerminatedStr> = Vec::new();
        let mut cur_span = None;

        for (mut line, line_span) in spanned_input.into_iter() {
            if let Some(new_tag) = CvlDocBuilder::tag_from_content2(line.content) {
                if !cur_desc.is_empty() {
                    let desc = std::mem::take(&mut cur_desc);
                    let doc_tag = DocumentationTag::new(cur_tag, String::from_iter(desc), cur_span);

                    tags.push(doc_tag);
                }

                line.content = &line.content[new_tag.len() + 1..];
                cur_tag = new_tag;
                cur_span = {
                    let start = line_span.start;
                    let span = start..start + cur_tag.len();
                    Some(converter.to_range(span))
                };
            }

            cur_desc.push(line);
        }

        // this check deals with the cases where the body was empty,
        // or contained only whitespace lines.
        // otherwise we are guaranteed to have an in-progress tag that should be pushed.
        if !cur_desc.is_empty() {
            let doc_tag = DocumentationTag::new(cur_tag, String::from_iter(cur_desc), cur_span);
            tags.push(doc_tag);
        }

        tags
    }
}

pub(super) fn split_starred_doc_lines(
    stream: Vec<char>,
    span: Span,
) -> Vec<(TerminatedLine, Span)> {
    let not_padding = |c: &char| !c.is_ascii_whitespace() && *c != '*';
    static PADDING: &[char] = &[' ', '\t', '*'];
    let mut next_line_start = span.start;

    stream
        .split_inclusive(|&c| c == '\n')
        .map(|line| {
            let line_start = next_line_start;
            next_line_start += line.len();

            (line, line_start)
        })
        .map(|(line, line_start)| {
            let trimmed_start = line.iter().position(not_padding).unwrap_or(0);
            let trimmed_end = line.iter().rposition(not_padding).unwrap_or(line.len());
            let trimmed_span = (line_start + trimmed_start)..(line_start + trimmed_end);

            let terminated_line = TerminatedLine::from_char_slice(line)
                .trim_start(PADDING)
                .trim_end(PADDING);

            (terminated_line, trimmed_span)
        })
        .collect()
}

// preserves newlines, strips prefixes, updates span for each line
pub struct ContentLines<'a, 'b> {
    input: &'a str,
    span: Span,
    chars_to_trim: &'b [char],
}

impl<'a, 'b> ContentLines<'a, 'b> {
    pub fn new(input: &'a str, span: Span, chars_to_trim: &'b [char]) -> ContentLines<'a, 'b> {
        ContentLines {
            input,
            span,
            chars_to_trim,
        }
    }

    fn next_split(&self) -> usize {
        self.input
            .find('\n')
            .map(|i| i + 1)
            .unwrap_or(self.input.len())
    }
}

impl<'a, 'b> Iterator for ContentLines<'a, 'b> {
    type Item = (TerminatedStr<'a>, Span);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.input.is_empty() {
            return None;
        }

        let split_point = self.next_split();

        let (line, rest) = self.input.split_at(split_point);

        self.input = rest;

        let cur_span_start = self.span.start;
        let cur_span_end = cur_span_start + split_point;
        let span_of_line = cur_span_start..cur_span_end;
        self.span.start = cur_span_end;

        {
            //potentially skipping first and last lines for multiline starred comments
            let trimmed = line.trim();
            if trimmed == "/**" || trimmed == "*/" {
                return self.next();
            }
        }

        let mut terminated = TerminatedStr::from(line);
        let line_to_trim = &terminated.content;
        terminated.content = line_to_trim.trim_matches(self.chars_to_trim).trim();

        Some((terminated, span_of_line))
    }
}

pub struct TerminatedStr<'a> {
    content: &'a str,
    ter: Terminator,
}

impl<'a> From<&'a str> for TerminatedStr<'a> {
    fn from(line: &'a str) -> Self {
        use Terminator::*;

        for ter in [CRLF, LF, CR, EOF] {
            if let Some(without_ter) = line.strip_suffix(ter.as_str()) {
                return TerminatedStr {
                    content: without_ter,
                    ter,
                };
            }
        }

        unreachable!()
    }
}

impl ToString for TerminatedStr<'_> {
    fn to_string(&self) -> String {
        let line_chars = self.content.chars();
        let ter_chars = self.ter.as_str().chars();
        line_chars.chain(ter_chars).collect()
    }
}

impl<'a> FromIterator<TerminatedStr<'a>> for String {
    fn from_iter<T: IntoIterator<Item = TerminatedStr<'a>>>(iter: T) -> Self {
        let mut joined = String::new();
        for ter_line in iter {
            joined.push_str(ter_line.content);
            joined.push_str(ter_line.ter.as_str());
        }

        while joined.ends_with(&['\r', '\n']) {
            joined.pop();
        }

        joined
    }
}
