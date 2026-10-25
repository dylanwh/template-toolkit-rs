use std::{cell::RefCell, collections::HashMap, fmt, rc::Rc};

use crate::{
    context::Context,
    document::Document,
    error::{ControlFlow, TemplateException},
    parser::ast::Block,
    value::{CallContext, MethodResult, TemplateObject, Value},
};

#[derive(Clone)]
pub struct View {
    pub context: Context,
    pub inner: Rc<RefCell<ViewInner>>,
}

#[derive(Debug, Clone)]
pub struct ViewInner {
    pub map: Rc<RefCell<HashMap<String, Value>>>,
    pub blocks: HashMap<String, Rc<Document>>,
    pub data: Rc<RefCell<HashMap<String, Value>>>,
    pub base: Option<View>,
    pub prefix: String,
    pub suffix: String,
    pub notfound: String,
    pub item: String,
    pub method: String,
    pub include_prefix: String,
    pub include_naked: bool,
    pub view_prefix: String,
    pub view_naked: bool,
    pub silent: bool,
    pub sealed_config: bool,
    pub is_sealed: bool,
}

impl fmt::Debug for View {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("View")
            .field("inner", &self.inner.borrow())
            .finish()
    }
}

#[bon::bon]
impl View {
    #[builder]
    pub fn build(
        context: Context,
        #[builder(default)] config: HashMap<String, Value>,
    ) -> Result<Self, TemplateException> {
        Self::new(context, config)
    }
}

const CONFIG_KEYS: &[&str] = &[
    "base",
    "prefix",
    "suffix",
    "notfound",
    "item",
    "method",
    "include_prefix",
    "include_naked",
    "view_prefix",
    "view_naked",
    "silent",
    "sealed",
];

impl View {
    pub fn new(
        context: Context,
        mut config: HashMap<String, Value>,
    ) -> Result<Self, TemplateException> {
        let mut map_entries = HashMap::new();
        map_entries.insert("HASH".to_string(), Value::Str("hash".to_string()));
        map_entries.insert("ARRAY".to_string(), Value::Str("list".to_string()));
        map_entries.insert("TEXT".to_string(), Value::Str("text".to_string()));
        map_entries.insert("default".to_string(), Value::Str(String::new()));

        if let Some(Value::Hash(user_map)) = config.remove("map") {
            for (k, v) in user_map.borrow().iter() {
                map_entries.insert(k.clone(), v.clone());
            }
        }
        if let Some(def_val) = config.remove("default") {
            if !map_entries
                .get("default")
                .map(|v| v.is_truthy())
                .unwrap_or(false)
            {
                map_entries.insert("default".to_string(), def_val);
            }
        }

        let mut blocks = HashMap::new();
        if let Some(Value::Hash(b_rc)) = config.remove("blocks") {
            for (k, v) in b_rc.borrow().iter() {
                if let Value::Object(obj) = v {
                    if let Some(doc) = obj.as_any().downcast_ref::<Document>() {
                        blocks.insert(k.clone(), Rc::new(doc.clone()));
                    }
                }
            }
        }

        let base = if let Some(base_val) = config.remove("base") {
            if let Value::Object(obj) = base_val {
                if let Some(v) = obj.as_any().downcast_ref::<View>() {
                    Some(v.clone())
                } else {
                    return Err(TemplateException::new(
                        "view",
                        "Invalid base specified for view",
                    ));
                }
            } else {
                return Err(TemplateException::new(
                    "view",
                    "Invalid base specified for view",
                ));
            }
        } else {
            None
        };

        let method = config
            .remove("method")
            .filter(|v| v.is_defined())
            .map(|v| v.to_tt_string())
            .unwrap_or_else(|| "present".to_string());

        let sealed_config = config
            .remove("sealed")
            .filter(|v| v.is_defined())
            .map(|v| v.is_truthy())
            .unwrap_or(true);

        let prefix = config
            .remove("prefix")
            .map(|v| v.to_tt_string())
            .unwrap_or_default();
        let suffix = config
            .remove("suffix")
            .map(|v| v.to_tt_string())
            .unwrap_or_default();
        let notfound = config
            .remove("notfound")
            .map(|v| v.to_tt_string())
            .unwrap_or_default();
        let silent = config
            .remove("silent")
            .map(|v| v.is_truthy())
            .unwrap_or(false);
        let item = config
            .remove("item")
            .filter(|v| v.is_truthy())
            .map(|v| v.to_tt_string())
            .unwrap_or_else(|| "item".to_string());
        let include_prefix = config
            .remove("include_prefix")
            .filter(|v| v.is_truthy())
            .map(|v| v.to_tt_string())
            .unwrap_or_else(|| "include_".to_string());
        let include_naked = config
            .remove("include_naked")
            .filter(|v| v.is_defined())
            .map(|v| v.is_truthy())
            .unwrap_or(true);
        let view_prefix = config
            .remove("view_prefix")
            .filter(|v| v.is_truthy())
            .map(|v| v.to_tt_string())
            .unwrap_or_else(|| "view_".to_string());
        let view_naked = config
            .remove("view_naked")
            .map(|v| v.is_truthy())
            .unwrap_or(false);

        let mut data_map = HashMap::new();
        if let Some(ref b) = base {
            for (k, v) in b.inner.borrow().data.borrow().iter() {
                data_map.insert(k.clone(), v.clone());
            }
        }
        for (k, v) in config {
            data_map.insert(k, v);
        }

        Ok(Self {
            context,
            inner: Rc::new(RefCell::new(ViewInner {
                map: Rc::new(RefCell::new(map_entries)),
                blocks,
                data: Rc::new(RefCell::new(data_map)),
                base,
                prefix,
                suffix,
                notfound,
                item,
                method,
                include_prefix,
                include_naked,
                view_prefix,
                view_naked,
                silent,
                sealed_config,
                is_sealed: false,
            })),
        })
    }

