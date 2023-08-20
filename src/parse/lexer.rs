use super::helpers::*;
use crate::parse::types::Token;
use crate::util::Span;
use chumsky::prelude::*;

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

    let string = none_of('"')
        .at_least_once()
        .collect()
        .map(Token::String)
        .delimited_by(just('"'), just('"'))
        .boxed();

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
            "import" => Token::Import,
            "builtin" => Token::Builtin,
            "use" => Token::Use,
            "as" => Token::As,
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
        string,
        keyword_or_ident,
        other,
    ])
    .map_with_span(|token, span| (token, span))
    .padded()
    .repeated()
}
