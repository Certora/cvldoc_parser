use cvldoc_parser_core::util::Span;
use cvldoc_parser_core::{Ast, CvlElement, DocumentationTag};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pythonize::pythonize;
use serde_json::{Map, Value};

#[pyclass(name = "CvlElement", frozen)]
pub struct CvlElementPy {
    #[pyo3(get)]
    pub doc: Vec<DocumentationTagPy>,
    #[pyo3(get)]
    pub ast: AstPy,
    inner: CvlElement,
}

#[pymethods]
impl CvlElementPy {
    pub fn span(&self) -> SpanPy {
        self.inner.span().into()
    }

    pub fn raw(&self) -> &str {
        self.inner.raw()
    }

    pub fn element_name(&self) -> Option<&str> {
        self.inner.ast.name()
    }

    pub fn element_returns(&self) -> Option<&str> {
        self.inner.ast.returns()
    }

    pub fn element_params(&self) -> Option<Vec<(String, String)>> {
        if let Some(params) = self.inner.ast.params() {
            let params = params
                .into_iter()
                .cloned()
                .map(|param| (param.ty, param.name))
                .collect();
            Some(params)
        } else {
            None
        }
    }

    fn __repr__(&self, py: Python) -> PyResult<String> {
        let ast = self.ast.__repr__(py)?;
        let doc = repr_of_list(&self.doc, |doc_tag| doc_tag.__repr__());

        let repr = format!("CvlElement({doc}, {ast})");
        Ok(repr)
    }
}

impl CvlElementPy {
    pub fn new(py: Python, inner: CvlElement) -> PyResult<CvlElementPy> {
        let serialized = CvlElementPy::serialize_ast(&inner.ast)
            .ok_or_else(|| PyValueError::new_err("ast serialization failed"))?;

        let data = pythonize(py, &serialized)?;
        let kind = AstKindPy::from(&inner.ast);
        let ast = AstPy { kind, data };

        let doc = inner.doc.iter().map(DocumentationTagPy::from).collect();

        Ok(CvlElementPy { doc, ast, inner })
    }

    fn serialize_ast(ast: &Ast) -> Option<Map<String, Value>> {
        let Ok(Value::Object(mut map)) = serde_json::to_value(ast) else {
            return None;
        };

        const ENUM_VARIANT_TAG: &str = "type";
        map.remove(ENUM_VARIANT_TAG).expect("internally tagged");

        Some(map)
    }
}

#[pyclass(name = "Ast", get_all, frozen)]
#[derive(Clone)]
pub struct AstPy {
    kind: AstKindPy,
    data: PyObject,
}

#[pyclass(name = "AstKind", frozen)]
#[derive(Debug, Clone)]
pub enum AstKindPy {
    FreeFormComment,
    Rule,
    Invariant,
    Function,
    Definition,
    GhostFunction,
    GhostMapping,
    Methods,
    Import,
    Using,
    UseRule,
    UseBuiltinRule,
    UseInvariant,
    HookSload,
    HookSstore,
    HookCreate,
    HookOpcode,
}

#[pymethods]
impl AstKindPy {
    pub fn __str__(&self) -> &str {
        match self {
            AstKindPy::FreeFormComment => "freeform comment",
            AstKindPy::Rule => "rule",
            AstKindPy::Invariant => "invariant",
            AstKindPy::Function => "function",
            AstKindPy::Definition => "definition",
            AstKindPy::GhostFunction | AstKindPy::GhostMapping => "ghost",
            AstKindPy::Methods => "methods",
            AstKindPy::Import => "import",
            AstKindPy::Using => "using",
            AstKindPy::UseRule | AstKindPy::UseBuiltinRule | AstKindPy::UseInvariant => "use",
            AstKindPy::HookSload
            | AstKindPy::HookSstore
            | AstKindPy::HookCreate
            | AstKindPy::HookOpcode => "hook",
        }
    }
}

impl From<&Ast> for AstKindPy {
    fn from(ast: &Ast) -> Self {
        match ast {
            Ast::FreeFormComment { .. } => AstKindPy::FreeFormComment,
            Ast::Rule { .. } => AstKindPy::Rule,
            Ast::Invariant { .. } => AstKindPy::Invariant,
            Ast::Function { .. } => AstKindPy::Function,
            Ast::Definition { .. } => AstKindPy::Definition,
            Ast::GhostFunction { .. } => AstKindPy::GhostFunction,
            Ast::GhostMapping { .. } => AstKindPy::GhostMapping,
            Ast::Methods { .. } => AstKindPy::Methods,
            Ast::Import { .. } => AstKindPy::Import,
            Ast::Using { .. } => AstKindPy::Using,
            Ast::UseRule { .. } => AstKindPy::UseRule,
            Ast::UseBuiltinRule { .. } => AstKindPy::UseBuiltinRule,
            Ast::UseInvariant { .. } => AstKindPy::UseInvariant,
            Ast::HookSload { .. } => AstKindPy::HookSload,
            Ast::HookSstore { .. } => AstKindPy::HookSstore,
            Ast::HookCreate { .. } => AstKindPy::HookCreate,
            Ast::HookOpcode { .. } => AstKindPy::HookOpcode,
        }
    }
}

#[pymethods]
impl AstPy {
    fn __repr__(&self, py: Python) -> PyResult<String> {
        let kind = &self.kind;
        let data = self.data.as_ref(py).repr()?;
        let repr = format!("{kind:?}({data})");
        Ok(repr)
    }
}

#[pyclass(name = "Span", get_all, frozen)]
#[derive(Debug, Clone)]
pub struct SpanPy {
    pub start: usize,
    pub end: usize,
}

impl From<Span> for SpanPy {
    fn from(span: Span) -> SpanPy {
        let Span { start, end } = span;
        SpanPy { start, end }
    }
}

#[pymethods]
impl SpanPy {
    fn __repr__(&self) -> String {
        let SpanPy { start, end } = self;
        format!("Span({start}, {end})")
    }
}

#[pyclass(name = "DocumentationTag", get_all, frozen)]
#[derive(Debug, Clone)]
pub struct DocumentationTagPy {
    pub kind: String,
    pub description: String,
}

impl From<&DocumentationTag> for DocumentationTagPy {
    fn from(value: &DocumentationTag) -> Self {
        DocumentationTagPy {
            kind: value.kind.to_string(),
            description: value.description.to_owned(),
        }
    }
}

#[pymethods]
impl DocumentationTagPy {
    pub fn param_name_and_description(&self) -> Option<(&str, &str)> {
        if self.kind == "param" {
            let description = self.description.trim();
            let (param_name, param_description) =
                description.split_once(|c| char::is_ascii_whitespace(&c))?;
            let param_description = param_description.trim_start();

            Some((param_name, param_description))
        } else {
            None
        }
    }

    fn __repr__(&self) -> String {
        let DocumentationTagPy { kind, description } = self;
        format!("DocumentationTag({kind}, {description})")
    }
}

fn repr_of_list<T>(elements: &[T], fmt: impl Fn(&T) -> String) -> String {
    let mut buf = String::new();

    buf.push('[');

    if let Some((last, all_except_last)) = elements.split_last() {
        for elem in all_except_last {
            buf.push_str(fmt(elem).as_str());
            buf.push_str(", ");
        }

        buf.push_str(fmt(last).as_str());
    }

    buf.push(']');

    buf
}