    pub fn add_blocks(&self, defblocks: HashMap<String, Block>) {
        let mut inner = self.inner.borrow_mut();
        for (k, b) in defblocks {
            inner
                .blocks
                .insert(k.clone(), Rc::new(Document::from_block(b, Some(k))));
        }
    }

    pub fn seal(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.is_sealed = inner.sealed_config;
    }

    pub fn unseal(&self) {
        self.inner.borrow_mut().is_sealed = false;
    }

    pub fn clone_view(&self, mut overrides: HashMap<String, Value>) -> Result<Self, ControlFlow> {
        let cur = self.inner.borrow().clone();
        let mut new_map = cur.map.borrow().clone();
        if let Some(Value::Hash(m_rc)) = overrides.remove("map") {
            for (k, v) in m_rc.borrow().iter() {
                new_map.insert(k.clone(), v.clone());
            }
        }
        if let Some(def_val) = overrides.remove("default") {
            if def_val.is_defined() {
                new_map.insert("default".to_string(), def_val);
            }
        }

        let base = if let Some(b_val) = overrides.remove("base") {
            if let Value::Object(obj) = b_val {
                obj.as_any().downcast_ref::<View>().cloned()
            } else {
                cur.base.clone()
            }
        } else {
            cur.base.clone()
        };

        let prefix = overrides
            .remove("prefix")
            .filter(|v| v.is_defined())
            .map(|v| v.to_tt_string())
            .unwrap_or(cur.prefix);
        let suffix = overrides
            .remove("suffix")
            .filter(|v| v.is_defined())
            .map(|v| v.to_tt_string())
            .unwrap_or(cur.suffix);
        let notfound = overrides
            .remove("notfound")
            .filter(|v| v.is_defined())
            .map(|v| v.to_tt_string())
            .unwrap_or(cur.notfound);
        let item = overrides
            .remove("item")
            .filter(|v| v.is_defined())
            .map(|v| v.to_tt_string())
            .unwrap_or(cur.item);
        let method = overrides
            .remove("method")
            .filter(|v| v.is_defined())
            .map(|v| v.to_tt_string())
            .unwrap_or(cur.method);
        let include_prefix = overrides
            .remove("include_prefix")
            .filter(|v| v.is_defined())
            .map(|v| v.to_tt_string())
            .unwrap_or(cur.include_prefix);
        let include_naked = overrides
            .remove("include_naked")
            .filter(|v| v.is_defined())
            .map(|v| v.is_truthy())
            .unwrap_or(cur.include_naked);
        let view_prefix = overrides
            .remove("view_prefix")
            .filter(|v| v.is_defined())
            .map(|v| v.to_tt_string())
            .unwrap_or(cur.view_prefix);
        let view_naked = overrides
            .remove("view_naked")
            .filter(|v| v.is_defined())
            .map(|v| v.is_truthy())
            .unwrap_or(cur.view_naked);

        let mut new_data = cur.data.borrow().clone();
        for (k, v) in overrides {
            new_data.insert(k, v);
        }

        Ok(Self {
            context: self.context.clone(),
            inner: Rc::new(RefCell::new(ViewInner {
                map: Rc::new(RefCell::new(new_map)),
                blocks: cur.blocks,
                data: Rc::new(RefCell::new(new_data)),
                base,
                prefix,
                suffix,
                notfound,
                item,
                method,
                include_prefix,
                include_naked,
                view_prefix,
                view_naked,
                silent: cur.silent,
                sealed_config: cur.sealed_config,
                is_sealed: cur.is_sealed,
            })),
        })
    }

