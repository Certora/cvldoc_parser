use crate::{NatSpec, Tag};
use lsp_types::{Diagnostic, Range};

impl NatSpec {
    pub fn enumerate_diagnostics(&self, natspec_range: Range) -> Vec<Diagnostic> {
        let mut warnings = Vec::new();
        let mut add = |message: String, range: &Range| {
            let diag = Diagnostic {
                range: range.to_owned(),
                message,
                ..Default::default()
            };
            warnings.push(diag);
        };

        if let NatSpec::Documentation { tags, associated } = self {
            if let Some(associated) = associated {
                if tags
                    .iter()
                    .map(|tag| &tag.kind)
                    .all(|kind| *kind != Tag::Notice)
                {
                    //Any applicable item is missing a notice
                    add("associated element is undocumented".into(), &natspec_range);
                }

                for tag in tags {
                    if let Some(param_name) = tag.param_name() {
                        if associated.params.iter().all(|(_, name)| name != param_name) {
                            //A @param is provided for a non-existent parameter
                            let error_desc = format!("no such parameter: {param_name}");
                            add(error_desc, tag.range.as_ref().unwrap());
                        }
                    }
                }
            } else {
                add(
                    "no associated element for documentation block".into(),
                    &natspec_range,
                );
            }

            for tag in tags {
                if let Some(unexpected_tag) = tag.kind.unexpected_tag() {
                    //Unrecognized tags appear anywhere
                    let error_desc = format!("@{unexpected_tag} is unrecognized");
                    add(error_desc, tag.range.as_ref().unwrap());
                }
            }
        }

        warnings
    }
}
