use std::{
    fmt::{Display, Formatter},
    ops::Not,
    string::ParseError,
};

use itertools::Itertools;

use crate::{
    util::span_to_range::{RangeConverter, Span},
    Param,
};

use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Token {
    Ghost,
    Definition,
    Rule,
    Invariant,
    Methods,
    Function,
    Mapping,
    Returns,
    Filtered,
    CvlDocSlashed,
    CvlDocStarred,
    FreeFormSlashed,
    FreeFormStarred,
    RoundOpen,
    RoundClose,
    SquareOpen,
    SquareClose,
    CurlyOpen,
    CurlyClose,
    Ident(String),
    Number(String),
    Other(String),
    Dot,
    SingleLineComment,
    MultiLineComment,
    Comma,
    Semicolon,
    Equals,
    Arrow,
    Axiom,
    Using,
    Hook,
    Preserved,
}

const SYNC_TOKENS: [Token; 10] = [
    Token::FreeFormSlashed,
    Token::FreeFormStarred,
    Token::CvlDocSlashed,
    Token::CvlDocStarred,
    Token::Ghost,
    Token::Definition,
    Token::Rule,
    Token::Invariant,
    Token::Methods,
    Token::Function,
];

const INVARIANT_STOP_TOKENS: [Token; 14] = [
    Token::FreeFormSlashed,
    Token::FreeFormStarred,
    Token::CvlDocSlashed,
    Token::CvlDocStarred,
    Token::Ghost,
    Token::Definition,
    Token::Rule,
    Token::Invariant,
    Token::Methods,
    Token::Function,
    Token::Hook,
    Token::Axiom,
    Token::Using,
    Token::Hook,
];

impl Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Token::Ghost => "ghost",
            Token::Definition => "definition",
            Token::Rule => "rule",
            Token::Invariant => "invariant",
            Token::Methods => "methods",
            Token::Function => "function",
            Token::Mapping => "mapping",
            Token::Returns => "returns",
            Token::Filtered => "filtered",
            Token::Ident(data) | Token::Other(data) => data.as_str(),
            Token::RoundOpen => "(",
            Token::RoundClose => ")",
            Token::SquareOpen => "[",
            Token::SquareClose => "]",
            Token::CurlyOpen => "{",
            Token::CurlyClose => "}",
            Token::Dot => ".",
            Token::Comma => ",",
            Token::Semicolon => ";",
            Token::Equals => "=",
            Token::Arrow => "=>",
            Token::Number(num) => num.as_str(),
            Token::Axiom => "axiom",
            Token::Using => "using",
            Token::Hook => "hook",
            Token::Preserved => "preserved",
            // Token::SingleLineComment => "SLC",
            // Token::MultiLineComment => "MLC",
            // Token::CvlDocSlashed => panic!("{self:?}"),
            // Token::CvlDocStarred => panic!("{self:?}"),
            // Token::FreeFormSlashed => panic!("{self:?}"),
            // Token::FreeFormStarred => panic!("{self:?}"),
            _ => panic!("{self:?}"),
        };

        write!(f, "{s}")
    }
}

