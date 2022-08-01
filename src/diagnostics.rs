use crate::{AssociatedElement, DeclarationKind, DocumentationTag, NatSpec, Tag};
use lsp_types::{Diagnostic, Range};

impl Tag {
    fn supported_declarations(&self) -> &[DeclarationKind] {
        use DeclarationKind::*;
        match self {
            Tag::Title => &[Rule, Invariant],
            Tag::Notice => &[Rule, Invariant, Function, Definition, Ghost, Methods],
            Tag::Dev => &[Rule, Invariant, Function, Definition, Ghost, Methods],
            Tag::Param => &[Rule, Invariant, Function, Definition, Ghost],
            Tag::Return => &[Function, Definition, Ghost],
            Tag::Formula => &[Rule],
            _ => &[],
        }
    }

    fn supports(&self, declaration_kind: DeclarationKind) -> bool {
        self.supported_declarations().contains(&declaration_kind)
    }
}

impl AssociatedElement {
    fn defines_param(&self, param: &str) -> bool {
        self.params.iter().any(|(_, name)| name == param)
    }
}

impl DocumentationTag {}

impl NatSpec {
    pub fn enumerate_diagnostics(&self) -> Vec<Diagnostic> {
        let mut warnings = Vec::new();

        if let NatSpec::Documentation {
            tags,
            associated,
            range: natspec_range,
        } = self
        {
            let mut add = |message: String, range: Option<Range>| {
                let range = range.unwrap_or(*natspec_range);
                let diag = Diagnostic {
                    range,
                    message,
                    ..Default::default()
                };
                warnings.push(diag);
            };

            if let Some(associated) = associated {
                if tags.iter().all(|tag| tag.kind != Tag::Notice) {
                    //Any applicable item is missing a notice
                    add("associated element is undocumented".into(), None);
                }

                let tags_with_params = tags.iter().filter_map(|tag| {
                    let param = tag.param_name()?;
                    Some((tag, param))
                });

                for (i, (tag, param)) in tags_with_params.enumerate() {
                    if !associated.defines_param(param) {
                        //A @param is provided for a non-existent parameter
                        add(format!("no such parameter: {param}"), tag.range);
                    } else if tags[..i].iter().any(|tag| tag.param_name() == Some(param)) {
                        //Each parameter must be documented at most once
                        add("parameter is already documented".into(), tag.range);
                    }
                }

                let decl_kind = associated.kind;
                for tag in tags {
                    if !tag.kind.supports(decl_kind) {
                        let error_desc = format!("this tag is unsupported for {decl_kind} blocks");
                        add(error_desc, tag.range);
                    }
                }
            } else {
                add("no associated element for documentation block".into(), None);
            }

            for tag in tags {
                if let Some(unexpected_tag) = tag.kind.unexpected_tag() {
                    //Unrecognized tags appear anywhere
                    let error_desc = format!("@{unexpected_tag} is unrecognized");
                    add(error_desc, tag.range);
                }
            }
        }

        warnings
    }
}
