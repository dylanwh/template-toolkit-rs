//! serde support: Rust data in and out of templates.
//!
//! ```
//! use serde::{Deserialize, Serialize};
//! use template_toolkit::serde_value::{to_vars, Serde};
//! use template_toolkit::Template;
//!
//! #[derive(Serialize)]
//! struct Page { title: String, tags: Vec<String>, draft: bool }
//!
//! #[derive(Deserialize)]
//! struct Opts { width: usize, fill: Option<char> }
//!
//! let tt = Template::new();
//! tt.define_function("pad", |s: String, opts: Serde<Opts>| {
//!     let fill = opts.0.fill.unwrap_or(' ').to_string();
//!     format!("{}{s}", fill.repeat(opts.0.width.saturating_sub(s.len())))
//! });
//! let page = Page { title: "Hi".into(), tags: vec!["a".into(), "b".into()], draft: false };
//! let out = tt
//!     .process_text("[% pad(title, { width => '4', fill => '.' }) %] [% tags.join(',') %][% ' (draft)' IF draft %]",
//!                   Some(to_vars(&page).unwrap()))
//!     .unwrap();
//! assert_eq!(out, "..Hi a,b");
//! ```
//!
//! Serializing maps Rust data onto template values the way Perl data would
//! look: structs and maps become hashes, sequences and tuples lists, `None`
//! and `()` undefined, `bool` Perl's `1` / `""`, unit enum variants their
//! name, and other enum variants a one-key hash `{ Variant => ... }`
//! (serde's externally tagged form).  Integers outside `i64` become decimal
//! strings, which keeps them exact.
//!
//! Deserializing applies the same Perl coercions as [`FromValue`]: a
//! template's `'8'` is a fine `usize`, undefined is `None`, and a scalar
//! where a sequence is expected is a one-element list (as `FOREACH` treats
//! it).  Structural mismatches, such as a list for a struct, are errors.

use std::{collections::HashMap, fmt};

use serde::{
    de::{self, DeserializeOwned, IntoDeserializer, Visitor},
    ser::{self, Serialize},
};

use crate::{
    convert::{kind_of, ConvError, FromValue, IntoResult},
    error::{ControlFlow, TemplateException},
    value::Value,
};

/// Serialization or deserialization failure.
#[derive(Debug, Clone, PartialEq)]
pub struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error(msg.to_string())
    }
}

impl From<Error> for ControlFlow {
    fn from(e: Error) -> Self {
        ControlFlow::Exception(TemplateException::new("undef", e.0))
    }
}

/// Convert any `Serialize` value to a template value.
pub fn to_value<T: Serialize + ?Sized>(value: &T) -> Result<Value, Error> {
    value.serialize(ValueSerializer)
}

/// Convert a struct or map into template variables, for `process`.
pub fn to_vars<T: Serialize + ?Sized>(value: &T) -> Result<HashMap<String, Value>, Error> {
    match to_value(value)? {
        Value::Hash(map) => Ok(map.borrow().clone()),
        other => Err(Error(format!(
            "template variables must be a struct or map, got {}",
            kind_of(&other)
        ))),
    }
}

/// Convert a template value to any `Deserialize` type.
pub fn from_value<T: DeserializeOwned>(value: &Value) -> Result<T, Error> {
    T::deserialize(ValueDeserializer(value.clone()))
}

/// Wraps a serde type as a typed-callback argument or return value:
/// `|opts: Serde<Opts>| ...` deserializes the argument, and returning
/// `Serde(report)` serializes the result.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Serde<T>(pub T);

impl<T: DeserializeOwned> FromValue for Serde<T> {
    fn from_value(value: &Value) -> Result<Self, ConvError> {
        from_value(value)
            .map(Serde)
            .map_err(|e| ConvError::Message(e.0))
    }
}

impl<T: Serialize> IntoResult for Serde<T> {
    fn into_result(self) -> Result<Value, ControlFlow> {
        to_value(&self.0).map_err(Into::into)
    }
}

fn int_value<T: TryInto<i64> + fmt::Display + Copy>(n: T) -> Value {
    match n.try_into() {
        Ok(i) => Value::Int(i),
        Err(_) => Value::Str(n.to_string()),
    }
}

