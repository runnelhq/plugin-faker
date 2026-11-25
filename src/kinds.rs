//! The kinds of values: their options in the configuration, the check of the options
//! (`transform.init` reports mistakes at the field of the configuration) and the
//! generation of a value.

use fake::locales::{DE_DE, Data, EN};
use runnel_plugin::{Config, Field, Problem, Value, types};
use serde::Deserialize;

use crate::data::{de, en};
use crate::dates;
use crate::rng::Rng;

/// The language of the data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Locale {
    #[default]
    De,
    En,
}

impl Locale {
    pub fn parse(text: &str) -> Option<Locale> {
        match text.trim() {
            "" | "de" => Some(Locale::De),
            "en" => Some(Locale::En),
            _ => None,
        }
    }
}

/// A field to generate: its name, kind and the options of the kind.
#[derive(Debug, Clone, Default, Deserialize, Config)]
#[serde(default, rename_all = "camelCase")]
pub struct FieldSpec {
    /// Name of the field, e.g. customerId
    #[runnel(required, example = "id")]
    pub name: String,
    /// What to generate: a person, an address, a contact, a number, a date or a value of your own
    #[runnel(
        required,
        example = "integer",
        values = "firstName|lastName|fullName|salutation|birthDate|email|phone|street|houseNumber|postcode|city|country|integer|decimal|float|boolean|date|datetime|uuid|pick|pattern"
    )]
    pub kind: String,
    /// Smallest number (integer, decimal, float)
    pub min: Option<f64>,
    /// Largest number (integer, decimal, float)
    pub max: Option<f64>,
    /// Digits after the decimal point (decimal, float)
    pub scale: Option<u32>,
    /// Share of true values from 0 to 1 (boolean), default 0.5
    pub rate: Option<f64>,
    /// Earliest date or time (date, datetime), e.g. 2020-01-01; for email the fields with the name, e.g. firstName, lastName
    pub from: String,
    /// Latest date or time (date, datetime), e.g. 2025-12-31; for birthDate the day the age refers to (default today)
    pub to: String,
    /// Youngest age in years (birthDate), default 18
    pub min_age: Option<u32>,
    /// Oldest age in years (birthDate), default 80
    pub max_age: Option<u32>,
    /// Values to pick from (pick)
    pub values: Vec<String>,
    /// Weight of each value (pick), e.g. 3, 1, 1
    pub weights: Vec<f64>,
    /// Pattern (pattern): # a digit, ? a capital letter, * a digit or capital letter, \ the next character as it is, e.g. KD-#####
    pub pattern: String,
}

/// A checked field, ready to generate values.
#[derive(Debug, Clone)]
pub struct Compiled {
    pub name: String,
    pub type_: &'static str,
    generator: Gen,
}

#[derive(Debug, Clone)]
enum Gen {
    FirstName,
    LastName,
    FullName,
    Salutation,
    BirthDate {
        min_age: u32,
        max_age: u32,
        on: Option<i64>,
    },
    Email {
        from: Vec<usize>,
    },
    Phone,
    Street,
    HouseNumber,
    Postcode,
    City,
    Country,
    Integer {
        min: i64,
        max: i64,
    },
    Decimal {
        min: i128,
        max: i128,
        scale: u32,
    },
    Float {
        min: f64,
        max: f64,
        scale: Option<u32>,
    },
    Boolean {
        rate: f64,
    },
    Date {
        from: i64,
        to: i64,
    },
    Datetime {
        from: i64,
        to: i64,
    },
    Uuid,
    Pick {
        values: Vec<String>,
        cumulative: Vec<f64>,
    },
    Pattern(String),
}

/// Whether a text still holds a variable or expression (`${…}`, `{{ … }}`), as in
/// transform.init before a run; such values are not checked.
pub fn is_variable(text: &str) -> bool {
    text.contains("${") || text.contains("{{")
}

