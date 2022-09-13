use super::*;
use crate::{AssociatedElement, Param, Ty};
use once_cell::sync::Lazy;
use regex::Regex;

// enum Token {
//     Block(String),
//     Rule,
//     Methods,
//     Invariant,
//     Function,
//     Ghost,
//     Definition,
// }

//grabs all text between a pair of curly brackets, including the brackets.
//it keeps going through nested brackets, until it find a closing bracket that
//matches the opening curly bracket (that is, the string up to that point is "balanced")
//note this does not validate that the brackets are
//still balanced past the last balanced closing bracket.
pub(super) fn balanced_curly_brackets<'src>() -> BoxedParser<'src, char, String, Simple<char>> {
    let lb = just('{').map(String::from);
    let rb = just('}').map(String::from);
    let content = none_of("{}").repeated().at_least(1).map(String::from_iter);

    let block = recursive(|block| {
        let between = content.or(block).repeated().map(String::from_iter);

        lb.chain(between)
            .chain(rb)
            .map(|v: Vec<String>| v.into_iter().collect())
    });

    block
        .map(|s: String| -> String {
            let without_brackets = s
                .strip_prefix('{')
                .and_then(|s| s.strip_suffix('}'))
                .expect("starts and ends with brackets");

            without_brackets.trim().to_string()
        })
        .boxed()
}

trait StringExt {
    fn cleanup(self) -> Self;
}
impl StringExt for String {
    fn cleanup(self) -> String {
        static JUNK: Lazy<Regex> = Lazy::new(|| {
            let single_line_comment = r"//.*\n";
            let multi_line_comment = r"/\*(?:.|\n)*\*/";
            let line_breaks = r"\r|\n";

            let pattern = format!("{single_line_comment}|{multi_line_comment}|{line_breaks}");

            Regex::new(&pattern).unwrap()
        });

        static MULTIPLE_SPACES: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s{2,}").unwrap());

        let text = self.trim();
        let text = JUNK.replace_all(text, "");
        MULTIPLE_SPACES.replace_all(text.as_ref(), " ").into_owned()
    }
}

/// according to the grammar, it is required to have some amount of whitespace immediately after
/// some tokens. however, this may again be followed by comments.
fn mandatory_sep<'src>() -> BoxedParser<'src, char, (), Simple<char>> {
    let mandatory_ws = text::whitespace().at_least(1);

    mandatory_ws.ignore_then(optional_sep()).boxed()
}