    pub fn template_name(&self, template: &str) -> String {
        if template.is_empty() {
            return String::new();
        }
        let inner = self.inner.borrow();
        format!("{}{}{}", inner.prefix, template, inner.suffix)
    }

    pub fn resolve_template(&self, name: &str) -> Result<Value, ControlFlow> {
        if name.is_empty() {
            return Err(ControlFlow::Exception(TemplateException::new(
                "view",
                "no view template specified",
            )));
        }

        let (block_opt, base_opt, notfound) = {
            let inner = self.inner.borrow();
            (
                inner.blocks.get(name).cloned(),
                inner.base.clone(),
                inner.notfound.clone(),
            )
        };

        if let Some(doc) = block_opt {
            return Ok(Value::Object(doc));
        }

        let tname = self.template_name(name);
        let first_err = match self.context.template(&tname) {
            Ok(doc) => return Ok(Value::Object(doc)),
            Err(e) => e,
        };

        if let Some(base) = base_opt {
            if let Ok(val) = base.resolve_template(name) {
                return Ok(val);
            }
        }

        if !notfound.is_empty() {
            let nf_block = self.inner.borrow().blocks.get(&notfound).cloned();
            if let Some(doc) = nf_block {
                return Ok(Value::Object(doc));
            }
            let nf_name = self.template_name(&notfound);
            if let Ok(doc) = self.context.template(&nf_name) {
                return Ok(Value::Object(doc));
            }
        }

        Err(ControlFlow::Exception(TemplateException::new(
            "view",
            first_err.info.to_tt_string(),
        )))
    }

    pub fn include(
        &self,
        template: &str,
        mut vars: HashMap<String, Value>,
    ) -> Result<Value, ControlFlow> {
        let tmpl_val = self.resolve_template(template)?;
        if !vars.contains_key("view") {
            vars.insert("view".to_string(), Value::Object(Rc::new(self.clone())));
        }
        let out = self.context.include_value(&tmpl_val, Some(vars))?;
        Ok(Value::Str(out))
    }

    pub fn view_item(
        &self,
        template: &str,
        item: Option<Value>,
        mut vars: HashMap<String, Value>,
    ) -> Result<Value, ControlFlow> {
        if let Some(it) = item {
            if it.is_defined() {
                let item_key = self.inner.borrow().item.clone();
                vars.insert(item_key, it);
            }
        }
        self.include(template, vars)
    }