/// The type of the field (ADR 0007 of Runnel) of a kind, None for an unknown kind.
pub fn field_type(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "firstName" | "lastName" | "fullName" | "salutation" | "email" | "phone" | "street" | "houseNumber"
        | "postcode" | "city" | "country" | "uuid" | "pick" | "pattern" => types::STRING,
        "birthDate" | "date" => types::DATE,
        "datetime" => types::DATETIME,
        "integer" => types::INT,
        "decimal" => types::DECIMAL,
        "float" => types::FLOAT,
        "boolean" => types::BOOL,
        _ => return None,
    })
}

const MAX_SCALE: u32 = 9;

/// Checks the fields and compiles them. `known` are the fields of the record before the
/// first of them (for `from` of email); problems have paths below `path`, e.g.
/// `fields[2].kind`. Options with variables (`${…}`) are taken as valid; a field whose kind
/// is a variable is left out (the third value is then true: the output is open).
pub fn compile(specs: &[FieldSpec], known: &[Field], path: &str) -> (Vec<Compiled>, Vec<Problem>, bool) {
    let mut open = false;
    let mut out = Vec::new();
    let mut problems = Vec::new();
    let mut names: Vec<String> = known.iter().map(|f| f.name.clone()).collect();
    for (i, s) in specs.iter().enumerate() {
        let at = |o: &str| format!("{path}[{i}].{o}");
        let mut problem = |o: &str, msg: String| problems.push(Problem::error(&at(o), msg));
        if s.name.trim().is_empty() {
            problem("name", "the field needs a name".into());
            continue;
        }
        if specs[..i].iter().any(|o| o.name == s.name) {
            problem("name", format!("field {:?} is defined twice", s.name));
            continue;
        }
        if is_variable(&s.kind) {
            open = true;
            continue;
        }
        let Some(type_) = field_type(&s.kind) else {
            problem(
                "kind",
                format!("unknown kind {:?}; see the list of kinds in the help", s.kind),
            );
            continue;
        };
        match gen_of(s, &names, &mut problem) {
            Some(generator) => out.push(Compiled {
                name: s.name.clone(),
                type_,
                generator,
            }),
            None => continue,
        }
        if !names.contains(&s.name) {
            names.push(s.name.clone());
        }
    }
    (out, problems, open)
}

