use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    error::{ControlFlow, TemplateException},
    value::{CallContext, Callable, MethodResult, Value},
    vmethods::{vmethod_hash_import, VMethodRegistry, GLOBAL_VMETHODS},
};

/// Global flag equivalent to `$Template::Stash::PRIVATE = qr/^[_.]/`.
/// When true (the default), variable names starting with `_` or `.` are hidden.
static PRIVATE_ENABLED: AtomicBool = AtomicBool::new(true);

pub fn set_private_enabled(enabled: bool) {
    PRIVATE_ENABLED.store(enabled, Ordering::SeqCst);
}

pub fn is_private_enabled() -> bool {
    PRIVATE_ENABLED.load(Ordering::SeqCst)
}

fn is_private_key(key: &str, private_check: bool) -> bool {
    private_check && (key.starts_with('_') || key.starts_with('.'))
}

#[derive(Debug, Clone)]
pub struct Stash {
    pub root: Rc<RefCell<HashMap<String, Value>>>,
    pub parent: Option<Box<Stash>>,
    pub strict: bool,
    pub debug: bool,
    pub context_mode: bool,
    pub private_check: Option<bool>,
    pub vmethods: Option<Rc<RefCell<VMethodRegistry>>>,
}

#[bon::bon]
impl Stash {
    #[builder]
    pub fn build(
        params: Option<HashMap<String, Value>>,
        #[builder(default = false)] strict: bool,
        #[builder(default = false)] debug: bool,
        #[builder(default = false)] context_mode: bool,
        private_check: Option<bool>,
        vmethods: Option<Rc<RefCell<VMethodRegistry>>>,
    ) -> Self {
        let mut s = Self::new(params);
        s.strict = strict;
        s.debug = debug;
        s.context_mode = context_mode;
        s.private_check = private_check;
        s.vmethods = vmethods;
        s
    }
}

impl Default for Stash {
    fn default() -> Self {
        Self::new(None)
    }
}

impl Stash {
    pub fn new(params: Option<HashMap<String, Value>>) -> Self {
        let mut map = HashMap::new();
        map.insert("global".to_string(), Value::new_hash());
        let mut stash = Self {
            root: Rc::new(RefCell::new(map)),
            parent: None,
            strict: false,
            debug: false,
            context_mode: false,
            private_check: None,
            vmethods: None,
        };
        if let Some(p) = params {
            stash.update(p);
        }
        stash
    }

    pub fn new_context(params: Option<HashMap<String, Value>>) -> Self {
        let mut s = Self::new(params);
        s.context_mode = true;
        s
    }

    fn check_private(&self, key: &str) -> bool {
        let enabled = self.private_check.unwrap_or_else(is_private_enabled);
        is_private_key(key, enabled)
    }

    pub fn clone_stash(&self, params: Option<HashMap<String, Value>>) -> Self {
        let mut new_map = self.root.borrow().clone();
        let mut import_hash: Option<Value> = None;

        if let Some(mut p) = params {
            if let Some(imp) = p.get("import") {
                if matches!(imp, Value::Hash(_)) {
                    import_hash = p.remove("import");
                }
            }
            for (k, v) in p {
                new_map.insert(k, v);
            }
        }

        if let Some(Value::Hash(imp_rc)) = import_hash {
            for (k, v) in imp_rc.borrow().iter() {
                new_map.insert(k.clone(), v.clone());
            }
        }

        Self {
            root: Rc::new(RefCell::new(new_map)),
            parent: Some(Box::new(self.clone())),
            strict: self.strict,
            debug: self.debug,
            context_mode: self.context_mode,
            private_check: self.private_check,
            vmethods: self.vmethods.clone(),
        }
    }

    pub fn declone(self) -> Self {
        if let Some(parent) = self.parent {
            *parent
        } else {
            self
        }
    }

    pub fn update(&mut self, mut params: HashMap<String, Value>) {
        let mut import_hash: Option<Value> = None;
        if let Some(imp) = params.get("import") {
            if matches!(imp, Value::Hash(_)) {
                import_hash = params.remove("import");
            }
        }
        let mut root_borrow = self.root.borrow_mut();
        if let Some(Value::Hash(imp_rc)) = import_hash {
            for (k, v) in imp_rc.borrow().iter() {
                root_borrow.insert(k.clone(), v.clone());
            }
        }
        for (k, v) in params {
            root_borrow.insert(k, v);
        }
    }

