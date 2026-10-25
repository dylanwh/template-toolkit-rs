#![forbid(unsafe_code)]

pub mod constants;
pub mod context;
pub mod document;
pub mod error;
pub mod filters;
pub mod iterator;
pub mod parser;
pub mod plugins;
pub mod provider;
pub mod regex_cache;
pub mod service;
pub mod stash;
pub mod strftime;
pub mod value;
pub mod view;
pub mod vmethods;

use std::{cell::RefCell, collections::HashMap, fs, path::Path, rc::Rc};

use crate::{
    constants::{debug_flags, DebugFlagsInput, CHOMP_NONE},
    context::{BlockDef, Context},
    error::TemplateException,
    filters::{FilterDef, Filters},
    parser::{ConstantsNamespace, Parser},
    plugins::PluginFactory,
    provider::{IncludePathItem, Provider},
    service::{ErrorSpec, Service},
    stash::Stash,
    value::Value,
};

#[derive(Clone, Debug)]
pub struct Template {
    service: Service,
    output_path: Option<String>,
    output: Option<String>,
    last_error: Rc<RefCell<Option<TemplateException>>>,
}

#[bon::bon]
impl Template {
    #[builder(start_fn = builder, finish_fn = build)]
    pub fn build(
        // Parser options
        start_tag: Option<String>,
        end_tag: Option<String>,
        tag_style: Option<String>,
        outline_tag: Option<String>,
        #[builder(default = false)] anycase: bool,
        #[builder(default = false)] interpolate: bool,
        #[builder(default = CHOMP_NONE)] pre_chomp: usize,
        #[builder(default = CHOMP_NONE)] post_chomp: usize,
        #[builder(default = false)] v1dollar: bool,
        constants: Option<HashMap<String, Value>>,
        constants_namespace: Option<String>,
        namespace: Option<HashMap<String, ConstantsNamespace>>,
        #[builder(default = false)] trace_vars: bool,
        parser: Option<Parser>,

        // Provider options
        include_path: Option<Vec<IncludePathItem>>,
        delimiter: Option<String>,
        #[builder(default = false)] absolute: bool,
        #[builder(default = false)] relative: bool,
        default_template: Option<String>,
        cache_size: Option<usize>,
        stat_ttl: Option<u64>,
        compile_ext: Option<String>,
        compile_dir: Option<String>,
        #[builder(default = false)] tolerant: bool,
        unicode: Option<bool>,
        encoding: Option<String>,
        load_templates: Option<Vec<Rc<RefCell<Provider>>>>,
        prefix_map: Option<HashMap<String, Vec<Rc<RefCell<Provider>>>>>,

        // Filters options
        filters: Option<HashMap<String, FilterDef>>,
        load_filters: Option<Vec<Rc<RefCell<Filters>>>>,

        // Plugins options
        plugins: Option<HashMap<String, Rc<dyn PluginFactory>>>,
        plugin_base: Option<Vec<String>>,

        // Stash options
        stash: Option<Stash>,
        variables: Option<HashMap<String, Value>>,
        #[builder(default = false)] strict: bool,
        #[builder(default = false)] context_stash: bool,

        // Context options
        context: Option<Context>,
        blocks: Option<HashMap<String, BlockDef>>,
        views: Option<Vec<(String, HashMap<String, Value>)>>,
        #[builder(default = false)] trim: bool,
        #[builder(default = false)] recursion: bool,
        #[builder(default = false)] expose_blocks: bool,
        debug: Option<DebugFlagsInput>,
        debug_format: Option<String>,

        // Service options
        service: Option<Service>,
        #[builder(default)] pre_process: Vec<String>,
        process: Option<Vec<String>>,
        #[builder(default)] post_process: Vec<String>,
        #[builder(default)] wrapper: Vec<String>,
        error: Option<ErrorSpec>,
        #[builder(default = true)] auto_reset: bool,

        // Output options
        output_path: Option<String>,
        output: Option<String>,
    ) -> Result<Self, TemplateException> {
        let resolved_debug: u32 = match debug {
            // purely numeric DEBUG values are used as-is; only textual ones are converted
            Some(DebugFlagsInput::Number(n)) => n,
            Some(inp) => match debug_flags(inp) {
                Ok(out) => out.as_u32().unwrap_or(0),
                Err(msg) => return Err(TemplateException::new("undef", msg)),
            },
            None => 0,
        };

        if let Some(svc) = service {
            return Ok(Self {
                service: svc,
                output_path,
                output,
                last_error: Rc::new(RefCell::new(None)),
            });
        }

        let ctx = match context {
            Some(c) => c,
            None => {
                let mut ns_map = namespace.unwrap_or_default();
                if let Some(consts) = constants {
                    let cns_key = constants_namespace.unwrap_or_else(|| "constants".to_string());
                    ns_map.insert(cns_key, ConstantsNamespace::new(consts));
                }

                let parser = match parser {
                    Some(p) => p,
                    None => Parser::builder()
                        .maybe_start_tag(start_tag)
                        .maybe_end_tag(end_tag)
                        .maybe_tag_style(tag_style)
                        .maybe_outline_tag(outline_tag)
                        .anycase(anycase)
                        .interpolate(interpolate)
                        .pre_chomp(pre_chomp)
                        .post_chomp(post_chomp)
                        .v1dollar(v1dollar)
                        .namespaces(ns_map)
                        .trace_vars(trace_vars)
                        .debug(resolved_debug)
                        .build()
                        .map_err(|e| TemplateException::new("parse", e))?,
                };

                let providers = match load_templates {
                    Some(p) => p,
                    None => {
                        let prov = Provider::builder()
                            .maybe_include_path(include_path)
                            .maybe_delimiter(delimiter)
                            .absolute(absolute)
                            .relative(relative)
                            .maybe_default(default_template)
                            .maybe_cache_size(cache_size)
                            .maybe_stat_ttl(stat_ttl)
                            .maybe_compile_ext(compile_ext)
                            .maybe_compile_dir(compile_dir)
                            .tolerant(tolerant)
                            .maybe_unicode(unicode)
                            .maybe_encoding(encoding)
                            .parser(Rc::new(RefCell::new(parser)))
                            .build()
                            .map_err(|e| TemplateException::new("file", e))?;
                        vec![Rc::new(RefCell::new(prov))]
                    }
                };

                let filter_providers = match load_filters {
                    Some(fp) => fp,
                    None => {
                        let mut f = Filters::new();
                        if let Some(defs) = filters {
                            for (k, v) in defs {
                                f.store(k, v);
                            }
                        }
                        f.tolerant = tolerant;
                        vec![Rc::new(RefCell::new(f))]
                    }
                };

                let stash_obj = match stash {
                    Some(s) => Some(s),
                    None if context_stash => Some(Stash::new_context(None)),
                    None => None,
                };

                Context::builder()
                    .load_templates(providers)
                    .maybe_prefix_map(prefix_map)
                    .load_filters(filter_providers)
                    .maybe_plugin_factories(plugins)
                    .maybe_plugin_base(plugin_base)
                    .maybe_stash(stash_obj)
                    .maybe_variables(variables)
                    .maybe_blocks(blocks)
                    .maybe_views(views)
                    .recursion(recursion)
                    .trim(trim)
                    .expose_blocks(expose_blocks)
                    .maybe_debug_format(debug_format)
                    .debug(resolved_debug)
                    .strict(strict)
                    .build()?
            }
        };

        let svc = Service::builder()
            .context(ctx)
            .pre_process(pre_process)
            .maybe_process(process)
            .post_process(post_process)
            .wrapper(wrapper)
            .maybe_error(error)
            .auto_reset(auto_reset)
            .build();

        Ok(Self {
            service: svc,
            output_path,
            output,
            last_error: Rc::new(RefCell::new(None)),
        })
    }
}

