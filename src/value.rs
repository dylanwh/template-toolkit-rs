use std::{any::Any, cell::RefCell, cmp::Ordering, collections::HashMap, fmt, rc::Rc};

use crate::error::{ControlFlow, TemplateException};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallContext {
    List,
    Scalar,
}

#[derive(Debug, Clone)]
pub enum MethodResult {
    Ok(Value),
    Err(ControlFlow),
    NotFound,
}

pub trait Callable {
    fn call(&self, args: &[Value], ctx: CallContext) -> Result<Value, ControlFlow>;

    fn type_name(&self) -> &str {
        "CODE"
    }
}

impl<F> Callable for F
where
    F: Fn(&[Value], CallContext) -> Result<Value, ControlFlow>,
{
    fn call(&self, args: &[Value], ctx: CallContext) -> Result<Value, ControlFlow> {
        (self)(args, ctx)
    }
}

pub trait TemplateObject: fmt::Debug {
    fn type_name(&self) -> &str;

    fn to_tt_string(&self) -> String {
        format!("{}=OBJECT", self.type_name())
    }

    fn is_truthy(&self) -> bool {
        true
    }

    fn call_method(&self, method: &str, args: &[Value], ctx: CallContext) -> MethodResult;

    fn as_hash(&self) -> Option<Rc<RefCell<HashMap<String, Value>>>> {
        None
    }

    fn as_list(&self) -> Option<Rc<RefCell<Vec<Value>>>> {
        None
    }

    fn equals_value(&self, _other: &Value) -> Option<bool> {
        None
    }

    fn as_any(&self) -> &dyn Any;
}

#[derive(Clone)]
pub enum Value {
    Undefined,
    Int(i64),
    Float(f64),
    Str(String),
    List(Rc<RefCell<Vec<Value>>>),
    Hash(Rc<RefCell<HashMap<String, Value>>>),
    Function(Rc<dyn Callable>),
    Object(Rc<dyn TemplateObject>),
    Exception(Rc<RefCell<TemplateException>>),
    ScalarRef(Rc<RefCell<String>>),
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Undefined => write!(f, "Undefined"),
            Value::Int(i) => write!(f, "Int({})", i),
            Value::Float(fl) => write!(f, "Float({})", fl),
            Value::Str(s) => write!(f, "Str({:?})", s),
            Value::List(l) => write!(f, "List({:?})", l.borrow()),
            Value::Hash(h) => write!(f, "Hash({:?})", h.borrow()),
            Value::Function(func) => write!(f, "Function({})", func.type_name()),
            Value::Object(o) => write!(f, "Object({:?})", o),
            Value::Exception(e) => write!(f, "Exception({:?})", e.borrow()),
            Value::ScalarRef(s) => write!(f, "ScalarRef({:?})", s.borrow()),
        }
    }
}

/// Safe Rust implementation of C `%.15g` float formatting matching Perl's default stringification.
pub fn format_perl_float(val: f64) -> String {
    if val.is_nan() {
        return "NaN".to_string();
    }
    if val.is_infinite() {
        return if val.is_sign_positive() {
            "Inf".to_string()
        } else {
            "-Inf".to_string()
        };
    }
    if val == 0.0 {
        return "0".to_string();
    }

    let sign = if val.is_sign_negative() { "-" } else { "" };
    let abs = val.abs();

    // Format with 14 digits after decimal in scientific notation = 15 significant digits
    let sci = format!("{:.14e}", abs);
    let Some((mantissa, exp_str)) = sci.split_once('e') else {
        return val.to_string();
    };
    let exp: i32 = exp_str.parse().unwrap_or(0);
    let digits: String = mantissa.chars().filter(|&c| c != '.').collect();

    if exp < -4 || exp >= 15 {
        // Exponential notation
        let first = &digits[..1];
        let frac = digits[1..].trim_end_matches('0');
        let exp_sign = if exp < 0 { '-' } else { '+' };
        let exp_abs = exp.unsigned_abs();
        if frac.is_empty() {
            format!("{}{}e{}{:02}", sign, first, exp_sign, exp_abs)
        } else {
            format!("{}{}.{}e{}{:02}", sign, first, frac, exp_sign, exp_abs)
        }
    } else if exp >= 0 {
        let int_len = (exp + 1) as usize;
        if int_len >= digits.len() {
            let mut s = String::with_capacity(sign.len() + int_len);
            s.push_str(sign);
            s.push_str(&digits);
            for _ in 0..(int_len - digits.len()) {
                s.push('0');
            }
            s
        } else {
            let int_part = &digits[..int_len];
            let frac_part = digits[int_len..].trim_end_matches('0');
            if frac_part.is_empty() {
                format!("{}{}", sign, int_part)
            } else {
                format!("{}{}.{}", sign, int_part, frac_part)
            }
        }
    } else {
        // -4 <= exp < 0
        let leading_zeros = (-1 - exp) as usize;
        let mut frac = String::with_capacity(leading_zeros + digits.len());
        for _ in 0..leading_zeros {
            frac.push('0');
        }
        frac.push_str(&digits);
        let trimmed_frac = frac.trim_end_matches('0');
        if trimmed_frac.is_empty() {
            "0".to_string()
        } else {
            format!("{}0.{}", sign, trimmed_frac)
        }
    }
}