fn hash_of(pairs: Vec<(String, Value)>) -> Value {
    Value::hash(pairs.into_iter().collect())
}

// ---------------------------------------------------------------------------
// Serializer: Rust -> Value
// ---------------------------------------------------------------------------

struct ValueSerializer;

impl ser::Serializer for ValueSerializer {
    type Ok = Value;
    type Error = Error;
    type SerializeSeq = SeqBuilder;
    type SerializeTuple = SeqBuilder;
    type SerializeTupleStruct = SeqBuilder;
    type SerializeTupleVariant = SeqBuilder;
    type SerializeMap = MapBuilder;
    type SerializeStruct = MapBuilder;
    type SerializeStructVariant = MapBuilder;

    fn serialize_bool(self, v: bool) -> Result<Value, Error> {
        Ok(if v {
            Value::Int(1)
        } else {
            Value::Str(String::new())
        })
    }
    fn serialize_i8(self, v: i8) -> Result<Value, Error> {
        Ok(Value::Int(v.into()))
    }
    fn serialize_i16(self, v: i16) -> Result<Value, Error> {
        Ok(Value::Int(v.into()))
    }
    fn serialize_i32(self, v: i32) -> Result<Value, Error> {
        Ok(Value::Int(v.into()))
    }
    fn serialize_i64(self, v: i64) -> Result<Value, Error> {
        Ok(Value::Int(v))
    }
    fn serialize_i128(self, v: i128) -> Result<Value, Error> {
        Ok(int_value(v))
    }
    fn serialize_u8(self, v: u8) -> Result<Value, Error> {
        Ok(Value::Int(v.into()))
    }
    fn serialize_u16(self, v: u16) -> Result<Value, Error> {
        Ok(Value::Int(v.into()))
    }
    fn serialize_u32(self, v: u32) -> Result<Value, Error> {
        Ok(Value::Int(v.into()))
    }
    fn serialize_u64(self, v: u64) -> Result<Value, Error> {
        Ok(int_value(v))
    }
    fn serialize_u128(self, v: u128) -> Result<Value, Error> {
        Ok(int_value(v))
    }
    fn serialize_f32(self, v: f32) -> Result<Value, Error> {
        Ok(Value::Float(v.into()))
    }
    fn serialize_f64(self, v: f64) -> Result<Value, Error> {
        Ok(Value::Float(v))
    }
    fn serialize_char(self, v: char) -> Result<Value, Error> {
        Ok(Value::Str(v.to_string()))
    }
    fn serialize_str(self, v: &str) -> Result<Value, Error> {
        Ok(Value::Str(v.to_string()))
    }
    /// Perl strings are byte strings; non-UTF-8 bytes are replaced.
    fn serialize_bytes(self, v: &[u8]) -> Result<Value, Error> {
        Ok(Value::Str(String::from_utf8_lossy(v).into_owned()))
    }
    fn serialize_none(self) -> Result<Value, Error> {
        Ok(Value::Undefined)
    }
    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<Value, Error> {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<Value, Error> {
        Ok(Value::Undefined)
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Value, Error> {
        Ok(Value::Undefined)
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
    ) -> Result<Value, Error> {
        Ok(Value::Str(variant.to_string()))
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Value, Error> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Value, Error> {
        Ok(hash_of(vec![(variant.to_string(), value.serialize(self)?)]))
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<SeqBuilder, Error> {
        Ok(SeqBuilder {
            items: Vec::with_capacity(len.unwrap_or(0)),
            variant: None,
        })
    }
    fn serialize_tuple(self, len: usize) -> Result<SeqBuilder, Error> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_struct(self, _name: &'static str, len: usize) -> Result<SeqBuilder, Error> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<SeqBuilder, Error> {
        Ok(SeqBuilder {
            items: Vec::with_capacity(len),
            variant: Some(variant),
        })
    }
    fn serialize_map(self, len: Option<usize>) -> Result<MapBuilder, Error> {
        Ok(MapBuilder {
            entries: Vec::with_capacity(len.unwrap_or(0)),
            key: None,
            variant: None,
        })
    }
    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<MapBuilder, Error> {
        self.serialize_map(Some(len))
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<MapBuilder, Error> {
        Ok(MapBuilder {
            entries: Vec::with_capacity(len),
            key: None,
            variant: Some(variant),
        })
    }
}

struct SeqBuilder {
    items: Vec<Value>,
    /// Set for tuple variants, which serialize as `{ Variant => [...] }`.
    variant: Option<&'static str>,
}

impl SeqBuilder {
    fn push<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.items.push(to_value(value)?);
        Ok(())
    }

    fn finish(self) -> Result<Value, Error> {
        let list = Value::list(self.items);
        Ok(match self.variant {
            Some(variant) => hash_of(vec![(variant.to_string(), list)]),
            None => list,
        })
    }
}

impl ser::SerializeSeq for SeqBuilder {
    type Ok = Value;
    type Error = Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.push(value)
    }
    fn end(self) -> Result<Value, Error> {
        self.finish()
    }
}

impl ser::SerializeTuple for SeqBuilder {
    type Ok = Value;
    type Error = Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.push(value)
    }
    fn end(self) -> Result<Value, Error> {
        self.finish()
    }
}

impl ser::SerializeTupleStruct for SeqBuilder {
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.push(value)
    }
    fn end(self) -> Result<Value, Error> {
        self.finish()
    }
}

impl ser::SerializeTupleVariant for SeqBuilder {
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.push(value)
    }
    fn end(self) -> Result<Value, Error> {
        self.finish()
    }
}

