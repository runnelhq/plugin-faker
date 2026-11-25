//! Shared by the tests.

#![allow(dead_code)]

use serde_json::{Value as Json, json};

/// The kinds of this version, with options where they need some.
pub fn all_kinds() -> Vec<Json> {
    vec![
        json!({"name": "firstName", "kind": "firstName"}),
        json!({"name": "lastName", "kind": "lastName"}),
        json!({"name": "fullName", "kind": "fullName"}),
        json!({"name": "salutation", "kind": "salutation"}),
        json!({"name": "birthDate", "kind": "birthDate", "minAge": 20, "maxAge": 30, "to": "2026-11-25"}),
        json!({"name": "email", "kind": "email"}),
        json!({"name": "emailFrom", "kind": "email", "from": "firstName, lastName"}),
        json!({"name": "phone", "kind": "phone"}),
        json!({"name": "street", "kind": "street"}),
        json!({"name": "houseNumber", "kind": "houseNumber"}),
        json!({"name": "postcode", "kind": "postcode"}),
        json!({"name": "city", "kind": "city"}),
        json!({"name": "country", "kind": "country"}),
        json!({"name": "integer", "kind": "integer", "min": -5, "max": 5}),
        json!({"name": "decimal", "kind": "decimal", "min": 1, "max": 2, "scale": 3}),
        json!({"name": "float", "kind": "float", "min": 10, "max": 20, "scale": 1}),
        json!({"name": "boolean", "kind": "boolean", "rate": 0.25}),
        json!({"name": "date", "kind": "date", "from": "2024-02-01", "to": "2024-02-29"}),
        json!({"name": "datetime", "kind": "datetime", "from": "2024-01-01T00:00:00+01:00", "to": "2024-01-01T23:59:59Z"}),
        json!({"name": "uuid", "kind": "uuid"}),
        json!({"name": "pick", "kind": "pick", "values": ["A", "B", "C"], "weights": [0, 1, 3]}),
        json!({"name": "pattern", "kind": "pattern", "pattern": "KD-#####-??"}),
    ]
}
