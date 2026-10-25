use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    fmt, fs,
    path::Path,
    rc::Rc,
};

use serde::{Deserialize, Serialize};

use crate::{
    parser::ast::{Block, ParsedDocument, VariableTrace},
    value::{CallContext, MethodResult, TemplateObject, Value},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledDocumentData {
    pub block: Block,
    pub defblocks: HashMap<String, Block>,
    pub metadata: HashMap<String, String>,
    pub variables: HashMap<String, VariableTrace>,
}

pub struct Document {
    pub block: Block,
    pub defblocks: HashMap<String, Block>,
    pub metadata: RefCell<HashMap<String, Value>>,
    pub variables: HashMap<String, VariableTrace>,
    pub hot: Cell<bool>,
    pub from_compiled: bool,
}

impl fmt::Debug for Document {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Document")
            .field("metadata", &self.metadata.borrow())
            .field("hot", &self.hot.get())
            .finish()
    }
}

impl Clone for Document {
    fn clone(&self) -> Self {
        Self {
            block: self.block.clone(),
            defblocks: self.defblocks.clone(),
            metadata: RefCell::new(self.metadata.borrow().clone()),
            variables: self.variables.clone(),
            hot: Cell::new(self.hot.get()),
            from_compiled: self.from_compiled,
        }
    }
}

impl Document {
    pub fn new(parsed: ParsedDocument, extra_meta: Option<HashMap<String, Value>>) -> Self {
        let mut meta_map = HashMap::new();
        if let Some(extra) = extra_meta {
            for (k, v) in extra {
                meta_map.insert(k, v);
            }
        }
        for (k, v) in parsed.metadata {
            meta_map.insert(k, Value::Str(v));
        }
        Self {
            block: parsed.block,
            defblocks: parsed.defblocks,
            metadata: RefCell::new(meta_map),
            variables: parsed.variables,
            hot: Cell::new(false),
            from_compiled: false,
        }
    }

    pub fn from_block(block: Block, name: Option<String>) -> Self {
        let mut meta_map = HashMap::new();
        if let Some(n) = name {
            meta_map.insert("name".to_string(), Value::Str(n));
        }
        Self {
            block,
            defblocks: HashMap::new(),
            metadata: RefCell::new(meta_map),
            variables: HashMap::new(),
            hot: Cell::new(false),
            from_compiled: false,
        }
    }

    pub fn name(&self) -> String {
        self.metadata
            .borrow()
            .get("name")
            .map(|v| v.to_tt_string())
            .unwrap_or_default()
    }

    pub fn set_meta(&self, key: impl Into<String>, val: Value) {
        self.metadata.borrow_mut().insert(key.into(), val);
    }

    pub fn get_meta(&self, key: &str) -> Option<Value> {
        self.metadata.borrow().get(key).cloned()
    }

    pub fn variables_value(&self) -> Value {
        fn convert_trace(map: &HashMap<String, VariableTrace>) -> Value {
            let mut out = HashMap::new();
            for (k, v) in map {
                out.insert(k.clone(), convert_trace(&v.children));
            }
            Value::from_hash(out)
        }
        convert_trace(&self.variables)
    }

    pub fn to_compiled_json(&self) -> Result<String, String> {
        let mut str_meta = HashMap::new();
        for (k, v) in self.metadata.borrow().iter() {
            if k != "caller" && k != "callers" {
                str_meta.insert(k.clone(), v.to_tt_string());
            }
        }
        let data = CompiledDocumentData {
            block: self.block.clone(),
            defblocks: self.defblocks.clone(),
            metadata: str_meta,
            variables: self.variables.clone(),
        };
        serde_json::to_string_pretty(&data).map_err(|e| e.to_string())
    }

    pub fn from_compiled_json(
        json_str: &str,
        extra_meta: Option<HashMap<String, Value>>,
    ) -> Result<Self, String> {
        let data: CompiledDocumentData =
            serde_json::from_str(json_str).map_err(|e| e.to_string())?;
        let mut doc = Self::new(
            ParsedDocument {
                block: data.block,
                defblocks: data.defblocks,
                metadata: data.metadata,
                variables: data.variables,
            },
            extra_meta,
        );
        doc.from_compiled = true;
        Ok(doc)
    }

    pub fn write_compiled_file(&self, path: &Path) -> Result<(), String> {
        if path.as_os_str().is_empty() {
            return Err("invalid filename".to_string());
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let content = self.to_compiled_json()?;
        fs::write(path, content).map_err(|e| e.to_string())
    }
}

impl TemplateObject for Document {
    fn type_name(&self) -> &str {
        "Template::Document"
    }

    fn call_method(&self, method: &str, _args: &[Value], _ctx: CallContext) -> MethodResult {
        if let Some(val) = self.metadata.borrow().get(method).cloned() {
            return MethodResult::Ok(val);
        }
        match method {
            "meta" => {
                let map = self.metadata.borrow().clone();
                MethodResult::Ok(Value::from_hash(map))
            }
            "variables" => MethodResult::Ok(self.variables_value()),
            "blocks" => {
                let mut bmap = HashMap::new();
                for (k, b) in &self.defblocks {
                    let doc = Document::from_block(b.clone(), Some(k.clone()));
                    bmap.insert(k.clone(), Value::Object(Rc::new(doc)));
                }
                MethodResult::Ok(Value::from_hash(bmap))
            }
            _ => MethodResult::Ok(Value::Undefined),
        }
    }

    fn as_hash(&self) -> Option<Rc<RefCell<HashMap<String, Value>>>> {
        Some(Rc::new(RefCell::new(self.metadata.borrow().clone())))
    }

    fn to_tt_string(&self) -> String {
        "Template::Document".to_string()
    }
}
