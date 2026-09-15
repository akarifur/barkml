//! End-to-end tests for the domain-neutral grammar boundary (docs/07).
//!
//! Application vocabulary (Stead's `app`, `profile`, `override`, `stead`, ...)
//! must behave like any other block identifier: no built-in meaning, no
//! selection or merge side effects, no schema enforcement beyond generic
//! syntax and type rules.

use std::io::Cursor;

use barkml::{Loader, Scope, StandardLoader, Statement, from_str};

fn parse_unresolved(input: &str) -> Statement {
    let mut loader = StandardLoader::default();
    loader
        .skip_macro_resolution()
        .expect("disable resolution")
        .add_module("main", &mut Cursor::new(input.as_bytes()), None)
        .expect("add module")
        .read()
        .expect("parse unresolved")
}

fn labels_of(block: &Statement) -> Vec<String> {
    block
        .identity()
        .1
        .iter()
        .map(|v| v.as_string().cloned().expect("labels are string values"))
        .collect()
}

#[test]
fn stead_blocks_parse_like_arbitrary_block_names() {
    let stead = parse_unresolved(
        r#"
        app "firefox" {
            enabled = true
        }

        profile "minimal" {
            override {
                app "firefox" {
                    enabled = false
                }
            }
        }
        "#,
    );

    let (id, labels, app) = stead.blocks().next().expect("app block present");
    assert_eq!(id, "app");
    assert_eq!(labels_of(app), vec!["firefox".to_string()]);
    assert_eq!(labels.len(), 1);

    // An identically shaped document with arbitrary names parses to the same
    // shape: identity kind, label, and nested assignment.
    let arbitrary = parse_unresolved(
        r#"
        widget "gizmo" {
            enabled = true
        }

        bundle "tiny" {
            override {
                widget "gizmo" {
                    enabled = false
                }
            }
        }
        "#,
    );

    let (arbitrary_id, _, arbitrary_app) = arbitrary.blocks().next().expect("widget block");
    assert_eq!(arbitrary_id, "widget");
    assert_eq!(labels_of(arbitrary_app), vec!["gizmo".to_string()]);

    let enabled = |s: &Statement| {
        s.get_child("app", &["firefox"])
            .or_else(|| s.get_child("widget", &["gizmo"]))
            .expect("labeled block")
            .find_child("enabled")
            .expect("assignment")
            .get_value()
            .expect("value")
            .as_bool()
            .copied()
            .expect("bool")
    };
    assert_eq!(enabled(&stead), enabled(&arbitrary));
}

#[test]
fn unrecognized_consumer_vocabulary_is_not_a_syntax_error() {
    let module = parse_unresolved(
        r#"
        provider "github" {
            endpoint = "https://api.github.com"
            retries = 3
        }

        frobnicate "mode-9" {
            strategy = "aggressive"
            semver_floor = 1.2.3
            requirement = >=2.0.0
        }
        "#,
    );
    assert!(module.get_child("provider", &["github"]).is_some());
    assert!(module.get_child("frobnicate", &["mode-9"]).is_some());
}

#[test]
fn parsing_and_resolution_perform_no_consumer_actions() {
    // A profile/override document stays structurally inert: no profile is
    // selected and no override is merged by parse or generic resolution.
    let input = r#"
    vars {
        enabled = true
    }

    profile "minimal" {
        override {
            app "firefox" {
                enabled = false
            }
        }
    }

    app "firefox" {
        enabled = vars.enabled
    }
    "#;

    let module = from_str(input).expect("parse and resolve");

    // The override block is still nested under the profile, untouched.
    let profile = module
        .get_child("profile", &["minimal"])
        .expect("profile block preserved");
    let (nested_id, nested_labels, _) = profile
        .find_child("override")
        .expect("override not merged away")
        .blocks()
        .next()
        .expect("nested app preserved");
    assert_eq!(nested_id, "app");
    assert_eq!(nested_labels.len(), 1);

    // Generic resolution resolved the reference, not the override: the root
    // app keeps the value its own assignment resolved to.
    let app = module.get_child("app", &["firefox"]).expect("root app");
    let enabled = app
        .find_child("enabled")
        .expect("assignment")
        .get_value()
        .expect("value");
    assert_eq!(enabled.as_bool(), Some(&true));
}

#[test]
fn explicit_context_resolution_without_ambient_host_discovery() {
    let user_config = r#"
    app "firefox" {
        enabled = host.use_browser
    }
    "#;

    let host_facts = r#"
    host {
        use_browser = true
    }
    "#;

    let mut loader = StandardLoader::default();
    loader
        .skip_macro_resolution()
        .expect("disable resolution")
        .add_module("main", &mut Cursor::new(host_facts.as_bytes()), None)
        .expect("host facts module")
        .add_module("main", &mut Cursor::new(user_config.as_bytes()), None)
        .expect("user module");

    let module = loader.read().expect("read unresolved");
    let mut scope = Scope::new(&module);
    let resolved = scope.apply().expect("generic resolution");

    let enabled = resolved
        .get_child("app", &["firefox"])
        .expect("labeled block")
        .find_child("enabled")
        .expect("assignment")
        .get_value()
        .expect("value");
    assert_eq!(enabled.as_bool(), Some(&true));
}

#[test]
fn labels_and_locations_survive_for_downstream_diagnostics() {
    let module = parse_unresolved(
        r#"
        app "firefox" {
            enabled = true
        }
        "#,
    );

    let (_, labels, block) = module.blocks().next().expect("app block");
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0].as_string().map(String::as_str), Some("firefox"));

    // Source locations remain attached for consumer-side diagnostics.
    let assignment = block.find_child("enabled").expect("assignment");
    assert!(assignment.meta.location.line > 0 || assignment.meta.location.column > 0);
}

#[test]
fn generic_literals_are_not_stead_extensions() {
    let module = from_str(
        r#"
        cache {
            release = 1.2.3
            floor = >=2.0.0
            count = 10u32
        }
        "#,
    )
    .expect("parse and resolve");

    let cache = module.find_child("cache").expect("cache block");
    let version = cache
        .find_child("release")
        .expect("version assignment")
        .get_value()
        .expect("value");
    assert_eq!(version.as_version(), Some(&semver::Version::new(1, 2, 3)));

    let floor = cache
        .find_child("floor")
        .expect("floor assignment")
        .get_value()
        .expect("value");
    assert!(floor.as_require().is_some());

    let count = cache
        .find_child("count")
        .expect("count assignment")
        .get_value()
        .expect("value");
    assert_eq!(count.as_u32(), Some(&10));
}
