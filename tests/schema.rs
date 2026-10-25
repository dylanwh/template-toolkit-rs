use serde_json::{json, Value as Json};
use template_toolkit::{
    provider::IncludePathItem,
    schema::{Inference, SchemaOptions},
    Template,
};

fn infer(text: &str) -> Inference {
    Template::new()
        .infer_schema_text(text, &SchemaOptions::default())
        .expect("infer")
}

fn schema(text: &str) -> Json {
    infer(text).schema
}

/// The schema at a dotted path of property names ("*" for items).
fn at<'a>(schema: &'a Json, path: &str) -> &'a Json {
    path.split('.').fold(schema, |s, key| match key {
        "*" => &s["items"],
        key => &s["properties"][key],
    })
}

fn required(schema: &Json) -> Vec<&str> {
    schema["required"]
        .as_array()
        .map(|r| r.iter().map(|v| v.as_str().unwrap()).collect())
        .unwrap_or_default()
}

#[test]
fn dotted_names_make_objects() {
    let s = schema("[% user.name %] [% user.address.city %]");
    assert_eq!(at(&s, "user")["type"], "object");
    assert_eq!(
        at(&s, "user.address.city")["type"],
        json!(["string", "number"])
    );
    assert_eq!(required(&s), ["user"]);
    assert_eq!(required(at(&s, "user")), ["address", "name"]);
}

#[test]
fn foreach_makes_arrays_of_items() {
    let s =
        schema("[% FOREACH row IN rows %][% row.id + 1 %][% loop.count %][% END %][% list.0.x %]");
    assert_eq!(at(&s, "rows")["type"], "array");
    assert_eq!(at(&s, "rows.*.id")["type"], "number");
    assert_eq!(required(at(&s, "rows.*")), ["id"]);
    assert_eq!(at(&s, "list")["type"], "array");
    assert!(at(&s, "loop").is_null(), "loop is TT's");
    assert!(at(&s, "row").is_null(), "row is the loop variable");
}

#[test]
fn locals_are_not_inputs() {
    let s = schema(concat!(
        "[% x = 1 %][% x %]",
        "[% MACRO m(p) BLOCK %][% p.q %][% END %][% m(arg) %]",
        "[% USE d = Date %][% d.format %]",
        "[% TRY %][% CATCH %][% error.info %][% END %]",
        "[% c = BLOCK %]..[% END %][% c %]",
    ));
    let props: Vec<_> = s["properties"].as_object().unwrap().keys().collect();
    assert_eq!(props, ["arg"]);
}

#[test]
fn aliases_follow_the_input() {
    let s = schema("[% u = data.user %][% u.name %]");
    assert!(at(&s, "data.user.name").is_object());
    assert_eq!(required(at(&s, "data.user")), ["name"]);
}

#[test]
fn uses_before_set_are_inputs() {
    let s = schema("[% x.y %][% x = 1 %][% x.z %]");
    assert!(at(&s, "x.y").is_object());
    assert!(at(&s, "x.z").is_null());
}

#[test]
fn conditional_uses_are_optional() {
    let s = schema(concat!(
        "[% IF flag %][% inner %][% END %]",
        "[% a || 'none' %][% b ? c : d %]",
        "[% DEFAULT title = 'Untitled' %][% title %]",
        "[% always %]",
    ));
    assert_eq!(required(&s), ["always"]);
    assert_eq!(at(&s, "a")["default"], "none");
    assert_eq!(at(&s, "title")["default"], "Untitled");
}

#[test]
fn set_in_a_loop_may_be_an_input_after_it() {
    let s = schema("[% FOREACH i IN list %][% last = i %][% last %][% END %][% last %]");
    assert!(at(&s, "last").is_object());
    assert!(!required(&s).contains(&"last"));
}

#[test]
fn vmethods_hint_types() {
    let s = schema(concat!(
        "[% FOREACH k IN h.keys %][% k %][% END %]",
        "[% FOREACH p IN prefs.pairs %][% p.value.label %][% END %]",
        "[% tags.join(', ') %][% name.upper %][% n.inc %]",
        "[% cart.items.size %][% m.$k %]",
    ));
    assert_eq!(at(&s, "h")["type"], "object");
    assert_eq!(
        at(&s, "prefs")["additionalProperties"]["properties"]["label"]["type"],
        json!(["string", "number"])
    );
    assert_eq!(at(&s, "tags")["type"], "array");
    assert_eq!(at(&s, "name")["type"], json!(["string", "number"]));
    assert_eq!(at(&s, "n")["type"], "number");
    // `items` without parentheses is a field, not the hash vmethod.
    assert_eq!(at(&s, "cart.items")["type"], json!(["object", "array"]));
    assert!(at(&s, "m")["additionalProperties"].is_object());
}

