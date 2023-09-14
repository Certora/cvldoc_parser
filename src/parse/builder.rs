use super::terminated_str::TerminatedStr;
use super::types::Token;
use super::{cvl_parser, lexer::cvl_lexer, Intermediate, Span, Style};
use crate::util::ByteSpan;
use crate::{Ast, CvlElement, DocumentationTag, TagKind};
use chumsky::{Parser, Stream};
use color_eyre::eyre::{bail, eyre};
use color_eyre::Result;
use core::panic;
use std::sync::Arc;

struct DocumentationBuilder<'src> {
    kind: TagKind,
    desc: Vec<TerminatedStr<'src>>,
    span: Span,
}

impl<'src> DocumentationBuilder<'src> {
    fn new(entire_span: Span) -> DocumentationBuilder<'src> {
        DocumentationBuilder {
            kind: TagKind::default(),
            desc: Vec::new(),
            span: entire_span,
        }
    }
}

impl DocumentationTag {
    fn from_spanned_iter<'src>(
        input: impl IntoIterator<Item = (TerminatedStr<'src>, Span)>,
        entire_span: Span,
    ) -> Vec<DocumentationTag> {
        let mut tags = Vec::new();

        let mut builder = DocumentationBuilder::new(entire_span);

        for (mut line, line_span) in input {
            if let Some(new_tag) = Builder::tag_from_content(line.content) {
                if builder.previous_tag_still_in_progress() {
                    tags.push(builder.build_current());
                }

                if let Some(after_tag) = line.content.get(new_tag.len() + 1..) {
                    line.content = after_tag;
                }

                builder.kind = new_tag;

                builder.span.start = line_span.start;
            }

            builder.span.end = line_span.end;
            builder.push_line(line);
        }

        // if the body wasn't empty, we are guaranteed to have
        // an in-progress tag that should be pushed here.
        if builder.previous_tag_still_in_progress() {
            tags.push(builder.build_current());
        }

        tags
    }
}

impl<'a> DocumentationBuilder<'a> {
    fn previous_tag_still_in_progress(&self) -> bool {
        !self.desc.is_empty()
    }

    fn push_line(&mut self, line: TerminatedStr<'a>) {
        self.desc.push(line);
    }

    fn build_current(&mut self) -> DocumentationTag {
        let desc = std::mem::take(&mut self.desc);

        DocumentationTag {
            kind: self.kind.clone(),
            description: String::from_iter(desc),
            span: self.span.clone(),
        }
    }
}

trait ToSpan {
    fn to_span(&self) -> Span;
}

impl ToSpan for Span {
    fn to_span(&self) -> Span {
        self.clone()
    }
}

enum DocOrAst {
    Doc(Vec<DocumentationTag>),
    Ast(Ast),
}

pub struct Builder<'src>(&'src str);

impl<'src> Builder<'src> {
    pub fn new(src: &'src str) -> Self {
        Builder(src)
    }

    pub fn lex(&self) -> Result<Vec<(Token, Span)>> {
        let mut lexed = cvl_lexer()
            .parse(self.0)
            .map_err(|_| eyre!("lexing failed"))?;
        lexed.retain(|(tok, _)| !matches!(tok, Token::SingleLineComment | Token::MultiLineComment));

        Ok(lexed)
    }

    fn parse(&self, lexed: Vec<(Token, Span)>) -> Result<Vec<(Intermediate, Span)>> {
        let end_span = {
            let len = self.0.chars().count();
            len..len + 1
        };
        let stream = Stream::from_iter(end_span, lexed.into_iter());
        let (parsing_results, _errors) = cvl_parser().parse_recovery(stream);
        parsing_results.ok_or_else(|| eyre!("parsing failed"))
    }

    pub fn build(self) -> Result<Vec<CvlElement>> {
        let lexed = self.lex().unwrap();
        let parsed = self.parse(lexed).unwrap();
        self.output_cvl_elements(parsed)
    }

    const fn chars_to_trim<'a>(style: Style) -> &'a [char] {
        match style {
            Style::Slashed => &['/'],
            Style::Starred => &['/', '*'],
        }
    }

    //this panics, because a failure is an unrecoverable logic error
    fn slice(&self, s: impl Into<Span>) -> &str {
        let span: Span = s.into();
        span.byte_slice(self.0)
            .unwrap_or_else(|| panic!("{:?}: not in source bounds", span))
    }

