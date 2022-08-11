use super::*;
use crate::{AssociatedElement, Param, Ty};

/// according to the grammar, it is required to have some amount of whitespace immediately after
/// some tokens. however, this may again be followed by comments.
fn mandatory_token_separator<'src>() -> BoxedParser<'src, char, (), Simple<char>> {
    let mandatory_ws = text::whitespace().at_least(1);

    mandatory_ws.ignore_then(optional_token_separator()).boxed()
}

/// when parsing the block associated with the documentation, we are dealing with
/// a stream of tokens. tokens may be separated by some combination of whitespace or comments.
/// since we do not go through a lexing stage that filters them out, we must assume
/// that they may exist (possibly repeatedly) between any valid token of the associated block.
fn optional_token_separator_immediately_after_doc<'src>(
) -> BoxedParser<'src, char, (), Simple<char>> {
    let single_line_comment_between_tokens = just("//")
        .then(none_of('/').rewind())
        .then(take_to_newline_or_end())
        .ignored();

    //we cannot use the usual multi-line comment parser here, since it is
    //now allowed to have "/**" as a comment starter.
    let multi_line_comment_between_tokens = just("/*").then(take_to_starred_terminator()).ignored();

    let comment = choice((
        single_line_comment_between_tokens,
        multi_line_comment_between_tokens,
    ))
    .padded();

    comment.repeated().ignored().padded().boxed()
}

fn optional_token_separator<'src>() -> BoxedParser<'src, char, (), Simple<char>> {
    //we cannot use the usual multi-line comment parser here, since it is
    //now allowed to have "/**" as a comment starter.
    let multi_line_comment_between_tokens = just("/*").then(take_to_starred_terminator()).ignored();

    let comment = choice((single_line_cvl_comment(), multi_line_comment_between_tokens)).padded();

    comment.repeated().ignored().padded().boxed()
}

fn param_filters<'src>() -> BoxedParser<'src, char, String, Simple<char>> {
    just("filtered")
        .ignore_then(optional_token_separator())
        .ignore_then(balanced_curly_brackets())
        .boxed()
}

fn ty<'src>() -> BoxedParser<'src, char, String, Simple<char>> {
    //stub
    text::ident().boxed()
}

fn param_list<'src>() -> BoxedParser<'src, char, Vec<Param>, Simple<char>> {
    let args = ty()
        .then_ignore(mandatory_token_separator())
        .then(text::ident().or_not())
        .padded_by(optional_token_separator())
        .boxed();

    args.separated_by(just(','))
        .delimited_by(just('('), just(')'))
        .boxed()
}

fn unnamed_param_list<'src>() -> BoxedParser<'src, char, Vec<Ty>, Simple<char>> {
    let single_ty = ty().then_ignore(optional_token_separator());

    single_ty
        .separated_by(just(','))
        .delimited_by(just('('), just(')'))
        .boxed()
}

fn invariant_decl<'src>() -> BoxedParser<'src, char, AssociatedElement, Simple<char>> {
    let invariant_start = just("invariant").then(mandatory_token_separator());

    //temporary workaround. parsing this is hard. this is ambiguous after the parameter decl.
    // let invariant_filtered_then_block = take_until(param_filters())
    //     .ignore_then(optional_token_separator())
    //     .ignore_then(balanced_curly_brackets());
    // let invariant_then_block = take_until(balanced_curly_brackets()).map(|(_proof, block)| block);
    // let ending = invariant_filtered_then_block.or(invariant_then_block);

    invariant_start
        .ignore_then(decl_name())
        .then_ignore(optional_token_separator())
        .then(param_list())
        .map(|(name, params)| AssociatedElement::Invariant {
            name,
            params,
            invariant: String::new(),
            block: String::new(),
        })
        .boxed()
}

fn methods_decl<'src>() -> BoxedParser<'src, char, AssociatedElement, Simple<char>> {
    let methods_start = just("methods").then(mandatory_token_separator());

    methods_start
        .ignore_then(optional_token_separator())
        .ignore_then(balanced_curly_brackets())
        .map(|block| AssociatedElement::Methods { block })
        .boxed()
}

