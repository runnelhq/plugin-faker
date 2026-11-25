//! faker.value, nullRate and the kinds of bank, company and tax.

mod common;

use runnel_plugin::testing::Client;
use runnel_plugin::{InitInput, TO_MAIN, code};
use serde_json::json;

#[test]
fn value_action() {
    let c = Client::start(&faker::plugin());
    let iban = c
        .execute("faker.value", json!({"kind": "iban", "country": "AT", "seed": 1}))
        .unwrap();
    let iban = iban.as_str().unwrap();
    assert!(iban.starts_with("AT") && iban.len() == 20, "{iban}");
    assert_eq!(
        c.execute("faker.value", json!({"kind": "iban", "country": "AT", "seed": 1}))
            .unwrap(),
        iban,
        "same seed"
    );
    let n = c
        .execute("faker.value", json!({"kind": "integer", "min": "5", "max": "5"}))
        .unwrap();
    assert_eq!(n, 5, "texts from variables are numbers");
    let d = c
        .execute(
            "faker.value",
            json!({"kind": "decimal", "min": 1, "max": 1, "scale": 2}),
        )
        .unwrap();
    assert_eq!(d, "1.00");
    let name = c
        .execute("faker.value", json!({"kind": "fullName", "locale": "en", "seed": 2}))
        .unwrap();
    assert!(name.as_str().unwrap().contains(' '));
    let none = c
        .execute("faker.value", json!({"kind": "uuid", "nullRate": 1}))
        .unwrap();
    assert!(none.is_null());
    for (config, want) in [
        (json!({"kind": "nonsense"}), r#"kind: unknown kind "nonsense""#),
        (
            json!({"kind": "integer", "min": 2, "max": 1}),
            "min: min (2) is greater than max (1)",
        ),
        (
            json!({"kind": "iban", "country": "FR"}),
            r#"country: iban supports the countries DE, AT, CH, LI, LU, NL, GB, BE, not "FR""#,
        ),
        (json!({"kind": "uuid", "locale": "fr"}), r#"unknown locale "fr""#),
    ] {
        let err = c.execute("faker.value", config.clone()).unwrap_err();
        assert_eq!(err.code, code::ACTION_FAILED);
        assert!(err.message.contains(want), "{config}: {err}");
    }
    let d = c.describe().unwrap();
    assert_eq!(d["actions"][0]["type"], "faker.value");
    assert_eq!(d["actions"][0]["schema"]["required"], json!(["kind"]));
}

#[test]
fn null_rate() {
    let c = Client::start(&faker::plugin());
    let config = json!({"count": 1000, "seed": 4, "fields": [
        {"name": "a", "kind": "integer", "nullRate": 0.2},
        {"name": "b", "kind": "integer"},
        {"name": "c", "kind": "lastName", "nullRate": 1},
    ]});
    let res = c.run_transform("faker.rows", config, &[]).unwrap();
    let r = res.records(TO_MAIN);
    let nulls = r.iter().filter(|x| x["a"].is_null()).count();
    assert!((140..260).contains(&nulls), "nullRate 0.2: {nulls} of 1000");
    assert!(r.iter().all(|x| !x["b"].is_null() && x["c"].is_null()));
    let res = c
        .init(
            "faker.rows",
            json!({"count": 1, "fields": [{"name": "a", "kind": "uuid", "nullRate": 2}]}),
            &InitInput::default(),
        )
        .unwrap();
    assert_eq!(res.problems[0].path, "fields[0].nullRate");
}

#[test]
fn bank_company_tax_and_lorem() {
    let c = Client::start(&faker::plugin());
    for locale in ["de", "en"] {
        let config = json!({"count": 200, "locale": locale, "seed": 6, "fields": common::all_kinds()});
        let res = c.run_transform("faker.rows", config, &[]).unwrap();
        assert_eq!(res.error, None);
        for r in res.records(TO_MAIN) {
            let text = |k: &str| r[k].as_str().unwrap().to_string();
            let (iban, country) = (text("iban"), if locale == "de" { "DE" } else { "GB" });
            assert!(iban.starts_with(country), "{iban}");
            let rotated = format!("{}{}", &iban[4..], &iban[..4]);
            let rem = rotated.chars().fold(0u32, |rem, ch| {
                let v = ch.to_digit(36).unwrap();
                if v >= 10 {
                    (rem * 100 + v) % 97
                } else {
                    (rem * 10 + v) % 97
                }
            });
            assert_eq!(rem, 1, "{iban}");
            assert!(text("ibanAt").starts_with("AT"));
            assert_eq!(&text("bic")[4..6], country);
            assert!(text("vatId").starts_with(country));
            assert_eq!(text("words").split(' ').count(), 3);
            assert_eq!(text("sentences").matches('.').count(), 2);
            let company = text("company");
            if locale == "de" {
                assert!(!company.contains(" and "), "{company}");
            }
        }
    }
    let res = c
        .init(
            "faker.rows",
            json!({"count": 1, "fields": [
                {"name": "a", "kind": "vatId", "country": "US"},
                {"name": "b", "kind": "bic", "country": "Deutschland"},
                {"name": "c", "kind": "lorem", "words": 0},
            ]}),
            &InitInput::default(),
        )
        .unwrap();
    let paths: Vec<&str> = res.problems.iter().map(|p| p.path.as_str()).collect();
    assert_eq!(paths, ["fields[0].country", "fields[1].country", "fields[2].words"]);
}
