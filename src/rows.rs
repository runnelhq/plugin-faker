//! faker.rows: an input that generates records.

use runnel_plugin::{
    Config, Context, Error, Field, InitInput, InitResult, Problem, RecordSchema, Stream, TO_MAIN, Transform,
    transform_category,
};
use serde::Deserialize;

use crate::kinds::{self, FieldSpec, Locale, RecordRng, Shared};

/// The largest count of records.
pub const MAX_COUNT: i64 = 10_000_000;

/// Records per transform.emit.
const BATCH: usize = 500;

#[derive(Debug, Clone, Default, Deserialize, Config)]
#[serde(default)]
pub struct RowsConfig {
    /// How many records to generate, at most 10,000,000
    #[runnel(required, example = "1000")]
    pub count: i64,
    /// Language of names, addresses and phone numbers
    #[runnel(values = "de|en", default = "de")]
    pub locale: String,
    /// Start of the random numbers: the same seed and configuration give the same data; without, the log names the seed used
    pub seed: Option<u64>,
    /// The fields of the records
    #[runnel(required)]
    pub fields: Vec<FieldSpec>,
}

pub fn transform() -> Transform<RowsConfig> {
    Transform::new("faker.rows", "Fake rows", "Generates records with test data", run)
        .category(transform_category::INPUT)
        .example(
            "type: faker.rows\nwith:\n  count: 1000\n  locale: de\n  seed: 42\n  fields:\n    - { name: id, kind: integer, min: 1, max: 99999 }\n    - { name: firstName, kind: firstName }\n    - { name: lastName, kind: lastName }\n    - { name: email, kind: email, from: \"firstName, lastName\" }\n    - { name: city, kind: city }\n",
        )
        .init(init)
}

/// The checks shared by init and run: problems with paths in with:.
pub fn check(c: &RowsConfig) -> (Vec<kinds::Compiled>, Vec<Problem>, Locale, bool) {
    let mut problems = Vec::new();
    if !(0..=MAX_COUNT).contains(&c.count) {
        problems.push(Problem::error(
            "count",
            format!("count must be between 0 and {MAX_COUNT}, not {}", c.count),
        ));
    }
    let locale = Locale::parse(&c.locale)
        .or_else(|| kinds::is_variable(&c.locale).then_some(Locale::De))
        .unwrap_or_else(|| {
            problems.push(Problem::error(
                "locale",
                format!("unknown locale {:?}; use de or en", c.locale),
            ));
            Locale::De
        });
    if c.fields.is_empty() {
        problems.push(Problem::error("fields", "at least one field is needed"));
    }
    let (compiled, p, open) = kinds::compile(&c.fields, &[], "fields");
    problems.extend(p);
    (compiled, problems, locale, open)
}

fn init(_ctx: &Context, _input: &InitInput, c: RowsConfig) -> InitResult {
    let (compiled, problems, _, open) = check(&c);
    let fields = compiled.iter().map(|f| Field::new(&f.name, f.type_)).collect();
    InitResult {
        output: Some(RecordSchema { fields, open }),
        problems,
    }
}

/// A seed for runs without one; the log names it so the data can be made again.
pub fn random_seed() -> u64 {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    crate::rng::fnv(format!("{t}/{}", std::process::id()).as_bytes()) >> 11
}

/// The first problem as error of the run.
pub fn first_error(problems: &[Problem]) -> Result<(), Error> {
    match problems.iter().find(|p| !p.warning) {
        Some(p) => Err(format!("{}: {}", p.path, p.message).into()),
        None => Ok(()),
    }
}

fn run(s: &mut Stream, c: RowsConfig) -> Result<(), Error> {
    let (compiled, problems, locale, open) = check(&c);
    first_error(&problems)?;
    if open {
        return Err("a kind is still a variable; resolve it before the run".into());
    }
    let seed = c.seed.unwrap_or_else(random_seed);
    if c.seed.is_none() {
        s.log.info(format_args!(
            "seed {seed} (set seed: {seed} to generate the same records again)"
        ));
    }
    let fields: Vec<Field> = compiled.iter().map(|f| Field::new(&f.name, f.type_)).collect();
    let count = c.count as u64;
    let mut record = 0u64;
    while record < count {
        if s.is_cancelled() {
            return Err("cancelled".into());
        }
        let n = (count - record).min(BATCH as u64);
        let rows: Vec<_> = (record..record + n)
            .map(|i| {
                let rng = RecordRng { seed, record: i };
                let mut shared = Shared::default();
                let mut row = Vec::with_capacity(compiled.len());
                for f in &compiled {
                    let v = f.generate(locale, &rng, &mut shared, &row);
                    row.push(v);
                }
                row
            })
            .collect();
        s.emit(TO_MAIN, &fields, &rows)?;
        record += n;
    }
    Ok(())
}
