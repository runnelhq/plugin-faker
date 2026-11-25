//! faker.rows with the test client of the SDK.

use std::collections::HashSet;

mod common;

use common::all_kinds;
use faker::kinds::field_type;
use runnel_plugin::testing::Client;
use runnel_plugin::{InitInput, TO_MAIN, types};
use serde_json::{Map, Value as Json, json};

fn rows(c: &Client, locale: &str, count: i64, seed: u64, fields: Vec<Json>) -> Vec<Map<String, Json>> {
    let res = c
        .run_transform(
            "faker.rows",
            json!({"count": count, "locale": locale, "seed": seed, "fields": fields}),
            &[],
        )
        .unwrap();
    assert_eq!(res.error, None);
    res.records(TO_MAIN)
}

fn text<'a>(r: &'a Map<String, Json>, k: &str) -> &'a str {
    r[k].as_str().unwrap_or_else(|| panic!("{k} is not a text: {r:?}"))
}

#[test]
fn every_kind_in_de_and_en() {
    let c = Client::start(&faker::plugin());
    for locale in ["de", "en"] {
        let records = rows(&c, locale, 300, 1, all_kinds());
        assert_eq!(records.len(), 300);
        let mut picks = [0; 3];
        let mut trues = 0;
        let mut uuids = HashSet::new();
        for r in &records {
            for k in [
                "firstName",
                "lastName",
                "street",
                "houseNumber",
                "city",
                "country",
                "phone",
            ] {
                assert!(!text(r, k).trim().is_empty(), "{locale} {k}: {r:?}");
            }
            assert_eq!(
                text(r, "fullName"),
                format!("{} {}", text(r, "firstName"), text(r, "lastName"))
            );
            let sal = text(r, "salutation");
            match locale {
                "de" => assert!(["Frau", "Herr"].contains(&sal), "{sal}"),
                _ => assert!(["Ms.", "Mr."].contains(&sal), "{sal}"),
            }
            let birth = text(r, "birthDate");
            assert!(("1995-11-26"..="2006-11-25").contains(&birth), "{birth}");
            for k in ["email", "emailFrom"] {
                let e = text(r, k);
                assert!(
                    e.contains('@') && e.split('@').nth(1).unwrap().starts_with("example."),
                    "{e}"
                );
                assert!(e.is_ascii(), "{e}");
            }
            let first = text(r, "firstName")
                .to_lowercase()
                .replace('ü', "ue")
                .replace('ö', "oe")
                .replace('ä', "ae");
            assert!(text(r, "emailFrom").starts_with(&first), "{r:?}");
            let pc = text(r, "postcode");
            assert!(pc.len() == 5 && pc.bytes().all(|b| b.is_ascii_digit()), "{pc}");
            let n = r["integer"].as_i64().unwrap();
            assert!((-5..=5).contains(&n));
            let d = text(r, "decimal");
            assert!(d.len() == 5 && ("1.000"..="2.000").contains(&d), "{d}");
            let f = r["float"].as_f64().unwrap();
            assert!((10.0..=20.0).contains(&f) && (f * 10.0).fract() == 0.0, "{f}");
            trues += usize::from(r["boolean"].as_bool().unwrap());
            assert!(("2024-02-01"..="2024-02-29").contains(&text(r, "date")));
            let dt = text(r, "datetime");
            assert!(("2023-12-31T23:00:00Z"..="2024-01-01T23:59:59Z").contains(&dt), "{dt}");
            let u = text(r, "uuid");
            assert!(u.len() == 36 && &u[14..15] == "4" && "89ab".contains(&u[19..20]), "{u}");
            uuids.insert(u.to_string());
            picks[["A", "B", "C"].iter().position(|p| *p == text(r, "pick")).unwrap()] += 1;
            let p = text(r, "pattern");
            assert!(
                p.starts_with("KD-")
                    && p[3..8].bytes().all(|b| b.is_ascii_digit())
                    && p[9..].bytes().all(|b| b.is_ascii_uppercase()),
                "{p}"
            );
        }
        assert!((40..110).contains(&trues), "rate 0.25: {trues} of 300");
        assert_eq!(picks[0], 0, "weight 0 is never picked");
        assert!(picks[2] > picks[1] * 2, "weights 1:3: {picks:?}");
        assert_eq!(uuids.len(), 300);
    }
}

#[test]
fn german_data() {
    let c = Client::start(&faker::plugin());
    let fields = vec![
        json!({"name": "salutation", "kind": "salutation"}),
        json!({"name": "firstName", "kind": "firstName"}),
        json!({"name": "street", "kind": "street"}),
        json!({"name": "postcode", "kind": "postcode"}),
        json!({"name": "city", "kind": "city"}),
        json!({"name": "phone", "kind": "phone"}),
    ];
    for r in rows(&c, "de", 500, 9, fields) {
        let (pc, city) = (text(&r, "postcode"), text(&r, "city"));
        let ok = match city {
            "Berlin" => ("10115"..="14199").contains(&pc),
            "München" => ("80331"..="81929").contains(&pc),
            "Leipzig" => ("04103"..="04357").contains(&pc),
            _ => true,
        };
        assert!(ok, "{pc} {city}");
        assert!(!text(&r, "street").contains(' '), "{r:?}");
        let phone = text(&r, "phone");
        assert!(
            !phone.starts_with("01")
                || phone.starts_with("015")
                || phone.starts_with("016")
                || phone.starts_with("017"),
            "{phone}"
        );
        assert!(!phone.contains('x') && !phone.contains('-'), "no US numbers: {phone}");
    }
}

