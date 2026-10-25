mod common;
use std::{cell::RefCell, path::Path, rc::Rc};

use common::*;
use template_toolkit::{
    constants::CHOMP_ALL,
    error::{ControlFlow, TemplateException},
    filters::{self, FilterDef, FilterFactoryOutcome, Filters},
    value::Value,
    Template,
};

/// Perl-evaluation features (PERL/RAWPERL blocks, the `evalperl`/`perl`
/// filters actually evaluating code, EVAL_PERL) are out of scope for the Rust
/// port.  Cases that need them are replaced by an empty case that keeps any
/// `-- use NAME --` processor switch, so later cases still run on the right
/// processor.
fn drop_perl_cases(corpus: &str) -> String {
    let mut out = String::new();
    let mut first = true;
    for case in corpus.split("-- test --") {
        if first {
            out.push_str(case);
            first = false;
            continue;
        }
        let needs_perl = case.contains("[% PERL")
            || case.contains("FILTER evalperl")
            || case.contains("FILTER perl")
            || case.contains("-- use evalperl --")
                && !case.contains("\n-- expect --\n-- process --");
        if needs_perl {
            out.push_str("-- test --\n");
            if case.contains("-- use evalperl --") {
                out.push_str("-- use evalperl --\n");
            }
            out.push_str("-- expect --\n\n");
        } else {
            out.push_str("-- test --");
            out.push_str(case);
        }
    }
    out
}

#[test]
fn filter() {
    let dir = "Template-Toolkit-3.009/t/test/tmp";
    let file = "xyz";
    let (a, b, c, d) = ("alpha", "bravo", "charlie", "delta");
    let stderr = Rc::new(RefCell::new(String::new()));

    let stderr_fn = stderr.clone();
    let params = vars(vec![
        ("a", a.into()),
        ("b", b.into()),
        ("c", c.into()),
        ("d", d.into()),
        ("list", strs(&[a, b, c, d])),
        ("text", "The cat sat on the mat".into()),
        ("outfile", file.into()),
        (
            "stderr",
            Value::function(move |_, _| Ok(Value::Str(stderr_fn.borrow().clone()))),
        ),
        // Upstream: bless(\&despace, 'anything') -- a blessed coderef which the
        // stash does not auto-call.  Rust Values have no blessed coderefs, so
        // use the equivalent `sub { \&despace }` form that the stash calls to
        // obtain the filter.
        (
            "despace",
            Value::function(|_, _| {
                Ok(Value::function(|args, _| {
                    let t = args.first().map(|v| v.to_tt_string()).unwrap_or_default();
                    let re = regex::Regex::new(r"\s+").unwrap();
                    Ok(Value::Str(re.replace_all(&t, "_").into_owned()))
                }))
            }),
        ),
        ("widetext", "wide:\u{65e5}\u{672c}\u{8a9e}".into()),
        (
            "use_rfc2732",
            Value::function(|_, _| {
                filters::use_rfc2732();
                Ok(Value::Str(String::new()))
            }),
        ),
        (
            "use_rfc3986",
            Value::function(|_, _| {
                filters::use_rfc3986();
                Ok(Value::Str(String::new()))
            }),
        ),
    ]);

    let microjive = FilterDef::Static(Rc::new(|t: &str| {
        Ok(regex::Regex::new(r"(?is)microsoft")
            .unwrap()
            .replace_all(t, "The 'Soft")
            .into_owned())
    }));
    let microsloth = FilterDef::Static(Rc::new(|t: &str| {
        Ok(regex::Regex::new(r"(?is)microsoft")
            .unwrap()
            .replace_all(t, "Microsloth")
            .into_owned())
    }));
    let censor = FilterDef::Dynamic(Rc::new(|args: &[Value]| {
        let words: Vec<String> = args.iter().map(|v| v.to_tt_string()).collect();
        Ok(FilterFactoryOutcome::Ok(Rc::new(move |t: &str| {
            let mut t = t.to_string();
            for w in &words {
                t = regex::Regex::new(&format!("(?is){}", w))
                    .unwrap()
                    .replace_all(&t, "[** CENSORED **]")
                    .into_owned();
            }
            Ok(t)
        })))
    }));
    let badfact = FilterDef::Dynamic(Rc::new(|_| Ok(FilterFactoryOutcome::InvalidReturn)));
    let barfilt = FilterDef::Dynamic(Rc::new(|args: &[Value]| {
        let foad = args.first().map(|v| v.to_number() as i64).unwrap_or(0);
        match foad {
            0 => Ok(FilterFactoryOutcome::Error(TemplateException::new(
                "filter", "barfed",
            ))),
            1 => Ok(FilterFactoryOutcome::Error(TemplateException::new(
                "dead", "deceased",
            ))),
            2 => Err(ControlFlow::Exception(TemplateException::new(
                "undef",
                "keeled over",
            ))),
            _ => Err(ControlFlow::Exception(TemplateException::new(
                "unwell",
                "sick as a parrot",
            ))),
        }
    }));

    let make_filters = |path: Option<String>| {
        let mut f = Filters::new();
        f.stderr_sink = Some(stderr.clone());
        f.output_path = path.map(Into::into);
        f.store("nonfilt", FilterDef::InvalidEntry);
        f.store("microjive", microjive.clone());
        f.store("microsloth", microsloth.clone());
        f.store("censor", censor.clone());
        f.store("badfact", badfact.clone());
        f.store("badfilt", FilterDef::InvalidEntry);
        f.store("barfilt", barfilt.clone());
        Rc::new(RefCell::new(f))
    };

    let _ = std::fs::remove_file(Path::new(dir).join(file));

    let tt1 = Template::builder()
        .interpolate(true)
        .post_chomp(CHOMP_ALL)
        .load_filters(vec![make_filters(None)])
        .build()
        .unwrap();
    // config2 upstream also sets EVAL_PERL (out of scope) and BARVAL (only used from PERL code).
    let tt2 = Template::builder()
        .load_filters(vec![make_filters(Some(dir.to_string()))])
        .build()
        .unwrap();

    tt2.context().define_filter(
        "another",
        FilterDef::Dynamic(Rc::new(|args: &[Value]| {
            let n = args.first().map(|v| v.to_number() as usize).unwrap_or(0);
            Ok(FilterFactoryOutcome::Ok(Rc::new(move |t: &str| {
                Ok(t.repeat(n))
            })))
        })),
    );

    let corpus = drop_perl_cases(include_str!("corpus/filter.tt"));
    test_expect(&corpus, vec![("default", tt1), ("evalperl", tt2)], params);

    assert!(
        Path::new(dir).join(file).is_file(),
        "{}/{} exists",
        dir,
        file
    );
    let _ = std::fs::remove_file(Path::new(dir).join(file));
}