#[test]
fn filters_type_their_arguments() {
    let out = infer(concat!(
        "[% bio | truncate(maxlen, more) | upper %]",
        "[% FILTER repeat(times) %][% body %][% END %]",
        "[% FILTER $chosen %]x[% END %]",
        "[% src | eval %]",
    ));
    let s = &out.schema;
    assert_eq!(at(s, "bio")["type"], json!(["string", "number"]));
    assert_eq!(at(s, "maxlen")["type"], "number");
    assert_eq!(at(s, "more")["type"], json!(["string", "number"]));
    assert_eq!(at(s, "times")["type"], "number");
    assert_eq!(at(s, "body")["type"], json!(["string", "number"]));
    assert_eq!(at(s, "chosen")["type"], "string");
    assert_eq!(
        out.warnings.len(),
        2,
        "$chosen and eval: {:?}",
        out.warnings
    );
}

#[test]
fn methods_are_flagged() {
    let s = schema("[% obj.method(1).x %]");
    assert_eq!(at(&s, "obj.method")["x-tt-callable"], true);
    assert!(required(at(&s, "obj")).is_empty());
}

#[test]
fn blocks_are_followed_with_their_arguments() {
    let s = schema(concat!(
        "[% BLOCK row %][% item.name %] [% extra %][% END %]",
        "[% INCLUDE row item = product %]",
        "[% BLOCK unused %][% other %][% END %]",
    ));
    assert!(at(&s, "product.name").is_object());
    assert!(at(&s, "item").is_null());
    assert!(at(&s, "other").is_object(), "unused blocks are scanned");
    assert_eq!(required(&s), ["extra", "product"]);
}

#[test]
fn include_and_wrapper_files_are_followed() {
    let dir = std::env::temp_dir().join(format!("tt-schema-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("layout.tt"),
        "<title>[% site.name %]</title>[% content %]",
    )
    .unwrap();
    std::fs::write(dir.join("footer.tt"), "[% year %] [% site.owner %]").unwrap();
    let tt = Template::builder()
        .include_path(vec![IncludePathItem::Path(dir.to_string_lossy().into())])
        .build()
        .unwrap();
    let out = tt
        .infer_schema_text(
            "[% WRAPPER layout.tt %][% INCLUDE footer.tt year = now.year %][% INCLUDE $dynamic %][% END %]",
            &SchemaOptions::default(),
        )
        .unwrap();
    let s = &out.schema;
    assert!(at(s, "site.name").is_object());
    assert!(at(s, "site.owner").is_object());
    assert!(at(s, "now.year").is_object());
    assert!(at(s, "year").is_null() && at(s, "content").is_null());
    assert_eq!(out.warnings.len(), 1, "{:?}", out.warnings);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn recursion_terminates() {
    let s = schema("[% BLOCK tree %][% node.name %][% FOREACH node IN node.children %][% INCLUDE tree %][% END %][% END %][% INCLUDE tree %]");
    assert!(at(&s, "node.name").is_object());
    assert_eq!(at(&s, "node.children")["type"], "array");
}

#[test]
fn sources_are_recorded() {
    let out = Template::new()
        .infer_schema_text(
            "\n[% a %]\n\n[% a %]",
            &SchemaOptions {
                sources: true,
                title: None,
            },
        )
        .unwrap();
    assert_eq!(
        at(&out.schema, "a")["x-tt-source"],
        json!(["input text:2", "input text:4"])
    );
}

#[test]
fn modules_and_filters_are_listed() {
    let out = infer(concat!(
        "[% USE Date %][% USE fmt = Format('%05d') %][% fmt(n) %]",
        "[% name | html %][% bio | truncate(40) | upper %]",
        "[% FILTER keep = replace('a', 'b') %]x[% END %][% w | keep %]",
        "[% BLOCK side %][% USE Dumper %][% q | uri | markdown %][% END %]",
        "[% x | money('USD') %][% y | eval %]",
    ));
    let names = |set: &std::collections::BTreeSet<String>| set.iter().cloned().collect::<Vec<_>>();
    assert_eq!(names(&out.modules), ["Date", "Dumper", "Format"]);
    // Built-in filters and the template's own aliases are left out.
    assert_eq!(names(&out.filters), ["markdown", "money"]);
    let report = out.report();
    assert_eq!(report["modules"], json!(["Date", "Dumper", "Format"]));
    assert!(report["variables"]["properties"]["bio"].is_object());
}

#[test]
fn item_with_a_literal_key_is_that_key() {
    let s = schema(
        r#"[% user.item("name") %] [% user.item('address').city %] [% rows.item(0).id %] [% conf.item(key) %]"#,
    );
    assert_eq!(at(&s, "user.name")["type"], json!(["string", "number"]));
    assert_eq!(
        at(&s, "user.address.city")["type"],
        json!(["string", "number"])
    );
    assert_eq!(required(at(&s, "user")), ["address", "name"]);
    assert_eq!(at(&s, "rows")["type"], "array");
    assert!(at(&s, "rows.*.id").is_object());
    assert!(
        at(&s, "key").is_object(),
        "a variable key is an input itself"
    );
    assert_eq!(schema("[% user.item('name') %]"), schema("[% user.name %]"));
}
