use std::{any::Any, cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    constants::STATUS_DONE,
    value::{CallContext, MethodResult, TemplateObject, Value},
};

#[derive(Debug, Clone)]
pub struct TemplateIterator {
    state: Rc<RefCell<IteratorState>>,
}

#[derive(Debug, Clone)]
struct IteratorState {
    data: Vec<Value>,
    started: bool,
    index: usize,
    size: usize,
    max: isize,
    count: usize,
    first: bool,
    last: bool,
}

impl TemplateIterator {
    pub fn new(target: &Value) -> Self {
        let data = match target {
            // Perl: `my $data = shift || [ ];`
            v if !v.is_truthy() => Vec::new(),
            Value::List(l) => l.borrow().clone(),
            Value::Hash(h) => {
                let borrow = h.borrow();
                let mut keys: Vec<&String> = borrow.keys().collect();
                keys.sort();
                keys.into_iter()
                    .map(|k| {
                        let mut pair = HashMap::new();
                        pair.insert("key".to_string(), Value::Str(k.clone()));
                        pair.insert(
                            "value".to_string(),
                            borrow.get(k).cloned().unwrap_or(Value::Undefined),
                        );
                        Value::hash(pair)
                    })
                    .collect()
            }
            Value::Object(ref o) => {
                if let Some(existing) = o.as_any().downcast_ref::<TemplateIterator>() {
                    return existing.clone();
                }
                match o.call_method("as_list", &[], CallContext::List) {
                    MethodResult::Ok(Value::List(l)) => l.borrow().clone(),
                    MethodResult::Ok(v) => vec![v],
                    _ => {
                        if let Some(list_rc) = o.as_list() {
                            list_rc.borrow().clone()
                        } else if let Some(hash_rc) = o.as_hash() {
                            let borrow = hash_rc.borrow();
                            let mut keys: Vec<&String> = borrow.keys().collect();
                            keys.sort();
                            keys.into_iter()
                                .map(|k| {
                                    let mut pair = HashMap::new();
                                    pair.insert("key".to_string(), Value::Str(k.clone()));
                                    pair.insert(
                                        "value".to_string(),
                                        borrow.get(k).cloned().unwrap_or(Value::Undefined),
                                    );
                                    Value::hash(pair)
                                })
                                .collect()
                        } else {
                            vec![target.clone()]
                        }
                    }
                }
            }
            other => vec![other.clone()],
        };
        let size = data.len();
        let max = size as isize - 1;
        Self {
            state: Rc::new(RefCell::new(IteratorState {
                data,
                started: false,
                index: 0,
                size,
                max,
                count: 0,
                first: false,
                last: false,
            })),
        }
    }

    pub fn get_first(&self) -> (Value, Option<u32>) {
        let mut st = self.state.borrow_mut();
        if st.data.is_empty() {
            return (Value::Undefined, Some(STATUS_DONE));
        }
        st.started = true;
        st.index = 0;
        st.count = 1;
        st.first = true;
        st.last = st.size <= 1;
        (st.data[0].clone(), None)
    }

    pub fn get_next(&self) -> (Value, Option<u32>) {
        let mut st = self.state.borrow_mut();
        if !st.started {
            drop(st);
            return self.get_first();
        }
        if (st.index as isize) >= st.max {
            return (Value::Undefined, Some(STATUS_DONE));
        }
        st.index += 1;
        st.count = st.index + 1;
        st.first = false;
        st.last = (st.index as isize) == st.max;
        (st.data[st.index].clone(), None)
    }

    pub fn get_all(&self) -> (Value, Option<u32>) {
        let mut st = self.state.borrow_mut();
        if st.data.is_empty() {
            return (Value::Undefined, Some(STATUS_DONE));
        }
        let start = if st.started { st.index + 1 } else { 0 };
        if start >= st.size {
            return (Value::Undefined, Some(STATUS_DONE));
        }
        st.started = true;
        st.index = st.size - 1;
        st.count = st.size;
        st.first = false;
        st.last = true;
        let rest = st.data[start..].to_vec();
        (Value::list(rest), None)
    }

    pub fn size(&self) -> usize {
        self.state.borrow().size
    }

    pub fn max(&self) -> isize {
        self.state.borrow().max
    }

    pub fn index(&self) -> usize {
        self.state.borrow().index
    }

    pub fn count(&self) -> usize {
        self.state.borrow().count
    }

    pub fn number(&self) -> usize {
        self.state.borrow().count
    }

    pub fn first(&self) -> bool {
        self.state.borrow().first
    }

    pub fn last(&self) -> bool {
        self.state.borrow().last
    }

    pub fn prev(&self) -> Value {
        let st = self.state.borrow();
        if st.started && st.index > 0 {
            st.data
                .get(st.index - 1)
                .cloned()
                .unwrap_or(Value::Undefined)
        } else {
            Value::Undefined
        }
    }

    pub fn next(&self) -> Value {
        let st = self.state.borrow();
        if st.started && (st.index as isize) < st.max {
            st.data
                .get(st.index + 1)
                .cloned()
                .unwrap_or(Value::Undefined)
        } else {
            Value::Undefined
        }
    }

    pub fn odd(&self) -> i64 {
        (self.state.borrow().count % 2) as i64
    }

    pub fn even(&self) -> i64 {
        if self.state.borrow().count.is_multiple_of(2) {
            1
        } else {
            0
        }
    }

    pub fn parity(&self) -> &'static str {
        if self.state.borrow().count % 2 == 1 {
            "odd"
        } else {
            "even"
        }
    }
}

impl TemplateObject for TemplateIterator {
    fn type_name(&self) -> &str {
        "Template::Iterator"
    }

    fn call_method(&self, method: &str, args: &[Value], _ctx: CallContext) -> MethodResult {
        match method {
            "_raw_data" => MethodResult::Ok(Value::list(self.state.borrow().data.clone())),
            "size" => MethodResult::Ok(Value::Int(self.size() as i64)),
            "max" => MethodResult::Ok(Value::Int(self.max() as i64)),
            "index" => MethodResult::Ok(Value::Int(self.index() as i64)),
            "count" | "number" => MethodResult::Ok(Value::Int(self.count() as i64)),
            "first" => {
                if let Some(arg) = args.first() {
                    self.state.borrow_mut().first = arg.is_truthy();
                }
                MethodResult::Ok(Value::Int(if self.first() { 1 } else { 0 }))
            }
            "last" => {
                if let Some(arg) = args.first() {
                    self.state.borrow_mut().last = arg.is_truthy();
                }
                MethodResult::Ok(Value::Int(if self.last() { 1 } else { 0 }))
            }
            "prev" => MethodResult::Ok(self.prev()),
            "next" => MethodResult::Ok(self.next()),
            "odd" => MethodResult::Ok(Value::Int(self.odd())),
            "even" => MethodResult::Ok(Value::Int(self.even())),
            "parity" => MethodResult::Ok(Value::Str(self.parity().to_string())),
            "get_first" => MethodResult::Ok(self.get_first().0),
            "get_next" => MethodResult::Ok(self.get_next().0),
            "get_all" => MethodResult::Ok(self.get_all().0),
            _ => MethodResult::NotFound,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
