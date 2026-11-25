//! The Runnel plugin faker: test data (`faker.rows`) and pseudonyms (`faker.fill`), with data from
//! fake-rs and own lists for German.

mod data;
pub mod dates;
pub mod fill;
pub mod kinds;
pub mod rng;
pub mod rows;

use runnel_plugin::Plugin;

/// Must match version in plugin.yaml (a test checks it).
pub const VERSION: &str = "0.1.0";

/// The plugin with its transforms; main serves it, the tests call it directly.
pub fn plugin() -> Plugin {
    let mut p = Plugin::new("faker", VERSION);
    p.transform(rows::transform());
    p.transform(fill::transform());
    p
}
