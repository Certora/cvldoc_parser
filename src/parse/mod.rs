pub mod builder;
mod helpers;
mod terminated_str;
pub mod types;

use crate::util::Span;
use chumsky::prelude::*;
use helpers::*;
use types::{Intermediate, Style, Token};

// fn unpack_tuple4<A, B, C, D>((((a, b), c), d): (((A, B), C), D)) -> (A, B, C, D) {
//     (a, b, c, d)
// }

pub fn cvl_lexer() -> impl Parser<char, Vec<(Token, Span)>, Error = Simple<char>> {
    let cvldoc_slashed_line = just("///")
        .then_ignore(none_of('/').rewind())
        .then(take_until(newline_or_end()));
    let cvldoc_slashed = cvldoc_slashed_line
        .at_least_once()
        .to(Token::CvlDocSlashed)
        .boxed();
    let cvldoc_starred = just("/**")
        .then_ignore(none_of("*/").rewind())
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

        let header = just("/***").then(just('*').repeated()).then(endings);
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
        .then(none_of('/').rewind())
        .then(take_until(newline_or_end()))
        .to(Token::SingleLineComment);
    let multi_line_comment = {
        let proper_comment = recursive(|proper_comment| {
            let content = just("/*").or(just("*/")).not().ignored();

            content
                .or(proper_comment)
                .repeated()
                .delimited_by(just("/*"), just("*/"))
                .ignored()
        });

        proper_comment.to(Token::MultiLineComment)
    };
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
        // otherwise, this would capture block delimiters.
        static IMPORTANT_SIGILS: &[char] = &['{', '}'];

        filter(|ch: &char| !ch.is_ascii_whitespace() && !IMPORTANT_SIGILS.contains(ch))
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

fn decl_parser() -> impl Parser<Token, Intermediate, Error = Simple<Token>> {
    let rule_decl = {
        let optional_params = param_list().or_not().map(Option::unwrap_or_default);

        let param_filters = just(Token::Filtered).ignore_then(code_block());
        let optional_filter = param_filters.or_not();

        just(Token::Rule)
            .ignore_then(ident())
            .then(optional_params)
            .then(optional_filter)
            .then(code_block())
            .map(|(((name, params), filters), block)| Intermediate::Rule {
                name,
                params,
                filters,
                block,
            })
            .labelled("rule declaration")
            .boxed()
    };
    let function_decl = {
        let optional_returns = returns_type().or_not();
        just(Token::Function)
            .ignore_then(ident())
            .then(param_list())
            .then(optional_returns)
            .then(code_block())
            .map(
                |(((name, params), returns), block)| Intermediate::Function {
                    name,
                    params,
                    returns,
                    block,
                },
            )
            .labelled("function declaration")
            .boxed()
    };

    let methods_decl = just(Token::Methods)
        .ignore_then(code_block())
        .map(Intermediate::Methods)
        .labelled("methods declaration")
        .boxed();

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
                .map_with_span(|_, span| span)
        };

        //in correct code, a param filters block must be balanced
        let filtered_block = just(Token::Filtered)
            .then(balanced(Token::CurlyOpen, Token::CurlyClose))
            .map_with_span(|_, span| span);

        //in correct code, a proof block must be balanced
        let invariant_proof = code_block();

        just(Token::Invariant)
            .ignore_then(ident())
            .then(param_list())
            .then(invariant_expression)
            .then(filtered_block.or_not())
            .then(invariant_proof.or_not())
            .map(
                |((((name, params), invariant), filters), proof)| Intermediate::Invariant {
                    name,
                    params,
                    invariant,
                    filters,
                    proof,
                },
            )
            .labelled("invariant declaration")
            .boxed()
    };

    let ghost_decl = {
        let unnamed_param_list =
            param_list().map(|params| params.into_iter().map(|(ty, _)| ty).collect());
        let optional_code_block = code_block().map(Some).or(just(Token::Semicolon).to(None));

        let with_mapping = just(Token::Ghost)
            .ignore_then(ty())
            .then(ident())
            .then(optional_code_block.clone())
            .map(|((mapping, name), block)| Intermediate::GhostMapping {
                mapping,
                name,
                axioms: block,
            })
            .labelled("ghost declaration (with mapping)");

        let without_mapping = just(Token::Ghost)
            .ignore_then(ident())
            .then(unnamed_param_list)
            .then(returns_type())
            .then(optional_code_block)
            .map(|(((name, ty_list), returns), block)| Intermediate::Ghost {
                name,
                ty_list,
                returns,
                axioms: block,
            })
            .labelled("ghost declaration (without mapping)");

        with_mapping.or(without_mapping).boxed()
    };

    let definition_decl = {
        let rhs = none_of(Token::Semicolon)
            .at_least_once()
            .then_ignore(just(Token::Semicolon))
            .map(String::from_iter);

        just(Token::Definition)
            .ignore_then(ident())
            .then(param_list())
            .then(returns_type())
            .then_ignore(just(Token::Equals))
            .then(rhs)
            .map(
                |(((name, params), returns), definition)| Intermediate::Definition {
                    name,
                    params,
                    returns,
                    definition,
                },
            )
            .recover_with(skip_until(SYNC_TOKENS, |_| Intermediate::ParseError))
            .labelled("definition declaration")
            .boxed()
    };

    choice([
        rule_decl,
        function_decl,
        methods_decl,
        invariant_decl,
        ghost_decl,
        definition_decl,
    ])
}

fn cvl_parser() -> impl Parser<Token, Vec<(Intermediate, Span)>, Error = Simple<Token>> {
    let freeform = select! {
        Token::FreeFormSlashed => Style::Slashed,
        Token::FreeFormStarred => Style::Starred,
    }
    .map_with_span(Intermediate::FreeFormComment)
    .labelled("freeform")
    .boxed();

    let cvl_doc = select! {
        Token::CvlDocSlashed => Style::Slashed,
        Token::CvlDocStarred => Style::Starred,
    }
    .map_with_span(Intermediate::Documentation)
    .labelled("documentation")
    .boxed();

    let decl = decl_parser().boxed();

    choice([freeform, cvl_doc, decl])
        .recover_with(skip_until(SYNC_TOKENS, |_| Intermediate::ParseError))
        .map_with_span(|intermediate, span| (intermediate, span))
        .repeated()
}

#[cfg(test)]
mod tests;