    pub fn print(&self, args: &[Value]) -> Result<Value, ControlFlow> {
        if args.len() > 1 {
            if let Some(Value::Hash(cfg_rc)) = args.last() {
                let cfg = cfg_rc.borrow().clone();
                let cloned = self.clone_view(cfg)?;
                return cloned.print(&args[..args.len() - 1]);
            }
        }

        let (method, map_snapshot) = {
            let inner = self.inner.borrow();
            (inner.method.clone(), inner.map.borrow().clone())
        };

        let mut output = String::new();
        for item in args {
            let type_str = match item {
                Value::Hash(_) => "HASH",
                Value::List(_) => "ARRAY",
                Value::Function(_) => "CODE",
                Value::Object(o) => o.type_name(),
                Value::Exception(_) => "Template::Exception",
                _ => "TEXT",
            };

            let mut template_name: Option<String> = None;

            if type_str == "TEXT" {
                template_name = map_snapshot.get("TEXT").map(|v| v.to_tt_string());
            } else if let Some(mapped) = map_snapshot.get(type_str).filter(|v| v.is_defined()) {
                template_name = Some(mapped.to_tt_string());
            } else {
                // Check if object implements `method` (e.g. `present`)
                if let Value::Object(obj) = item {
                    if !method.is_empty() {
                        let view_val = Value::Object(Rc::new(self.clone()));
                        match obj.call_method(&method, &[view_val], CallContext::Scalar) {
                            MethodResult::Ok(res) => {
                                output.push_str(&res.to_tt_string());
                                continue;
                            }
                            MethodResult::Err(cf) => return Err(cf),
                            MethodResult::NotFound => {}
                        }
                    }
                    // Also check if object has an underlying hash with `$method` key!
                    if let Some(h_rc) = obj.as_hash() {
                        if let Some(nt) = h_rc.borrow().get(&method).filter(|v| v.is_defined()) {
                            let newtype = nt.to_tt_string();
                            let exact_key = format!("{}=>{}", method, newtype);
                            if let Some(t) = map_snapshot.get(&exact_key).filter(|v| v.is_defined())
                            {
                                template_name = Some(t.to_tt_string());
                            } else if let Some(star) = map_snapshot
                                .get(&format!("{}=>*", method))
                                .filter(|v| v.is_defined())
                            {
                                template_name = Some(star.to_tt_string().replace('*', &newtype));
                            }
                        }
                    }
                } else if let Value::Hash(h_rc) = item {
                    if let Some(nt) = h_rc.borrow().get(&method).filter(|v| v.is_defined()) {
                        let newtype = nt.to_tt_string();
                        let exact_key = format!("{}=>{}", method, newtype);
                        if let Some(t) = map_snapshot.get(&exact_key).filter(|v| v.is_defined()) {
                            template_name = Some(t.to_tt_string());
                        } else if let Some(star) = map_snapshot
                            .get(&format!("{}=>*", method))
                            .filter(|v| v.is_defined())
                        {
                            template_name = Some(star.to_tt_string().replace('*', &newtype));
                        }
                    }
                }

                if template_name.is_none() {
                    let def_tmpl = map_snapshot
                        .get("default")
                        .map(|v| v.to_tt_string())
                        .unwrap_or_default();
                    if !def_tmpl.is_empty() {
                        template_name = Some(def_tmpl);
                    } else {
                        let mut clean = String::new();
                        let mut in_nonword = false;
                        for c in type_str.chars() {
                            if c.is_ascii_alphanumeric() || c == '_' {
                                in_nonword = false;
                                clean.push(c);
                            } else if !in_nonword {
                                in_nonword = true;
                                clean.push('_');
                            }
                        }
                        template_name = Some(clean);
                    }
                }
            }

            // Special check when item is a Hash and `HASH` mapped to something vs `$method=>$newtype`:
            // Wait: in View.pm, if `map->{HASH}` is `undef`, then `elsif (ref($item) eq 'HASH' && defined($newtype = $item->{$method}) ...)` runs!
            if let Some(tname) = template_name {
                if !tname.is_empty() {
                    let res = self.view_item(&tname, Some(item.clone()), HashMap::new())?;
                    output.push_str(&res.to_tt_string());
                }
            }
        }

        Ok(Value::Str(output))
    }

    fn parse_vars_arg(args: &[Value]) -> HashMap<String, Value> {
        let mut vars = HashMap::new();
        if let Some(Value::Hash(h_rc)) = args.first() {
            for (k, v) in h_rc.borrow().iter() {
                vars.insert(k.clone(), v.clone());
            }
        }
        vars
    }
}

