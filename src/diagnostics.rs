use crate::{AssociatedElement, CvlDoc, Tag, DocData};
use lsp_types::{Diagnostic, DiagnosticSeverity, Range};

impl AssociatedElement {
    fn supported_tags(&self) -> &[Tag] {
        use Tag::*;
        match self {
            AssociatedElement::Rule { .. } => &[Title, Notice, Dev, Param, Formula],
            AssociatedElement::Invariant { .. } => &[Title, Notice, Dev, Param],
            AssociatedElement::Function { .. } => &[Notice, Dev, Param, Return],
            AssociatedElement::Definition { .. } => &[Notice, Dev, Param, Return],
            AssociatedElement::Ghost { .. } | AssociatedElement::GhostMapping { .. } => {
                &[Notice, Dev, Param, Return]
            }
            AssociatedElement::Methods { .. } => &[Notice, Dev],
        }
    }

    fn supports(&self, tag: &Tag) -> bool {
        self.supported_tags().contains(tag)
    }

    fn defines_param(&self, param: &str) -> bool {
        match self {
            AssociatedElement::Rule { params, .. }
            | AssociatedElement::Invariant { params, .. }
            | AssociatedElement::Function { params, .. }
            | AssociatedElement::Definition { params, .. } => params
                .iter()
                .filter_map(|(_, name)| name.as_ref())
                .any(|name| name == param),
            _ => false,
        }
    }
}

impl CvlDoc {
    pub fn enumerate_diagnostics(&self) -> Vec<Diagnostic> {
        let mut warnings = Vec::new();

        if let DocData::Documentation { tags, associated } = &self.data {
            const WARNING: DiagnosticSeverity = DiagnosticSeverity::WARNING;
            const ERROR: DiagnosticSeverity = DiagnosticSeverity::ERROR;

            let mut add = |message: String, diag_range: Option<Range>, severity: DiagnosticSeverity| {
                let diag = Diagnostic {
                    range: diag_range.unwrap_or(self.range),
                    severity: Some(severity),
                    message,
                    ..Default::default()
                };
                warnings.push(diag);
            };

            if let Some(associated) = associated {
                if tags.iter().all(|tag| tag.kind != Tag::Notice) {
                    //Any applicable item is missing a notice
                    add("associated element is undocumented".into(), None, WARNING);
                }

                let tags_with_params = tags.iter().filter_map(|tag| {
                    let param = tag.param_name()?;
                    Some((tag, param))
                });

                for (i, (tag, param)) in tags_with_params.enumerate() {
                    if !associated.defines_param(param) {
                        //A @param is provided for a non-existent parameter
                        add(format!("no such parameter: {param}"), tag.range, ERROR);
                    } else if tags[..i].iter().any(|tag| tag.param_name() == Some(param)) {
                        //Each parameter must be documented at most once
                        add("parameter is already documented".into(), tag.range, ERROR);
                    }
                }

                for tag in tags {
                    if !associated.supports(&tag.kind) {
                        let error_desc = format!("this tag is unsupported for {associated} blocks");
                        add(error_desc, tag.range, ERROR);
                    }
                }
            } else {
                let error_desc = "no associated element for CVLDoc documentation block".into();
                add(error_desc, None, ERROR);
            }

            for tag in tags {
                if let Some(unexpected_tag) = tag.kind.unexpected_tag() {
                    //Unrecognized tags appear anywhere
                    let error_desc = format!("@{unexpected_tag} is unrecognized");
                    add(error_desc, tag.range, WARNING);
                }
            }
        }

        warnings
    }
}