    fn owned_slice(&self, s: impl Into<Span>) -> String {
        self.slice(s).to_owned()
    }

    fn tag_from_content(content: &str) -> Option<TagKind> {
        if content.starts_with('@') {
            let tag_end = content
                .find(|c: char| c.is_ascii_whitespace())
                .unwrap_or(content.len());

            content[..tag_end].try_into().ok()
        } else {
            None
        }
    }

    fn output_cvl_elements(
        &self,
        parsing_results: Vec<(Intermediate, Span)>,
    ) -> Result<Vec<CvlElement>> {
        let src_ref = Arc::from(self.0);

        let mut elements = Vec::new();
        let mut current_doc: Option<Vec<DocumentationTag>> = None;
        let mut current_doc_span: Option<Span> = None;

        for parse_result in parsing_results {
            let Ok((doc_or_ast, span)) = self.process_intermediate(parse_result) else {
                continue;
            };

            match doc_or_ast {
                DocOrAst::Ast(ast @ Ast::FreeFormComment { .. }) => {
                    // assert!(current_doc.is_none(), "documentation followed by freeform");
                    elements.push(CvlElement {
                        doc: Vec::new(),
                        ast,
                        element_span: span,
                        doc_span: None,
                        src: Arc::clone(&src_ref),
                    });
                }
                DocOrAst::Ast(ast) => {
                    let (doc, doc_span) = match (current_doc.take(), current_doc_span.take()) {
                        (Some(doc), Some(doc_span)) => (doc, Some(doc_span)),
                        (None, None) => (Vec::new(), None),
                        (Some(_), None) => panic!("got doc without doc_span"),
                        (None, Some(_)) => panic!("got doc_span without doc"),
                    };

                    elements.push(CvlElement {
                        doc,
                        ast,
                        element_span: span,
                        doc_span,
                        src: Arc::clone(&src_ref),
                    });
                }
                DocOrAst::Doc(doc) => {
                    // assert!(
                    //     current_doc.is_none(),
                    //     "documentation followed by documentation"
                    // );
                    current_doc = Some(doc);
                    current_doc_span = Some(span);
                    continue;
                }
            }
        }

        Ok(elements)
    }