pub fn lexer() -> impl Parser<char, Vec<(Token, Span)>, Error = Simple<char>> {
    let cvldoc_slashed_line = just("///")
        .then_ignore(none_of('/').rewind())
        .then(take_until(newline_or_end()));
    let cvldoc_slashed = cvldoc_slashed_line
        .at_least_once()
        .to(Token::CvlDocSlashed)
        .boxed();
    let cvldoc_starred = just("/**")
        .then_ignore(none_of('*').rewind())
        .then(take_until(just("*/")))
        .to(Token::CvlDocStarred)
        .boxed();
    let freeform_slashed_line = just("////").then(take_until(newline_or_end()));
    let freeform_slashed = freeform_slashed_line
        .at_least_once()
        .to(Token::FreeFormSlashed)
        .boxed();
    let freeform_starred = just("/***")
        .then(take_until(just("*/")))
        .to(Token::FreeFormStarred)
        .boxed();
    let freeform_starred_alternative = {
        //this is verbose and hideous

        let middle_endings = choice((just("*/\r\n"), just("*/\n")));
        let endings = choice((just("/\r\n"), just("/\n"), just("/").then_ignore(end())));

        let header = just("/***")
            .then(just('*').repeated())
            .then(endings.clone());
        let middle = just("/***").then(take_until(middle_endings));

        middle.padded_by(header).to(Token::FreeFormStarred).boxed()
    };

    let sigil = {
        let single_char = select! {
            '(' => Token::RoundOpen,
            ')' => Token::RoundClose,
            '[' => Token::SquareOpen,
            ']' => Token::SquareClose,
            '{' => Token::CurlyOpen,
            '}' => Token::CurlyClose,
            '.' => Token::Dot,
            ',' => Token::Comma,
            ';' => Token::Semicolon,
            '=' => Token::Equals,
        };
        let arrow = just("=>").to(Token::Arrow);

        choice((single_char, arrow)).boxed()
    };

    let single_line_comment = just("//")
        .then(none_of('/'))
        .then(take_until(newline_or_end()))
        .to(Token::SingleLineComment);
    let multi_line_comment = just("/*")
        .then(none_of('*'))
        .then(take_until(just("*/")))
        .to(Token::MultiLineComment);
    let comment = single_line_comment.or(multi_line_comment).boxed();

    let num = {
        static NUMS: [char; 10] = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
        let numeric = filter(|c| NUMS.contains(c)).at_least_once();
        let sign = one_of('-').or_not();
        sign.padded()
            .then(numeric)
            .map(|(sign, numeric)| sign.into_iter().chain(numeric).collect())
            .map(Token::Number)
            .boxed()
    };

    let keyword_or_ident = text::ident()
        .map(|ident: String| match ident.as_str() {
            "ghost" => Token::Ghost,
            "definition" => Token::Definition,
            "rule" => Token::Rule,
            "invariant" => Token::Invariant,
            "methods" => Token::Methods,
            "function" => Token::Function,
            "mapping" => Token::Mapping,
            "returns" => Token::Returns,
            "filtered" => Token::Filtered,
            "axiom" => Token::Axiom,
            "using" => Token::Using,
            "hook" => Token::Hook,
            "preserved" => Token::Preserved,
            _ => Token::Ident(ident),
        })
        .boxed();
    let other = {
        let not_whitespace = |c: &char| c.is_ascii_whitespace().not();
        filter(not_whitespace)
            .at_least_once()
            .collect()
            .map(Token::Other)
            .boxed()
    };

    choice([
        cvldoc_slashed,
        cvldoc_starred,
        freeform_slashed,
        freeform_starred_alternative,
        freeform_starred,
        comment,
        num,
        sigil,
        keyword_or_ident,
        other,
    ])
    .map_with_span(|token, span| (token, span))
    .padded()
    .repeated()
}

#[derive(Debug, Clone)]
pub enum Style {
    Slashed,
    Starred,
}

#[derive(Debug, Clone)]
pub enum Ast {
    FreeFormComment(Style, Span),
    Documentation(Style, Span),
    Methods {
        block: CodeChunk,
    },
    Function {
        name: String,
        params: Vec<(String, Option<String>)>,
        returns: Option<String>,
        block: CodeChunk,
    },
    // Comment,
    ParseError,
    // MappingTy(String),
    // Ident(String),
    GhostMapping {
        mapping: String,
        name: String,
        block: CodeChunk,
    },
    Ghost {
        name: String,
        ty_list: Vec<String>,
        returns: String,
        block: Option<CodeChunk>,
    },
    Rule {
        name: String,
        params: Vec<(String, Option<String>)>,
        filters: Option<CodeChunk>,
        block: CodeChunk,
    },
    Definition {
        name: String,
        params: Vec<(String, Option<String>)>,
        returns: String,
        definition: String,
    },
    InvariantSimplified {
        name: String,
        params: Vec<(String, Option<String>)>,
        spanned_tail: Vec<(Token, Span)>,
    },
    Invariant {
        name: String,
        params: Vec<(String, Option<String>)>,
        invariant: CodeChunk,
        filters: Option<CodeChunk>,
        proof: Option<CodeChunk>,
    },
    // Block(Span),
}

// fn ident() -> impl Parser<Token, String, Error = Simple<Token>> {
//     select! { Token::Ident(ident) => ident }
// }

fn balanced(l: Token, r: Token) -> impl Parser<Token, Vec<Token>, Error = Simple<Token>> + Clone {
    let open = just(l.clone());
    let close = just(r.clone());

    let content = none_of([l, r])
        .at_least_once()
        .boxed();

    recursive(|block| {
        let between = content.or(block).repeated().flatten();

        open.chain(between).chain(close)
    })
}

trait ParserExt<I, O, P> {
    fn at_least_once(self) -> chumsky::combinator::Repeated<P>;
}