    pub fn get_raw(&self, key: &str) -> Option<Value> {
        self.root.borrow().get(key).cloned()
    }

    pub fn set_raw(&mut self, key: impl Into<String>, val: Value) {
        self.root.borrow_mut().insert(key.into(), val);
    }

    /// Normalize a `Value` identifier (either `Value::List` `[item, args, ...]` or `Value::Str` `"foo.bar"`)
    /// into pairs of `(Value, Vec<Value>)`.
    pub fn normalize_ident(ident: &Value) -> Result<Vec<(Value, Vec<Value>)>, Vec<Value>> {
        match ident {
            Value::List(list_rc) => {
                let b = list_rc.borrow();
                let mut pairs = Vec::with_capacity(b.len() / 2);
                let mut i = 0;
                while i < b.len() {
                    let item = b[i].clone();
                    let args_val = if i + 1 < b.len() {
                        b[i + 1].clone()
                    } else {
                        Value::Int(0)
                    };
                    let args = match args_val {
                        Value::List(a_rc) => a_rc.borrow().clone(),
                        _ => Vec::new(),
                    };
                    pairs.push((item, args));
                    i += 2;
                }
                Ok(pairs)
            }
            Value::Str(s) if s.contains('.') => {
                let mut pairs = Vec::new();
                for part in s.split('.') {
                    let clean = if let Some(idx) = part.find('(') {
                        &part[..idx]
                    } else {
                        part
                    };
                    pairs.push((Value::Str(clean.to_string()), Vec::new()));
                }
                Ok(pairs)
            }
            other => Err(vec![other.clone()]),
        }
    }

    /// Get a variable value using a `Value` identifier (`Value::Str` or `Value::List` of pairs).
    pub fn get(&self, ident: &Value, extra_args: Option<&[Value]>) -> Result<Value, ControlFlow> {
        let result = match Self::normalize_ident(ident) {
            Ok(mut pairs) => {
                if let Some(extra) = extra_args {
                    if let Some(last) = pairs.last_mut() {
                        last.1.extend_from_slice(extra);
                    }
                }
                self.eval_chain(&pairs)?
            }
            Err(single) => {
                let item = &single[0];
                let args = extra_args.unwrap_or(&[]);
                self.dotop_root(item, args, false, None)?
            }
        };

        if result.is_defined() {
            Ok(result)
        } else {
            self.undefined(ident, extra_args)
        }
    }

    /// Convenience helper for getting a variable by `&str`.
    pub fn get_str(&self, ident: &str) -> Result<Value, ControlFlow> {
        self.get(&Value::Str(ident.to_string()), None)
    }

    /// Convenience helper for evaluating pre-built `(item, args)` pairs directly from the AST evaluator.
    pub fn get_pairs(
        &self,
        pairs: &[(Value, Vec<Value>)],
        extra_args: Option<&[Value]>,
    ) -> Result<Value, ControlFlow> {
        let result = if pairs.len() == 1 && extra_args.is_none() {
            let (item, args) = &pairs[0];
            if let Value::Str(s) = item {
                if s.contains('.') {
                    let ident_val = Value::Str(s.clone());
                    return self.get(&ident_val, if args.is_empty() { None } else { Some(args) });
                }
            }
            self.dotop_root(item, args, false, None)?
        } else if let Some(extra) = extra_args {
            let mut owned = pairs.to_vec();
            if let Some(last) = owned.last_mut() {
                last.1.extend_from_slice(extra);
            }
            self.eval_chain(&owned)?
        } else {
            self.eval_chain(pairs)?
        };

        if result.is_defined() {
            Ok(result)
        } else {
            let ident_val = Self::pairs_to_value(pairs);
            self.undefined(&ident_val, extra_args)
        }
    }

