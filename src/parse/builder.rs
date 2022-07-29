use crate::util::span_to_range::{RangeConverter, Span, Spanned};
use crate::{AssociatedElement, DeclarationKind, DocumentationTag, NatSpec, Tag};
use color_eyre::eyre::bail;
use color_eyre::Report;
use itertools::Itertools;

#[derive(Debug, Clone)]
pub enum NatSpecBuilder {
    FreeFormComment {
        header: String,
        block: Option<String>,
    },
    Documentation {
        spanned_body: Vec<Spanned<String>>,
        element_under_doc: Option<UnderDoc>,
    },
    CommentedOutBlock,
    ParseError,
}

#[derive(Debug, Clone)]
pub struct UnderDoc(pub DeclarationKind, pub String, pub Vec<(String, String)>);

impl From<UnderDoc> for AssociatedElement {
    fn from(under: UnderDoc) -> Self {
        let UnderDoc(kind, name, params) = under;
        AssociatedElement { kind, name, params }
    }
}

impl NatSpecBuilder {
    pub fn build_with_converter(self, converter: RangeConverter) -> Result<NatSpec, Report> {
        match self {
            NatSpecBuilder::FreeFormComment { header, block } => {
                let free_form = match block {
                    Some(block) => NatSpec::MultiLineFreeForm { header, block },
                    _ => NatSpec::SingleLineFreeForm { header },
                };
                Ok(free_form)
            }
            NatSpecBuilder::Documentation {
                spanned_body,
                element_under_doc,
            } => {
                if spanned_body.is_empty() {
                    bail!("documentation has no body");
                }
                let tags = NatSpecBuilder::process_doc_body(&spanned_body, converter);

                let associated = element_under_doc.map(AssociatedElement::from);

                Ok(NatSpec::Documentation { tags, associated })
            }
            NatSpecBuilder::CommentedOutBlock => bail!("currently commented out code is not parsed"),
            NatSpecBuilder::ParseError => bail!("parse errors can not be converted"),
        }
    }

    fn process_doc_body(
        spanned_body: &[(String, Span)],
        converter: RangeConverter,
    ) -> Vec<DocumentationTag> {
        let mut tags = Vec::new();

        let mut cur_tag = Tag::default();
        let mut cur_desc = String::new();
        let mut cur_span = None;

        let whitespace = &[' ', '\t'];

        for (line, line_span) in spanned_body {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let not_finished_with_previous_tag = !cur_desc.is_empty();

            if line.starts_with('@') {
                if not_finished_with_previous_tag {
                    let doc_tag =
                        DocumentationTag::new(cur_tag.clone(), cur_desc.clone(), cur_span);
                    tags.push(doc_tag);

                    cur_desc.clear();
                }

                let (tag, desc) = line.split_once(whitespace).unwrap_or_else(|| {
                    //I'm not sure if it is an error to have a line that starts with @,
                    //but has no (horizontal) whitespace. for now we accept this.

                    //note that this condition includes newlines
                    let last_non_whitespace =
                        line.rfind(|c: char| !c.is_ascii_whitespace()).unwrap();
                    line.split_at(last_non_whitespace)
                });

                cur_tag = tag.into();

                cur_desc.push_str(desc);

                cur_span = {
                    let start = line_span.start;
                    let span = start..start + tag.chars().count();
                    Some(converter.to_range(span))
                };
            } else {
                //then it is a run-on description
                if not_finished_with_previous_tag {
                    cur_desc.push('\n');
                }
                cur_desc.push_str(line);
            }
        }

        // this check deals with the cases where the body was empty,
        // or contained only whitespace lines.
        // otherwise we are guaranteed to have an in-progress tag that should be pushed.
        if !cur_desc.is_empty() {
            let doc_tag = DocumentationTag::new(cur_tag, cur_desc, cur_span);
            tags.push(doc_tag);
        }

        tags
    }

    pub(super) fn free_form_multi_line_from_body(body: String) -> NatSpecBuilder {
        let padding: &[_] = &[' ', '\t', '*', '\n'];
        let mut lines = body.lines().map(|line| line.trim_matches(padding));

        let header = lines
            .next()
            .map(String::from)
            .expect("must exist from parser definition");

        let block = {
            let joined = lines.filter(|line| !line.is_empty()).join("\n");

            if !joined.is_empty() {
                Some(joined)
            } else {
                None
            }
        };

        NatSpecBuilder::FreeFormComment { header, block }
    }
}

pub(super) fn split_starred_doc_lines(stream: Vec<char>, span: Span) -> Vec<(String, Span)> {
    let not_padding = |c: &char| !c.is_ascii_whitespace() && *c != '*';
    let mut next_line_start = span.start;

    stream
        .split_inclusive(|&c| c == '\n')
        .map(|line| {
            //we still update the start position
            //even if the line is later skipped.
            let line_start = next_line_start;
            next_line_start += line.len();

            (line, line_start)
        })
        .filter_map(|(line, line_start)| {
            let trimmed_start = line.iter().position(not_padding)?;
            let trimmed_end = line.iter().rposition(not_padding)?;

            let trimmed_span = (line_start + trimmed_start)..(line_start + trimmed_end);
            let trimmed_line = line[trimmed_start..=trimmed_end].iter().collect();

            Some((trimmed_line, trimmed_span))
        })
        .collect()
}
