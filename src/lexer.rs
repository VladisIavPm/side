use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone)]
pub enum Token {
    // Ключевые слова
    #[token("proc")]
    Proc,
    #[token("form")]
    Form,
    #[token("link")]
    Link,
    #[token("set")]
    Set,
    #[token("fix")]
    Fix,
    #[token("check")]
    Check,
    #[token("else")]
    Else,
    #[token("loop")]
    Loop,
    #[token("wait")]
    Wait,
    #[token("stop")]
    Stop,
    #[token("give")]
    Give,
    #[token("trap")]
    Trap,
    #[token("entry")]
    Entry,
    #[token("new")]
    New,
    #[token("call")]
    Call,
    #[token("log")]
    Log,
    #[token("and")]
    And,
    #[token("or")]
    Or,
    #[token("not")]
    Not,
    #[token("as")]
    As,
    #[token("start")]
    Start,
    #[token("end")]
    End,

    // Литералы
    #[regex(r"-?[0-9]+", |lex| lex.slice().parse().map_err(|_| ()))]
    Whole(i64),
    #[regex(r"-?[0-9]+\.[0-9]+", |lex| lex.slice().parse().map_err(|_| ()))]
    Fraction(f64),
    #[regex(r#""([^"\\]|\\t|\\n|\\")*""#, |lex| {
        let slice = lex.slice();
        Some(slice[1..slice.len()-1].to_string())
    })]
    String(String),
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("none")]
    None,

    // Идентификаторы
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| Some(lex.slice().to_string()))]
    Identifier(String),

    // Операторы и разделители
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("=")]
    Assign,
    #[token("==")]
    Equal,
    #[token("!=")]
    NotEqual,
    #[token(">")]
    Greater,
    #[token("<")]
    Less,
    #[token(">=")]
    GreaterEqual,
    #[token("<=")]
    LessEqual,
    #[token(".")]
    Dot,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token("::")]
    DoubleColon,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,

    // Пропуски
    #[regex(r"[ \t\n\r]+", logos::skip)]
    Whitespace,
    #[regex(r"#[^\n]*", logos::skip)]
    Comment,

    // Ошибки — просто вариант (атрибут #[error] не нужен)
    Error,
}