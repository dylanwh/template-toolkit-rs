#![forbid(unsafe_code)]

mod clock;
pub mod constants;
pub mod context;
pub mod convert;
pub mod document;
pub mod error;
pub mod filters;
pub mod iterator;
pub mod parser;
pub mod plugins;
pub mod provider;
pub mod regex_cache;
pub mod serde_value;
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
    convert::{FromArgs, IntoFilter, IntoResult, IntoText, NativeFn},
    error::{ControlFlow, TemplateException},
    filters::{Filter, FilterDef, FilterFactory, FilterFactoryOutcome, Filters},
    parser::{ConstantsNamespace, Parser},
    plugins::PluginFactory,
    provider::{IncludePathItem, Provider},
    service::{ErrorSpec, Service},
    stash::Stash,
    value::{CallContext, Value},
    vmethods::VMethod,
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

    // Callback registration.  Each kind comes in up to three forms, as in
    // mlua: closures with typed arguments (`|a: i64, b: String|`, converted
    // with Perl-style coercion; see [`convert`]), closures that also get the
    // [`Context`] (`*_with_context`), and types implementing the trait
    // (`*_impl`).  Closure forms are bounded on `Fn` directly so their
    // signatures are inferred.

    /// A static filter: `[% text | name %]` or `[% FILTER name %]`.  The
    /// closure takes the text and returns a `String` or
    /// `Result<String, ControlFlow>`.
    pub fn define_filter<F, R>(&self, name: impl Into<String>, filter: F) -> &Self
    where
        F: Fn(&str) -> R + 'static,
        R: IntoText,
    {
        self.define_filter_impl(name, move |text: &str| filter(text).into_text())
    }

    /// A static filter implemented by a type.
    pub fn define_filter_impl(
        &self,
        name: impl Into<String>,
        filter: impl Filter + 'static,
    ) -> &Self {
        self.context()
            .define_filter(name, FilterDef::Static(Rc::new(filter)));
        self
    }

    /// A dynamic filter, built per use from its arguments: `[% text |
    /// name(args) %]`.  The closure takes typed arguments and returns a
    /// filter (any `Fn(&str) -> String`, or a `Result` of one).
    pub fn define_filter_factory<A, F>(&self, name: impl Into<String>, factory: F) -> &Self
    where
        A: FromArgs + 'static,
        F: NativeFn<A> + 'static,
        F::Output: IntoFilter,
    {
        let name = name.into();
        let fname = name.clone();
        self.define_filter_factory_impl(name, move |args: &[Value]| {
            convert::call_typed(&factory, args, &fname)?
                .into_filter()
                .map(FilterFactoryOutcome::Ok)
        })
    }

    /// A dynamic filter whose factory also gets the context, as Perl's do.
    pub fn define_filter_factory_with_context<F, R>(
        &self,
        name: impl Into<String>,
        factory: F,
    ) -> &Self
    where
        F: Fn(&Context, &[Value]) -> R + 'static,
        R: IntoFilter,
    {
        self.define_filter_factory_impl(name, ContextFilterFactory(factory))
    }

    /// A dynamic filter factory implemented by a type.
    pub fn define_filter_factory_impl(
        &self,
        name: impl Into<String>,
        factory: impl FilterFactory + 'static,
    ) -> &Self {
        self.context()
            .define_filter(name, FilterDef::Dynamic(Rc::new(factory)));
        self
    }

    /// A plugin for `[% USE name(args) %]`, built from typed arguments.
    pub fn define_plugin<A, F>(&self, name: impl Into<String>, plugin: F) -> &Self
    where
        A: FromArgs + 'static,
        F: NativeFn<A> + 'static,
        F::Output: IntoResult,
    {
        let name = name.into();
        let fname = name.clone();
        self.define_plugin_impl(name, move |args: &[Value]| {
            convert::call_typed(&plugin, args, &fname)
                .and_then(IntoResult::into_result)
                .map_err(plugin_error)
        })
    }

    /// A plugin whose constructor also gets the context, as Perl's do.
    pub fn define_plugin_with_context<F, R>(&self, name: impl Into<String>, plugin: F) -> &Self
    where
        F: Fn(&Context, &[Value]) -> R + 'static,
        R: IntoResult,
    {
        self.define_plugin_impl(name, ContextPlugin(plugin))
    }

    /// A plugin factory implemented by a type.
    pub fn define_plugin_impl(
        &self,
        name: impl Into<String>,
        plugin: impl PluginFactory + 'static,
    ) -> &Self {
        self.context().register_plugin(name, Rc::new(plugin));
        self
    }

    /// A virtual method on `"scalar"`, `"list"`, `"hash"` or `"root"`
    /// values: `[% value.name(args) %]`.  The value is the closure's first
    /// argument: `|s: String, width: Option<i64>| ...`.
    pub fn define_vmethod<A, F>(&self, vtype: &str, name: &str, vmethod: F) -> &Self
    where
        A: FromArgs + 'static,
        F: NativeFn<A> + 'static,
        F::Output: IntoResult,
    {
        let fname = name.to_string();
        self.define_vmethod_impl(vtype, name, move |target: &Value, args: &[Value]| {
            let mut all = Vec::with_capacity(args.len() + 1);
            all.push(target.clone());
            all.extend_from_slice(args);
            convert::call_typed(&vmethod, &all, &fname)?.into_result()
        })
    }

    /// A virtual method implemented by a type, or a closure over the raw
    /// `(&Value, &[Value])`.
    pub fn define_vmethod_impl(
        &self,
        vtype: &str,
        name: &str,
        vmethod: impl VMethod + 'static,
    ) -> &Self {
        self.context().define_vmethod(vtype, name, vmethod);
        self
    }

    /// A global template variable, visible to every `process` call.
    pub fn define_variable(&self, name: impl Into<String>, value: impl Into<Value>) -> &Self {
        self.context().stash().set_raw(name, value.into());
        self
    }

    /// A global function with typed arguments: `[% name(args) %]`.
    pub fn define_function<A, F>(&self, name: impl Into<String>, function: F) -> &Self
    where
        A: FromArgs + 'static,
        F: NativeFn<A> + 'static,
        F::Output: IntoResult,
    {
        let name = name.into();
        let fname = name.clone();
        self.define_function_raw(name, move |args: &[Value], _: CallContext| {
            convert::call_typed(&function, args, &fname)?.into_result()
        })
    }

    /// A global function over the raw argument list and calling context.
    pub fn define_function_raw<F, R>(&self, name: impl Into<String>, function: F) -> &Self
    where
        F: Fn(&[Value], CallContext) -> R + 'static,
        R: IntoResult,
    {
        let f = move |args: &[Value], cx: CallContext| function(args, cx).into_result();
        self.define_variable(name, Value::Function(Rc::new(f)))
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

/// Adapts a `Fn(&Context, &[Value])` closure into a [`FilterFactory`].
struct ContextFilterFactory<F>(F);

impl<F, R> FilterFactory for ContextFilterFactory<F>
where
    F: Fn(&Context, &[Value]) -> R,
    R: IntoFilter,
{
    fn create(
        &self,
        context: &Context,
        args: &[Value],
    ) -> Result<FilterFactoryOutcome, ControlFlow> {
        (self.0)(context, args)
            .into_filter()
            .map(FilterFactoryOutcome::Ok)
    }
}

/// Adapts a `Fn(&Context, &[Value])` closure into a [`PluginFactory`].
struct ContextPlugin<F>(F);

impl<F, R> PluginFactory for ContextPlugin<F>
where
    F: Fn(&Context, &[Value]) -> R,
    R: IntoResult,
{
    fn load(&self, context: &Context, args: &[Value]) -> Result<Value, TemplateException> {
        (self.0)(context, args).into_result().map_err(plugin_error)
    }
}

/// Plugins report a `TemplateException`; other control flow from a plugin
/// constructor has no meaning there.
fn plugin_error(cf: ControlFlow) -> TemplateException {
    match cf {
        ControlFlow::Exception(e) => e,
        _ => TemplateException::new(
            "plugin",
            "plugin constructor exited with a control directive",
        ),
    }
}
