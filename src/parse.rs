pub mod builder;
mod helpers;
mod lexer;
mod terminated_str;
pub mod types;

use crate::util::Span;
use chumsky::prelude::*;
use helpers::*;
use types::{Intermediate, Style, Token};

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
            .ignore_then(function_ident())
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

        struct Spans(Span, Option<Span>, Option<Span>);

        //in correct code, a param filters block must be balanced
        let filtered_block = just(Token::Filtered)
            .then(balanced(Token::CurlyOpen, Token::CurlyClose))
            .map_with_span(|_, span| span);

        //in correct code, a proof block must be balanced
        let invariant_proof = code_block();

        let single_invariant = none_of(Token::Semicolon)
            .at_least_once()
            .map_with_span(|_, span| Spans(span, None, None))
            .then_ignore(just(Token::Semicolon))
            .boxed();

        let with_filtered_block = none_of(Token::Filtered)
            .then_ignore(just(Token::Filtered).rewind())
            .map_with_span(|_, span| span)
            .then(filtered_block)
            .then(invariant_proof.clone().or_not())
            .map(|((inv, filtered), proof)| Spans(inv, Some(filtered), proof))
            .boxed();

        let with_proof = none_of(Token::CurlyOpen)
            .then_ignore(just(Token::CurlyOpen).rewind())
            .map_with_span(|_, span| span)
            .then(invariant_proof)
            .map(|(inv, proof)| Spans(inv, None, Some(proof)))
            .boxed();

        just(Token::Invariant)
            .ignore_then(ident())
            .then(param_list())
            .then(choice([single_invariant, with_filtered_block, with_proof]))
            .map(
                |((name, params), Spans(invariant, filters, proof))| Intermediate::Invariant {
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
            .map_with_span(|_, span| span);

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

/// here for backwards-compatibility with CVL1
/// assumes valid endings of the invariant keyword have already been tried
#[allow(unused)]
fn invariant_expression_without_semicolon() -> impl Parser<Token, Span, Error = Simple<Token>> {
    let input_end = end().ignored();

    let stop = one_of(INVARIANT_STOP_TOKENS).ignored().or(input_end);

    none_of(INVARIANT_STOP_TOKENS)
        .at_least_once()
        .then_ignore(stop.rewind())
        .map_with_span(|_, span| span)
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

    let failure = any().to(Intermediate::ParseError).boxed();

    choice([freeform, cvl_doc, decl, failure])
        .recover_with(skip_until(SYNC_TOKENS, |_| Intermediate::ParseError))
        .map_with_span(|intermediate, span| (intermediate, span))
        .repeated()
}

#[cfg(test)]
mod tests;
