use crate::{AssociatedElement, Documentation, DocumentationTag, FreeForm};
use natspec_parser::{
    AssociatedElement as AssociatedElementR, DocumentationTag as DocumentationTagR,
    NatSpec as NatSpecR,
};
use pyo3::{IntoPy, Py, PyAny, Python};

impl From<DocumentationTagR> for DocumentationTag {
    fn from(
        DocumentationTagR {
            kind, description, ..
        }: DocumentationTagR,
    ) -> Self {
        DocumentationTag {
            kind: kind.to_string(),
            description,
        }
    }
}

impl From<AssociatedElementR> for AssociatedElement {
    fn from(AssociatedElementR { kind, name, params, ..}: AssociatedElementR) -> Self {
        AssociatedElement {
            kind: kind.to_string(),
            name,
            params,
        }
    }
}

impl Documentation {
    pub fn from_rust_repr_components(
        tags: Vec<DocumentationTagR>,
        associated: Option<AssociatedElementR>,
    ) -> Documentation {
        let tags_wrapper = tags.into_iter().map(Into::into).collect();
        Documentation {
            tags: tags_wrapper,
            associated: associated.map(Into::into),
        }
    }
}

impl FreeForm {
    pub fn with_block(header: String, block: String) -> FreeForm {
        FreeForm {
            header,
            block: Some(block),
        }
    }

    pub fn without_block(header: String) -> FreeForm {
        FreeForm {
            header,
            block: None,
        }
    }
}

pub fn natspec_to_py_object(natspec: NatSpecR, py: Python<'_>) -> Py<PyAny> {
    match natspec {
        NatSpecR::SingleLineFreeForm { header } => FreeForm::without_block(header).into_py(py),
        NatSpecR::MultiLineFreeForm { header, block } => {
            FreeForm::with_block(header, block).into_py(py)
        }
        NatSpecR::Documentation { tags, associated } => {
            Documentation::from_rust_repr_components(tags, associated).into_py(py)
        }
    }
}