#[test]
fn salutation_fits_the_first_name() {
    let c = Client::start(&faker::plugin());
    let fields = vec![
        json!({"name": "s", "kind": "salutation"}),
        json!({"name": "f", "kind": "firstName"}),
    ];
    let records = rows(&c, "de", 200, 3, fields);
    let female: HashSet<&str> = records
        .iter()
        .filter(|r| text(r, "s") == "Frau")
        .map(|r| text(r, "f"))
        .collect();
    let male: HashSet<&str> = records
        .iter()
        .filter(|r| text(r, "s") == "Herr")
        .map(|r| text(r, "f"))
        .collect();
    assert!(!female.is_empty() && !male.is_empty());
    assert!(female.is_disjoint(&male), "{female:?} {male:?}");
    assert!(female.contains("Anna") || female.contains("Maria") || female.len() > 10);
}

/// The same seed and configuration give the same records on every platform; the values of a
/// field do not change when other fields are added. testdata/rows-*.json are the expected
/// records (UPDATE=1 writes them anew; check the diff).
#[test]
fn reproducible_with_fixed_values() {
    let c = Client::start(&faker::plugin());
    for locale in ["de", "en"] {
        let got = rows(&c, locale, 5, 42, all_kinds());
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata")
            .join(format!("rows-{locale}.json"));
        let text = serde_json::to_string_pretty(&got).unwrap() + "\n";
        if std::env::var("UPDATE").is_ok_and(|v| v == "1") {
            std::fs::write(&path, &text).unwrap();
        }
        let want: Json = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            serde_json::to_value(&got).unwrap(),
            want,
            "{locale}: records differ from {}",
            path.display()
        );
        assert_eq!(rows(&c, locale, 5, 42, all_kinds()), got, "same seed, same records");
        assert_ne!(rows(&c, locale, 5, 43, all_kinds()), got, "other seed, other records");
        let few = rows(
            &c,
            locale,
            5,
            42,
            vec![all_kinds()[13].clone(), all_kinds()[19].clone()],
        );
        for (a, b) in few.iter().zip(&got) {
            assert_eq!(a["integer"], b["integer"]);
            assert_eq!(a["uuid"], b["uuid"]);
        }
    }
}

#[test]
fn counts_and_batches() {
    let c = Client::start(&faker::plugin());
    let f = || vec![json!({"name": "n", "kind": "integer"})];
    assert!(rows(&c, "de", 0, 1, f()).is_empty());
    let res = c
        .run_transform("faker.rows", json!({"count": "1234", "seed": "1", "fields": f()}), &[])
        .unwrap();
    assert_eq!(res.error, None);
    assert_eq!(res.records(TO_MAIN).len(), 1234);
    assert_eq!(res.emits.len(), 3);
    // without seed, the log names the seed used
    let res = c
        .run_transform("faker.rows", json!({"count": 1, "fields": f()}), &[])
        .unwrap();
    assert_eq!(res.error, None);
    assert!(c.log_contains("INFO seed "), "{:?}", c.log());
}

#[test]
fn rejections_in_the_run() {
    let c = Client::start(&faker::plugin());
    for (config, want) in [
        (
            json!({"count": 10, "fields": [{"name": "x", "kind": "nonsense"}]}),
            r#"fields[0].kind: unknown kind "nonsense""#,
        ),
        (
            json!({"count": 10, "fields": [{"name": "x", "kind": "integer", "min": 5, "max": 1}]}),
            "fields[0].min: min (5) is greater than max (1)",
        ),
        (
            json!({"count": 10_000_001, "fields": [{"name": "x", "kind": "uuid"}]}),
            "count: count must be between 0 and 10000000, not 10000001",
        ),
        (
            json!({"count": -1, "fields": [{"name": "x", "kind": "uuid"}]}),
            "count must be between 0 and 10000000",
        ),
        (
            json!({"count": 1, "locale": "fr", "fields": [{"name": "x", "kind": "uuid"}]}),
            r#"locale: unknown locale "fr""#,
        ),
        (
            json!({"count": 1, "fields": []}),
            "fields: at least one field is needed",
        ),
        (
            json!({"count": "many", "fields": []}),
            r#"field "count": expected a whole number, got string"#,
        ),
    ] {
        let res = c.run_transform("faker.rows", config.clone(), &[]).unwrap();
        let err = res.error.unwrap_or_default();
        assert!(err.contains(want), "{config}: {err:?}, want {want:?}");
        assert!(res.emits.is_empty());
    }
}

