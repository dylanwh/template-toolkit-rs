//! Typed arguments and return values for Rust callbacks, so functions,
//! vmethods, filter factories and plugins can be ordinary Rust closures:
//!
//! ```
//! use template_toolkit::Template;
//!
//! let tt = Template::new();
//! tt.define_function("add", |a: i64, b: i64| a + b);
//! tt.define_vmethod("scalar", "pad", |s: String, width: Option<i64>| {
//!     format!("{:>1$}", s, width.unwrap_or(8) as usize)
//! });
//! assert_eq!(tt.process_text("[% add('3', 4) %]|[% s = 'x'; s.pad(3) %]", None).unwrap(), "7|  x");
//! ```
//!
//! Modelled on mlua's `FromLua`/`FromLuaMulti`/`IntoLuaMulti` traits.
//! Template values are Perl-ish, so scalar conversions coerce the way TT
//! does (`"3"` is the number 3, undefined is `""` or `0`); only structural
//! mismatches, such as a hash where a list is expected, are errors, and they
//! are reported as ``bad argument #2 to `name`: expected list, got HASH``.

use std::{collections::HashMap, rc::Rc};

use crate::{
    error::{ControlFlow, TemplateException},
    filters::{Filter, FilterFn},
    value::Value,
};

/// Why a template value could not be converted.
#[derive(Debug, Clone, PartialEq)]
pub enum ConvError {
    /// The wrong kind of value; names what was expected (e.g. `"list"`).
    Expected(&'static str),
    /// Any other failure, such as a serde "missing field" error.
    Message(String),
}

impl From<&'static str> for ConvError {
    fn from(expected: &'static str) -> Self {
        ConvError::Expected(expected)
    }
}

/// Conversion from a template value to a Rust argument type.
pub trait FromValue: Sized {
    fn from_value(value: &Value) -> Result<Self, ConvError>;
}

impl FromValue for Value {
    fn from_value(value: &Value) -> Result<Self, ConvError> {
        Ok(value.clone())
    }
}

impl FromValue for String {
    fn from_value(value: &Value) -> Result<Self, ConvError> {
        Ok(value.to_tt_string())
    }
}

impl FromValue for bool {
    fn from_value(value: &Value) -> Result<Self, ConvError> {
        Ok(value.is_truthy())
    }
}

impl FromValue for f64 {
    fn from_value(value: &Value) -> Result<Self, ConvError> {
        Ok(value.to_number())
    }
}

macro_rules! from_value_int {
    ($($t:ty),*) => {$(
        impl FromValue for $t {
            fn from_value(value: &Value) -> Result<Self, ConvError> {
                <$t>::try_from(value.to_int())
                    .map_err(|_| ConvError::Expected(concat!("an integer in range for ", stringify!($t))))
            }
        }
    )*};
}
from_value_int!(i64, i32, u32, usize);

/// Missing or undefined arguments are `None`.
impl<T: FromValue> FromValue for Option<T> {
    fn from_value(value: &Value) -> Result<Self, ConvError> {
        if value.is_defined() {
            T::from_value(value).map(Some)
        } else {
            Ok(None)
        }
    }
}

impl<T: FromValue> FromValue for Vec<T> {
    fn from_value(value: &Value) -> Result<Self, ConvError> {
        match value {
            Value::List(items) => items.borrow().iter().map(T::from_value).collect(),
            _ => Err(ConvError::Expected("list")),
        }
    }
}

impl<T: FromValue> FromValue for HashMap<String, T> {
    fn from_value(value: &Value) -> Result<Self, ConvError> {
        match value {
            Value::Hash(map) => map
                .borrow()
                .iter()
                .map(|(k, v)| Ok((k.clone(), T::from_value(v)?)))
                .collect(),
            _ => Err(ConvError::Expected("hash")),
        }
    }
}

/// Conversion from a Rust return value to a template value.
pub trait IntoValue {
    fn into_value(self) -> Value;
}

impl IntoValue for Value {
    fn into_value(self) -> Value {
        self
    }
}

