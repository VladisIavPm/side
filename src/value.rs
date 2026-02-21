use crate::error::RuntimeError;
use std::fmt;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Whole(i64),
    Fraction(f64),
    String(String),
    Bool(bool),
    None,
    Object {
        form_name: String,
        fields: HashMap<String, Value>,
    },
    List(Vec<Value>),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Whole(n) => write!(f, "{}", n),
            Value::Fraction(x) => write!(f, "{}", x),
            Value::String(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", b),
            Value::None => write!(f, "none"),
            Value::Object { form_name, fields } => {
                write!(f, "{} {{ ", form_name)?;
                for (i, (name, val)) in fields.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", name, val)?;
                }
                write!(f, " }}")
            }
            Value::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
        }
    }
}

impl Value {
    pub fn add(self, other: Value) -> Result<Value, RuntimeError> {
    match (self, other) {
        (Value::Whole(a), Value::Whole(b)) => Ok(Value::Whole(a + b)),
        (Value::Fraction(a), Value::Fraction(b)) => Ok(Value::Fraction(a + b)),
        (Value::Whole(a), Value::Fraction(b)) => Ok(Value::Fraction(a as f64 + b)),
        (Value::Fraction(a), Value::Whole(b)) => Ok(Value::Fraction(a + b as f64)),
        (Value::String(a), Value::String(b)) => Ok(Value::String(a + &b)),
        (Value::String(s), Value::None) => Ok(Value::String(s + "none")),
        (Value::None, Value::String(s)) => Ok(Value::String("none".to_string() + &s)),
        (Value::String(s), Value::List(l)) => Ok(Value::String(s + &format!("{}", Value::List(l)))),
        (Value::List(l), Value::String(s)) => Ok(Value::String(format!("{}", Value::List(l)) + &s)),
        (Value::String(s), Value::Whole(n)) => Ok(Value::String(s + &n.to_string())),
        (Value::String(s), Value::Fraction(f)) => Ok(Value::String(s + &f.to_string())),
        (Value::Whole(n), Value::String(s)) => Ok(Value::String(n.to_string() + &s)),
        (Value::Fraction(f), Value::String(s)) => Ok(Value::String(f.to_string() + &s)),
        (a, b) => Err(RuntimeError::TypeError(format!("Cannot add {:?} and {:?}", a, b))),
    }
}

    pub fn sub(self, other: Value) -> Result<Value, RuntimeError> {
        match (self, other) {
            (Value::Whole(a), Value::Whole(b)) => Ok(Value::Whole(a - b)),
            (Value::Fraction(a), Value::Fraction(b)) => Ok(Value::Fraction(a - b)),
            (Value::Whole(a), Value::Fraction(b)) => Ok(Value::Fraction(a as f64 - b)),
            (Value::Fraction(a), Value::Whole(b)) => Ok(Value::Fraction(a - b as f64)),
            _ => Err(RuntimeError::TypeError("Type mismatch for subtraction".to_string())),
        }
    }

    pub fn mul(self, other: Value) -> Result<Value, RuntimeError> {
        match (self, other) {
            (Value::Whole(a), Value::Whole(b)) => Ok(Value::Whole(a * b)),
            (Value::Fraction(a), Value::Fraction(b)) => Ok(Value::Fraction(a * b)),
            (Value::Whole(a), Value::Fraction(b)) => Ok(Value::Fraction(a as f64 * b)),
            (Value::Fraction(a), Value::Whole(b)) => Ok(Value::Fraction(a * b as f64)),
            _ => Err(RuntimeError::TypeError("Type mismatch for multiplication".to_string())),
        }
    }

    pub fn div(self, other: Value) -> Result<Value, RuntimeError> {
        match (self, other) {
            (Value::Whole(a), Value::Whole(b)) if b != 0 => Ok(Value::Whole(a / b)),
            (Value::Fraction(a), Value::Fraction(b)) if b != 0.0 => Ok(Value::Fraction(a / b)),
            (Value::Whole(a), Value::Fraction(b)) if b != 0.0 => Ok(Value::Fraction(a as f64 / b)),
            (Value::Fraction(a), Value::Whole(b)) if b != 0 => Ok(Value::Fraction(a / b as f64)),
            (_, Value::Whole(0)) | (_, Value::Fraction(0.0)) => Err(RuntimeError::DivisionByZero),
            _ => Err(RuntimeError::TypeError("Type mismatch for division".to_string())),
        }
    }

