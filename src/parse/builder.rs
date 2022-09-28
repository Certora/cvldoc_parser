use super::terminated_line::TerminatedLine;
use crate::parse::terminated_line::JoinToString;
use crate::util::span_to_range::{RangeConverter, Span};
use crate::{AssociatedElement, CvlDoc, DocData, DocumentationTag, Tag};
use color_eyre::eyre::{bail, ensure, eyre};
use color_eyre::Report;
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
    CommentedOutBlock,
    CommentedOutLine,
    ParseError,
}

impl CvlDocBuilder {
    fn raw_data(src: Rope, span: Span) -> Result<String, Report> {
        src.get_slice(span)
            .map(String::from)
            .ok_or_else(|| eyre!("span is outside of file bounds"))
    }

    pub fn build(self, converter: RangeConverter, src: Rope) -> Result<CvlDoc, Report> {
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