impl<I: Clone, O, P: Parser<I, O>> ParserExt<I, O, P> for P {
    fn at_least_once(self) -> chumsky::combinator::Repeated<P> {
        self.repeated().at_least(1)
    }
}

fn balanced_stringified(
    l: Token,
    r: Token,
) -> impl Parser<Token, String, Error = Simple<Token>> + Clone {
    let open = just(l.clone());
    let close = just(r.clone());

    let content = none_of([l, r])
        .at_least_once()
        .map(String::from_iter)
        .boxed();

    recursive(|block| {
        let between = content
            .or(block)
            .repeated()
            .map(|strings| strings.join(" "));

        open.then(between)
            .then(close)
            .map(|((open, content), close)| format!("{open}{content}{close}"))
    })
}

fn mapping_ty() -> impl Parser<Token, String, Error = Simple<Token>> + Clone {
    just(Token::Mapping)
        .ignore_then(balanced_stringified(Token::RoundOpen, Token::RoundClose))
        .map(|content| format!("mapping{content}"))
}

fn ident() -> impl Parser<Token, String, Error = Simple<Token>> + Clone {
    select! { Token::Ident(ident) => ident }
}

fn ty() -> impl Parser<Token, String, Error = Simple<Token>> + Clone {
    let call = ident()
        .then_ignore(just(Token::Dot))
        .then(ident())
        .map(|(lhs, rhs)| format!("{lhs}.{rhs}"));

    let array_ty = {
        let array_subscript = balanced_stringified(Token::SquareOpen, Token::SquareClose)
            .at_least_once()
            .map(String::from_iter);
        let caller = ident().or(call.clone());

        caller
            .then(array_subscript)
            .map(|(caller, subscript)| format!("{caller}{subscript}"))
    };

    choice((array_ty, mapping_ty(), call, ident())).labelled("type")
}

fn param_list() -> impl Parser<Token, Vec<(String, Option<String>)>, Error = Simple<Token>> + Clone
{
    ty().then(ident().or_not())
        .separated_by(just(Token::Comma))
        .delimited_by(just(Token::RoundOpen), just(Token::RoundClose))
        .labelled("param list")
}

fn code_block() -> impl Parser<Token, CodeChunk, Error = Simple<Token>> + Clone {
    balanced(Token::CurlyOpen, Token::CurlyClose).map_with_span(|_, span| CodeChunk(span))
}

// fn unpack_tuple4<A, B, C, D>((((a, b), c), d): (((A, B), C), D)) -> (A, B, C, D) {
//     (a, b, c, d)
// }

#[derive(Debug, Clone)]
pub struct CodeChunk(pub Span);

impl CodeChunk {
    pub fn to_str<'a>(&self, src: &'a str) -> Option<&'a str> {
        src.get(self.0.clone())
    }
}