fn gen_of(s: &FieldSpec, names: &[String], problem: &mut impl FnMut(&str, String)) -> Option<Gen> {
    let mut s = s.clone();
    for text in [&mut s.from, &mut s.to] {
        if is_variable(text) {
            text.clear();
        }
    }
    if is_variable(&s.pattern) {
        s.pattern = "#".into();
    }
    if s.kind == "email" && is_variable(&s.from) {
        s.from.clear();
    }
    let s = &s;
    let range = |problem: &mut dyn FnMut(&str, String), dmin: f64, dmax: f64| -> Option<(f64, f64)> {
        let (min, max) = (s.min.unwrap_or(dmin), s.max.unwrap_or(dmax));
        if !min.is_finite() || !max.is_finite() {
            problem("min", "min and max must be numbers".into());
            return None;
        }
        if min > max {
            problem("min", format!("min ({min}) is greater than max ({max})"));
            return None;
        }
        Some((min, max))
    };
    let scale = |problem: &mut dyn FnMut(&str, String), default: u32| -> Option<u32> {
        let sc = s.scale.unwrap_or(default);
        if sc > MAX_SCALE {
            problem("scale", format!("scale must be at most {MAX_SCALE}"));
            return None;
        }
        Some(sc)
    };
    Some(match s.kind.as_str() {
        "firstName" => Gen::FirstName,
        "lastName" => Gen::LastName,
        "fullName" => Gen::FullName,
        "salutation" => Gen::Salutation,
        "birthDate" => {
            let (min_age, max_age) = (s.min_age.unwrap_or(18), s.max_age.unwrap_or(80));
            if min_age > max_age {
                problem(
                    "minAge",
                    format!("minAge ({min_age}) is greater than maxAge ({max_age})"),
                );
                return None;
            }
            if max_age > 150 {
                problem("maxAge", "maxAge must be at most 150".into());
                return None;
            }
            let on = if s.to.is_empty() {
                None
            } else {
                match dates::parse_date(&s.to) {
                    Some(d) => Some(d),
                    None => {
                        problem("to", format!("{:?} is not a date (e.g. 2026-11-24)", s.to));
                        return None;
                    }
                }
            };
            Gen::BirthDate { min_age, max_age, on }
        }
        "email" => {
            let mut from = Vec::new();
            for f in s.from.split(',').map(str::trim).filter(|f| !f.is_empty()) {
                match names.iter().position(|n| n == f) {
                    Some(i) => from.push(i),
                    None => {
                        problem("from", format!("field {f:?} does not come before this field"));
                        return None;
                    }
                }
            }
            Gen::Email { from }
        }
        "phone" => Gen::Phone,
        "street" => Gen::Street,
        "houseNumber" => Gen::HouseNumber,
        "postcode" => Gen::Postcode,
        "city" => Gen::City,
        "country" => Gen::Country,
        "integer" => {
            let (min, max) = range(problem, 1.0, 1000.0)?;
            if min.fract() != 0.0 || max.fract() != 0.0 || min < -9.0e18 || max > 9.0e18 {
                problem("min", "min and max of an integer must be whole numbers".into());
                return None;
            }
            Gen::Integer {
                min: min as i64,
                max: max as i64,
            }
        }
        "decimal" => {
            let (min, max) = range(problem, 0.0, 1000.0)?;
            let scale = scale(problem, 2)?;
            let f = 10f64.powi(scale as i32);
            if min.abs() * f > 1e30 || max.abs() * f > 1e30 {
                problem("max", "min and max are too large".into());
                return None;
            }
            Gen::Decimal {
                min: (min * f).ceil() as i128,
                max: (max * f).floor() as i128,
                scale,
            }
        }
        "float" => {
            let (min, max) = range(problem, 0.0, 1.0)?;
            let scale = match s.scale {
                Some(_) => Some(scale(problem, 0)?),
                None => None,
            };
            Gen::Float { min, max, scale }
        }
        "boolean" => {
            let rate = s.rate.unwrap_or(0.5);
            if !(0.0..=1.0).contains(&rate) {
                problem("rate", "rate must be between 0 and 1".into());
                return None;
            }
            Gen::Boolean { rate }
        }
        "date" | "datetime" => {
            let dt = s.kind == "datetime";
            let parse = |t: &str| {
                if dt {
                    dates::parse_datetime(t)
                } else {
                    dates::parse_date(t)
                }
            };
            let mut bound = |opt: &str, text: &str, default: &str| -> Option<i64> {
                let t = if text.is_empty() { default } else { text };
                let v = parse(t);
                if v.is_none() {
                    problem(
                        opt,
                        format!(
                            "{t:?} is not a {} (e.g. 2026-11-24)",
                            if dt { "date or time" } else { "date" }
                        ),
                    );
                }
                v
            };
            let from = bound("from", &s.from, "2020-01-01");
            let to = bound("to", &s.to, if dt { "2029-12-31T23:59:59Z" } else { "2029-12-31" });
            let (from, to) = (from?, to?);
            if from > to {
                problem("from", format!("from ({}) is after to ({})", s.from, s.to));
                return None;
            }
            if dt {
                Gen::Datetime { from, to }
            } else {
                Gen::Date { from, to }
            }
        }
        "uuid" => Gen::Uuid,
        "pick" => {
            if s.values.is_empty() {
                problem("values", "pick needs values".into());
                return None;
            }
            let weights = if s.weights.is_empty() {
                vec![1.0; s.values.len()]
            } else {
                s.weights.clone()
            };
            if weights.len() != s.values.len() {
                problem(
                    "weights",
                    format!("{} weights for {} values", weights.len(), s.values.len()),
                );
                return None;
            }
            if weights.iter().any(|w| !w.is_finite() || *w < 0.0) || weights.iter().sum::<f64>() <= 0.0 {
                problem(
                    "weights",
                    "weights must not be negative, and at least one must be greater than 0".into(),
                );
                return None;
            }
            let mut sum = 0.0;
            let cumulative = weights.iter().map(|w| {
                sum += w;
                sum
            });
            Gen::Pick {
                values: s.values.clone(),
                cumulative: cumulative.collect(),
            }
        }
        "pattern" => {
            if s.pattern.is_empty() {
                problem("pattern", "pattern needs a pattern, e.g. KD-#####".into());
                return None;
            }
            Gen::Pattern(s.pattern.clone())
        }
        _ => return None,
    })
}

