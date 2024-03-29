pub mod builder;
mod helpers;
mod lexer;
mod terminated_str;
#[cfg(test)]
mod tests;
pub mod types;

use crate::util::Span;
use chumsky::prelude::*;
use helpers::slot::slot_pattern;
use helpers::*;
use types::{Intermediate, Style, Token};

fn decl_parser() -> impl Parser<Token, Intermediate, Error = Simple<Token>> {
    let rule_decl = {
        let optional_params = named_param_list().or_not();

        just(Token::Rule)
            .ignore_then(ident())
            .then(optional_params)
            .then(filtered_block().or_not())
            .then(code_block())
            .map(|(((name, params), filters), block)| Intermediate::Rule {
                name,
                params,
                filters,
                block,
            })
            .labelled("rule declaration")
    };
    let function_decl = just(Token::Function)
        .ignore_then(function_ident())
        .then(named_param_list())
        .then(returns_type().or_not())
        .then(code_block())
        .map(
            |(((name, params), returns), block)| Intermediate::Function {
                name,
                params,
                returns,
                block,
            },
        )
        .labelled("function declaration");

    let methods_decl = just(Token::Methods)
        .ignore_then(code_block())
        .map(Intermediate::Methods)
        .labelled("methods declaration");

    let invariant_decl = {
        //after the parameter list, rest of the invariant declaration
        //is split into these three sections, in order:
        // (1) the invariant expression itself (mandatory)
        // (2) param filters block (optional)
        // (3) the invariant proof (optional)

        struct Spans(Span, Option<Span>, Option<Span>);

        let single_invariant = single_expr()
            .then(just(Token::Semicolon))
            .map_with_span(|_, span| Spans(span, None, None));

        let with_filtered_block = none_of(Token::Filtered)
            .then_ignore(just(Token::Filtered).rewind())
            .map_with_span(|_, span| span)
            .then(filtered_block())
            .then(code_block().or_not())
            .map(|((inv, filtered), proof)| Spans(inv, Some(filtered), proof));

        let with_proof = single_expr()
            .or_not()
            .map_with_span(|_, span| span)
            .then(code_block())
            .map(|(inv, proof)| Spans(inv, None, Some(proof)));

        just(Token::Invariant)
            .ignore_then(ident())
            .then(named_param_list())
            .then(choice((single_invariant, with_filtered_block, with_proof)))
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
    };

    let ghost_decl = {
        let persistent_ghost = just(Token::Persistent).then(just(Token::Ghost)).to(true);
        let regular_ghost = just(Token::Ghost).to(false);

        let opening_tokens = persistent_ghost.or(regular_ghost);

        let with_mapping = opening_tokens
            .clone()
            .then(ty())
            .then(ident())
            .then(optional_code_block())
            .map(
                |(((persistent, mapping), name), axioms)| Intermediate::GhostMapping {
                    persistent,
                    mapping,
                    name,
                    axioms,
                },
            )
            .labelled("ghost declaration (with mapping)");

        let without_mapping = opening_tokens
            .then(ident())
            .then(unnamed_param_list())
            .then(returns_type())
            .then(optional_code_block())
            .map(
                |((((persistent, name), ty_list), returns), axioms)| Intermediate::GhostFunction {
                    persistent,
                    name,
                    ty_list,
                    returns,
                    axioms,
                },
            )
            .labelled("ghost declaration (without mapping)");

        with_mapping.or(without_mapping)
    };

    let definition_decl = {
        let rhs = none_of(Token::Semicolon)
            .repeated()
            .at_least(1)
            .map_with_span(|_, span| span)
            .then_ignore(just(Token::Semicolon));

        just(Token::Definition)
            .ignore_then(ident())
            .then(named_param_list())
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
    };

    let hook_decl = {
        let sload = just(Token::Sload)
            .ignore_then(named_param())
            .then(slot_pattern())
            .then_ignore(just(Token::Storage))
            .then(code_block())
            .map(
                |((loaded_value, slot_pattern), block)| Intermediate::HookSload {
                    loaded: loaded_value,
                    slot_pattern,
                    block,
                },
            );

        let sstore = just(Token::Sstore)
            .ignore_then(slot_pattern())
            .then(named_param())
            .then(
                named_param()
                    .delimited_by(just(Token::RoundOpen), just(Token::RoundClose))
                    .or_not(),
            )
            .then_ignore(just(Token::Storage))
            .then(code_block())
            .map(
                |(((slot_pattern, stored_value), old_value), block)| Intermediate::HookSstore {
                    stored: stored_value,
                    old: old_value,
                    slot_pattern,
                    block,
                },
            );

        let create = just(Token::Create)
            .ignore_then(
                named_param().delimited_by(just(Token::RoundOpen), just(Token::RoundClose)),
            )
            .then(code_block())
            .map(|(created, block)| Intermediate::HookCreate { created, block });

        let opcode = ident()
            .then(named_param_list().or_not())
            .then(named_param().or_not())
            .then(code_block())
            .map(
                |(((opcode, params), returned_value), block)| Intermediate::HookOpcode {
                    opcode,
                    params,
                    returns: returned_value,
                    block,
                },
            );

        just(Token::Hook)
            .ignore_then(choice((sload, sstore, create, opcode)))
            .labelled("hook statement")
    };

    let import_stmt = just(Token::Import)
        .ignore_then(string())
        .then_ignore(just(Token::Semicolon))
        .map(Intermediate::Import)
        .labelled("import statement");

    let use_stmt = {
        let invariant = just(Token::Invariant)
            .ignore_then(ident())
            .then(choice((code_block().map(Some), semicolon_ender())))
            .map(|(name, proof)| Intermediate::UseInvariant { name, proof });

        let builtin_rule = just(Token::Builtin)
            .ignore_then(just(Token::Rule))
            .ignore_then(ident())
            .then_ignore(just(Token::Semicolon))
            .map(|name| Intermediate::UseBuiltinRule { name });

        let rule = just(Token::Rule)
            .ignore_then(ident())
            .then(choice((filtered_block().map(Some), semicolon_ender())))
            .map(|(name, filters)| Intermediate::UseRule { name, filters });

        just(Token::Use)
            .ignore_then(choice((invariant, builtin_rule, rule)))
            .labelled("use statement")
    };

    let using_stmt = just(Token::Using)
        .ignore_then(ident())
        .then_ignore(just(Token::As))
        .then(ident())
        .then_ignore(just(Token::Semicolon))
        .map(|(contract_name, spec_name)| Intermediate::Using {
            contract_name,
            spec_name,
        })
        .labelled("using statement");

    choice((
        rule_decl,
        function_decl,
        methods_decl,
        invariant_decl,
        ghost_decl,
        hook_decl,
        definition_decl,
        import_stmt,
        use_stmt,
        using_stmt,
    ))
}

fn cvl_parser() -> impl Parser<Token, Vec<(Intermediate, Span)>, Error = Simple<Token>> {
    let freeform = select! {
        Token::FreeFormSlashed => Style::Slashed,
        Token::FreeFormStarred => Style::Starred,
    }
    .map_with_span(Intermediate::FreeFormComment)
    .labelled("freeform");

    let cvl_doc = select! {
        Token::CvlDocSlashed => Style::Slashed,
        Token::CvlDocStarred => Style::Starred,
    }
    .map_with_span(Intermediate::Documentation)
    .labelled("documentation");

    let decl = decl_parser();

    let failure = any().to(Intermediate::ParseError);

    choice((freeform, cvl_doc, decl, failure))
        .recover_with(skip_until(SYNC_TOKENS, |_| Intermediate::ParseError))
        .map_with_span(|intermediate, span| (intermediate, span))
        .repeated()
}