    pub fn rem(self, other: Value) -> Result<Value, RuntimeError> {
        match (self, other) {
            (Value::Whole(a), Value::Whole(b)) if b != 0 => Ok(Value::Whole(a % b)),
            (_, Value::Whole(0)) => Err(RuntimeError::DivisionByZero),
            _ => Err(RuntimeError::TypeError("Remainder only for integers".to_string())),
        }
    }

    pub fn eq(self, other: Value) -> Result<Value, RuntimeError> {
        Ok(Value::Bool(self == other))
    }

    pub fn ne(self, other: Value) -> Result<Value, RuntimeError> {
        Ok(Value::Bool(self != other))
    }

    pub fn lt(self, other: Value) -> Result<Value, RuntimeError> {
        match (self, other) {
            (Value::Whole(a), Value::Whole(b)) => Ok(Value::Bool(a < b)),
            (Value::Fraction(a), Value::Fraction(b)) => Ok(Value::Bool(a < b)),
            (Value::Whole(a), Value::Fraction(b)) => Ok(Value::Bool((a as f64) < b)),
            (Value::Fraction(a), Value::Whole(b)) => Ok(Value::Bool(a < (b as f64))),
            (Value::String(a), Value::String(b)) => Ok(Value::Bool(a < b)),
            (a, b) => Err(RuntimeError::TypeError(format!("Cannot compare {:?} and {:?}", a, b))),
        }
    }

    pub fn le(self, other: Value) -> Result<Value, RuntimeError> {
        match (self, other) {
            (Value::Whole(a), Value::Whole(b)) => Ok(Value::Bool(a <= b)),
            (Value::Fraction(a), Value::Fraction(b)) => Ok(Value::Bool(a <= b)),
            (Value::Whole(a), Value::Fraction(b)) => Ok(Value::Bool((a as f64) <= b)),
            (Value::Fraction(a), Value::Whole(b)) => Ok(Value::Bool(a <= (b as f64))),
            (Value::String(a), Value::String(b)) => Ok(Value::Bool(a <= b)),
            (a, b) => Err(RuntimeError::TypeError(format!("Cannot compare {:?} and {:?}", a, b))),
        }
    }

    pub fn gt(self, other: Value) -> Result<Value, RuntimeError> {
        match (self, other) {
            (Value::Whole(a), Value::Whole(b)) => Ok(Value::Bool(a > b)),
            (Value::Fraction(a), Value::Fraction(b)) => Ok(Value::Bool(a > b)),
            (Value::Whole(a), Value::Fraction(b)) => Ok(Value::Bool((a as f64) > b)),
            (Value::Fraction(a), Value::Whole(b)) => Ok(Value::Bool(a > (b as f64))),
            (Value::String(a), Value::String(b)) => Ok(Value::Bool(a > b)),
            (a, b) => Err(RuntimeError::TypeError(format!("Cannot compare {:?} and {:?}", a, b))),
        }
    }

    pub fn ge(self, other: Value) -> Result<Value, RuntimeError> {
        match (self, other) {
            (Value::Whole(a), Value::Whole(b)) => Ok(Value::Bool(a >= b)),
            (Value::Fraction(a), Value::Fraction(b)) => Ok(Value::Bool(a >= b)),
            (Value::Whole(a), Value::Fraction(b)) => Ok(Value::Bool((a as f64) >= b)),
            (Value::Fraction(a), Value::Whole(b)) => Ok(Value::Bool(a >= (b as f64))),
            (Value::String(a), Value::String(b)) => Ok(Value::Bool(a >= b)),
            (a, b) => Err(RuntimeError::TypeError(format!("Cannot compare {:?} and {:?}", a, b))),
        }
    }
}