struct MapBuilder {
    entries: Vec<(String, Value)>,
    key: Option<String>,
    /// Set for struct variants, which serialize as `{ Variant => {...} }`.
    variant: Option<&'static str>,
}

impl MapBuilder {
    fn finish(self) -> Result<Value, Error> {
        let hash = hash_of(self.entries);
        Ok(match self.variant {
            Some(variant) => hash_of(vec![(variant.to_string(), hash)]),
            None => hash,
        })
    }
}

impl ser::SerializeMap for MapBuilder {
    type Ok = Value;
    type Error = Error;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Error> {
        self.key = Some(key.serialize(KeySerializer)?);
        Ok(())
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        let key = self
            .key
            .take()
            .ok_or_else(|| Error("map value without a key".into()))?;
        self.entries.push((key, to_value(value)?));
        Ok(())
    }
    fn end(self) -> Result<Value, Error> {
        self.finish()
    }
}

impl ser::SerializeStruct for MapBuilder {
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        self.entries.push((key.to_string(), to_value(value)?));
        Ok(())
    }
    fn end(self) -> Result<Value, Error> {
        self.finish()
    }
}

impl ser::SerializeStructVariant for MapBuilder {
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        self.entries.push((key.to_string(), to_value(value)?));
        Ok(())
    }
    fn end(self) -> Result<Value, Error> {
        self.finish()
    }
}

/// Hash keys are strings, as in Perl: strings, chars, numbers, bools and
/// unit variants are accepted and stringified.
struct KeySerializer;

fn key_error() -> Error {
    Error("map keys must be strings, numbers, chars, bools or unit enum variants".into())
}

impl ser::Serializer for KeySerializer {
    type Ok = String;
    type Error = Error;
    type SerializeSeq = ser::Impossible<String, Error>;
    type SerializeTuple = ser::Impossible<String, Error>;
    type SerializeTupleStruct = ser::Impossible<String, Error>;
    type SerializeTupleVariant = ser::Impossible<String, Error>;
    type SerializeMap = ser::Impossible<String, Error>;
    type SerializeStruct = ser::Impossible<String, Error>;
    type SerializeStructVariant = ser::Impossible<String, Error>;