fn rule_decl<'src>() -> BoxedParser<'src, char, AssociatedElement, Simple<char>> {
    let rule_start = just("rule").then(mandatory_token_separator());

    rule_start
        .ignore_then(decl_name())
        .then(param_list())
        .then_ignore(optional_token_separator())
        .then(
            param_filters()
                .or_not()
                .then_ignore(optional_token_separator()),
        )
        .then(balanced_curly_brackets())
        .map(
            |(((name, params), filters), block)| AssociatedElement::Rule {
                name,
                params,
                filters,
                block,
            },
        )
        .boxed()
}

/// temporary workaround until actual mapping type is recognized
// fn mapping_ty<'src>() -> BoxedParser<'src, char, String, Simple<char>> {
//     let inner = ident()
//         .then_ignore(just("=>"))
//         .then(decl_name())
//         .map(|(l, r)| format!("{l} => {r}"))
//         .padded_by(mandatory_token_separator());
//     just("mapping")
//         .ignore_then(inner.delimited_by(just('('), just(')')))
//         .padded_by(optional_token_separator())
//         .boxed()
// }

fn mapping_ty<'src>() -> BoxedParser<'src, char, String, Simple<char>> {
    just("mapping")
        .rewind()
        .ignore_then(take_until(just(')')))
        .map(|(text, terminator)| {
            text.into_iter()
                .chain(std::iter::once(terminator))
                .collect()
        })
        .boxed()
}

fn ghost_decl<'src>() -> BoxedParser<'src, char, AssociatedElement, Simple<char>> {
    let optional_block = balanced_curly_brackets().map(Some).or(just(';').to(None));
    let ghost_start = just("ghost").then(mandatory_token_separator());

    let with_mapping = mapping_ty()
        .then_ignore(mandatory_token_separator())
        .then(decl_name())
        .then_ignore(optional_token_separator())
        .then(optional_block.clone())
        .map(|((mapping, name), block)| AssociatedElement::GhostMapping {
            name,
            mapping,
            block,
        });

    let without_mapping = decl_name()
        .then_ignore(optional_token_separator())
        .then(unnamed_param_list())
        .then_ignore(optional_token_separator())
        .then(returns_type())
        .then_ignore(optional_token_separator())
        .then(optional_block)
        .map(
            |(((name, ty_list), returns), block)| AssociatedElement::Ghost {
                name,
                ty_list,
                returns,
                block,
            },
        );

    ghost_start
        .ignore_then(with_mapping.or(without_mapping))
        .boxed()
}

fn returns_type<'src>() -> BoxedParser<'src, char, String, Simple<char>> {
    just("returns")
        .ignore_then(mandatory_token_separator())
        .ignore_then(ty())
        .boxed()
}

fn decl_name() -> impl Parser<char, String, Error = Simple<char>> {
    text::ident()
}

fn function_decl<'src>() -> BoxedParser<'src, char, AssociatedElement, Simple<char>> {
    let function_start = just("function").then(mandatory_token_separator());

    function_start
        .ignore_then(decl_name())
        .then(param_list().padded_by(optional_token_separator()))
        .then_ignore(optional_token_separator())
        .then(
            returns_type()
                .or_not()
                .then_ignore(optional_token_separator()),
        )
        .then(balanced_curly_brackets())
        .map(
            |(((name, params), returns), block)| AssociatedElement::Function {
                name,
                params,
                returns,
                block,
            },
        )
        .boxed()
}

fn definition_decl<'src>() -> BoxedParser<'src, char, AssociatedElement, Simple<char>> {
    let kind = just("definition");
    let before_definition = just('=').padded_by(optional_token_separator());
    let definition = take_until_without_terminator(just(';')).collect();

    kind.ignore_then(mandatory_token_separator())
        .ignore_then(decl_name())
        .then_ignore(optional_token_separator())
        .then(param_list())
        .then_ignore(optional_token_separator())
        .then(returns_type())
        .then_ignore(before_definition)
        .then(definition)
        .map(
            |(((name, params), returns), definition)| AssociatedElement::Definition {
                name,
                params,
                returns,
                definition,
            },
        )
        .boxed()
}

pub(super) fn associated_element<'src>() -> BoxedParser<'src, char, AssociatedElement, Simple<char>>
{
    let decl = choice([
        rule_decl(),
        methods_decl(),
        invariant_decl(),
        function_decl(),
        ghost_decl(),
        definition_decl(),
    ]);

    optional_token_separator_immediately_after_doc()
        .ignore_then(decl)
        .boxed()
}
