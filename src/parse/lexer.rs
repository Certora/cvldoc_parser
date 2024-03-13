use super::helpers::*;
use crate::parse::types::Token;
use crate::util::Span;
use chumsky::prelude::*;

pub fn cvl_lexer() -> impl Parser<char, Vec<(Token, Span)>, Error = Simple<char>> {
    let cvldoc_slashed_line = just("///")
        .then_ignore(none_of('/').rewind())
        .then(take_until(newline_or_end()));
    let cvldoc_slashed = cvldoc_slashed_line
        .repeated()
        .at_least(1)
        .to(Token::CvlDocSlashed);
    let cvldoc_starred = just("/**")
        .then_ignore(none_of("*/").rewind())
        .then(take_until(just("*/")))
        .to(Token::CvlDocStarred);
    let freeform_slashed_line = just("////").then(take_until(newline_or_end()));
    let freeform_slashed = freeform_slashed_line
        .repeated()
        .at_least(1)
        .to(Token::FreeFormSlashed);
    let freeform_starred = just("/***")
        .then(take_until(just("*/")))
        .to(Token::FreeFormStarred);
    let freeform_starred_alternative = {
        //this is verbose and hideous

        let middle_endings = choice((just("*/\r\n"), just("*/\n")));
        let endings = choice((just("/\r\n"), just("/\n"), just("/").then_ignore(end())));

        let header = just("/***").then(just('*').repeated()).then(endings);
        let middle = just("/***").then(take_until(middle_endings));

        middle.padded_by(header).to(Token::FreeFormStarred)
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
            '!' => Token::Excl,
            '+' => Token::Plus,
            '/' => Token::Slash,
        };
        let arrow = just("=>").to(Token::Arrow);

        choice((single_char, arrow))
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
    let comment = single_line_comment.or(multi_line_comment);

    let num = {
        let decimal = one_of("0123456789").repeated().at_least(1).collect();

        let hex_digits = one_of("0123456789ABCDEFabcdef")
            .repeated()
            .at_least(1)
            .collect::<String>();
        let hex = just("0x")
            .then(hex_digits)
            .map(|(head, tail)| format!("{head}{tail}"));

        choice((decimal, hex)).map(Token::Number)
    };

    let string = none_of('"')
        .repeated()
        .at_least(1)
        .collect()
        .map(Token::String)
        .delimited_by(just('"'), just('"'));

    let keyword_or_ident = text::ident().map(|ident: String| match ident.as_str() {
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
        "Sload" => Token::Sload,
        "Sstore" => Token::Sstore,
        "Create" => Token::Create,
        "STORAGE" => Token::Storage,
        "KEY" => Token::Key,
        "INDEX" => Token::Index,
        "slot" => Token::Slot,
        "offset" => Token::Offset,
        "exists" => Token::Exists,
        "forall" => Token::ForAll,
        "return" => Token::Return,
        "override" => Token::Override,
        "sig" => Token::Sig,
        "description" => Token::Description,
        "old" => Token::Old,
        "persistent" => Token::Persistent,
        _ => Token::Ident(ident),
    });
    let other = {
        // otherwise, this would capture block delimiters.
        static IMPORTANT_SIGILS: &[char] = &['{', '}'];

        filter(|ch: &char| !ch.is_ascii_whitespace() && !IMPORTANT_SIGILS.contains(ch))
            .repeated()
            .at_least(1)
            .collect()
            .map(Token::Other)
    };

    choice((
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
    ))
    .map_with_span(|token, span| (token, span))
    .padded()
    .repeated()
}
