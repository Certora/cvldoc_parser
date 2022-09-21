use super::{
    AssociatedElement, Diagnostic, Documentation, DocumentationTag, FreeForm, Position, Range,
    Severity,
};
use lsp_types::{
    Diagnostic as DiagnosticR, DiagnosticSeverity as DiagnosticSeverityR, Position as PositionR,
    Range as RangeR,
};
use cvldoc_parser::{
    AssociatedElement as AssociatedElementR, CvlDoc as CvlDocR, DocData as DocDataR,
    DocumentationTag as DocumentationTagR,
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
        raw: String,
        tags: Vec<DocumentationTagR>,
        associated: Option<AssociatedElementR>,
        range: RangeR,
    ) -> Documentation {
        let tags_wrapper = tags.into_iter().map(Into::into).collect();
        Documentation {
            raw,
            tags: tags_wrapper,
            associated: associated.map(AssociatedElement),
            range: range.into(),
        }
    }
}

impl From<Documentation> for CvlDocR {
    fn from(
        Documentation {
            raw,
            tags,
            associated,
            range,
        }: Documentation,
    ) -> Self {
        CvlDocR {
            raw,
            range: range.into(),
            data: DocDataR::Documentation {
                tags: tags.into_iter().map(Into::into).collect(),
                associated: associated.map(|wrapper| wrapper.0),
            },
        }
    }
}

impl From<FreeForm> for CvlDocR {
    fn from(FreeForm { raw, range, text }: FreeForm) -> Self {
        CvlDocR {
            raw,
            range: range.into(),
            data: DocDataR::FreeForm(text)
        }
    }
}

pub fn cvldoc_to_py_object(doc: CvlDocR, py: Python<'_>) -> Py<PyAny> {
    match doc.data {
        DocDataR::FreeForm(text) => {
            let range = doc.range.into();
            FreeForm { raw: doc.raw, range, text }.into_py(py)
        }
        DocDataR::Documentation { tags, associated } => {
            Documentation::from_rust_repr_components(doc.raw, tags, associated, doc.range).into_py(py)
        }
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
