use super::terminated_str::TerminatedStr;
use super::{Ast, CodeChunk, Style};
use crate::util::{RangeConverter, Span};
use crate::{AssociatedElement, CvlDoc, DocData, DocumentationTag, Tag};

use color_eyre::eyre::{bail, ensure, eyre};
use color_eyre::{Report, Result};
use itertools::Itertools;
use ropey::Rope;

#[derive(Debug, Clone)]
enum Processed {
    FreeFormComment(String),
    Documentation(Vec<DocumentationTag>),
    AssociatedElement(AssociatedElement),
}

pub struct Builder<'src> {
    src: &'src str,
    converter: RangeConverter,
}

impl<'src> Builder<'src> {
    pub fn new(src: &'src str) -> Self {
        let converter = RangeConverter::new(Rope::from_str(src));
        Builder { src, converter }
    }

    const fn chars_to_trim<'a>(style: Style) -> &'a [char] {
        match style {
            Style::Slashed => &['/'],
            Style::Starred => &['/', '*', ' ', '\t'],
        }
    }

    fn chunk(&self, code_chunk: &CodeChunk) -> String {
        let span = code_chunk.0.clone();
        self.src.get(span).unwrap().to_string()
    }

    fn parse_doc_tags(
        &self,
        input: impl IntoIterator<Item = (TerminatedStr<'src>, Span)>,
    ) -> Vec<DocumentationTag> {
        let mut tags = Vec::new();

        let mut cur_tag = Tag::default();
        let mut cur_desc: Vec<TerminatedStr> = Vec::new();
        let mut cur_span = None;

        for (mut line, line_span) in input.into_iter() {
            if let Some(new_tag) = Builder::tag_from_content(line.content) {
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
                    Some(self.converter.to_range(span))
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

    fn tag_from_content(content: &str) -> Option<Tag> {
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

    pub fn build(&self, parsing_results: Vec<(Ast, Span)>) -> Vec<CvlDoc> {
        let processed_spanned = parsing_results
            .into_iter()
            .map(|spanned_ast| {
                let span = spanned_ast.1.clone();
                let processed = self.process(spanned_ast);
                (processed, span)
            })
            .collect_vec();

        let mut cvl_docs = Vec::with_capacity(processed_spanned.len());
        
    }

    fn process(&self, (ast, span): (Ast, Span)) -> Processed {
        let input = self.src.get(span).unwrap();

        match ast {
            Ast::FreeFormComment(style, span) => {
                let text = ContentLines::new(input, span.clone(), Builder::chars_to_trim(style))
                    .into_iter()
                    .map(|(terminated_line, _)| terminated_line.to_string())
                    .collect();

                Processed::FreeFormComment(text)
            }
            Ast::Documentation(style, span) => {
                let body = ContentLines::new(input, span.clone(), Builder::chars_to_trim(style));
                let doc_tags = self.parse_doc_tags(body);
                Processed::Documentation(doc_tags)
            }
            Ast::Methods { block } => {
                let block = self.chunk(&block);
                let assoc = AssociatedElement::Methods { block };
                Processed::AssociatedElement(assoc)
            }
            Ast::Function {
                name,
                params,
                returns,
                block,
            } => {
                let block = self.chunk(&block);
                let assoc = AssociatedElement::Function {
                    name,
                    params,
                    returns,
                    block,
                };

                Processed::AssociatedElement(assoc)
            }
            Ast::ParseError => panic!("parse errors are not parsed"),
            Ast::GhostMapping {
                mapping,
                name,
                block,
            } => {
                let block = block.map(|c| self.chunk(&c));
                let assoc = AssociatedElement::GhostMapping {
                    name,
                    mapping,
                    block,
                };

                Processed::AssociatedElement(assoc)
            }
            Ast::Ghost {
                name,
                ty_list,
                returns,
                block,
            } => {
                let block = block.map(|c| self.chunk(&c));
                let assoc = AssociatedElement::Ghost {
                    name,
                    ty_list,
                    returns,
                    block,
                };

                Processed::AssociatedElement(assoc)
            }
            Ast::Rule {
                name,
                params,
                filters,
                block,
            } => {
                let block = self.chunk(&block);
                let filters = filters.map(|c| self.chunk(&c));
                let assoc = AssociatedElement::Rule {
                    name,
                    params,
                    filters,
                    block,
                };

                Processed::AssociatedElement(assoc)
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

                Processed::AssociatedElement(assoc)
            }
            Ast::Invariant {
                name,
                params,
                invariant,
                filters,
                proof,
            } => {
                let invariant = self.chunk(&invariant);
                let filters = filters.map(|c| self.chunk(&c));
                let proof = proof.map(|c| self.chunk(&c));

                let assoc = AssociatedElement::Invariant {
                    name,
                    params,
                    invariant,
                    filters,
                    proof,
                };

                Processed::AssociatedElement(assoc)
            }
        }
    }
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
