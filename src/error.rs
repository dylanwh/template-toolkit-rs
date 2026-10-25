use std::{any::Any, fmt};

use crate::value::{CallContext, MethodResult, TemplateObject, Value};

#[derive(Debug, Clone)]
pub enum ControlFlow {
    Exception(TemplateException),
    Return(String),
    Stop(String),
    Next(Option<String>),
    Last(Option<String>),
}

#[derive(Debug, Clone)]
pub struct TemplateException {
    pub type_: String,
    pub info: Value,
    pub text: String,
}

impl TemplateException {
    pub fn new(type_: impl Into<String>, info: impl Into<Value>) -> Self {
        Self {
            type_: type_.into(),
            info: info.into(),
            text: String::new(),
        }
    }

    pub fn with_text(
        type_: impl Into<String>,
        info: impl Into<Value>,
        text: impl Into<String>,
    ) -> Self {
        Self {
            type_: type_.into(),
            info: info.into(),
            text: text.into(),
        }
    }

    pub fn type_(&self) -> &str {
        &self.type_
    }

    pub fn error_type(&self) -> &str {
        &self.type_
    }

    pub fn info(&self) -> &Value {
        &self.info
    }

    pub fn info_str(&self) -> String {
        self.info.to_tt_string()
    }

    pub fn type_info(&self) -> (&str, String) {
        (&self.type_, self.info.to_tt_string())
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn prepend_text(&mut self, pre: &str) -> &str {
        if !pre.is_empty() {
            let mut s = String::with_capacity(pre.len() + self.text.len());
            s.push_str(pre);
            s.push_str(&self.text);
            self.text = s;
        }
        &self.text
    }

    pub fn as_string(&self) -> String {
        format!("{} error - {}", self.type_, self.info.to_tt_string())
    }

    pub fn select_handler<'a>(&self, options: &[&'a str]) -> Option<&'a str> {
        let mut htype = self.type_.as_str();
        while !htype.is_empty() {
            for &opt in options {
                if opt == htype {
                    return Some(opt);
                }
            }
            if let Some(pos) = htype.rfind('.') {
                htype = &htype[..pos];
            } else {
                break;
            }
        }
        None
    }
}

impl TemplateObject for TemplateException {
    fn type_name(&self) -> &str {
        "Template::Exception"
    }

    fn to_tt_string(&self) -> String {
        self.as_string()
    }

    fn call_method(&self, method: &str, _args: &[Value], _ctx: CallContext) -> MethodResult {
        match method {
            "type" => MethodResult::Ok(Value::Str(self.type_.clone())),
            "info" => MethodResult::Ok(self.info.clone()),
            "text" => MethodResult::Ok(Value::Str(self.text.clone())),
            "as_string" => MethodResult::Ok(Value::Str(self.as_string())),
            "type_info" => MethodResult::Ok(Value::from_list(vec![
                Value::Str(self.type_.clone()),
                self.info.clone(),
            ])),
            _ => MethodResult::NotFound,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl fmt::Display for TemplateException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

impl std::error::Error for TemplateException {}

impl PartialEq for TemplateException {
    fn eq(&self, other: &Self) -> bool {
        self.as_string() == other.as_string()
    }
}

impl PartialEq<str> for TemplateException {
    fn eq(&self, other: &str) -> bool {
        self.as_string() == other
    }
}

impl PartialEq<&str> for TemplateException {
    fn eq(&self, other: &&str) -> bool {
        self.as_string() == *other
    }
}