pub fn parse_perl_number(s: &str) -> (f64, bool) {
    Value::parse_perl_number(s)
}

impl Value {
    pub fn list(items: Vec<Value>) -> Self {
        Value::List(Rc::new(RefCell::new(items)))
    }

    pub fn from_list(items: Vec<Value>) -> Self {
        Self::list(items)
    }

    pub fn new_list() -> Self {
        Self::list(Vec::new())
    }

    pub fn hash(map: HashMap<String, Value>) -> Self {
        Value::Hash(Rc::new(RefCell::new(map)))
    }

    pub fn from_hash(map: HashMap<String, Value>) -> Self {
        Self::hash(map)
    }

    pub fn new_hash() -> Self {
        Self::hash(HashMap::new())
    }

    pub fn object<T: TemplateObject + 'static>(obj: T) -> Self {
        Value::Object(Rc::new(obj))
    }

    pub fn function<F>(f: F) -> Self
    where
        F: Fn(&[Value], CallContext) -> Result<Value, ControlFlow> + 'static,
    {
        Value::Function(Rc::new(f))
    }

    pub fn exception(e: TemplateException) -> Self {
        Value::Object(Rc::new(e))
    }

    pub fn is_defined(&self) -> bool {
        !matches!(self, Value::Undefined)
    }

    pub fn is_ref(&self) -> bool {
        matches!(
            self,
            Value::List(_)
                | Value::Hash(_)
                | Value::Function(_)
                | Value::Object(_)
                | Value::Exception(_)
                | Value::ScalarRef(_)
        )
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Undefined => false,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0 && !f.is_nan(),
            Value::Str(s) => !s.is_empty() && s != "0",
            Value::List(_) => true,
            Value::Hash(_) => true,
            Value::Function(_) => true,
            Value::Object(o) => o.is_truthy(),
            Value::Exception(_) => true,
            Value::ScalarRef(_) => true,
        }
    }

    pub fn to_tt_string(&self) -> String {
        match self {
            Value::Undefined => String::new(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => format_perl_float(*f),
            Value::Str(s) => s.clone(),
            Value::List(l) => format!("ARRAY(0x{:x})", l.as_ptr() as usize),
            Value::Hash(h) => format!("HASH(0x{:x})", h.as_ptr() as usize),
            Value::Function(f) => format!("CODE(0x{:x})", Rc::as_ptr(f) as *const () as usize),
            Value::Object(o) => o.to_tt_string(),
            Value::Exception(e) => e.borrow().as_string(),
            Value::ScalarRef(s) => format!("SCALAR(0x{:x})", s.as_ptr() as usize),
        }
    }

    pub fn to_str(&self) -> String {
        self.to_tt_string()
    }

    pub fn parse_perl_number(s: &str) -> (f64, bool) {
        let trimmed = s.trim_start();
        if trimmed.is_empty() {
            return (0.0, false);
        }
        if let Ok(n) = s.trim().parse::<f64>() {
            return (n, true);
        }
        let mut end = 0;
        let bytes = trimmed.as_bytes();
        if end < bytes.len() && (bytes[end] == b'+' || bytes[end] == b'-') {
            end += 1;
        }
        let mut has_digits = false;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            has_digits = true;
            end += 1;
        }
        if end < bytes.len() && bytes[end] == b'.' {
            end += 1;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                has_digits = true;
                end += 1;
            }
        }
        if has_digits && end < bytes.len() && (bytes[end] == b'e' || bytes[end] == b'E') {
            let exp_start = end;
            end += 1;
            if end < bytes.len() && (bytes[end] == b'+' || bytes[end] == b'-') {
                end += 1;
            }
            let mut exp_digits = false;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                exp_digits = true;
                end += 1;
            }
            if !exp_digits {
                end = exp_start;
            }
        }
        if has_digits {
            if let Ok(n) = trimmed[..end].parse::<f64>() {
                return (n, end == trimmed.trim_end().len());
            }
        }
        (0.0, false)
    }

    pub fn to_number(&self) -> f64 {
        self.to_number_checked().0
    }

    pub fn to_number_checked(&self) -> (f64, bool) {
        match self {
            Value::Undefined => (0.0, false),
            Value::Int(i) => (*i as f64, true),
            Value::Float(f) => (*f, true),
            Value::Str(s) => Self::parse_perl_number(s),
            Value::List(l) => (l.borrow().len() as f64, true),
            Value::Hash(h) => (h.borrow().len() as f64, true),
            Value::Function(f) => (Rc::as_ptr(f) as *const () as usize as f64, true),
            Value::Object(o) => Self::parse_perl_number(&o.to_tt_string()),
            Value::Exception(e) => Self::parse_perl_number(&e.borrow().as_string()),
            Value::ScalarRef(s) => (s.as_ptr() as usize as f64, true),
        }
    }

    pub fn to_int(&self) -> i64 {
        match self {
            Value::Int(i) => *i,
            _ => self.to_number() as i64,
        }
    }

    pub fn ref_type(&self) -> String {
        match self {
            Value::Undefined | Value::Int(_) | Value::Float(_) | Value::Str(_) => String::new(),
            Value::List(_) => "ARRAY".to_string(),
            Value::Hash(_) => "HASH".to_string(),
            Value::Function(f) => f.type_name().to_string(),
            Value::Object(o) => o.type_name().to_string(),
            Value::Exception(_) => "Template::Exception".to_string(),
            Value::ScalarRef(_) => "SCALAR".to_string(),
        }
    }

    pub fn seq(&self, other: &Value) -> bool {
        if let Value::Object(o) = self {
            if let Some(res) = o.equals_value(other) {
                return res;
            }
        }
        if let Value::Object(o) = other {
            if let Some(res) = o.equals_value(self) {
                return res;
            }
        }
        self.to_tt_string() == other.to_tt_string()
    }

    pub fn str_eq(&self, other: &Value) -> bool {
        self.seq(other)
    }

    pub fn scomp(&self, other: &Value) -> Ordering {
        self.to_tt_string().cmp(&other.to_tt_string())
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::Str(s.to_string())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::Str(s)
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Int(i)
    }
}

impl From<i32> for Value {
    fn from(i: i32) -> Self {
        Value::Int(i as i64)
    }
}

impl From<usize> for Value {
    fn from(i: usize) -> Self {
        Value::Int(i as i64)
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Int(if b { 1 } else { 0 })
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(v) => v.into(),
            None => Value::Undefined,
        }
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        Value::list(v.into_iter().map(Into::into).collect())
    }
}

impl<K: Into<String>, V: Into<Value>> From<HashMap<K, V>> for Value {
    fn from(m: HashMap<K, V>) -> Self {
        Value::hash(m.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
    }
}

impl From<TemplateException> for Value {
    fn from(e: TemplateException) -> Self {
        Value::Object(Rc::new(e))
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Undefined, Value::Undefined) => true,
            (Value::Undefined, _) | (_, Value::Undefined) => false,
            _ => self.seq(other),
        }
    }
}
