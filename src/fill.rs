//! faker.fill: sets or replaces fields of incoming records, e.g. to pseudonymize real data.

use runnel_plugin::{
    Config, Context, Error, Field, InitInput, InitResult, Problem, RecordSchema, Row, Stream, TO_MAIN, Transform, Value,
};
use serde::Deserialize;

use crate::kinds::{self, Compiled, FieldSpec, Locale, RecordRng, Shared};
use crate::rows::{first_error, random_seed};

#[derive(Debug, Clone, Default, Deserialize, Config)]
#[serde(default)]
pub struct FillConfig {
    /// Language of names, addresses and phone numbers
    #[runnel(values = "de|en", default = "de")]
    pub locale: String,
    /// Start of the random numbers; with consistent it is the key of the pseudonyms: keep it secret, the same seed gives the same replacements
    pub seed: Option<u64>,
    /// The same input value gives the same replacement (from value, field and seed), so that keys still match across tables
    pub consistent: bool,
    /// The fields to set (new) or replace (existing)
    #[runnel(required)]
    pub fields: Vec<FieldSpec>,
}

pub fn transform() -> Transform<FillConfig> {
    Transform::new("faker.fill", "Fake fields", "Sets or replaces fields of every record with test data", run)
        .example(
            "type: faker.fill\nwith:\n  consistent: true\n  seed: ${PSEUDONYM_SEED}\n  fields:\n    - { name: lastName, kind: lastName }\n    - { name: email, kind: email }\n",
        )
        .parallel()
        .init(init)
}

struct Plan {
    locale: Locale,
    compiled: Vec<Compiled>,
    /// Position of each compiled field in the output.
    at: Vec<usize>,
    /// Whether the field was in the input (consistent replaces by its value).
    existed: Vec<bool>,
    fields: Vec<Field>,
}

fn plan(c: &FillConfig, input: &[Field]) -> (Plan, Vec<Problem>, bool) {
    let mut problems = Vec::new();
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
    let (compiled, p, open) = kinds::compile(&c.fields, input, "fields");
    problems.extend(p);
    let mut fields = input.to_vec();
    let (mut at, mut existed) = (Vec::new(), Vec::new());
    for f in &compiled {
        match fields.iter().position(|x| x.name == f.name) {
            Some(i) => {
                fields[i].type_ = f.type_.to_string();
                at.push(i);
                existed.push(i < input.len());
            }
            None => {
                fields.push(Field::new(&f.name, f.type_));
                at.push(fields.len() - 1);
                existed.push(false);
            }
        }
    }
    (
        Plan {
            locale,
            compiled,
            at,
            existed,
            fields,
        },
        problems,
        open,
    )
}

fn init(_ctx: &Context, input: &InitInput, c: FillConfig) -> InitResult {
    let (plan, mut problems, open) = plan(&c, &input.input.fields);
    if c.consistent && c.seed.is_none() {
        problems.push(Problem::warning(
            "seed",
            "without seed, consistent uses seed 0: the replacements are the same everywhere and can be reversed by trying values",
        ));
    }
    InitResult {
        output: Some(RecordSchema {
            fields: plan.fields,
            open: open || input.input.open,
        }),
        problems,
    }
}

fn run(s: &mut Stream, c: FillConfig) -> Result<(), Error> {
    let seed = match (c.seed, c.consistent) {
        (Some(seed), _) => seed,
        (None, true) => 0,
        (None, false) => {
            let seed = random_seed();
            s.log.info(format_args!(
                "seed {seed} (set seed: {seed} to generate the same values again)"
            ));
            seed
        }
    };
    let mut current: Option<(Vec<Field>, Plan)> = None;
    let mut record = 0u64;
    while let Some(b) = s.next()? {
        if current.as_ref().is_none_or(|(f, _)| *f != b.fields) {
            let (p, problems, open) = plan(&c, &b.fields);
            first_error(&problems)?;
            if open {
                return Err("a kind is still a variable; resolve it before the run".into());
            }
            current = Some((b.fields.clone(), p));
        }
        let (_, p) = current.as_ref().expect("plan");
        let width = p.fields.len();
        let rows: Vec<Row> = b
            .rows
            .into_iter()
            .map(|mut row| {
                row.resize(width, Value::Null);
                let rng = RecordRng { seed, record };
                record += 1;
                let mut shared = Shared::default();
                for (i, f) in p.compiled.iter().enumerate() {
                    let at = p.at[i];
                    row[at] = if c.consistent && p.existed[i] {
                        consistent(f, p.locale, seed, &row[at], &row)
                    } else {
                        f.generate(p.locale, &rng, &mut shared, &row)
                    };
                }
                row
            })
            .collect();
        s.emit(TO_MAIN, &p.fields, &rows)?;
    }
    Ok(())
}

/// The replacement of a value that depends only on the value, the field and the seed;
/// null stays null.
fn consistent(f: &Compiled, locale: Locale, seed: u64, original: &Value, row: &[Value]) -> Value {
    if original.is_null() {
        return Value::Null;
    }
    let key = match original.as_str() {
        Some(t) => t.to_string(),
        None => original.to_json().to_string(),
    };
    // The field is part of the key: the same text in two fields gets two replacements.
    let rng = RecordRng {
        seed,
        record: crate::rng::fnv(format!("{}\0{key}", f.name).as_bytes()),
    };
    f.generate(locale, &rng, &mut Shared::default(), row)
}
