use super::{AstPy, CvlElementPy, DocumentationTagPy, SpanPy};
use cvldoc_parser_core::{util::Span, CvlElement, DocumentationTag};

impl From<DocumentationTag> for DocumentationTagPy {
    fn from(doc_tag: DocumentationTag) -> Self {
        DocumentationTagPy {
            kind: doc_tag.kind.to_string(),
            description: doc_tag.description,
            span: doc_tag.span.into(),
        }
    }
}

impl From<Span> for SpanPy {
    fn from(span: Span) -> Self {
        SpanPy {
            start: span.start,
            end: span.end,
        }
    }
}

impl From<CvlElement> for CvlElementPy {
    fn from(element: CvlElement) -> Self {
        CvlElementPy {
            doc: element.doc.into_iter().flatten().map(Into::into).collect(),
            ast: AstPy(element.ast),
            element_span: element.element_span,
            doc_span: element.doc_span,
            src: element.src,
        }
    }
}

// use super::{Definition, FreeFormComment, Function, Ghost, GhostMapping, Invariant, Methods, Rule};
// use cvldoc_parser_core::Ast;
// use pyo3::prelude::*;

// impl IntoPy<PyObject> for AstPy {
//     fn into_py(self, py: Python) -> PyObject {
//         match self.0 {
//             Ast::FreeFormComment { text } => FreeFormComment { text },
//             Ast::Rule {
//                 name,
//                 params,
//                 filters,
//                 block,
//             } => Rule {
//                 name,
//                 params,
//                 filters,
//                 block,
//             },
//             Ast::Invariant {
//                 name,
//                 params,
//                 invariant,
//                 filters,
//                 proof,
//             } => Invariant {
//                 name,
//                 params,
//                 invariant,
//                 filters,
//                 proof,
//             },
//             Ast::Function {
//                 name,
//                 params,
//                 returns,
//                 block,
//             } => Function {
//                 name,
//                 params,
//                 returns,
//                 block,
//             },
//             Ast::Definition {
//                 name,
//                 params,
//                 returns,
//                 definition,
//             } => Definition {
//                 name,
//                 params,
//                 returns,
//                 definition,
//             },
//             Ast::Ghost {
//                 name,
//                 ty_list,
//                 returns,
//                 axioms,
//             } => Ghost {
//                 name,
//                 ty_list,
//                 returns,
//                 axioms,
//             },
//             Ast::GhostMapping {
//                 name,
//                 mapping,
//                 axioms,
//             } => GhostMapping {
//                 name,
//                 mapping,
//                 axioms,
//             },
//             Ast::Methods { block } => Methods { block },
//         }
//     }
// }
