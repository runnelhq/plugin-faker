//! faker.fill with the test client of the SDK.

mod common;

use runnel_plugin::testing::Client;
use runnel_plugin::{Batch, Field, InitInput, RecordSchema, TO_MAIN, Value, types};
use serde_json::{Map, Value as Json, json};

fn people(names: &[(&str, &str)]) -> Batch {
    Batch {
        fields: vec![
            Field::new("id", types::INT),
            Field::new("firstName", types::STRING),
            Field::new("lastName", types::STRING),
            Field::new("email", types::STRING),
        ],
        rows: names
            .iter()
            .enumerate()
            .map(|(i, (f, l))| {
                let first = if f.is_empty() { Value::Null } else { Value::from(*f) };
                vec![
                    Value::Int(i as i64),
                    first,
                    Value::from(*l),
                    Value::from(format!("{f}.{l}@real.example").as_str()),
                ]
            })
            .collect(),
    }
}

fn fill(c: &Client, config: Json, batches: &[Batch]) -> Vec<Map<String, Json>> {
    let res = c.run_transform("faker.fill", config, batches).unwrap();
    assert_eq!(res.error, None);
    res.records(TO_MAIN)
}

#[test]
fn replaces_and_adds_fields() {
    let c = Client::start(&faker::plugin());
    let config = json!({"seed": 1, "fields": [
        {"name": "lastName", "kind": "lastName"},
        {"name": "email", "kind": "email", "from": "firstName, lastName"},
        {"name": "score", "kind": "integer", "min": 1, "max": 9},
    ]});
    let res = c
        .run_transform(
            "faker.fill",
            config,
            &[people(&[("Anna", "Muster"), ("Ben", "Beispiel")])],
        )
        .unwrap();
    assert_eq!(res.error, None);
    assert_eq!(
        res.emits[0]
            .fields
            .iter()
            .map(|f| (f.name.as_str(), f.type_.as_str()))
            .collect::<Vec<_>>(),
        [
            ("id", "int"),
            ("firstName", "string"),
            ("lastName", "string"),
            ("email", "string"),
            ("score", "int")
        ]
    );
    let r = res.records(TO_MAIN);
    assert_eq!(r[0]["id"], 0);
    assert_eq!(r[0]["firstName"], "Anna", "fields that are not named stay");
    assert_ne!(r[0]["lastName"], "Muster");
    let last = r[0]["lastName"].as_str().unwrap().to_lowercase();
    let email = r[0]["email"].as_str().unwrap();
    assert!(
        email.starts_with("anna.") && email.contains('@') && !email.contains("real.example"),
        "{email}"
    );
    assert!(!last.is_empty());
    assert!((1..=9).contains(&r[1]["score"].as_i64().unwrap()));
}

