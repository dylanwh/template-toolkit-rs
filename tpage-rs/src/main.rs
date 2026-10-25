//! `tpage-rs`: a port of Template::Toolkit's `tpage` command.
//!
//! Options mirror `tpage` (AppConfig names, with their aliases).  Options
//! that only make sense for Perl (`--eval_perl`, `--load_perl`, `--module`,
//! `--perl5lib`, the directive `--template_directive_*` switches) are
//! accepted for compatibility and ignored with a warning.

use std::{
    collections::HashMap,
    io::{self, Read, Write},
};

use clap::Parser;
use color_eyre::eyre::{eyre, Result, WrapErr};
use template_toolkit::{
    constants::{DebugFlagsInput, CHOMP_NONE, CHOMP_ONE},
    context::set_while_max,
    provider::IncludePathItem,
    service::ErrorSpec,
    value::Value,
    Template,
};

#[derive(Parser, Debug)]
#[command(
    name = "tpage-rs",
    version,
    about = "Process templates with the Template Toolkit"
)]
struct Args {
    /// Templates to process ("-" or none for STDIN)
    files: Vec<String>,

    /// Disable absolute filenames (enabled by default, as in tpage)
    #[arg(long = "noabsolute", alias = "no_absolute")]
    no_absolute: bool,
    /// Disable relative filenames (enabled by default, as in tpage)
    #[arg(long = "norelative", alias = "no_relative")]
    no_relative: bool,
    /// Accepted for compatibility; absolute paths are on by default
    #[arg(long, hide = true)]
    absolute: bool,
    /// Accepted for compatibility; relative paths are on by default
    #[arg(long, hide = true)]
    relative: bool,

    /// Ignore case of directive keywords
    #[arg(long)]
    anycase: bool,
    /// Interpolate embedded $variables in plain text
    #[arg(long)]
    interpolate: bool,
    /// Chomp whitespace before directives
    #[arg(long = "pre_chomp", visible_alias = "prechomp")]
    pre_chomp: bool,
    /// Chomp whitespace after directives
    #[arg(long = "post_chomp", visible_alias = "postchomp")]
    post_chomp: bool,
    /// Trim leading and trailing whitespace from output
    #[arg(long)]
    trim: bool,
    /// Allow recursive templates
    #[arg(long)]
    recursion: bool,
    /// Expose BLOCKs defined in templates as template/block names
    #[arg(long = "expose_blocks")]
    expose_blocks: bool,

    /// Define a template variable (name=value)
    #[arg(long, visible_alias = "variables", value_name = "NAME=VALUE", value_parser = parse_define)]
    define: Vec<(String, String)>,
    /// Add a directory to the INCLUDE_PATH
    #[arg(
        short = 'I',
        long = "include_path",
        visible_alias = "include",
        value_name = "DIR"
    )]
    include_path: Vec<String>,
    /// Template(s) to process before each main template
    #[arg(
        long = "pre_process",
        visible_alias = "preprocess",
        value_name = "TEMPLATE"
    )]
    pre_process: Vec<String>,
    /// Template(s) to process after each main template
    #[arg(
        long = "post_process",
        visible_alias = "postprocess",
        value_name = "TEMPLATE"
    )]
    post_process: Vec<String>,
    /// Process this template instead of the main one
    #[arg(long, value_name = "TEMPLATE")]
    process: Option<String>,
    /// Wrap each main template in this template
    #[arg(long, value_name = "TEMPLATE")]
    wrapper: Option<String>,
    /// Default template if the requested one is not found
    #[arg(long, value_name = "TEMPLATE")]
    default: Option<String>,
    /// Template to process if an error occurs
    #[arg(long, value_name = "TEMPLATE")]
    error: Option<String>,
    /// Debugging flags (e.g. "dirs,undef")
    #[arg(long, value_name = "FLAGS")]
    debug: Option<String>,
    /// Directive start tag
    #[arg(long = "start_tag", visible_alias = "starttag")]
    start_tag: Option<String>,
    /// Directive end tag
    #[arg(long = "end_tag", visible_alias = "endtag")]
    end_tag: Option<String>,
    /// Named tag style (e.g. "html", "star")
    #[arg(long = "tag_style", visible_alias = "tagstyle")]
    tag_style: Option<String>,
    /// Extension for compiled template files
    #[arg(long = "compile_ext")]
    compile_ext: Option<String>,
    /// Directory for compiled template files
    #[arg(long = "compile_dir")]
    compile_dir: Option<String>,
    /// Plugin base namespace
    #[arg(long = "plugin_base", visible_alias = "pluginbase")]
    plugin_base: Vec<String>,
    /// Make the environment available as the 'env' variable
    #[arg(long)]
    envvars: bool,
    /// Maximum WHILE loop iterations
    #[arg(long = "while_max", alias = "template_directive_while_max")]
    while_max: Option<usize>,

    /// Perl-only (not supported): enable PERL blocks
    #[arg(long = "eval_perl", hide = true)]
    eval_perl: bool,
    /// Perl-only (not supported): load Perl plugin modules
    #[arg(long = "load_perl", hide = true)]
    load_perl: bool,
    /// Perl-only (not supported): Template subclass module
    #[arg(long, hide = true)]
    module: Option<String>,
    /// Perl-only (not supported): extra @INC directories
    #[arg(long, visible_alias = "perllib", hide = true)]
    perl5lib: Vec<String>,
    /// Perl-only (not supported): generated-code debugging
    #[arg(long = "template_directive_debug", hide = true)]
    directive_debug: bool,
    /// Perl-only (not supported): pretty-print generated code
    #[arg(long = "template_directive_pretty", hide = true)]
    directive_pretty: bool,
}