/// when parsing the block associated with the documentation, we are dealing with
/// a stream of tokens. tokens may be separated by some combination of whitespace or comments.
/// since we do not go through a lexing stage that filters them out, we must assume
/// that they may exist (possibly repeatedly) between any valid token of the associated block.
fn optional_sep_immediately_after_doc<'src>() -> BoxedParser<'src, char, (), Simple<char>> {
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

fn optional_sep<'src>() -> BoxedParser<'src, char, (), Simple<char>> {
    //we cannot use the usual multi-line comment parser here, since it is
    //now allowed to have "/**" as a comment starter.
    let multi_line_comment_between_tokens = just("/*").then(take_to_starred_terminator()).ignored();

    let comment = choice((single_line_cvl_comment(), multi_line_comment_between_tokens)).padded();

    comment.repeated().ignored().padded().boxed()
}

fn param_filters<'src>() -> BoxedParser<'src, char, String, Simple<char>> {
    just("filtered")
        .ignore_then(optional_sep())
        .ignore_then(balanced_curly_brackets())
        .boxed()
}

/// this is overly lenient. it is an approximation and is not meant to verify type correctness.
/// it's just here to be able to recognize types that the compiler would accept.
/// among other issues, this does not bother trimming all possible comments and whitespace.
fn ty<'src>() -> BoxedParser<'src, char, Ty, Simple<char>> {
    let id = text::ident();
    let call = id
        .chain(just('.'))
        .chain(id)
        .map(|v: Vec<char>| v.into_iter().collect());
    let mapping = mapping_ty();
    let array_ty = {
        let subscript = just('[')
            .then(take_until(just(']')))
            .map(|(_lb, (subscript, _rb))| String::from_iter(subscript).cleanup());

        id.or(call)
            .then_ignore(optional_sep())
            .then(subscript)
            .map(|(id, subscript)| format!("{id}[{subscript}]"))
    };

    choice((array_ty, mapping, call, id)).boxed()
}

fn param_list<'src>() -> BoxedParser<'src, char, Vec<Param>, Simple<char>> {
    let param_name = mandatory_sep().ignore_then(text::ident());
    let args = ty()
        .then(param_name.or_not())
        .padded_by(optional_sep())
        .boxed();

    args.separated_by(just(','))
        .delimited_by(just('('), just(')'))
        .boxed()
}

fn unnamed_param_list<'src>() -> BoxedParser<'src, char, Vec<Ty>, Simple<char>> {
    let single_ty = ty().then_ignore(optional_sep());

    single_ty
        .separated_by(just(','))
        .delimited_by(just('('), just(')'))
        .boxed()
}

fn invariant_decl<'src>() -> BoxedParser<'src, char, AssociatedElement, Simple<char>> {
    let invariant_start = just("invariant")
        .ignore_then(mandatory_sep())
        .ignore_then(decl_name())
        .then_ignore(optional_sep())
        .then(param_list());

    let filters_and_optional_block = param_filters()
        .map(Some)
        .then_ignore(optional_sep())
        .then(balanced_curly_brackets().or_not());
    let block_without_filters = balanced_curly_brackets().map(|block| (None, Some(block)));

    //horrible terrible hack to deal with ambiguous termination of string
    let stopping = [
        "ghost",
        "definition",
        "hook",
        "axiom",
        "rule",
        "invariant",
        "methods",
        "function",
        "using",
    ]
    .map(just);
    let new_natspec_start = just("///").or(just("/**").then_ignore(none_of('/')));
    let end_at_stopping_word = end()
        .or(choice(stopping).ignored())
        .or(new_natspec_start.ignored())
        .to((None, None))
        .rewind();

    let invariant_ending = take_until(
        filters_and_optional_block
            .or(block_without_filters)
            .or(end_at_stopping_word),
    )
    .map(|(invariant, (filters, block))| (String::from_iter(invariant).cleanup(), filters, block));

    invariant_start
        .then(invariant_ending)
        .map(
            |((name, params), (invariant, filters, block))| AssociatedElement::Invariant {
                name,
                params,
                invariant,
                filters,
                block,
            },
        )
        .boxed()
}

fn methods_decl<'src>() -> BoxedParser<'src, char, AssociatedElement, Simple<char>> {
    let methods_start = just("methods").then(optional_sep());

    methods_start
        .ignore_then(balanced_curly_brackets())
        .map(|block| AssociatedElement::Methods { block })
        .boxed()
}

fn rule_decl<'src>() -> BoxedParser<'src, char, AssociatedElement, Simple<char>> {
    let rule_start = just("rule").then(mandatory_sep());
    let optional_params = param_list().or_not().map(Option::unwrap_or_default);

    rule_start
        .ignore_then(decl_name())
        .then(optional_params)
        .then_ignore(optional_sep())
        .then(param_filters().or_not().then_ignore(optional_sep()))
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

fn mapping_ty<'src>() -> BoxedParser<'src, char, String, Simple<char>> {
    just("mapping")
        .ignore_then(optional_sep())
        .ignore_then(just('('))
        .ignore_then(take_until_without_terminator(just(')')))
        .map(|body| {
            let body = String::from_iter(body).cleanup();
            format!("mapping({body})")
        })
        .boxed()
}

fn ghost_decl<'src>() -> BoxedParser<'src, char, AssociatedElement, Simple<char>> {
    let optional_block = balanced_curly_brackets().map(Some).or(just(';').to(None));
    let ghost_start = just("ghost").then(mandatory_sep());

    let with_mapping = ty()
        .then_ignore(mandatory_sep())
        .then(decl_name())
        .then_ignore(optional_sep())
        .then(optional_block.clone())
        .map(|((mapping, name), block)| AssociatedElement::GhostMapping {
            name,
            mapping,
            block,
        });

    let without_mapping = decl_name()
        .then_ignore(optional_sep())
        .then(unnamed_param_list())
        .then_ignore(optional_sep())
        .then(returns_type())
        .then_ignore(optional_sep())
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
        .ignore_then(mandatory_sep())
        .ignore_then(ty())
        .boxed()
}

fn decl_name() -> impl Parser<char, String, Error = Simple<char>> {
    text::ident()
}

fn function_decl<'src>() -> BoxedParser<'src, char, AssociatedElement, Simple<char>> {
    let function_start = just("function").then(mandatory_sep());

    function_start
        .ignore_then(decl_name())
        .then(param_list().padded_by(optional_sep()))
        .then_ignore(optional_sep())
        .then(returns_type().or_not().then_ignore(optional_sep()))
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
    let before_definition = just('=').padded_by(optional_sep());
    let definition = take_until_without_terminator(just(';'))
        .collect()
        .map(StringExt::cleanup);

    kind.ignore_then(mandatory_sep())
        .ignore_then(decl_name())
        .then_ignore(optional_sep())
        .then(param_list())
        .then_ignore(optional_sep())
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

    optional_sep_immediately_after_doc()
        .ignore_then(decl)
        .boxed()
}