/// What fields of a record share: one person (first name, last name, salutation, email
/// fit together) and one place (postcode fits the city).
#[derive(Debug, Default)]
pub struct Shared {
    person: Option<Person>,
    place: Option<(String, String)>,
}

#[derive(Debug)]
struct Person {
    female: bool,
    first: String,
    last: String,
}

/// The source of randomness of one record: a value depends on the seed, the record and the
/// name of the field only.
pub struct RecordRng {
    pub seed: u64,
    pub record: u64,
}

impl RecordRng {
    pub fn field(&self, name: &str) -> Rng {
        Rng::for_key(self.seed, self.record, name)
    }
}

impl Compiled {
    /// Generates the value; `row` holds the values of the fields before (for email with
    /// `from`).
    pub fn generate(&self, loc: Locale, rng: &RecordRng, shared: &mut Shared, row: &[Value]) -> Value {
        let mut r = rng.field(&self.name);
        let s = |x: String| Value::String(x);
        match &self.generator {
            Gen::FirstName => s(person(loc, rng, shared).first.clone()),
            Gen::LastName => s(person(loc, rng, shared).last.clone()),
            Gen::FullName => {
                let p = person(loc, rng, shared);
                s(format!("{} {}", p.first, p.last))
            }
            Gen::Salutation => {
                let female = person(loc, rng, shared).female;
                let sal = if loc == Locale::De {
                    de::SALUTATION
                } else {
                    en::SALUTATION
                };
                s(sal[usize::from(!female)].to_string())
            }
            Gen::BirthDate { min_age, max_age, on } => {
                let on = on.unwrap_or_else(dates::today);
                let latest = dates::years_before(on, *min_age);
                let earliest = dates::years_before(on, *max_age + 1) + 1;
                Value::Date(dates::format_date(r.range_i64(earliest, latest)))
            }
            Gen::Email { from } => {
                let local = if from.is_empty() {
                    let p = person(loc, rng, shared);
                    format!("{}.{}", mailbox(&p.first), mailbox(&p.last))
                } else {
                    let parts: Vec<String> = from
                        .iter()
                        .filter_map(|i| row.get(*i))
                        .map(|v| match v {
                            Value::Null => String::new(),
                            v => mailbox(&v.as_str().map_or_else(|| v.to_json().to_string(), str::to_string)),
                        })
                        .filter(|p| !p.is_empty())
                        .collect();
                    if parts.is_empty() {
                        format!("user{}", r.range_i64(1, 9999))
                    } else {
                        parts.join(".")
                    }
                };
                let number = if r.chance(0.3) {
                    r.range_i64(1, 99).to_string()
                } else {
                    String::new()
                };
                let domains = if loc == Locale::De {
                    de::EMAIL_DOMAINS
                } else {
                    en::EMAIL_DOMAINS
                };
                s(format!("{local}{number}@{}", r.pick(domains)))
            }
            Gen::Phone => s(fill(
                r.pick(if loc == Locale::De { de::PHONE } else { en::PHONE }),
                &mut r,
            )),
            Gen::Street => s(match loc {
                Locale::De => format!("{}{}", r.pick(de::STREET), r.pick(de::STREET_SUFFIX)),
                Locale::En => format!("{} {}", r.pick(EN::NAME_LAST_NAME), r.pick(EN::ADDRESS_STREET_SUFFIX)),
            }),
            Gen::HouseNumber => s(match loc {
                Locale::De => {
                    let n = 1 + r.below(150).min(r.below(150));
                    let suffix = if r.chance(0.1) {
                        ["a", "b", "c"][r.below(3) as usize]
                    } else {
                        ""
                    };
                    format!("{n}{suffix}")
                }
                Locale::En => (1 + r.below(9999)).to_string(),
            }),
            Gen::Postcode => s(place(loc, rng, shared).0.clone()),
            Gen::City => s(place(loc, rng, shared).1.clone()),
            Gen::Country => s(match loc {
                Locale::De => r.pick(de::COUNTRIES).to_string(),
                Locale::En => r.pick(EN::ADDRESS_COUNTRY).to_string(),
            }),
            Gen::Integer { min, max } => Value::Int(r.range_i64(*min, *max)),
            Gen::Decimal { min, max, scale } => {
                let span = (max - min + 1).max(1) as u128;
                let n = min + ((u128::from(r.next_u64()) << 64 | u128::from(r.next_u64())) % span) as i128;
                Value::Decimal(format_scaled(n, *scale))
            }
            Gen::Float { min, max, scale } => {
                let mut f = min + r.unit() * (max - min);
                if let Some(sc) = scale {
                    let p = 10f64.powi(*sc as i32);
                    f = ((f * p).round() / p).clamp(*min, *max);
                }
                Value::Float(f)
            }
            Gen::Boolean { rate } => Value::Bool(r.chance(*rate)),
            Gen::Date { from, to } => Value::Date(dates::format_date(r.range_i64(*from, *to))),
            Gen::Datetime { from, to } => Value::Datetime(dates::format_datetime(r.range_i64(*from, *to))),
            Gen::Uuid => {
                let (a, b) = (r.next_u64(), r.next_u64());
                let a = (a & 0xFFFF_FFFF_FFFF_0FFF) | 0x4000; // version 4
                let b = (b & 0x3FFF_FFFF_FFFF_FFFF) | 0x8000_0000_0000_0000; // variant 1
                s(format!(
                    "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
                    a >> 32,
                    (a >> 16) & 0xFFFF,
                    a & 0xFFFF,
                    b >> 48,
                    b & 0xFFFF_FFFF_FFFF
                ))
            }
            Gen::Pick { values, cumulative } => {
                let x = r.unit() * cumulative.last().copied().unwrap_or(1.0);
                let i = cumulative.iter().position(|c| x < *c).unwrap_or(values.len() - 1);
                s(values[i].clone())
            }
            Gen::Pattern(p) => s(pattern(p, &mut r)),
        }
    }
}