impl Default for Template {
    fn default() -> Self {
        Self::builder().build().expect("default Template")
    }
}

impl Template {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn service(&self) -> &Service {
        &self.service
    }

    pub fn context(&self) -> &Context {
        self.service.context()
    }

    pub fn error(&self) -> Option<TemplateException> {
        self.last_error.borrow().clone()
    }

    pub fn process_text(
        &self,
        text: &str,
        vars: Option<HashMap<String, Value>>,
    ) -> Result<String, TemplateException> {
        let input = Value::ScalarRef(Rc::new(RefCell::new(text.to_string())));
        self.process(&input, vars)
    }

    pub fn process_file(
        &self,
        filename: &str,
        vars: Option<HashMap<String, Value>>,
    ) -> Result<String, TemplateException> {
        let input = Value::Str(filename.to_string());
        self.process(&input, vars)
    }

    pub fn process(
        &self,
        input: &Value,
        vars: Option<HashMap<String, Value>>,
    ) -> Result<String, TemplateException> {
        *self.last_error.borrow_mut() = None;
        match self.service.process_input(input, vars) {
            Ok(out) => Ok(out),
            Err(e) => {
                *self.last_error.borrow_mut() = Some(e.clone());
                Err(e)
            }
        }
    }

    /// Process to the file named by the OUTPUT option (relative to OUTPUT_PATH).
    pub fn process_to_output(
        &self,
        input: &Value,
        vars: Option<HashMap<String, Value>>,
    ) -> Result<(), TemplateException> {
        match self.output.clone() {
            Some(out) => self.process_to_file(input, vars, &out),
            None => {
                let ex = TemplateException::new("undef", "no OUTPUT option set");
                *self.last_error.borrow_mut() = Some(ex.clone());
                Err(ex)
            }
        }
    }

    pub fn process_to_file(
        &self,
        input: &Value,
        vars: Option<HashMap<String, Value>>,
        out_file: &str,
    ) -> Result<(), TemplateException> {
        let output = self.process(input, vars)?;
        let full_path = match &self.output_path {
            Some(op) if !op.is_empty() => format!("{}/{}", op.trim_end_matches('/'), out_file),
            _ => out_file.to_string(),
        };
        let path = Path::new(&full_path);
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                if let Err(e) = fs::create_dir_all(parent) {
                    let ex = TemplateException::new("undef", e.to_string());
                    *self.last_error.borrow_mut() = Some(ex.clone());
                    return Err(ex);
                }
            }
        }
        if let Err(e) = fs::write(path, output) {
            let ex = TemplateException::new("undef", format!("{}: {}", full_path, e));
            *self.last_error.borrow_mut() = Some(ex.clone());
            return Err(ex);
        }
        Ok(())
    }
}