#[test]
fn init_tells_the_types_and_problems() {
    let c = Client::start(&faker::plugin());
    let res = c
        .init(
            "faker.rows",
            json!({"count": 10, "fields": all_kinds()}),
            &InitInput::default(),
        )
        .unwrap();
    assert!(res.problems.is_empty(), "{:?}", res.problems);
    let out = res.output.unwrap();
    assert!(!out.open);
    assert_eq!(out.fields.len(), all_kinds().len());
    for f in &out.fields {
        let kind = all_kinds().into_iter().find(|k| k["name"] == f.name.as_str()).unwrap()["kind"]
            .as_str()
            .unwrap()
            .to_string();
        assert_eq!(f.type_, field_type(&kind).unwrap(), "{}", f.name);
    }
    let types_of = |n: &str| out.fields.iter().find(|f| f.name == n).unwrap().type_.clone();
    assert_eq!(types_of("birthDate"), types::DATE);
    assert_eq!(types_of("decimal"), types::DECIMAL);
    assert_eq!(types_of("datetime"), types::DATETIME);
    assert_eq!(types_of("integer"), types::INT);

    let bad = json!({"count": 20_000_000, "locale": "fr", "fields": [
        {"name": "a", "kind": "nonsense"},
        {"name": "b", "kind": "integer", "min": 5, "max": 1},
        {"name": "c", "kind": "email", "from": "z"},
        {"name": "d", "kind": "date", "from": "1.1.2020"},
        {"name": "e", "kind": "pick", "values": ["x"], "weights": [1, 2]},
        {"name": "e", "kind": "uuid"},
        {"name": "f", "kind": "pattern"},
        {"name": "g", "kind": "birthDate", "minAge": 50, "maxAge": 20},
        {"name": "h", "kind": "decimal", "scale": 12},
        {"name": "", "kind": "uuid"},
    ]});
    let res = c.init("faker.rows", bad, &InitInput::default()).unwrap();
    let got: Vec<(String, String)> = res
        .problems
        .iter()
        .map(|p| (p.path.clone(), p.message.clone()))
        .collect();
    let want = [
        ("count", "count must be between 0 and 10000000, not 20000000"),
        ("locale", r#"unknown locale "fr"; use de or en"#),
        (
            "fields[0].kind",
            r#"unknown kind "nonsense"; see the list of kinds in the help"#,
        ),
        ("fields[1].min", "min (5) is greater than max (1)"),
        ("fields[2].from", r#"field "z" does not come before this field"#),
        ("fields[3].from", r#""1.1.2020" is not a date (e.g. 2026-11-24)"#),
        ("fields[4].weights", "2 weights for 1 values"),
        ("fields[5].name", r#"field "e" is defined twice"#),
        ("fields[6].pattern", "pattern needs a pattern, e.g. KD-#####"),
        ("fields[7].minAge", "minAge (50) is greater than maxAge (20)"),
        ("fields[8].scale", "scale must be at most 9"),
        ("fields[9].name", "the field needs a name"),
    ];
    assert_eq!(got, want.map(|(a, b)| (a.to_string(), b.to_string())));
    // Unresolved variables are not problems of the check; a kind from a variable opens the output.
    let vars = json!({"count": "${N}", "seed": "${SEED}", "locale": "${LOCALE}", "fields": [
        {"name": "x", "kind": "uuid"},
        {"name": "d", "kind": "date", "from": "${START}", "to": "{{ today() }}"},
        {"name": "e", "kind": "email", "from": "${NAMES}"},
        {"name": "p", "kind": "pattern", "pattern": "${PATTERN}"},
    ]});
    let res = c.init("faker.rows", vars, &InitInput::default()).unwrap();
    assert!(res.problems.is_empty(), "{:?}", res.problems);
    assert!(!res.output.unwrap().open);
    let one = json!({"count": 1, "fields": [{"name": "x", "kind": "${KIND}"}, {"name": "y", "kind": "uuid"}]});
    let res = c.init("faker.rows", one, &InitInput::default()).unwrap();
    assert!(res.problems.is_empty(), "{:?}", res.problems);
    let out = res.output.unwrap();
    assert!(out.open);
    assert_eq!(out.fields.len(), 1);
}

#[test]
fn describe() {
    let c = Client::start(&faker::plugin());
    let d = c.describe().unwrap();
    let t = &d["transforms"][0];
    assert_eq!(t["type"], "faker.rows");
    assert_eq!(t["category"], "input");
    assert_eq!(t["init"], true);
    assert_eq!(t["schema"]["required"], json!(["count", "fields"]));
    assert_eq!(
        t["schema"]["properties"]["fields"]["items"]["required"],
        json!(["name", "kind"])
    );
}

#[test]
fn version_matches_plugin_yaml() {
    let yaml = include_str!("../plugin.yaml");
    assert!(
        yaml.lines().any(|l| l == format!("version: {}", faker::VERSION)),
        "plugin.yaml has another version than {}",
        faker::VERSION
    );
}