    fn serialize_bool(self, v: bool) -> Result<String, Error> {
        Ok(if v { "1".into() } else { String::new() })
    }
    fn serialize_i8(self, v: i8) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_i16(self, v: i16) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_i32(self, v: i32) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_i64(self, v: i64) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_i128(self, v: i128) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_u8(self, v: u8) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_u16(self, v: u16) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_u32(self, v: u32) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_u64(self, v: u64) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_u128(self, v: u128) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_f32(self, v: f32) -> Result<String, Error> {
        Ok(Value::Float(v.into()).to_tt_string())
    }
    fn serialize_f64(self, v: f64) -> Result<String, Error> {
        Ok(Value::Float(v).to_tt_string())
    }
    fn serialize_char(self, v: char) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_str(self, v: &str) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<String, Error> {
        Ok(String::from_utf8_lossy(v).into_owned())
    }
    fn serialize_none(self) -> Result<String, Error> {
        Err(key_error())
    }
    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<String, Error> {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<String, Error> {
        Err(key_error())
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<String, Error> {
        Err(key_error())
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
    ) -> Result<String, Error> {
        Ok(variant.to_string())
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<String, Error> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<String, Error> {
        Err(key_error())
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Error> {
        Err(key_error())
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Error> {
        Err(key_error())
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Error> {
        Err(key_error())
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Error> {
        Err(key_error())
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Error> {
        Err(key_error())
    }
    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Error> {
        Err(key_error())
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Error> {
        Err(key_error())
    }
}

// ---------------------------------------------------------------------------
// Deserializer: Value -> Rust
// ---------------------------------------------------------------------------

/// Deserializes from a template value.  It owns a (cheap, `Rc`-sharing)
/// clone so lists and hashes can be walked without holding `RefCell`
/// borrows across visitor calls.
pub struct ValueDeserializer(pub Value);

impl ValueDeserializer {
    fn invalid(&self, expected: &str) -> Error {
        Error(format!("expected {expected}, got {}", kind_of(&self.0)))
    }

    /// A hash, or an object that exposes one.
    fn as_entries(&self) -> Option<Vec<(String, Value)>> {
        match &self.0 {
            Value::Hash(map) => Some(
                map.borrow()
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            ),
            Value::Object(obj) => obj.as_hash().map(|map| {
                map.borrow()
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            }),
            _ => None,
        }
    }

    /// A list, or an object that exposes one.
    fn as_items(&self) -> Option<Vec<Value>> {
        match &self.0 {
            Value::List(items) => Some(items.borrow().clone()),
            Value::Object(obj) => obj.as_list().map(|items| items.borrow().clone()),
            _ => None,
        }
    }

    fn is_scalar(&self) -> bool {
        matches!(
            self.0,
            Value::Undefined
                | Value::Int(_)
                | Value::Float(_)
                | Value::Str(_)
                | Value::ScalarRef(_)
        )
    }

    /// A scalar as an integer, with Perl's numeric coercion.
    fn int(&self) -> Result<i64, Error> {
        if self.is_scalar() {
            Ok(self.0.to_int())
        } else {
            Err(self.invalid("an integer"))
        }
    }
}

macro_rules! deserialize_signed {
    ($($method:ident => $visit:ident: $t:ty),*) => {$(
        fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
            let n = self.int()?;
            visitor.$visit(<$t>::try_from(n).map_err(|_| Error(format!("{n} is out of range for {}", stringify!($t))))?)
        }
    )*};
}

macro_rules! deserialize_unsigned {
    ($($method:ident => $visit:ident: $t:ty),*) => {$(
        fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
            // Large values arrive as decimal strings (see `to_value`).
            if let Value::Str(s) = &self.0 {
                if let Ok(n) = s.trim().parse::<$t>() {
                    return visitor.$visit(n);
                }
            }
            let n = self.int()?;
            visitor.$visit(<$t>::try_from(n).map_err(|_| Error(format!("{n} is out of range for {}", stringify!($t))))?)
        }
    )*};
}

impl<'de> de::Deserializer<'de> for ValueDeserializer {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match &self.0 {
            Value::Undefined => visitor.visit_unit(),
            Value::Int(n) => visitor.visit_i64(*n),
            Value::Float(f) => visitor.visit_f64(*f),
            Value::Str(s) => visitor.visit_str(s),
            Value::ScalarRef(s) => visitor.visit_string(s.borrow().clone()),
            Value::List(_) => self.deserialize_seq(visitor),
            Value::Hash(_) => self.deserialize_map(visitor),
            Value::Object(_) if self.as_entries().is_some() => self.deserialize_map(visitor),
            Value::Object(_) if self.as_items().is_some() => self.deserialize_seq(visitor),
            Value::Object(_) | Value::Exception(_) => visitor.visit_string(self.0.to_tt_string()),
            Value::Function(_) => Err(Error("a function cannot be deserialized".into())),
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_bool(self.0.is_truthy())
    }

    deserialize_signed!(
        deserialize_i8 => visit_i8: i8,
        deserialize_i16 => visit_i16: i16,
        deserialize_i32 => visit_i32: i32,
        deserialize_i64 => visit_i64: i64
    );
    deserialize_unsigned!(
        deserialize_u8 => visit_u8: u8,
        deserialize_u16 => visit_u16: u16,
        deserialize_u32 => visit_u32: u32,
        deserialize_u64 => visit_u64: u64
    );

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_f64(visitor)
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        if self.is_scalar() {
            visitor.visit_f64(self.0.to_number())
        } else {
            Err(self.invalid("a number"))
        }
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        let s = self.0.to_tt_string();
        let mut chars = s.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) if self.is_scalar() => visitor.visit_char(c),
            _ => Err(Error(format!("expected a single character, got {s:?}"))),
        }
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_string(visitor)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        if self.is_scalar() || matches!(self.0, Value::Object(_) | Value::Exception(_)) {
            visitor.visit_string(self.0.to_tt_string())
        } else {
            Err(self.invalid("a string"))
        }
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_byte_buf(visitor)
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        if self.is_scalar() {
            visitor.visit_byte_buf(self.0.to_tt_string().into_bytes())
        } else {
            Err(self.invalid("a string"))
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        if self.0.is_defined() {
            visitor.visit_some(self)
        } else {
            visitor.visit_none()
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        visitor.visit_newtype_struct(self)
    }

    /// Lists as-is; undefined is empty and any other scalar a one-element
    /// list, as `FOREACH` treats them.
    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        let items = match self.as_items() {
            Some(items) => items,
            None if !self.0.is_defined() => Vec::new(),
            None if self.is_scalar() => vec![self.0.clone()],
            None => return Err(self.invalid("a list")),
        };
        visitor.visit_seq(SeqAccess(items.into_iter()))
    }

    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.as_entries() {
            Some(entries) => visitor.visit_map(MapAccess {
                entries: entries.into_iter(),
                value: None,
            }),
            None => Err(self.invalid("a hash")),
        }
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_map(visitor)
    }

    /// A string names a unit variant; a one-key hash `{ Variant => ... }`
    /// any variant.
    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        if self.is_scalar() {
            return visitor.visit_enum(self.0.to_tt_string().into_deserializer());
        }
        match self.as_entries() {
            Some(mut entries) if entries.len() == 1 => {
                let (variant, value) = entries.pop().unwrap();
                visitor.visit_enum(EnumAccess { variant, value })
            }
            _ => Err(self.invalid("an enum variant name or a one-key hash")),
        }
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_string(visitor)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_unit()
    }
}

struct SeqAccess(std::vec::IntoIter<Value>);

impl<'de> de::SeqAccess<'de> for SeqAccess {
    type Error = Error;

    fn next_element_seed<T: de::DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Error> {
        self.0
            .next()
            .map(|v| seed.deserialize(ValueDeserializer(v)))
            .transpose()
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.0.len())
    }
}

struct MapAccess {
    entries: std::vec::IntoIter<(String, Value)>,
    value: Option<Value>,
}

impl<'de> de::MapAccess<'de> for MapAccess {
    type Error = Error;

    fn next_key_seed<K: de::DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Error> {
        match self.entries.next() {
            Some((key, value)) => {
                self.value = Some(value);
                // Keys deserialize like any scalar, so numeric keys work too.
                seed.deserialize(ValueDeserializer(Value::Str(key)))
                    .map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V: de::DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, Error> {
        let value = self
            .value
            .take()
            .ok_or_else(|| Error("map value without a key".into()))?;
        seed.deserialize(ValueDeserializer(value))
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.entries.len())
    }
}

struct EnumAccess {
    variant: String,
    value: Value,
}

impl<'de> de::EnumAccess<'de> for EnumAccess {
    type Error = Error;
    type Variant = VariantAccess;

    fn variant_seed<V: de::DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, VariantAccess), Error> {
        let variant = seed.deserialize(ValueDeserializer(Value::Str(self.variant)))?;
        Ok((variant, VariantAccess(self.value)))
    }
}

struct VariantAccess(Value);

impl<'de> de::VariantAccess<'de> for VariantAccess {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        Ok(())
    }

    fn newtype_variant_seed<T: de::DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Error> {
        seed.deserialize(ValueDeserializer(self.0))
    }

    fn tuple_variant<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value, Error> {
        de::Deserializer::deserialize_seq(ValueDeserializer(self.0), visitor)
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        de::Deserializer::deserialize_map(ValueDeserializer(self.0), visitor)
    }
}

impl<'de> IntoDeserializer<'de, Error> for Value {
    type Deserializer = ValueDeserializer;

    fn into_deserializer(self) -> ValueDeserializer {
        ValueDeserializer(self)
    }
}

// ---------------------------------------------------------------------------
// Value itself: Serialize (e.g. to JSON) and Deserialize (e.g. from JSON)
// ---------------------------------------------------------------------------

/// Hashes serialize with sorted keys, so output is deterministic.  Functions
/// and objects without hash or list views serialize as their string form.
impl Serialize for Value {
    fn serialize<S: ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use ser::{SerializeMap, SerializeSeq};
        let seq = |items: &[Value], serializer: S| {
            let mut s = serializer.serialize_seq(Some(items.len()))?;
            for item in items {
                s.serialize_element(item)?;
            }
            s.end()
        };
        let map = |entries: &HashMap<String, Value>, serializer: S| {
            let mut keys: Vec<&String> = entries.keys().collect();
            keys.sort();
            let mut m = serializer.serialize_map(Some(keys.len()))?;
            for k in keys {
                m.serialize_entry(k, &entries[k])?;
            }
            m.end()
        };
        match self {
            Value::Undefined => serializer.serialize_none(),
            Value::Int(n) => serializer.serialize_i64(*n),
            Value::Float(f) => serializer.serialize_f64(*f),
            Value::Str(s) => serializer.serialize_str(s),
            Value::ScalarRef(s) => serializer.serialize_str(&s.borrow()),
            Value::List(items) => seq(&items.borrow(), serializer),
            Value::Hash(entries) => map(&entries.borrow(), serializer),
            Value::Object(obj) => {
                if let Some(entries) = obj.as_hash() {
                    map(&entries.borrow(), serializer)
                } else if let Some(items) = obj.as_list() {
                    seq(&items.borrow(), serializer)
                } else {
                    serializer.serialize_str(&obj.to_tt_string())
                }
            }
            Value::Exception(_) | Value::Function(_) => {
                serializer.serialize_str(&self.to_tt_string())
            }
        }
    }
}

impl<'de> de::Deserialize<'de> for Value {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Value, D::Error> {
        deserializer.deserialize_any(ValueVisitor)
    }
}

struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("any template value")
    }
    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Value, E> {
        Ok(if v {
            Value::Int(1)
        } else {
            Value::Str(String::new())
        })
    }
    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Value, E> {
        Ok(Value::Int(v))
    }
    fn visit_i128<E: de::Error>(self, v: i128) -> Result<Value, E> {
        Ok(int_value(v))
    }
    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Value, E> {
        Ok(int_value(v))
    }
    fn visit_u128<E: de::Error>(self, v: u128) -> Result<Value, E> {
        Ok(int_value(v))
    }
    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Value, E> {
        Ok(Value::Float(v))
    }
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Value, E> {
        Ok(Value::Str(v.to_string()))
    }
    fn visit_string<E: de::Error>(self, v: String) -> Result<Value, E> {
        Ok(Value::Str(v))
    }
    fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Value, E> {
        Ok(Value::Str(String::from_utf8_lossy(v).into_owned()))
    }
    fn visit_none<E: de::Error>(self) -> Result<Value, E> {
        Ok(Value::Undefined)
    }
    fn visit_some<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        de::Deserialize::deserialize(deserializer)
    }
    fn visit_unit<E: de::Error>(self) -> Result<Value, E> {
        Ok(Value::Undefined)
    }
    fn visit_newtype_struct<D: de::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Value, D::Error> {
        de::Deserialize::deserialize(deserializer)
    }
    fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let mut items = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(item) = seq.next_element()? {
            items.push(item);
        }
        Ok(Value::list(items))
    }
    fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut entries = HashMap::with_capacity(map.size_hint().unwrap_or(0));
        while let Some((k, v)) = map.next_entry::<Value, Value>()? {
            entries.insert(k.to_tt_string(), v);
        }
        Ok(Value::hash(entries))
    }
}