impl IntoValue for String {
    fn into_value(self) -> Value {
        Value::Str(self)
    }
}

impl IntoValue for &str {
    fn into_value(self) -> Value {
        Value::Str(self.to_string())
    }
}

/// Perl truth values: 1 and "".
impl IntoValue for bool {
    fn into_value(self) -> Value {
        if self {
            Value::Int(1)
        } else {
            Value::Str(String::new())
        }
    }
}

impl IntoValue for f64 {
    fn into_value(self) -> Value {
        Value::Float(self)
    }
}

macro_rules! into_value_int {
    ($($t:ty),*) => {$(
        impl IntoValue for $t {
            fn into_value(self) -> Value {
                Value::Int(self as i64)
            }
        }
    )*};
}
into_value_int!(i64, i32, u32, usize);

/// `()` returns nothing: the call renders as an empty string.
impl IntoValue for () {
    fn into_value(self) -> Value {
        Value::Str(String::new())
    }
}

impl<T: IntoValue> IntoValue for Option<T> {
    fn into_value(self) -> Value {
        self.map_or(Value::Undefined, IntoValue::into_value)
    }
}

impl<T: IntoValue> IntoValue for Vec<T> {
    fn into_value(self) -> Value {
        Value::list(self.into_iter().map(IntoValue::into_value).collect())
    }
}

impl<T: IntoValue> IntoValue for HashMap<String, T> {
    fn into_value(self) -> Value {
        Value::hash(self.into_iter().map(|(k, v)| (k, v.into_value())).collect())
    }
}

/// A callback's return: a value, or `Result<_, ControlFlow>` whose error
/// becomes a template exception.  `Err("message".into())` raises an `undef`
/// exception, like a Perl `die` in a code ref.
pub trait IntoResult {
    fn into_result(self) -> Result<Value, ControlFlow>;
}

impl<T: IntoValue> IntoResult for T {
    fn into_result(self) -> Result<Value, ControlFlow> {
        Ok(self.into_value())
    }
}

// The error type is fixed (not `E: Into<ControlFlow>`) so that a closure
// returning `Ok(x)` still infers it; `Err("message".into())` works via the
// `From<&str>`/`From<String>` impls below.
impl<T: IntoValue> IntoResult for Result<T, ControlFlow> {
    fn into_result(self) -> Result<Value, ControlFlow> {
        self.map(IntoValue::into_value)
    }
}

/// The remaining arguments, as the last parameter: `|first: String, rest: Rest<i64>|`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Rest<T>(pub Vec<T>);

/// A short description of a value's kind for error messages: `ARRAY`,
/// `HASH`, an object's type, `undef` or `scalar`.
pub(crate) fn kind_of(value: &Value) -> String {
    match value.ref_type() {
        t if !t.is_empty() => t,
        _ if !value.is_defined() => "undef".to_string(),
        _ => "scalar".to_string(),
    }
}

fn bad_argument(pos: usize, fname: &str, error: ConvError, got: &Value) -> ControlFlow {
    let detail = match error {
        ConvError::Expected(expected) => format!("expected {expected}, got {}", kind_of(got)),
        ConvError::Message(message) => message,
    };
    ControlFlow::Exception(TemplateException::new(
        "undef",
        format!("bad argument #{pos} to `{fname}`: {detail}"),
    ))
}

/// One parameter of a typed callback, taken from the argument list.
pub trait FromArg: Sized {
    /// Take this parameter from `args[*pos..]`, advancing `pos`.
    fn from_arg(args: &[Value], pos: &mut usize, fname: &str) -> Result<Self, ControlFlow>;
}

impl<T: FromValue> FromArg for T {
    fn from_arg(args: &[Value], pos: &mut usize, fname: &str) -> Result<Self, ControlFlow> {
        let value = args.get(*pos).unwrap_or(&Value::Undefined);
        *pos += 1;
        T::from_value(value).map_err(|e| bad_argument(*pos, fname, e, value))
    }
}

