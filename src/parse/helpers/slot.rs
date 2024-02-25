use super::*;

fn single_slot_access() -> impl Parser<Token, (), Error = Simple<Token>> {
    let pos = one_of([Token::Slot, Token::Offset]).then(num());
    pos.delimited_by(just(Token::RoundOpen), just(Token::RoundClose))
        .ignored()
}

fn double_slot_access() -> impl Parser<Token, (), Error = Simple<Token>> {
    let pos = one_of([Token::Slot, Token::Offset]).then(num());
    pos.separated_by(just(Token::Comma))
        .exactly(2)
        .delimited_by(just(Token::RoundOpen), just(Token::RoundClose))
        .ignored()
}

fn array_or_map_access() -> impl Parser<Token, (), Error = Simple<Token>> {
    let pos = one_of([Token::Key, Token::Index]).then(named_param());
    pos.delimited_by(just(Token::SquareOpen), just(Token::SquareClose))
        .ignored()
}

fn named_slot_pattern() -> impl Parser<Token, (), Error = Simple<Token>> {
    choice((ident().ignored(), one_of(USABLE_KEYWORDS).ignored()))
}

fn static_slot_pattern() -> impl Parser<Token, (), Error = Simple<Token>> {
    choice((
        named_slot_pattern(),
        single_slot_access(),
        double_slot_access(),
    ))
    .separated_by(dot())
    .at_least(1)
    .ignored()
}

pub fn slot_pattern() -> impl Parser<Token, Span, Error = Simple<Token>> {
    let ending = choice((
        array_or_map_access(),
        dot().then(named_slot_pattern()).ignored(),
        dot().then(single_slot_access()).ignored(),
    ));

    let slot_pattern_nested = recursive(|slot_pattern_nested| {
        static_slot_pattern()
            .then(ending.repeated().at_least(1))
            .then(slot_pattern_nested.or_not())
            .ignored()
    });

    slot_pattern_nested
        .or(static_slot_pattern())
        .map_with_span(|_, span| span)
}

fn dot() -> impl Parser<Token, (), Error = Simple<Token>> {
    just(Token::Dot).ignored()
}

const USABLE_KEYWORDS: [Token; 14] = [
    Token::Exists,
    Token::ForAll,
    Token::Using,
    Token::As,
    Token::Return,
    Token::Import,
    Token::Use,
    Token::Builtin,
    Token::Override,
    Token::Sig,
    Token::Description,
    Token::Invariant,
    Token::Preserved,
    Token::Old,
];
