use chumsky::text::ident;

use crate::DeclarationKind;

use super::*;

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
        .then(take_until(just('}')))
        .map(|(keyword, (block, rb))| {
            let block = String::from_iter(block);
            format!("{keyword}{block}{rb}")
        })
        .boxed()
}

fn ty<'src>() -> BoxedParser<'src, char, String, Simple<char>> {
    //stub
    text::ident().boxed()
}

type ParamList = Vec<(String, String)>;
type TyList = Vec<String>;

fn param_list<'src>() -> BoxedParser<'src, char, ParamList, Simple<char>> {
    let args = ty()
        .then_ignore(mandatory_token_separator())
        .then(text::ident())
        .padded_by(optional_token_separator())
        .boxed();

    args.separated_by(just(','))
        .delimited_by(just('('), just(')'))
        .boxed()
}

fn ty_list<'src>() -> BoxedParser<'src, char, TyList, Simple<char>> {
    let single_ty = ty().then_ignore(optional_token_separator());

    single_ty
        .separated_by(just(','))
        .delimited_by(just('('), just(')'))
        .boxed()
}

fn invariant_decl<'src>() -> BoxedParser<'src, char, UnderDoc, Simple<char>> {
    //temporary workaround. parsing this is hard. this is ambiguous after the parameter decl.
    let proof_filtered_then_block = take_until(param_filters())
        .ignore_then(optional_token_separator())
        .ignore_then(balanced_curly_brackets());
    let proof_then_block = take_until(balanced_curly_brackets()).map(|(_proof, block)| block);
    let ending = proof_filtered_then_block.or(proof_then_block);
    let kind = just("invariant").to(DeclarationKind::Invariant);

    kind.then_ignore(mandatory_token_separator())
        .then(text::ident())
        .then_ignore(optional_token_separator())
        .then(param_list())
        .then(mandatory_token_separator().ignore_then(ending).or_not())
        .map(|(((kind, name), params), block)| UnderDoc {
            kind,
            name: Some(name),
            params,
            block,
        })
        .boxed()
}

fn methods_decl<'src>() -> BoxedParser<'src, char, UnderDoc, Simple<char>> {
    just("methods")
        .to(DeclarationKind::Methods)
        .then_ignore(optional_token_separator())
        .then(balanced_curly_brackets())
        .map(|(kind, block)| UnderDoc {
            kind,
            name: None,
            params: Vec::new(),
            block: Some(block),
        })
        .boxed()
}

fn rule_decl<'src>() -> BoxedParser<'src, char, UnderDoc, Simple<char>> {
    let keyword = just("rule").to(DeclarationKind::Rule);
    keyword
        .then_ignore(mandatory_token_separator())
        .then(decl_name())
        .then_ignore(param_filters().then(optional_token_separator()).or_not())
        .then(param_list())
        .then_ignore(optional_token_separator())
        .then(balanced_curly_brackets())
        .map(|(((kind, name), params), block)| UnderDoc {
            kind,
            name: Some(name),
            params,
            block: Some(block),
        })
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

fn ghost_decl<'src>() -> BoxedParser<'src, char, UnderDoc, Simple<char>> {
    let kind = just("ghost").to(DeclarationKind::Ghost);
    let optional_block = balanced_curly_brackets().map(Some).or(just(';').to(None));

    let ghost_with_mapping = kind
        .then_ignore(mandatory_token_separator())
        .then(mapping_ty())
        .then_ignore(mandatory_token_separator())
        .then(ident())
        .then_ignore(optional_token_separator())
        .then(optional_block.clone())
        .map(|(((kind, _mapping_ty), name), block)| UnderDoc {
            kind,
            name: Some(name),
            params: Vec::new(),
            block,
        });
    let ghost_without_mapping = kind
        .then_ignore(mandatory_token_separator())
        .then(decl_name())
        .then_ignore(optional_token_separator())
        .then(ty_list())
        .then_ignore(optional_token_separator())
        .then_ignore(returns_type())
        .then_ignore(optional_token_separator())
        .then(optional_block.clone())
        .map(|(((kind, name), _ty_list), block)| UnderDoc {
            kind,
            name: Some(name),
            params: Vec::new(),
            block,
        });

    ghost_with_mapping.or(ghost_without_mapping).boxed()
    // keyword
    //     .then_ignore(mandatory_token_separator())
    //     .then(decl_name())
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

fn function_decl<'src>() -> BoxedParser<'src, char, UnderDoc, Simple<char>> {
    let kind = just("function").to(DeclarationKind::Function);
    kind.then_ignore(mandatory_token_separator())
        .then(decl_name())
        .then_ignore(optional_token_separator())
        .then(param_list())
        .then_ignore(optional_token_separator())
        .then_ignore(returns_type().then(optional_token_separator()).or_not())
        .then(balanced_curly_brackets())
        .map(|(((kind, name), params), block)| UnderDoc {
            kind,
            name: Some(name),
            params,
            block: Some(block),
        })
        .boxed()
}

fn definition_decl<'src>() -> BoxedParser<'src, char, UnderDoc, Simple<char>> {
    let kind = just("definition").to(DeclarationKind::Definition);
    kind.then_ignore(mandatory_token_separator())
        .then(decl_name())
        .then_ignore(optional_token_separator())
        .then(param_list())
        .then_ignore(optional_token_separator())
        .then_ignore(returns_type())
        .then_ignore(optional_token_separator())
        .then_ignore(just('='))
        .then_ignore(optional_token_separator())
        .then(take_until_without_terminator(just(';')).collect())
        .map(|(((kind, name), params), definition)| UnderDoc {
            kind,
            name: Some(name),
            params,
            block: Some(definition),
        })
        .boxed()
}

pub(super) fn under_doc<'src>() -> BoxedParser<'src, char, UnderDoc, Simple<char>> {
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
