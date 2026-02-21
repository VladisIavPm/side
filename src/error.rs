use thiserror::Error;
use crate::lexer::Token;
use crate::value::Value;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("[строка {line}] Неожиданный токен: {token:?}")]
    UnexpectedToken { line: usize, token: Token },
    #[error("[строка {line}] Ожидалось {expected}, найдено {found:?}")]
    Expected { line: usize, expected: String, found: Token },
    #[error("[строка {line}] Некорректное выражение слева от присваивания")]
    InvalidLValue { line: usize },
}

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("[ошибка] Переменная '{0}' не определена")]
    UndefinedVariable(String),
    #[error("[ошибка] Несоответствие типов: {0}")]
    TypeError(String),
    #[error("[ошибка] Деление на ноль")]
    DivisionByZero,
    #[error("[ошибка] Функция '{0}' не найдена")]
    UndefinedFunction(String),
    #[error("[ошибка] Поле '{0}' не найдено")]
    UndefinedField(String),
    #[error("[ошибка] Индекс за пределами списка")]
    IndexOutOfBounds,
    #[error("[ошибка] Нельзя индексировать не список")]
    NotIndexable,
    #[error("[ошибка] Break вне цикла")]
    Break,
    #[error("[ошибка] Return вне функции")]
    Return(Value),
}