fn parse_define(s: &str) -> Result<(String, String), String> {
    s.split_once('=')
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .ok_or_else(|| format!("expected NAME=VALUE, got '{}'", s))
}

fn warn_unsupported(args: &Args) {
    let ignored = [
        (args.eval_perl, "--eval_perl"),
        (args.load_perl, "--load_perl"),
        (args.module.is_some(), "--module"),
        (!args.perl5lib.is_empty(), "--perl5lib"),
        (args.directive_debug, "--template_directive_debug"),
        (args.directive_pretty, "--template_directive_pretty"),
    ];
    for (_, name) in ignored.iter().filter(|(set, _)| *set) {
        eprintln!("tpage-rs: {} is Perl-specific and is ignored", name);
    }
}

fn build(args: &Args) -> Result<Template> {
    let mut include_path = vec![IncludePathItem::Path(".".into())];
    include_path.extend(args.include_path.iter().cloned().map(IncludePathItem::Path));

    let variables: HashMap<String, Value> = args
        .define
        .iter()
        .map(|(k, v)| (k.clone(), Value::Str(v.clone())))
        .collect();
    let chomp = |on: bool| if on { CHOMP_ONE } else { CHOMP_NONE };

    Template::builder()
        .include_path(include_path)
        .absolute(!args.no_absolute)
        .relative(!args.no_relative)
        .anycase(args.anycase)
        .interpolate(args.interpolate)
        .pre_chomp(chomp(args.pre_chomp))
        .post_chomp(chomp(args.post_chomp))
        .trim(args.trim)
        .recursion(args.recursion)
        .expose_blocks(args.expose_blocks)
        .variables(variables)
        .pre_process(args.pre_process.clone())
        .post_process(args.post_process.clone())
        .maybe_process(args.process.clone().map(|p| vec![p]))
        .wrapper(args.wrapper.clone().into_iter().collect())
        .maybe_default_template(args.default.clone())
        .maybe_error(args.error.clone().map(ErrorSpec::Single))
        .maybe_debug(args.debug.clone().map(DebugFlagsInput::Text))
        .maybe_start_tag(args.start_tag.clone())
        .maybe_end_tag(args.end_tag.clone())
        .maybe_tag_style(args.tag_style.clone())
        .maybe_compile_ext(args.compile_ext.clone())
        .maybe_compile_dir(args.compile_dir.clone())
        .maybe_plugin_base((!args.plugin_base.is_empty()).then(|| args.plugin_base.clone()))
        .build()
        // TemplateException holds Rc values, so it is not Send + Sync.
        .map_err(|e| eyre!("{e}"))
}

fn run(args: &Args) -> Result<()> {
    warn_unsupported(args);
    if let Some(n) = args.while_max {
        set_while_max(n);
    }
    let template = build(args)?;

    let mut vars: HashMap<String, Value> = HashMap::new();
    if args.envvars {
        let env = std::env::vars().map(|(k, v)| (k, Value::Str(v))).collect();
        vars.insert("env".into(), Value::hash(env));
    }

    let files = if args.files.is_empty() {
        vec!["-".to_string()]
    } else {
        args.files.clone()
    };
    let mut stdout = io::stdout().lock();
    for file in &files {
        let output = if file == "-" {
            let mut text = String::new();
            io::stdin()
                .read_to_string(&mut text)
                .wrap_err("reading STDIN")?;
            template.process_text(&text, Some(vars.clone()))
        } else {
            template.process_file(file, Some(vars.clone()))
        }
        .map_err(|e| eyre!("{e}"))?;
        stdout.write_all(output.as_bytes())?;
    }
    stdout.flush()?;
    Ok(())
}

fn main() -> Result<()> {
    color_eyre::install()?;
    run(&Args::parse())
}
