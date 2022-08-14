use super::{
    AssociatedElement, Diagnostic, Documentation, DocumentationTag, FreeForm, Position, Range,
    Severity,
};
use lsp_types::{
    Diagnostic as DiagnosticR, DiagnosticSeverity as DiagnosticSeverityR, Position as PositionR,
    Range as RangeR,
};
use natspec_parser::{
    AssociatedElement as AssociatedElementR, DocumentationTag as DocumentationTagR,
    NatSpec as NatSpecR,
};
use pyo3::{IntoPy, Py, PyAny, Python};

impl From<DocumentationTagR> for DocumentationTag {
    fn from(
        DocumentationTagR {
            kind,
            description,
            range,
        }: DocumentationTagR,
    ) -> Self {
        DocumentationTag {
            kind: kind.to_string(),
            description,
            range: range.map(Into::into),
        }
    }
}

impl From<DocumentationTag> for DocumentationTagR {
    fn from(
        DocumentationTag {
            kind,
            description,
            range,
        }: DocumentationTag,
    ) -> Self {
        DocumentationTagR {
            kind: kind.as_str().into(),
            description,
            range: range.map(Into::into),
        }
    }
}

impl Documentation {
    pub fn from_rust_repr_components(
        tags: Vec<DocumentationTagR>,
        associated: Option<AssociatedElementR>,
        range: RangeR,
    ) -> Documentation {
        let tags_wrapper = tags.into_iter().map(Into::into).collect();
        Documentation {
            tags: tags_wrapper,
            associated: associated.map(AssociatedElement),
            range: range.into(),
        }
    }
}

impl From<Documentation> for NatSpecR {
    fn from(
        Documentation {
            tags,
            associated,
            range,
        }: Documentation,
    ) -> Self {
        NatSpecR::Documentation {
            tags: tags.into_iter().map(Into::into).collect(),
            associated: associated.map(|wrapper| wrapper.0),
            range: range.into(),
        }
    }
}

impl FreeForm {
    pub fn with_block(header: String, block: String, range: RangeR) -> FreeForm {
        FreeForm {
            header,
            block: Some(block),
            range: range.into(),
        }
    }

    pub fn without_block(header: String, range: RangeR) -> FreeForm {
        FreeForm {
            header,
            block: None,
            range: range.into(),
        }
    }
}

impl From<FreeForm> for NatSpecR {
    fn from(
        FreeForm {
            header,
            block,
            range,
        }: FreeForm,
    ) -> Self {
        let range = range.into();
        match block {
            Some(block) => NatSpecR::MultiLineFreeForm {
                header,
                block,
                range,
            },
            None => NatSpecR::SingleLineFreeForm { header, range },
        }
    }
}

pub fn natspec_to_py_object(natspec: NatSpecR, py: Python<'_>) -> Py<PyAny> {
    match natspec {
        NatSpecR::SingleLineFreeForm { header, range } => {
            FreeForm::without_block(header, range).into_py(py)
        }
        NatSpecR::MultiLineFreeForm {
            header,
            block,
            range,
        } => FreeForm::with_block(header, block, range).into_py(py),
        NatSpecR::Documentation {
            tags,
            associated,
            range,
        } => Documentation::from_rust_repr_components(tags, associated, range).into_py(py),
    }
}

impl From<RangeR> for Range {
    fn from(RangeR { start, end }: RangeR) -> Self {
        Range {
            start: start.into(),
            end: end.into(),
        }
    }
}

impl From<Range> for RangeR {
    fn from(Range { start, end }: Range) -> Self {
        RangeR {
            start: start.into(),
            end: end.into(),
        }
    }
}

impl From<PositionR> for Position {
    fn from(PositionR { line, character }: PositionR) -> Self {
        Position { line, character }
    }
}

impl From<Position> for PositionR {
    fn from(Position { line, character }: Position) -> Self {
        PositionR { line, character }
    }
}

impl From<DiagnosticSeverityR> for Severity {
    fn from(severity: DiagnosticSeverityR) -> Self {
        match severity {
            DiagnosticSeverityR::ERROR => Severity::Error,
            _ => Severity::Warning,
        }
    }
}

impl From<DiagnosticR> for Diagnostic {
    fn from(
        DiagnosticR {
            range,
            severity,
            message,
            ..
        }: DiagnosticR,
    ) -> Self {
        let severity = severity.map(Into::into).unwrap_or(Severity::Warning);
        Diagnostic {
            range: range.into(),
            description: message,
            severity,
        }
    }
}