pub fn actual_parser() -> impl Parser<Token, Vec<Ast>, Error = Simple<Token>> {
    let returns_type = just(Token::Returns).ignore_then(ty());

    let freeform = select! {
        Token::FreeFormSlashed => Style::Slashed,
        Token::FreeFormStarred => Style::Starred,
    }
    .map_with_span(Ast::FreeFormComment)
    .labelled("freeform")
    .boxed();

    let cvl_doc = select! {
        Token::CvlDocSlashed => Style::Slashed,
        Token::CvlDocStarred => Style::Starred,
    }
    .map_with_span(Ast::Documentation)
    .labelled("documentation")
    .boxed();

    let param_filters = just(Token::Filtered).ignore_then(code_block());

    let rule_decl = {
        let optional_params = param_list().or_not().map(Option::unwrap_or_default);
        let optional_filter = param_filters.or_not();

        just(Token::Rule)
            .ignore_then(ident())
            .then(optional_params)
            .then(optional_filter)
            .then(code_block())
            .map(|(((name, params), filters), block)| Ast::Rule {
                name,
                params,
                filters,
                block,
            })
            .labelled("rule declaration")
            .boxed()
    };
    let function_decl = {
        let optional_returns = returns_type.clone().or_not();
        just(Token::Function)
            .ignore_then(ident())
            .then(param_list())
            .then(optional_returns)
            .then(code_block())
            .map(|(((name, params), returns), block)| Ast::Function {
                name,
                params,
                returns,
                block,
            })
            .labelled("function declaration")
            .boxed()
    };

    let methods_decl = just(Token::Methods)
        .ignore_then(code_block())
        .map(|block| Ast::Methods { block })
        .labelled("methods declaration")
        .boxed();

    // let invariant_decl = {
    //     //this is an approximation. parsing this rigorously is difficult.
    //     let stopping_condition = one_of(INVARIANT_STOP_TOKENS).ignored().or(end().ignored());
    //     let tail = none_of(INVARIANT_STOP_TOKENS)
    //         .padded_by(comments())
    //         .map_with_span(|tok, span| (tok, span))
    //         .at_least_once();

    //     just(Token::Invariant)
    //         .ignore_then(ident())
    //         .then(param_list())
    //         .then(tail)
    //         .then_ignore(stopping_condition.rewind())
    //         .map(|((name, params), spanned_tail)| Ast::Invariant {
    //             name,
    //             params,
    //             spanned_tail,
    //         })
    //         .boxed()
    // };

    let invariant_decl = {
        //after the parameter list, rest of the invariant declaration
        //is split into these three sections, in order:
        // (1) the invariant expression itself (mandatory)
        // (2) param filters block (optional)
        // (3) the invariant proof (optional)
        let input_end = end().ignored();
        let invariant_expression = {
            //in correct code, the invariant expression must end at one of:
            // (1) the param filters block (must start with "invariant")
            // (2) the invariant proof (must start with an opening curly bracket)
            // (3) the next valid syntactic element after the invariant block

            let mut stop_tokens = vec![Token::Filtered, Token::CurlyOpen];
            stop_tokens.extend_from_slice(&INVARIANT_STOP_TOKENS);

            let stop = one_of(stop_tokens.clone()).ignored().or(input_end);

            none_of(stop_tokens)
                .at_least_once()
                .then_ignore(stop.rewind())
                .map_with_span(|_, span| CodeChunk(span))
        };

        //in correct code, a param filters block must be balanced
        let filtered_block = just(Token::Filtered)
            .then(balanced(Token::CurlyOpen, Token::CurlyClose))
            .map_with_span(|_, span| CodeChunk(span));

        //in correct code, a proof block must be balanced
        let invariant_proof = code_block();

        just(Token::Invariant)
            .ignore_then(ident())
            .then(param_list())
            .then(invariant_expression)
            .then(filtered_block.or_not())
            .then(invariant_proof.or_not())
            .map(
                |((((name, params), invariant), filters), proof)| Ast::Invariant {
                    name,
                    params,
                    invariant,
                    filters,
                    proof,
                },
            )
            .boxed()
    };

    let ghost_decl = {
        let unnamed_param_list =
            param_list().map(|params| params.into_iter().map(|(ty, _)| ty).collect());
        let optional_code_block = code_block().map(Some).or(just(Token::Semicolon).to(None));

        let with_mapping = just(Token::Ghost)
            .ignore_then(ty())
            .then(ident())
            .then(code_block())
            .map(|((mapping, name), block)| Ast::GhostMapping {
                mapping,
                name,
                block,
            })
            .labelled("ghost declaration (with mapping)");

        let without_mapping = just(Token::Ghost)
            .ignore_then(ident())
            .then(unnamed_param_list)
            .then(returns_type.clone())
            .then(optional_code_block)
            .map(|(((name, ty_list), returns), block)| Ast::Ghost {
                name,
                ty_list,
                returns,
                block,
            })
            .labelled("ghost declaration (without mapping)");

        with_mapping.or(without_mapping).boxed()
    };

    let definition_decl = {
        let rhs = none_of(Token::Semicolon)
            .at_least_once()
            .chain(just(Token::Semicolon))
            .map(String::from_iter);

        just(Token::Definition)
            .ignore_then(ident())
            .then(param_list())
            .then(returns_type)
            .then_ignore(just(Token::Equals))
            .then(rhs)
            .map(|(((name, params), returns), definition)| Ast::Definition {
                name,
                params,
                returns,
                definition,
            })
            .recover_with(skip_until(SYNC_TOKENS, |_| Ast::ParseError))
            .labelled("definition declaration")
            .boxed()
    };

    choice([
        freeform,
        cvl_doc,
        rule_decl,
        function_decl,
        methods_decl,
        invariant_decl,
        ghost_decl,
        definition_decl,
    ])
    .recover_with(skip_until(SYNC_TOKENS, |_| Ast::ParseError))
    .repeated()
}

fn comments() -> impl Parser<Token, Vec<Token>, Error = Simple<Token>> + Clone {
    choice([
        just(Token::SingleLineComment),
        just(Token::MultiLineComment),
    ])
    .repeated()
}

impl FromIterator<Token> for String {
    fn from_iter<T: IntoIterator<Item = Token>>(iter: T) -> Self {
        iter.into_iter().join(" ")
    }
}