    fn process_intermediate(
        &self,
        (intermediate, span): (Intermediate, Span),
    ) -> Result<(DocOrAst, Span)> {
        let process_result = match intermediate {
            Intermediate::FreeFormComment(style, span) => {
                let input = self.slice(span.clone());
                let text = ContentLines::new(input, span, Builder::chars_to_trim(style))
                    .map(|(ter_line, _span)| ter_line)
                    .collect();

                let ast = Ast::FreeFormComment { text };
                DocOrAst::Ast(ast)
            }
            Intermediate::Documentation(style, span) => {
                let input = self.slice(span.clone());
                let body = ContentLines::new(input, span.clone(), Builder::chars_to_trim(style));

                let doc = DocumentationTag::from_spanned_iter(body, span);
                DocOrAst::Doc(doc)
            }
            Intermediate::Methods(block) => {
                let block = self.trimmed_block_slice(block).to_string();

                let ast = Ast::Methods { block };
                DocOrAst::Ast(ast)
            }
            Intermediate::Function {
                name,
                params,
                returns,
                block,
            } => {
                let block = self.trimmed_block_slice(block).to_string();
                let ast = Ast::Function {
                    name,
                    params,
                    returns,
                    block,
                };

                DocOrAst::Ast(ast)
            }
            Intermediate::GhostMapping {
                mapping,
                name,
                axioms,
            } => {
                let axioms = axioms.map(|c| self.owned_slice(c));
                let ast = Ast::GhostMapping {
                    name,
                    mapping,
                    axioms,
                };

                DocOrAst::Ast(ast)
            }
            Intermediate::Ghost {
                name,
                ty_list,
                returns,
                axioms,
            } => {
                let axioms = axioms.map(|c| self.owned_slice(c));
                let ast = Ast::GhostFunction {
                    name,
                    ty_list,
                    returns,
                    axioms,
                };

                DocOrAst::Ast(ast)
            }
            Intermediate::Rule {
                name,
                params,
                filters,
                block,
            } => {
                let block = self.trimmed_block_slice(block).to_string();
                let params = params.unwrap_or_default();
                let filters = filters.map(|c| self.owned_slice(c));

                let ast = Ast::Rule {
                    name,
                    params,
                    filters,
                    block,
                };

                DocOrAst::Ast(ast)
            }
            Intermediate::Definition {
                name,
                params,
                returns,
                definition,
            } => {
                let definition = self.owned_slice(definition);

                let ast = Ast::Definition {
                    name,
                    params,
                    returns,
                    definition,
                };

                DocOrAst::Ast(ast)
            }
            Intermediate::Invariant {
                name,
                params,
                invariant,
                filters,
                proof,
            } => {
                let invariant = self.owned_slice(invariant);
                let filters = filters.map(|c| self.owned_slice(c));
                let proof = proof.map(|c| self.trimmed_block_slice(c).to_string());

                let ast = Ast::Invariant {
                    name,
                    params,
                    invariant,
                    filters,
                    proof,
                };

                DocOrAst::Ast(ast)
            }
            Intermediate::Import(imported) => DocOrAst::Ast(Ast::Import { imported }),
            Intermediate::UseBuiltinRule { name } => DocOrAst::Ast(Ast::UseBuiltinRule { name }),
            Intermediate::UseRule { name, filters } => {
                let filters = filters.map(|c| self.owned_slice(c));
                let ast = Ast::UseRule { name, filters };

                DocOrAst::Ast(ast)
            }
            Intermediate::UseInvariant { name, proof } => {
                let proof = proof.map(|c| self.trimmed_block_slice(c).to_string());
                let ast = Ast::UseInvariant { name, proof };

                DocOrAst::Ast(ast)
            }
            Intermediate::Using {
                contract_name,
                spec_name,
            } => DocOrAst::Ast(Ast::Using {
                contract_name,
                spec_name,
            }),
            Intermediate::ParseError => bail!("parse errors are not parsed"),
            Intermediate::HookSload {
                loaded,
                slot_pattern,
                block,
            } => {
                let slot_pattern = self.owned_slice(slot_pattern).to_string();
                let block = self.trimmed_block_slice(block).to_string();
                let ast = Ast::HookSload {
                    loaded,
                    slot_pattern,
                    block,
                };

                DocOrAst::Ast(ast)
            }
            Intermediate::HookSstore {
                stored,
                old,
                slot_pattern,
                block,
            } => {
                // we expect the old type to be the same as the new type
                let slot_pattern = self.owned_slice(slot_pattern).to_string();
                let block = self.trimmed_block_slice(block).to_string();
                let ast = Ast::HookSstore {
                    stored,
                    old,
                    slot_pattern,
                    block,
                };

                DocOrAst::Ast(ast)
            }
            Intermediate::HookCreate { created, block } => {
                let block = self.trimmed_block_slice(block).to_string();
                let ast = Ast::HookCreate { created, block };

                DocOrAst::Ast(ast)
            }
            Intermediate::HookOpcode {
                opcode,
                params,
                returns,
                block,
            } => {
                let params = params.unwrap_or_default();
                let block = self.trimmed_block_slice(block).to_string();
                let ast = Ast::HookOpcode {
                    opcode,
                    params,
                    returns,
                    block,
                };

                DocOrAst::Ast(ast)
            }
        };

        Ok((process_result, span))
    }

    fn trimmed_block_slice(&self, s: impl Into<Span>) -> &str {
        let slice = self.slice(s);
        let slice = slice.strip_prefix('{').unwrap_or(slice);
        let slice = slice.strip_suffix('}').unwrap_or(slice);
        slice.trim()
    }
}

// preserves newlines, strips prefixes, updates span for each line
pub struct ContentLines<'src, 'trim> {
    input: &'src str,
    span: Span,
    chars_to_trim: &'trim [char],
}

impl<'src, 'trim> ContentLines<'src, 'trim> {
    pub fn new(
        input: &'src str,
        span: Span,
        chars_to_trim: &'trim [char],
    ) -> ContentLines<'src, 'trim> {
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
            //potentially skip first and last lines for multiline starred comments
            let trimmed = line.trim();
            if trimmed == "/**" || trimmed == "*/" {
                return self.next();
            }
        }

        let mut terminated = TerminatedStr::from(line);

        let should_trim = |ch| self.chars_to_trim.contains(&ch) || ch.is_ascii_whitespace();
        terminated.content = terminated.content.trim_matches(should_trim);

        Some((terminated, span_of_line))
    }
}