fn person<'a>(loc: Locale, rng: &RecordRng, shared: &'a mut Shared) -> &'a Person {
    shared.person.get_or_insert_with(|| {
        let mut r = rng.field("#person");
        let female = r.chance(0.5);
        let (first, last) = match loc {
            Locale::De => (
                r.pick(if female { de::FEMALE } else { de::MALE }),
                r.pick(DE_DE::NAME_LAST_NAME),
            ),
            Locale::En => (
                r.pick(if female { en::FEMALE } else { en::MALE }),
                r.pick(EN::NAME_LAST_NAME),
            ),
        };
        Person {
            female,
            first: first.to_string(),
            last: last.to_string(),
        }
    })
}

fn place<'a>(loc: Locale, rng: &RecordRng, shared: &'a mut Shared) -> &'a (String, String) {
    shared.place.get_or_insert_with(|| {
        let mut r = rng.field("#place");
        match loc {
            Locale::De => {
                let (city, lo, hi) = *r.pick(de::CITIES);
                (
                    format!("{:05}", r.range_i64(i64::from(lo), i64::from(hi))),
                    city.to_string(),
                )
            }
            Locale::En => {
                let name = r.pick(EN::NAME_LAST_NAME);
                let suffix = r.pick(EN::ADDRESS_CITY_SUFFIX);
                let city = if r.chance(0.3) {
                    format!("{} {name}{suffix}", r.pick(EN::ADDRESS_CITY_PREFIX))
                } else {
                    format!("{name}{suffix}")
                };
                (format!("{:05}", r.range_i64(1001, 99950)), city)
            }
        }
    })
}