impl TemplateObject for View {
    fn type_name(&self) -> &str {
        "Template::View"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn call_method(&self, method: &str, args: &[Value], _ctx: CallContext) -> MethodResult {
        match method {
            "seal" => {
                self.seal();
                MethodResult::Ok(Value::Int(1))
            }
            "unseal" => {
                self.unseal();
                MethodResult::Ok(Value::Int(0))
            }
            "clone" => {
                let cfg = Self::parse_vars_arg(args);
                match self.clone_view(cfg) {
                    Ok(c) => MethodResult::Ok(Value::Object(Rc::new(c))),
                    Err(cf) => MethodResult::Err(cf),
                }
            }
            "print" => match self.print(args) {
                Ok(v) => MethodResult::Ok(v),
                Err(cf) => MethodResult::Err(cf),
            },
            "view" => {
                let tmpl = args.first().map(|v| v.to_tt_string()).unwrap_or_default();
                let item = args.get(1).cloned();
                let vars = if args.len() > 2 {
                    Self::parse_vars_arg(&args[2..])
                } else {
                    HashMap::new()
                };
                match self.view_item(&tmpl, item, vars) {
                    Ok(v) => MethodResult::Ok(v),
                    Err(cf) => MethodResult::Err(cf),
                }
            }
            "include" => {
                let tmpl = args.first().map(|v| v.to_tt_string()).unwrap_or_default();
                let vars = if args.len() > 1 {
                    Self::parse_vars_arg(&args[1..])
                } else {
                    HashMap::new()
                };
                match self.include(&tmpl, vars) {
                    Ok(v) => MethodResult::Ok(v),
                    Err(cf) => MethodResult::Err(cf),
                }
            }
            "template" => {
                let tmpl = args.first().map(|v| v.to_tt_string()).unwrap_or_default();
                match self.resolve_template(&tmpl) {
                    Ok(v) => MethodResult::Ok(v),
                    Err(cf) => MethodResult::Err(cf),
                }
            }
            "template_name" => {
                let tmpl = args.first().map(|v| v.to_tt_string()).unwrap_or_default();
                MethodResult::Ok(Value::Str(self.template_name(&tmpl)))
            }
            "default" => {
                let map_rc = Rc::clone(&self.inner.borrow().map);
                if let Some(first) = args.first() {
                    map_rc
                        .borrow_mut()
                        .insert("default".to_string(), first.clone());
                    MethodResult::Ok(first.clone())
                } else {
                    let val = map_rc
                        .borrow()
                        .get("default")
                        .cloned()
                        .unwrap_or(Value::Str(String::new()));
                    MethodResult::Ok(val)
                }
            }
            "map" => {
                let is_sealed = self.inner.borrow().is_sealed;
                if !args.is_empty() && is_sealed {
                    return MethodResult::Err(ControlFlow::Exception(TemplateException::new(
                        "view",
                        "cannot update config item in sealed view: map",
                    )));
                }
                if let Some(Value::Hash(new_map)) = args.first() {
                    self.inner.borrow_mut().map = Rc::clone(new_map);
                    return MethodResult::Ok(Value::Hash(Rc::clone(new_map)));
                }
                MethodResult::Ok(Value::Hash(Rc::clone(&self.inner.borrow().map)))
            }
            "data" => MethodResult::Ok(Value::Hash(Rc::clone(&self.inner.borrow().data))),
            _ => {
                // AUTOLOAD logic
                if method.starts_with('.') || method.starts_with('_') {
                    return MethodResult::Err(ControlFlow::Exception(TemplateException::new(
                        "view",
                        format!("attempt to view private member: {}", method),
                    )));
                }

                // 1. Check config items
                if CONFIG_KEYS.contains(&method) {
                    let is_sealed = self.inner.borrow().is_sealed;
                    if !args.is_empty() && is_sealed {
                        return MethodResult::Err(ControlFlow::Exception(TemplateException::new(
                            "view",
                            format!("cannot update config item in sealed view: {}", method),
                        )));
                    }
                    let mut inner = self.inner.borrow_mut();
                    if let Some(val) = args.first() {
                        match method {
                            "prefix" => inner.prefix = val.to_tt_string(),
                            "suffix" => inner.suffix = val.to_tt_string(),
                            "notfound" => inner.notfound = val.to_tt_string(),
                            "item" => inner.item = val.to_tt_string(),
                            "method" => inner.method = val.to_tt_string(),
                            "include_prefix" => inner.include_prefix = val.to_tt_string(),
                            "include_naked" => inner.include_naked = val.is_truthy(),
                            "view_prefix" => inner.view_prefix = val.to_tt_string(),
                            "view_naked" => inner.view_naked = val.is_truthy(),
                            "silent" => inner.silent = val.is_truthy(),
                            "sealed" => inner.sealed_config = val.is_truthy(),
                            "base" => {
                                if let Value::Object(o) = val {
                                    inner.base = o.as_any().downcast_ref::<View>().cloned();
                                }
                            }
                            _ => {}
                        }
                        return MethodResult::Ok(val.clone());
                    } else {
                        let res = match method {
                            "prefix" => Value::Str(inner.prefix.clone()),
                            "suffix" => Value::Str(inner.suffix.clone()),
                            "notfound" => Value::Str(inner.notfound.clone()),
                            "item" => Value::Str(inner.item.clone()),
                            "method" => Value::Str(inner.method.clone()),
                            "include_prefix" => Value::Str(inner.include_prefix.clone()),
                            "include_naked" => Value::Int(if inner.include_naked { 1 } else { 0 }),
                            "view_prefix" => Value::Str(inner.view_prefix.clone()),
                            "view_naked" => Value::Int(if inner.view_naked { 1 } else { 0 }),
                            "silent" => Value::Int(if inner.silent { 1 } else { 0 }),
                            "sealed" => Value::Int(if inner.sealed_config { 1 } else { 0 }),
                            "base" => inner
                                .base
                                .as_ref()
                                .map(|b| Value::Object(Rc::new(b.clone())))
                                .unwrap_or(Value::Str(String::new())),
                            _ => Value::Undefined,
                        };
                        return MethodResult::Ok(res);
                    }
                }

                // 2. Check existing data item
                let has_data_key = self.inner.borrow().data.borrow().contains_key(method);
                if has_data_key {
                    let (is_sealed, silent, data_rc) = {
                        let inner = self.inner.borrow();
                        (inner.is_sealed, inner.silent, Rc::clone(&inner.data))
                    };
                    if !args.is_empty() && is_sealed {
                        if !silent {
                            return MethodResult::Err(ControlFlow::Exception(
                                TemplateException::new(
                                    "view",
                                    format!("cannot update item in sealed view: {}", method),
                                ),
                            ));
                        }
                        let val = data_rc
                            .borrow()
                            .get(method)
                            .cloned()
                            .unwrap_or(Value::Undefined);
                        return MethodResult::Ok(val);
                    }
                    if let Some(first) = args.first() {
                        data_rc
                            .borrow_mut()
                            .insert(method.to_string(), first.clone());
                        return MethodResult::Ok(first.clone());
                    } else {
                        let val = data_rc
                            .borrow()
                            .get(method)
                            .cloned()
                            .unwrap_or(Value::Undefined);
                        return MethodResult::Ok(val);
                    }
                }

                // 3. If args provided and unsealed, set data item
                let is_sealed = self.inner.borrow().is_sealed;
                if !args.is_empty() && !is_sealed {
                    let first = args[0].clone();
                    self.inner
                        .borrow()
                        .data
                        .borrow_mut()
                        .insert(method.to_string(), first.clone());
                    return MethodResult::Ok(first);
                }

                // 4. Check view_prefix / include_prefix / include_naked / view_naked
                let (view_prefix, include_prefix, include_naked, view_naked) = {
                    let inner = self.inner.borrow();
                    (
                        inner.view_prefix.clone(),
                        inner.include_prefix.clone(),
                        inner.include_naked,
                        inner.view_naked,
                    )
                };

                if !view_prefix.is_empty() && method.starts_with(&view_prefix) {
                    let rest = &method[view_prefix.len()..];
                    let item = args.first().cloned();
                    let vars = if args.len() > 1 {
                        Self::parse_vars_arg(&args[1..])
                    } else {
                        HashMap::new()
                    };
                    return match self.view_item(rest, item, vars) {
                        Ok(v) => MethodResult::Ok(v),
                        Err(cf) => MethodResult::Err(cf),
                    };
                }

                if !include_prefix.is_empty() && method.starts_with(&include_prefix) {
                    let rest = &method[include_prefix.len()..];
                    let vars = Self::parse_vars_arg(args);
                    return match self.include(rest, vars) {
                        Ok(v) => MethodResult::Ok(v),
                        Err(cf) => MethodResult::Err(cf),
                    };
                }

                if include_naked {
                    let vars = Self::parse_vars_arg(args);
                    return match self.include(method, vars) {
                        Ok(v) => MethodResult::Ok(v),
                        Err(cf) => MethodResult::Err(cf),
                    };
                }

                if view_naked {
                    let item = args.first().cloned();
                    let vars = if args.len() > 1 {
                        Self::parse_vars_arg(&args[1..])
                    } else {
                        HashMap::new()
                    };
                    return match self.view_item(method, item, vars) {
                        Ok(v) => MethodResult::Ok(v),
                        Err(cf) => MethodResult::Err(cf),
                    };
                }

                MethodResult::Err(ControlFlow::Exception(TemplateException::new(
                    "view",
                    format!("no such view member: {}", method),
                )))
            }
        }
    }
}