impl<T: FromValue> FromArg for Rest<T> {
    fn from_arg(args: &[Value], pos: &mut usize, fname: &str) -> Result<Self, ControlFlow> {
        let rest = args.get(*pos..).unwrap_or(&[]);
        let mut out = Vec::with_capacity(rest.len());
        for value in rest {
            *pos += 1;
            out.push(T::from_value(value).map_err(|e| bad_argument(*pos, fname, e, value))?);
        }
        Ok(Rest(out))
    }
}

/// A callback's parameter list.  Missing arguments are undefined and extra
/// ones are ignored, as in Perl.
pub trait FromArgs: Sized {
    fn from_args(args: &[Value], fname: &str) -> Result<Self, ControlFlow>;
}

/// The text a filter returns: a `String`, or a `Result` whose error becomes
/// a template exception.
pub trait IntoText {
    fn into_text(self) -> Result<String, ControlFlow>;
}

impl IntoText for String {
    fn into_text(self) -> Result<String, ControlFlow> {
        Ok(self)
    }
}

impl IntoText for Result<String, ControlFlow> {
    fn into_text(self) -> Result<String, ControlFlow> {
        self
    }
}

/// What a filter factory returns: a filter, or a `Result` of one.
pub trait IntoFilter {
    fn into_filter(self) -> Result<FilterFn, ControlFlow>;
}

impl<F: Filter + 'static> IntoFilter for F {
    fn into_filter(self) -> Result<FilterFn, ControlFlow> {
        Ok(Rc::new(self))
    }
}

impl<F: Filter + 'static> IntoFilter for Result<F, ControlFlow> {
    fn into_filter(self) -> Result<FilterFn, ControlFlow> {
        self.map(|f| Rc::new(f) as FilterFn)
    }
}

/// A Rust function or closure callable with typed arguments `A` (a tuple).
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a typed template callback",
    label = "annotate the closure's parameter types, e.g. `|a: i64, b: String|`",
    note = "parameters must implement `FromValue` (or be `Rest<T>` last)"
)]
pub trait NativeFn<A> {
    type Output;
    fn call_native(&self, args: A) -> Self::Output;
}

macro_rules! native_fn {
    ($($t:ident),*) => {
        impl<$($t: FromArg),*> FromArgs for ($($t,)*) {
            #[allow(unused_variables, unused_mut, clippy::unused_unit)]
            fn from_args(args: &[Value], fname: &str) -> Result<Self, ControlFlow> {
                let mut pos = 0;
                Ok(($($t::from_arg(args, &mut pos, fname)?,)*))
            }
        }

        impl<Func, Ret, $($t),*> NativeFn<($($t,)*)> for Func
        where
            Func: Fn($($t),*) -> Ret,
        {
            type Output = Ret;
            #[allow(non_snake_case)]
            fn call_native(&self, ($($t,)*): ($($t,)*)) -> Ret {
                self($($t),*)
            }
        }
    };
}
native_fn!();
native_fn!(A1);
native_fn!(A1, A2);
native_fn!(A1, A2, A3);
native_fn!(A1, A2, A3, A4);
native_fn!(A1, A2, A3, A4, A5);
native_fn!(A1, A2, A3, A4, A5, A6);
native_fn!(A1, A2, A3, A4, A5, A6, A7);
native_fn!(A1, A2, A3, A4, A5, A6, A7, A8);

/// Decode `args` and call `f`; the glue behind the typed `define_*` methods.
pub fn call_typed<A: FromArgs, F: NativeFn<A>>(
    f: &F,
    args: &[Value],
    fname: &str,
) -> Result<F::Output, ControlFlow> {
    Ok(f.call_native(A::from_args(args, fname)?))
}

impl From<TemplateException> for ControlFlow {
    fn from(e: TemplateException) -> Self {
        ControlFlow::Exception(e)
    }
}

impl From<String> for ControlFlow {
    fn from(message: String) -> Self {
        ControlFlow::Exception(TemplateException::new("undef", message))
    }
}

impl From<&str> for ControlFlow {
    fn from(message: &str) -> Self {
        message.to_string().into()
    }
}