    fn pairs_to_value(pairs: &[(Value, Vec<Value>)]) -> Value {
        if pairs.len() == 1 && pairs[0].1.is_empty() {
            return pairs[0].0.clone();
        }
        let mut flat = Vec::with_capacity(pairs.len() * 2);
        for (item, args) in pairs {
            flat.push(item.clone());
            if args.is_empty() {
                flat.push(Value::Int(0));
            } else {
                flat.push(Value::from_list(args.clone()));
            }
        }
        Value::from_list(flat)
    }

    fn eval_chain(&self, pairs: &[(Value, Vec<Value>)]) -> Result<Value, ControlFlow> {
        let mut current_root: Option<Value> = None; // None means self.root (atroot)
        let mut result = Value::Undefined;
        let len = pairs.len();
        let mut i = 0;

        while i < len {
            let (item, args) = &pairs[i];
            let next_context = if self.context_mode && i + 1 < len {
                if let Value::Str(next_s) = &pairs[i + 1].0 {
                    if next_s == "scalar" || next_s == "ref" {
                        Some(next_s.as_str())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            result = match &current_root {
                None => self.dotop_root(item, args, false, next_context)?,
                Some(root_val) => self.dotop_val(root_val, item, args, false, next_context)?,
            };

            if next_context.is_some() {
                i += 2;
            } else {
                i += 1;
            }

            if !result.is_defined() {
                break;
            }
            current_root = Some(result.clone());
        }

        Ok(result)
    }

    /// Set a variable in the stash.
    pub fn set(&self, ident: &Value, value: Value, default: bool) -> Result<Value, ControlFlow> {
        let result = match Self::normalize_ident(ident) {
            Ok(pairs) => self.set_pairs(&pairs, value, default)?,
            Err(single) => {
                let item = &single[0];
                self.assign_root(item, &[], value, default)?
            }
        };

        if result.is_defined() {
            Ok(result)
        } else {
            Ok(Value::Str(String::new()))
        }
    }

    pub fn set_str(&self, ident: &str, value: Value, default: bool) -> Result<Value, ControlFlow> {
        self.set(&Value::Str(ident.to_string()), value, default)
    }

    pub fn set_pairs(
        &self,
        pairs: &[(Value, Vec<Value>)],
        value: Value,
        default: bool,
    ) -> Result<Value, ControlFlow> {
        if pairs.is_empty() {
            return Ok(Value::Str(String::new()));
        }
        if pairs.len() == 1 {
            let (item, args) = &pairs[0];
            if let Value::Str(s) = item {
                if s.contains('.') && args.is_empty() {
                    return self.set(&Value::Str(s.clone()), value, default);
                }
            }
            let res = self.assign_root(item, args, value, default)?;
            return Ok(if res.is_defined() {
                res
            } else {
                Value::Str(String::new())
            });
        }

        let mut current_root: Option<Value> = None;
        for (item, args) in &pairs[..pairs.len() - 1] {
            let next_val = match &current_root {
                None => self.dotop_root(item, args, true, None)?,
                Some(root_val) => self.dotop_val(root_val, item, args, true, None)?,
            };
            if !next_val.is_defined() {
                return Ok(Value::Str(String::new()));
            }
            current_root = Some(next_val);
        }

        let (last_item, last_args) = &pairs[pairs.len() - 1];
        let res = match &current_root {
            None => self.assign_root(last_item, last_args, value, default)?,
            Some(root_val) => self.assign_val(root_val, last_item, last_args, value, default)?,
        };

        Ok(if res.is_defined() {
            res
        } else {
            Value::Str(String::new())
        })
    }

    /// Return a reference closure (`\foo.bar(baz)`) that evaluates the last step when invoked.
    pub fn getref(
        &self,
        ident: &Value,
        extra_args: Option<&[Value]>,
    ) -> Result<Value, ControlFlow> {
        let pairs = match Self::normalize_ident(ident) {
            Ok(p) => p,
            Err(single) => vec![(
                single[0].clone(),
                extra_args.map(|s| s.to_vec()).unwrap_or_default(),
            )],
        };
        self.getref_pairs(&pairs)
    }

    pub fn getref_pairs(&self, pairs: &[(Value, Vec<Value>)]) -> Result<Value, ControlFlow> {
        if pairs.is_empty() {
            return Ok(Value::from_fn(|_, _| Ok(Value::Str(String::new()))));
        }

        let mut current_root: Option<Value> = None; // None means at_root
        let mut root_viable = true;

        for (item, args) in &pairs[..pairs.len() - 1] {
            let next_val = match &current_root {
                None => self.dotop_root(item, args, false, None)?,
                Some(r) => self.dotop_val(r, item, args, false, None)?,
            };
            if !next_val.is_defined() {
                root_viable = false;
                break;
            }
            current_root = Some(next_val);
        }

        if !root_viable {
            return Ok(Value::from_fn(|_, _| Ok(Value::Str(String::new()))));
        }

        let (last_item, last_args) = pairs[pairs.len() - 1].clone();
        let stash_clone = self.clone();

        let func = ReferenceCallable {
            stash: stash_clone,
            root: current_root,
            item: last_item,
            base_args: last_args,
        };

        Ok(Value::Function(Rc::new(func)))
    }

    pub fn undefined(&self, ident: &Value, _args: Option<&[Value]>) -> Result<Value, ControlFlow> {
        if self.strict {
            let reconstructed = self.reconstruct_ident(ident);
            Err(ControlFlow::Exception(TemplateException::new(
                "var.undef",
                format!("undefined variable: {}", reconstructed),
            )))
        } else {
            Ok(Value::Str(String::new()))
        }
    }

    pub fn reconstruct_ident(&self, ident: &Value) -> String {
        match ident {
            Value::List(list_rc) => {
                let b = list_rc.borrow();
                let mut output = Vec::new();
                let mut i = 0;
                while i < b.len() {
                    let mut name = b[i].to_tt_string();
                    if i + 1 < b.len() {
                        if let Value::List(args_rc) = &b[i + 1] {
                            let args_b = args_rc.borrow();
                            if !args_b.is_empty() {
                                let formatted_args: Vec<String> = args_b
                                    .iter()
                                    .map(|a| {
                                        let s = a.to_tt_string();
                                        if !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()) {
                                            s
                                        } else {
                                            format!("'{}'", s)
                                        }
                                    })
                                    .collect();
                                name.push('(');
                                name.push_str(&formatted_args.join(", "));
                                name.push(')');
                            }
                        }
                    }
                    output.push(name);
                    i += 2;
                }
                output.join(".")
            }
            other => other.to_tt_string(),
        }
    }

    pub fn dotop(
        &self,
        root: &Value,
        item: &Value,
        args: &[Value],
        lvalue: bool,
    ) -> Result<Value, ControlFlow> {
        self.dotop_val(root, item, args, lvalue, None)
    }

    fn with_vmethods<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&VMethodRegistry) -> R,
    {
        if let Some(ref vm) = self.vmethods {
            f(&vm.borrow())
        } else {
            GLOBAL_VMETHODS.with(|g| f(&g.borrow()))
        }
    }

    fn dotop_root(
        &self,
        item: &Value,
        args: &[Value],
        lvalue: bool,
        next_context: Option<&str>,
    ) -> Result<Value, ControlFlow> {
        if !item.is_defined() {
            return Ok(Value::Undefined);
        }
        let item_str = item.to_tt_string();
        if self.check_private(&item_str) {
            return Ok(Value::Undefined);
        }

        let scalar_context = next_context == Some("scalar");
        let return_ref = next_context == Some("ref");

        // 1. Check root hash
        let found = self.root.borrow().get(&item_str).cloned();
        if let Some(val) = found {
            if val.is_defined() {
                return self.handle_found_value(val, args, return_ref, scalar_context);
            }
        }

        // 2. If lvalue, auto-vivify intermediate hash in root
        if lvalue {
            let new_hash = Value::new_hash();
            self.root.borrow_mut().insert(item_str, new_hash.clone());
            return Ok(new_hash);
        }

        // 3. Check ROOT_OPS (inc, dec, import)
        let root_op = self.with_vmethods(|vm| vm.root_ops.get(&item_str).copied());
        if let Some(op) = root_op {
            let root_val = Value::Hash(Rc::clone(&self.root));
            let res = op(&root_val, args)?;
            return self.finalize_result(res, &item_str, return_ref, scalar_context);
        }

        // 4. Hash slice on root stash if item is a List
        if let Value::List(keys_rc) = item {
            let root_b = self.root.borrow();
            let mut slice = Vec::new();
            for k in keys_rc.borrow().iter() {
                let ks = k.to_tt_string();
                slice.push(root_b.get(&ks).cloned().unwrap_or(Value::Undefined));
            }
            return Ok(Value::from_list(slice));
        }

        if self.debug {
            return Err(ControlFlow::Exception(TemplateException::new(
                "undef",
                format!("{} is undefined\n", item_str),
            )));
        }

        Ok(Value::Undefined)
    }

    fn dotop_val(
        &self,
        root: &Value,
        item: &Value,
        args: &[Value],
        lvalue: bool,
        next_context: Option<&str>,
    ) -> Result<Value, ControlFlow> {
        if !root.is_defined() || !item.is_defined() {
            return Ok(Value::Undefined);
        }

        let item_str = item.to_tt_string();
        if !matches!(item, Value::List(_)) && self.check_private(&item_str) {
            return Ok(Value::Undefined);
        }

        let scalar_context = next_context == Some("scalar");
        let return_ref = next_context == Some("ref");

        match root {
            Value::Hash(hash_rc) => {
                if !matches!(item, Value::List(_)) {
                    let found = hash_rc.borrow().get(&item_str).cloned();
                    if let Some(val) = found {
                        if val.is_defined() {
                            return self.handle_found_value(val, args, return_ref, scalar_context);
                        }
                    }
                }

                if lvalue {
                    let new_hash = Value::new_hash();
                    hash_rc.borrow_mut().insert(item_str, new_hash.clone());
                    return Ok(new_hash);
                }

                if let Some(op) = self.with_vmethods(|vm| vm.hash_ops.get(&item_str).copied()) {
                    let res = op(root, args)?;
                    return self.finalize_result(res, &item_str, return_ref, scalar_context);
                }

                if let Value::List(keys_rc) = item {
                    let hash_b = hash_rc.borrow();
                    let mut slice = Vec::new();
                    for k in keys_rc.borrow().iter() {
                        let ks = k.to_tt_string();
                        slice.push(hash_b.get(&ks).cloned().unwrap_or(Value::Undefined));
                    }
                    return Ok(Value::from_list(slice));
                }

                if self.context_mode {
                    if let Some(sop) =
                        self.with_vmethods(|vm| vm.scalar_ops.get(&item_str).copied())
                    {
                        let keys: Vec<String> = hash_rc.borrow().keys().cloned().collect();
                        for k in keys {
                            let cur = hash_rc
                                .borrow()
                                .get(&k)
                                .cloned()
                                .unwrap_or(Value::Undefined);
                            let updated = sop(&cur, args)?;
                            hash_rc.borrow_mut().insert(k, updated);
                        }
                        return Ok(Value::Undefined);
                    }
                }
            }

            Value::List(list_rc) => {
                if !lvalue {
                    if self.context_mode && item_str == "array" {
                        return self.finalize_result(
                            root.clone(),
                            &item_str,
                            return_ref,
                            scalar_context,
                        );
                    }
                    if let Some(op) = self.with_vmethods(|vm| vm.list_ops.get(&item_str).copied()) {
                        let res = op(root, args)?;
                        return self.finalize_result(res, &item_str, return_ref, scalar_context);
                    }
                    if self.context_mode {
                        if let Some(sop) =
                            self.with_vmethods(|vm| vm.scalar_ops.get(&item_str).copied())
                        {
                            let len = list_rc.borrow().len();
                            for idx in 0..len {
                                let cur = list_rc.borrow()[idx].clone();
                                let updated = sop(&cur, args)?;
                                list_rc.borrow_mut()[idx] = updated;
                            }
                            return Ok(Value::Undefined);
                        }
                    }
                }

                if let Ok(idx) = item_str.parse::<isize>() {
                    let list_b = list_rc.borrow();
                    let len = list_b.len() as isize;
                    let actual_idx = if idx < 0 { len + idx } else { idx };
                    let val = if actual_idx >= 0 && (actual_idx as usize) < list_b.len() {
                        list_b[actual_idx as usize].clone()
                    } else {
                        Value::Undefined
                    };
                    drop(list_b);
                    if val.is_defined() {
                        return self.handle_found_value(val, args, return_ref, scalar_context);
                    }
                    return Ok(Value::Undefined);
                }

                if let Value::List(indices_rc) = item {
                    let list_b = list_rc.borrow();
                    let len = list_b.len() as isize;
                    let mut slice = Vec::new();
                    for idx_val in indices_rc.borrow().iter() {
                        if let Ok(idx) = idx_val.to_tt_string().parse::<isize>() {
                            let actual = if idx < 0 { len + idx } else { idx };
                            if actual >= 0 && (actual as usize) < list_b.len() {
                                slice.push(list_b[actual as usize].clone());
                            } else {
                                slice.push(Value::Undefined);
                            }
                        } else {
                            slice.push(Value::Undefined);
                        }
                    }
                    return Ok(Value::from_list(slice));
                }
            }

            Value::Object(obj_rc) => {
                if return_ref {
                    return Ok(Value::Str("CODE".to_string()));
                }
                let call_ctx = if scalar_context {
                    CallContext::Scalar
                } else {
                    CallContext::List
                };
                match obj_rc.call_method(&item_str, args, call_ctx) {
                    MethodResult::Ok(res) => {
                        return self.finalize_result(res, &item_str, return_ref, scalar_context);
                    }
                    MethodResult::Err(cf) => {
                        return Err(cf);
                    }
                    MethodResult::NotFound => {
                        // Try fallbacks: underlying hash -> HASH_OPS -> LIST_OPS -> SCALAR_OPS
                        if let Some(hash_rc) = obj_rc.as_hash() {
                            let found = hash_rc.borrow().get(&item_str).cloned();
                            if let Some(val) = found {
                                if val.is_defined() {
                                    return self.handle_found_value(
                                        val,
                                        args,
                                        return_ref,
                                        scalar_context,
                                    );
                                }
                            }
                            if let Some(op) =
                                self.with_vmethods(|vm| vm.hash_ops.get(&item_str).copied())
                            {
                                let target = if item_str == "scalar" || item_str == "assert" {
                                    root.clone()
                                } else {
                                    Value::Hash(hash_rc)
                                };
                                let res = op(&target, args)?;
                                return self.finalize_result(
                                    res,
                                    &item_str,
                                    return_ref,
                                    scalar_context,
                                );
                            }
                            if let Some(op) =
                                self.with_vmethods(|vm| vm.list_ops.get(&item_str).copied())
                            {
                                let promoted = Value::from_list(vec![root.clone()]);
                                let res = op(&promoted, args)?;
                                return self.finalize_result(
                                    res,
                                    &item_str,
                                    return_ref,
                                    scalar_context,
                                );
                            }
                        } else if let Some(list_rc) = obj_rc.as_list() {
                            let l_val = if item_str == "scalar" || item_str == "assert" {
                                root.clone()
                            } else {
                                Value::List(Rc::clone(&list_rc))
                            };
                            if let Some(op) =
                                self.with_vmethods(|vm| vm.list_ops.get(&item_str).copied())
                            {
                                let res = op(&l_val, args)?;
                                return self.finalize_result(
                                    res,
                                    &item_str,
                                    return_ref,
                                    scalar_context,
                                );
                            }
                            if let Ok(idx) = item_str.parse::<isize>() {
                                let list_b = list_rc.borrow();
                                let len = list_b.len() as isize;
                                let actual = if idx < 0 { len + idx } else { idx };
                                if actual >= 0 && (actual as usize) < list_b.len() {
                                    let val = list_b[actual as usize].clone();
                                    drop(list_b);
                                    return self.handle_found_value(
                                        val,
                                        args,
                                        return_ref,
                                        scalar_context,
                                    );
                                }
                            }
                        } else {
                            if let Some(op) =
                                self.with_vmethods(|vm| vm.scalar_ops.get(&item_str).copied())
                            {
                                let res = op(root, args)?;
                                return self.finalize_result(
                                    res,
                                    &item_str,
                                    return_ref,
                                    scalar_context,
                                );
                            }
                            if let Some(op) =
                                self.with_vmethods(|vm| vm.list_ops.get(&item_str).copied())
                            {
                                let promoted = Value::from_list(vec![root.clone()]);
                                let res = op(&promoted, args)?;
                                return self.finalize_result(
                                    res,
                                    &item_str,
                                    return_ref,
                                    scalar_context,
                                );
                            }
                        }
                    }
                }
            }

            // Scalar values (Int, Float, Str, Function, Exception)
            _ => {
                if !lvalue {
                    if self.context_mode {
                        if item_str == "array" {
                            let promoted = Value::from_list(vec![root.clone()]);
                            return self.finalize_result(
                                promoted,
                                &item_str,
                                return_ref,
                                scalar_context,
                            );
                        }
                        if let Ok(idx) = item_str.parse::<isize>() {
                            let val = if idx == 0 || idx == -1 {
                                root.clone()
                            } else {
                                Value::Undefined
                            };
                            if val.is_defined() {
                                return self.handle_found_value(
                                    val,
                                    args,
                                    return_ref,
                                    scalar_context,
                                );
                            }
                            return Ok(Value::Undefined);
                        }
                    }

                    if let Some(op) = self.with_vmethods(|vm| vm.scalar_ops.get(&item_str).copied())
                    {
                        let res = op(root, args)?;
                        return self.finalize_result(res, &item_str, return_ref, scalar_context);
                    }

                    if let Some(op) = self.with_vmethods(|vm| vm.list_ops.get(&item_str).copied()) {
                        let promoted = Value::from_list(vec![root.clone()]);
                        let res = op(&promoted, args)?;
                        return self.finalize_result(res, &item_str, return_ref, scalar_context);
                    }
                }

                if self.debug {
                    return Err(ControlFlow::Exception(TemplateException::new(
                        "undef",
                        format!(
                            "don't know how to access [ {} ].{}\n",
                            root.to_tt_string(),
                            item_str
                        ),
                    )));
                }
            }
        }

        if self.debug {
            return Err(ControlFlow::Exception(TemplateException::new(
                "undef",
                format!("{} is undefined\n", item_str),
            )));
        }

        Ok(Value::Undefined)
    }

    fn handle_found_value(
        &self,
        val: Value,
        args: &[Value],
        return_ref: bool,
        scalar_context: bool,
    ) -> Result<Value, ControlFlow> {
        if return_ref {
            return Ok(Value::Str(Self::perl_ref_name(&val)));
        }

        if let Value::Function(func_rc) = val {
            let ctx = if scalar_context {
                CallContext::Scalar
            } else {
                CallContext::List
            };
            let res = func_rc.call(args, ctx)?;
            return self.finalize_result(res, "", return_ref, scalar_context);
        }

        if scalar_context {
            match &val {
                Value::List(l) => return Ok(Value::Int(l.borrow().len() as i64)),
                Value::Hash(h) => return Ok(Value::Int(h.borrow().len() as i64)),
                _ => return Ok(val),
            }
        }

        Ok(val)
    }

    fn finalize_result(
        &self,
        res: Value,
        item_str: &str,
        return_ref: bool,
        scalar_context: bool,
    ) -> Result<Value, ControlFlow> {
        if !res.is_defined() {
            if self.debug && !item_str.is_empty() {
                return Err(ControlFlow::Exception(TemplateException::new(
                    "undef",
                    format!("{} is undefined\n", item_str),
                )));
            }
            return Ok(Value::Undefined);
        }

        if return_ref {
            return Ok(Value::Str(Self::perl_ref_name(&res)));
        }

        if scalar_context {
            match &res {
                Value::List(l) => return Ok(Value::Int(l.borrow().len() as i64)),
                Value::Hash(h) => return Ok(Value::Int(h.borrow().len() as i64)),
                _ => return Ok(res),
            }
        }

        Ok(res)
    }

    fn perl_ref_name(val: &Value) -> String {
        match val {
            Value::List(_) => "ARRAY".to_string(),
            Value::Hash(_) => "HASH".to_string(),
            Value::Function(_) => "CODE".to_string(),
            Value::Object(o) => o.type_name().to_string(),
            Value::Exception(_) => "Template::Exception".to_string(),
            _ => String::new(),
        }
    }

    fn assign_root(
        &self,
        item: &Value,
        _args: &[Value],
        value: Value,
        default: bool,
    ) -> Result<Value, ControlFlow> {
        if !item.is_defined() {
            return Ok(Value::Undefined);
        }
        let item_str = item.to_tt_string();
        if self.check_private(&item_str) {
            return Ok(Value::Undefined);
        }

        if item_str == "IMPORT" {
            if let Value::Hash(imp_rc) = value {
                let mut root_b = self.root.borrow_mut();
                for (k, v) in imp_rc.borrow().iter() {
                    root_b.insert(k.clone(), v.clone());
                }
            }
            return Ok(Value::Str(String::new()));
        }

        if default {
            let existing_truthy = self
                .root
                .borrow()
                .get(&item_str)
                .map(|v| v.is_truthy())
                .unwrap_or(false);
            if existing_truthy {
                return Ok(Value::Str(String::new()));
            }
        }

        self.root.borrow_mut().insert(item_str, value.clone());
        Ok(value)
    }

    fn assign_val(
        &self,
        root: &Value,
        item: &Value,
        args: &[Value],
        value: Value,
        default: bool,
    ) -> Result<Value, ControlFlow> {
        if !root.is_defined() || !item.is_defined() {
            return Ok(Value::Undefined);
        }
        let item_str = item.to_tt_string();
        if self.check_private(&item_str) {
            return Ok(Value::Undefined);
        }

        match root {
            Value::Hash(hash_rc) => {
                if item_str == "IMPORT" {
                    if let Value::Hash(_) = value {
                        let _ = vmethod_hash_import(root, &[value]);
                    }
                    return Ok(Value::Str(String::new()));
                }
                if default {
                    let existing_truthy = hash_rc
                        .borrow()
                        .get(&item_str)
                        .map(|v| v.is_truthy())
                        .unwrap_or(false);
                    if existing_truthy {
                        return Ok(Value::Str(String::new()));
                    }
                }
                hash_rc.borrow_mut().insert(item_str, value.clone());
                Ok(value)
            }
            Value::List(list_rc) => {
                if let Ok(idx) = item_str.parse::<isize>() {
                    let mut list_b = list_rc.borrow_mut();
                    let len = list_b.len() as isize;
                    let actual = if idx < 0 { len + idx } else { idx };
                    if actual < 0 {
                        return Ok(Value::Undefined);
                    }
                    let u_idx = actual as usize;
                    if default && u_idx < list_b.len() && list_b[u_idx].is_truthy() {
                        return Ok(Value::Str(String::new()));
                    }
                    if u_idx >= list_b.len() {
                        list_b.resize(u_idx + 1, Value::Undefined);
                    }
                    list_b[u_idx] = value.clone();
                    return Ok(value);
                }
                Ok(Value::Undefined)
            }
            Value::Object(obj_rc) => {
                if default {
                    if let MethodResult::Ok(existing) =
                        obj_rc.call_method(&item_str, &[], CallContext::Scalar)
                    {
                        if existing.is_truthy() {
                            return Ok(Value::Str(String::new()));
                        }
                    }
                }
                let mut call_args = args.to_vec();
                call_args.push(value);
                match obj_rc.call_method(&item_str, &call_args, CallContext::List) {
                    MethodResult::Ok(res) => Ok(res),
                    MethodResult::Err(cf) => Err(cf),
                    MethodResult::NotFound => Ok(Value::Undefined),
                }
            }
            _ => Ok(Value::Undefined),
        }
    }
}

struct ReferenceCallable {
    stash: Stash,
    root: Option<Value>,
    item: Value,
    base_args: Vec<Value>,
}

impl Callable for ReferenceCallable {
    fn call(&self, extra_args: &[Value], _ctx: CallContext) -> Result<Value, ControlFlow> {
        let mut combined = self.base_args.clone();
        combined.extend_from_slice(extra_args);
        let res = match &self.root {
            None => self.stash.dotop_root(&self.item, &combined, false, None)?,
            Some(r) => self
                .stash
                .dotop_val(r, &self.item, &combined, false, None)?,
        };
        Ok(res)
    }
}
