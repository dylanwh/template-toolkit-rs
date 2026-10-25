use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    context::{CompiledTemplate, Context},
    error::{ControlFlow, TemplateException},
    value::{TemplateObject, Value},
};

#[derive(Debug, Clone)]
pub enum ErrorSpec {
    Single(String),
    Map(HashMap<String, String>),
}

impl From<&str> for ErrorSpec {
    fn from(s: &str) -> Self {
        ErrorSpec::Single(s.to_string())
    }
}

impl From<String> for ErrorSpec {
    fn from(s: String) -> Self {
        ErrorSpec::Single(s)
    }
}

impl From<HashMap<String, String>> for ErrorSpec {
    fn from(m: HashMap<String, String>) -> Self {
        ErrorSpec::Map(m)
    }
}

#[derive(Clone)]
pub struct Service {
    pub context: Context,
    pub pre_process: Vec<String>,
    pub process_list: Option<Vec<String>>,
    pub post_process: Vec<String>,
    pub wrapper: Vec<String>,
    pub error_spec: Option<ErrorSpec>,
    pub auto_reset: bool,
    last_error: Rc<RefCell<Option<TemplateException>>>,
}

impl std::fmt::Debug for Service {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Service")
            .field("pre_process", &self.pre_process)
            .field("process_list", &self.process_list)
            .field("post_process", &self.post_process)
            .field("wrapper", &self.wrapper)
            .field("auto_reset", &self.auto_reset)
            .finish()
    }
}

#[bon::bon]
impl Service {
    #[builder]
    pub fn build(
        context: Option<Context>,
        #[builder(default)] pre_process: Vec<String>,
        process: Option<Vec<String>>,
        #[builder(default)] post_process: Vec<String>,
        #[builder(default)] wrapper: Vec<String>,
        error: Option<ErrorSpec>,
        #[builder(default = true)] auto_reset: bool,
    ) -> Self {
        Self {
            context: context.unwrap_or_default(),
            pre_process,
            process_list: process,
            post_process,
            wrapper,
            error_spec: error,
            auto_reset,
            last_error: Rc::new(RefCell::new(None)),
        }
    }
}

impl Default for Service {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl Service {
    pub fn new(context: Context) -> Self {
        Self::builder().context(context).build()
    }

    pub fn context(&self) -> &Context {
        &self.context
    }

    pub fn error(&self) -> Option<TemplateException> {
        self.last_error.borrow().clone()
    }

    pub fn process_input(
        &self,
        input: &Value,
        params: Option<HashMap<String, Value>>,
    ) -> Result<String, TemplateException> {
        *self.last_error.borrow_mut() = None;

        if self.auto_reset {
            self.context.reset();
        }

        let compiled = match self.context.template_item(input) {
            Ok(c) => c,
            Err(e) => {
                *self.last_error.borrow_mut() = Some(e.clone());
                return Err(e);
            }
        };

        let mut local_params = params.unwrap_or_default();
        if let CompiledTemplate::Document(ref doc) = compiled {
            local_params.insert(
                "template".to_string(),
                Value::Object(Rc::clone(doc) as Rc<dyn TemplateObject>),
            );
        }

        self.context.localise(Some(local_params));

        let mut output = String::new();
        let mut service_err: Option<TemplateException> = None;

        'service: {
            // 1. PRE_PROCESS
            for name in &self.pre_process {
                match self.context.process(name, None) {
                    Ok(s) => output.push_str(&s),
                    Err(cf) => {
                        service_err = Some(Self::cf_to_exception(cf));
                        break 'service;
                    }
                }
            }

            // 2. PROCESS
            let mut procout = String::new();
            let proc_res: Result<(), ControlFlow> = (|| {
                if let Some(ref plist) = self.process_list {
                    for name in plist {
                        let s = self.context.process(name, None)?;
                        procout.push_str(&s);
                    }
                } else {
                    let s = self
                        .context
                        .process_compiled_list(&[compiled], None, false)?;
                    procout.push_str(&s);
                }
                Ok(())
            })();

            let recovered_procout: Option<String> = match proc_res {
                Ok(()) => Some(procout),
                Err(cf) => {
                    let mut ex = Self::cf_to_exception(cf);
                    if !procout.is_empty() {
                        ex.prepend_text(&procout);
                    }
                    match self.recover(&mut ex) {
                        Some(rec) => Some(rec),
                        None => {
                            service_err = Some(ex);
                            break 'service;
                        }
                    }
                }
            };

            // 3. WRAPPER
            if let Some(mut wrapped_out) = recovered_procout {
                for name in self.wrapper.iter().rev() {
                    let mut wvars = HashMap::new();
                    wvars.insert("content".to_string(), Value::Str(wrapped_out));
                    match self.context.process(name, Some(wvars)) {
                        Ok(s) => wrapped_out = s,
                        Err(cf) => {
                            service_err = Some(Self::cf_to_exception(cf));
                            break 'service;
                        }
                    }
                }
                output.push_str(&wrapped_out);
            }

            // 4. POST_PROCESS
            for name in &self.post_process {
                match self.context.process(name, None) {
                    Ok(s) => output.push_str(&s),
                    Err(cf) => {
                        service_err = Some(Self::cf_to_exception(cf));
                        break 'service;
                    }
                }
            }
        }

        self.context.delocalise();

        if let Some(err) = service_err {
            *self.last_error.borrow_mut() = Some(err.clone());
            return Err(err);
        }

        Ok(output)
    }

    fn cf_to_exception(cf: ControlFlow) -> TemplateException {
        match cf {
            ControlFlow::Exception(ex) => ex,
            ControlFlow::Stop(txt) => TemplateException::with_text("stop", "", txt),
            ControlFlow::Return(txt) => TemplateException::with_text("return", "", txt),
            ControlFlow::Next(_) => TemplateException::new("undef", "NEXT outside loop"),
            ControlFlow::Last(_) => TemplateException::new("undef", "LAST outside loop"),
        }
    }

    fn recover(&self, error: &mut TemplateException) -> Option<String> {
        if error.type_() == "stop" {
            return Some(error.text().to_string());
        }

        let spec = self.error_spec.as_ref()?;
        let handler_name: &str = match spec {
            ErrorSpec::Single(s) => s.as_str(),
            ErrorSpec::Map(map) => {
                let keys: Vec<&str> = map.keys().map(|k| k.as_str()).collect();
                if let Some(matched) = error.select_handler(&keys) {
                    map.get(matched)?.as_str()
                } else if let Some(def) = map.get("default") {
                    def.as_str()
                } else {
                    return None;
                }
            }
        };

        let handler_compiled = match self.context.template_by_name(handler_name) {
            Ok(c) => c,
            Err(e) => {
                *error = e;
                return None;
            }
        };

        let _ = self.context.stash().set(
            &Value::Str("error".to_string()),
            Value::from(error.clone()),
            false,
        );

        match self
            .context
            .process_compiled_list(&[handler_compiled], None, false)
        {
            Ok(out) => Some(out),
            Err(cf) => {
                *error = Self::cf_to_exception(cf);
                None
            }
        }
    }
}
