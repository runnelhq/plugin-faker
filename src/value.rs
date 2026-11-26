//! faker.value: one value, e.g. for tests in workflows.

use runnel_plugin::{Action, Call, Config, Error, OutputField, Reference};
use serde::Deserialize;
use serde_json::Value as Json;

use crate::kinds::{self, FieldSpec, KindOptions, Locale, RecordRng, Shared};
use crate::rows::{first_error, random_seed};

#[derive(Debug, Clone, Default, Deserialize, Config)]
#[serde(default)]
pub struct ValueConfig {
    #[serde(flatten)]
    pub options: KindOptions,
    /// Language of names, addresses and phone numbers
    #[runnel(values = "de|en", default = "de")]
    pub locale: String,
    /// Start of the random numbers; the same seed gives the same value
    pub seed: Option<u64>,
}

pub fn action() -> Action<ValueConfig> {
    Action::new("faker.value", "Fake value", "Returns one value of test data", run)
        .example("type: faker.value\nwith:\n  kind: iban\n  country: DE\n")
        .reference(
            Reference::new()
                .output(OutputField::new("output", "any", "The value: a text, a number or true or false by its kind, null by nullRate"))
                .errors("Fails when kind is unknown, an option does not fit the kind, e.g. min greater than max, or locale is not de or en.")
        .limit("scale has at most 9 digits, lorem at most 1,000 words or 100 sentences, and maxAge of birthDate is at most 150."),
        )
}

fn run(call: &Call, c: ValueConfig) -> Result<Json, Error> {
    let Some(locale) = Locale::parse(&c.locale) else {
        return Err(format!("unknown locale {:?}; use de or en", c.locale).into());
    };
    let spec = FieldSpec {
        name: "value".into(),
        options: c.options,
    };
    let (compiled, problems, open) = kinds::compile(std::slice::from_ref(&spec), &[], "fields");
    // The paths of the problems are those of a list; the action has the options at the top.
    let problems: Vec<_> = problems
        .into_iter()
        .map(|mut p| {
            p.path = p.path.trim_start_matches("fields[0].").to_string();
            p
        })
        .collect();
    first_error(&problems)?;
    if open || compiled.is_empty() {
        return Err(format!("unknown kind {:?}", spec.options.kind).into());
    }
    let seed = c.seed.unwrap_or_else(random_seed);
    if c.seed.is_none() {
        call.log.debug(format_args!("seed {seed}"));
    }
    let rng = RecordRng { seed, record: 0 };
    Ok(compiled[0]
        .generate(locale, &rng, &mut Shared::default(), &[])
        .to_json())
}
