//! Throughput of faker.rows with five fields, with the test client of the SDK (without
//! Runnel): cargo test --release --test bench -- --ignored --nocapture

use std::time::Instant;

use runnel_plugin::TO_MAIN;
use runnel_plugin::testing::Client;
use serde_json::json;

#[test]
#[ignore = "benchmark: make bench"]
fn bench_rows() {
    let c = Client::start(&faker::plugin());
    let config = json!({"count": 200_000, "seed": 1, "fields": [
        {"name": "id", "kind": "integer", "min": 1, "max": 1_000_000},
        {"name": "firstName", "kind": "firstName"},
        {"name": "lastName", "kind": "lastName"},
        {"name": "email", "kind": "email"},
        {"name": "city", "kind": "city"},
    ]});
    let _ = c
        .run_transform(
            "faker.rows",
            json!({"count": 10_000, "seed": 1, "fields": config["fields"]}),
            &[],
        )
        .unwrap();
    let start = Instant::now();
    let res = c.run_transform("faker.rows", config, &[]).unwrap();
    let secs = start.elapsed().as_secs_f64();
    assert_eq!(res.error, None);
    assert_eq!(res.records(TO_MAIN).len(), 200_000);
    println!(
        "bench_rows: {:.0} records/s (200,000 records with five fields)",
        200_000.0 / secs
    );
}
