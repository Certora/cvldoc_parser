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
            Ast::GhostFunction { .. } | Ast::GhostMapping { .. } => &[Notice, Dev, Param, Return],
            Ast::Methods { .. } => &[Notice, Dev],
            Ast::FreeFormComment { .. } => &[Notice, Dev],
            Ast::Import { .. }
            | Ast::Using { .. }
            | Ast::UseRule { .. }
            | Ast::UseBuiltinRule { .. }
            | Ast::UseInvariant { .. }
            | Ast::HookSload { .. }
            | Ast::HookSstore { .. }
            | Ast::HookCreate { .. }
            | Ast::HookOpcode { .. } => &[Dev],
        }
    }

    fn supports(&self, tag: &TagKind) -> bool {
        self.supported_tags().contains(tag)
    }

    fn defines_param(&self, param_name: &str) -> bool {
        if let Some(params) = self.params() {
            params.iter().any(|param| param.name == param_name)
        } else {
            false
        }
    }
}

enum DiagSpan<'a> {
    #[allow(unused)]
    EntireDoc,
    SingleTag(&'a DocumentationTag),
}

impl CvlElement {
    pub fn enumerate_diagnostics(&self, converter: RangeConverter) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        let mut add = |message, diag_span, severity| {
            let span = match diag_span {
                DiagSpan::EntireDoc => self.element_span.clone(),
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

        // disabled for now. this diagnostic is overly broad.
        // if self.doc.iter().all(|tag| tag.kind != TagKind::Notice) {
        //     //Any applicable item is missing a notice
        //     let message = "associated element is undocumented".to_string();
        //     add(message, DiagSpan::EntireDoc, WARNING);
        // }

        let tags_with_params = self.doc.iter().filter_map(|tag| {
            let param = tag.param_name()?;
            Some((tag, param))
        });

        for (i, (tag, param)) in tags_with_params.enumerate() {
            if !self.ast.defines_param(param) {
                //A @param is provided for a non-existent parameter
                let message = format!("no such parameter: {param}");
                add(message, DiagSpan::SingleTag(tag), DiagnosticSeverity::ERROR);
            } else if self.doc[..i]
                .iter()
                .any(|tag| tag.param_name() == Some(param))
            {
                //Each parameter must be documented at most once
                let message = "parameter is already documented".to_string();
                add(message, DiagSpan::SingleTag(tag), DiagnosticSeverity::ERROR);
            }
        }

        for tag in &self.doc {
            if !self.ast.supports(&tag.kind) {
                let message = format!("this tag is unsupported for {} blocks", self.ast);
                add(message, DiagSpan::SingleTag(tag), DiagnosticSeverity::ERROR);
            }
        }

        diagnostics
    }
}
