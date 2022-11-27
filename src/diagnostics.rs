use crate::util::RangeConverter;
use crate::{Ast, CvlElement, DocumentationTag, TagKind};
use lsp_types::{Diagnostic, DiagnosticSeverity};

impl Ast {
    fn supported_tags(&self) -> &[TagKind] {
        use TagKind::*;
        match self {
            Ast::Rule { .. } => &[Title, Notice, Dev, Param, Formula],
            Ast::Invariant { .. } => &[Title, Notice, Dev, Param],
            Ast::Function { .. } => &[Notice, Dev, Param, Return],
            Ast::Definition { .. } => &[Notice, Dev, Param, Return],
            Ast::Ghost { .. } | Ast::GhostMapping { .. } => &[Notice, Dev, Param, Return],
            Ast::Methods { .. } => &[Notice, Dev],
            Ast::FreeFormComment(..) => todo!(),
        }
    }

    fn supports(&self, tag: &TagKind) -> bool {
        self.supported_tags().contains(tag)
    }

    fn defines_param(&self, param: &str) -> bool {
        self.params()
            .map(|params| {
                params
                    .iter()
                    .filter_map(|(_ty, name)| name.as_ref())
                    .any(|name| name == param)
            })
            .unwrap_or(false)
    }
}

const WARNING: DiagnosticSeverity = DiagnosticSeverity::WARNING;
const ERROR: DiagnosticSeverity = DiagnosticSeverity::ERROR;

enum DiagSpan<'a> {
    EntireDoc,
    SingleTag(&'a DocumentationTag),
}

impl CvlElement {
    pub fn enumerate_diagnostics(&self, converter: RangeConverter) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if !self.doc.is_empty() {
            let mut add = |message, diag_span, severity| {
                let span = match diag_span {
                    DiagSpan::EntireDoc => self.span.clone(),
                    DiagSpan::SingleTag(tag) => tag.span.clone(),
                };

                let diag = Diagnostic {
                    range: converter.to_range(span),
                    severity: Some(severity),
                    message,
                    ..Default::default()
                };
                diagnostics.push(diag);
            };

            if self.doc.iter().all(|tag| tag.kind != TagKind::Notice) {
                //Any applicable item is missing a notice
                let message = "associated element is undocumented".to_string();
                add(message, DiagSpan::EntireDoc, WARNING);
            }

            let tags_with_params = self.doc.iter().filter_map(|tag| {
                let param = tag.param_name()?;
                Some((tag, param))
            });

            for (i, (tag, param)) in tags_with_params.enumerate() {
                if !self.ast.defines_param(param) {
                    //A @param is provided for a non-existent parameter
                    let message = format!("no such parameter: {param}");
                    add(message, DiagSpan::SingleTag(tag), ERROR);
                } else if self.doc[..i]
                    .iter()
                    .any(|tag| tag.param_name() == Some(param))
                {
                    //Each parameter must be documented at most once
                    let message = "parameter is already documented".to_string();
                    add(message, DiagSpan::SingleTag(tag), ERROR);
                }
            }

            for tag in &self.doc {
                if !self.ast.supports(&tag.kind) {
                    let message = format!("this tag is unsupported for {} blocks", self.ast);
                    add(message, DiagSpan::SingleTag(tag), ERROR);
                }
            }

            for tag in &self.doc {
                if let TagKind::Unexpected(unexpected_tag) = &tag.kind {
                    //Unrecognized tags appear anywhere
                    let message = format!("@{unexpected_tag} is unrecognized");
                    add(message, DiagSpan::SingleTag(tag), WARNING);
                }
            }
        }

        diagnostics
    }
}