/// The part of an email address for a name: lowercase, umlauts written out, only letters
/// and digits ("Jürgen Groß" → "juergen.gross" for two parts).
fn mailbox(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars().flat_map(char::to_lowercase) {
        match c {
            'ä' => out.push_str("ae"),
            'ö' => out.push_str("oe"),
            'ü' => out.push_str("ue"),
            'ß' => out.push_str("ss"),
            ' ' | '-' | '.' if !out.is_empty() && !out.ends_with('.') => out.push('.'),
            c if c.is_ascii_alphanumeric() => out.push(c),
            c => out.push_str(&deunicode_char(c)),
        }
    }
    out.trim_matches('.').to_string()
}

fn deunicode_char(c: char) -> String {
    let mut s = String::new();
    for d in c.to_string().chars() {
        let base = match d {
            'à' | 'á' | 'â' | 'ã' | 'å' | 'æ' => "a",
            'ç' => "c",
            'è' | 'é' | 'ê' | 'ë' => "e",
            'ì' | 'í' | 'î' | 'ï' => "i",
            'ñ' => "n",
            'ò' | 'ó' | 'ô' | 'õ' | 'ø' => "o",
            'ù' | 'ú' | 'û' => "u",
            'ý' | 'ÿ' => "y",
            _ => "",
        };
        s.push_str(base);
    }
    s
}

/// `#` a digit, `^` a digit from 1 to 9, `~` a digit from 2 to 9.
fn fill(format: &str, r: &mut Rng) -> String {
    format
        .chars()
        .map(|c| match c {
            '#' => char::from(b'0' + r.below(10) as u8),
            '^' => char::from(b'1' + r.below(9) as u8),
            '~' => char::from(b'2' + r.below(8) as u8),
            c => c,
        })
        .collect()
}

/// `#` a digit, `?` a capital letter, `*` a digit or capital letter, `\` the next character
/// as it is.
fn pattern(p: &str, r: &mut Rng) -> String {
    const ALNUM: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut out = String::with_capacity(p.len());
    let mut chars = p.chars();
    while let Some(c) = chars.next() {
        match c {
            '#' => out.push(char::from(b'0' + r.below(10) as u8)),
            '?' => out.push(char::from(b'A' + r.below(26) as u8)),
            '*' => out.push(char::from(ALNUM[r.below(36) as usize])),
            '\\' => out.extend(chars.next()),
            c => out.push(c),
        }
    }
    out
}

/// n / 10^scale as exact text, e.g. 12345 with scale 2 → "123.45".
fn format_scaled(n: i128, scale: u32) -> String {
    if scale == 0 {
        return n.to_string();
    }
    let p = 10i128.pow(scale);
    let sign = if n < 0 { "-" } else { "" };
    let a = n.unsigned_abs();
    format!(
        "{sign}{}.{:0width$}",
        a / p as u128,
        a % p as u128,
        width = scale as usize
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helpers() {
        assert_eq!(format_scaled(12345, 2), "123.45");
        assert_eq!(format_scaled(-5, 2), "-0.05");
        assert_eq!(format_scaled(7, 0), "7");
        assert_eq!(mailbox("Jürgen Groß"), "juergen.gross");
        assert_eq!(mailbox("Anne-Marie"), "anne.marie");
        assert_eq!(mailbox("Zoë"), "zoe");
        let mut r = Rng::new(1);
        let p = pattern(r"KD-#####/??\#", &mut r);
        assert!(p.starts_with("KD-") && p.ends_with('#') && p.len() == 12, "{p}");
    }
}