/// consistent: the same value gives the same replacement – in other records, other batches
/// and other streams (tables) – so that keys still match; null stays null.
#[test]
fn consistent_pseudonyms() {
    let c = Client::start(&faker::plugin());
    let config = |seed: u64| {
        json!({"seed": seed, "consistent": true, "fields": [
            {"name": "firstName", "kind": "firstName"},
            {"name": "lastName", "kind": "lastName"},
            {"name": "id", "kind": "pattern", "pattern": "C-######"},
        ]})
    };
    let customers = fill(
        &c,
        config(7),
        &[people(&[("Anna", "Muster"), ("Ben", "Muster"), ("", "Muster")])],
    );
    let orders = fill(
        &c,
        config(7),
        &[
            people(&[("Carl", "Other")]),
            people(&[("Ben", "Muster"), ("Anna", "X")]),
        ],
    );
    assert_eq!(
        customers[0]["lastName"], customers[1]["lastName"],
        "same last name, same pseudonym"
    );
    assert_eq!(customers[0]["lastName"], orders[1]["lastName"]);
    assert_eq!(customers[1]["firstName"], orders[1]["firstName"]);
    assert_eq!(customers[0]["firstName"], orders[2]["firstName"]);
    assert_ne!(customers[0]["firstName"], customers[1]["firstName"]);
    assert_eq!(customers[0]["id"], orders[0]["id"], "id 0 in both tables");
    assert_eq!(customers[1]["id"], orders[2]["id"], "id 1 in both tables");
    assert!(customers[0]["id"].as_str().unwrap().starts_with("C-"));
    assert_eq!(customers[2]["firstName"], Json::Null);
    // the field is part of the key: the same text in another field gives another value
    // (the person from "Muster" as first name is another than from "Muster" as last name)
    let same = fill(&c, config(7), &[people(&[("Muster", "Muster")])]);
    let names = |r: &Map<String, Json>| (r["firstName"].clone(), r["lastName"].clone());
    assert_eq!(same[0]["lastName"], customers[0]["lastName"]);
    assert_ne!(names(&same[0]).0, names(&same[0]).1);
    // another seed, other pseudonyms
    let other = fill(&c, config(8), &[people(&[("Anna", "Muster")])]);
    assert_ne!(other[0]["lastName"], customers[0]["lastName"]);
    // without consistent, the same value gets different replacements
    let mut cfg = config(7);
    cfg["consistent"] = json!(false);
    let names: std::collections::HashSet<String> = fill(&c, cfg, &[people(&[("Anna", "Muster"); 20])])
        .iter()
        .map(|r| r["lastName"].to_string())
        .collect();
    assert!(names.len() > 5, "{names:?}");
}

#[test]
fn every_kind_in_fill() {
    let c = Client::start(&faker::plugin());
    for locale in ["de", "en"] {
        for consistent in [false, true] {
            let config = json!({"locale": locale, "seed": 3, "consistent": consistent, "fields": common::all_kinds()});
            let r = fill(&c, config, &[people(&[("Anna", "Muster"), ("Ben", "Beispiel")])]);
            assert_eq!(r.len(), 2);
            for k in common::all_kinds() {
                let name = k["name"].as_str().unwrap();
                assert!(!r[0][name].is_null(), "{locale} {consistent} {name}");
            }
        }
    }
}

#[test]
fn init_and_rejections() {
    let c = Client::start(&faker::plugin());
    let input = InitInput {
        input: RecordSchema {
            fields: vec![Field::new("id", types::STRING), Field::new("name", types::STRING)],
            open: false,
        },
        targets: vec![],
    };
    let config = json!({"consistent": true, "fields": [{"name": "id", "kind": "integer"}, {"name": "born", "kind": "birthDate"}]});
    let res = c.init("faker.fill", config, &input).unwrap();
    let out = res.output.unwrap();
    assert_eq!(
        out.fields,
        [
            Field::new("id", types::INT),
            Field::new("name", types::STRING),
            Field::new("born", types::DATE)
        ]
    );
    assert_eq!(res.problems.len(), 1);
    assert!(
        res.problems[0].warning && res.problems[0].path == "seed",
        "{:?}",
        res.problems
    );
    let open = InitInput {
        input: RecordSchema {
            fields: vec![],
            open: true,
        },
        targets: vec![],
    };
    let res = c
        .init(
            "faker.fill",
            json!({"seed": 1, "fields": [{"name": "x", "kind": "uuid"}]}),
            &open,
        )
        .unwrap();
    assert!(res.output.unwrap().open);
    let res = c
        .init("faker.fill", json!({"seed": 1, "fields": [{"name": "x", "kind": "nonsense"}, {"name": "y", "kind": "email", "from": "nope"}]}), &input)
        .unwrap();
    let paths: Vec<&str> = res.problems.iter().map(|p| p.path.as_str()).collect();
    assert_eq!(paths, ["fields[0].kind", "fields[1].from"]);
    // in the run, a mistake ends the stream
    let res = c
        .run_transform(
            "faker.fill",
            json!({"fields": [{"name": "x", "kind": "integer", "min": 3, "max": 1}]}),
            &[people(&[("a", "b")])],
        )
        .unwrap();
    assert_eq!(
        res.error.as_deref(),
        Some("fields[0].min: min (3) is greater than max (1)")
    );
}
