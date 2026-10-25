use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

use template_toolkit::{
    provider::{IncludePathItem, TemplateContent, TemplateSource},
    template_build_builder::{SetIncludePath, SetTemplateSource},
    Template, TemplateBuildBuilder,
};

/// Templates in memory, keyed by path, counting the calls the provider makes.
#[derive(Default)]
struct MemorySource {
    files: RefCell<HashMap<String, (String, u64)>>,
    loads: Cell<usize>,
    checks: Cell<usize>,
}

impl MemorySource {
    fn with(files: &[(&str, &str)]) -> Rc<Self> {
        let source = Rc::new(Self::default());
        for (path, text) in files {
            source.put(path, text, 1);
        }
        source
    }

    fn put(&self, path: &str, text: &str, modified: u64) {
        self.files
            .borrow_mut()
            .insert(path.to_string(), (text.to_string(), modified));
    }
}

impl TemplateSource for MemorySource {
    fn load(&self, path: &str) -> Result<Option<TemplateContent>, String> {
        self.loads.set(self.loads.get() + 1);
        if path.ends_with("broken.tt") {
            return Err("storage unavailable".into());
        }
        Ok(self
            .files
            .borrow()
            .get(path)
            .map(|(text, modified)| TemplateContent {
                bytes: text.clone().into_bytes(),
                modified: *modified,
            }))
    }

    fn modified(&self, path: &str) -> Option<u64> {
        self.checks.set(self.checks.get() + 1);
        self.files.borrow().get(path).map(|(_, modified)| *modified)
    }
}

/// A Template builder reading from `source` through these include paths.
fn template(
    source: &Rc<MemorySource>,
    paths: &[&str],
) -> TemplateBuildBuilder<SetIncludePath<SetTemplateSource>> {
    Template::builder()
        .template_source(source.clone())
        .include_path(
            paths
                .iter()
                .map(|p| IncludePathItem::Path(p.to_string()))
                .collect(),
        )
}

#[test]
fn directives_load_through_the_source() {
    let source = MemorySource::with(&[
        ("site/layout.tt", "<main>[% content %]</main>"),
        ("site/footer.tt", "(c) [% year %]"),
        (
            "posts/hello.tt",
            "[% WRAPPER layout.tt %]Hi [% INCLUDE footer.tt year = 2026 %][% END %]",
        ),
        ("site/raw.txt", "[% not parsed %]"),
    ]);
    let tt = template(&source, &["posts", "site"]).build().unwrap();
    assert_eq!(
        tt.process_file("hello.tt", None).unwrap(),
        "<main>Hi (c) 2026</main>"
    );
    assert_eq!(
        tt.process_text("[% INSERT raw.txt %]", None).unwrap(),
        "[% not parsed %]"
    );
}

#[test]
fn earlier_include_paths_win() {
    let source = MemorySource::with(&[("theme/header.tt", "theme"), ("base/header.tt", "base")]);
    let tt = template(&source, &["theme", "base"]).build().unwrap();
    assert_eq!(tt.process_file("header.tt", None).unwrap(), "theme");
    let tt = template(&source, &["base", "theme"]).build().unwrap();
    assert_eq!(tt.process_file("header.tt", None).unwrap(), "base");
}

#[test]
fn missing_templates_decline_and_default_applies() {
    let source = MemorySource::with(&[("t/404.tt", "not found: [% template.name %]")]);
    let tt = template(&source, &["t"]).build().unwrap();
    let err = tt.process_file("nope.tt", None).unwrap_err().to_string();
    assert!(err.contains("nope.tt: not found"), "{err}");

    let tt = template(&source, &["t"])
        .default_template("404.tt".into())
        .build()
        .unwrap();
    assert_eq!(
        tt.process_file("nope.tt", None).unwrap(),
        "not found: 404.tt"
    );
}

#[test]
fn source_errors_are_file_errors() {
    let source = MemorySource::with(&[]);
    let tt = template(&source, &["t"]).build().unwrap();
    let err = tt.process_file("broken.tt", None).unwrap_err().to_string();
    assert!(
        err.starts_with("file error") && err.contains("storage unavailable"),
        "{err}"
    );
}

#[test]
fn one_load_per_template_then_cached() {
    let source = MemorySource::with(&[
        ("t/page.tt", "[% INCLUDE part.tt %][% INCLUDE part.tt %]"),
        ("t/part.tt", "x"),
    ]);
    let tt = template(&source, &["t"]).stat_ttl(3600).build().unwrap();
    for _ in 0..3 {
        assert_eq!(tt.process_file("page.tt", None).unwrap(), "xx");
    }
    assert_eq!(source.loads.get(), 2, "page.tt and part.tt, once each");
    assert_eq!(source.checks.get(), 0, "nothing re-checked within STAT_TTL");
}

#[test]
fn changed_stamps_reload() {
    let source = MemorySource::with(&[("t/page.tt", "v1")]);
    let tt = template(&source, &["t"]).stat_ttl(0).build().unwrap();
    assert_eq!(tt.process_file("page.tt", None).unwrap(), "v1");

    // Same stamp: the cached copy stands, even though the text changed.
    source.put("t/page.tt", "v1 edited", 1);
    assert_eq!(tt.process_file("page.tt", None).unwrap(), "v1");

    source.put("t/page.tt", "v2", 2);
    assert_eq!(tt.process_file("page.tt", None).unwrap(), "v2");
}

#[test]
fn stamp_zero_is_never_rechecked() {
    let source = Rc::new(MemorySource::default());
    source.put("t/page.tt", "v1", 0);
    let tt = template(&source, &["t"]).stat_ttl(0).build().unwrap();
    assert_eq!(tt.process_file("page.tt", None).unwrap(), "v1");
    source.put("t/page.tt", "v2", 5);
    assert_eq!(tt.process_file("page.tt", None).unwrap(), "v1");
    assert_eq!(source.checks.get(), 0);
}
