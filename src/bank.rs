//! IBANs, BICs and VAT ids with valid check digits.

use crate::rng::Rng;

/// Countries of `iban`: those whose account number has no check digit of its own, and
/// Belgium (its check is computed).
pub const IBAN_COUNTRIES: &[&str] = &["DE", "AT", "CH", "LI", "LU", "NL", "GB", "BE"];

/// Countries of `vatId`.
pub const VAT_COUNTRIES: &[&str] = &["DE", "AT", "GB"];

fn digits(r: &mut Rng, n: usize) -> String {
    (0..n).map(|_| char::from(b'0' + r.below(10) as u8)).collect()
}

fn letters(r: &mut Rng, n: usize) -> String {
    (0..n).map(|_| char::from(b'A' + r.below(26) as u8)).collect()
}

/// An IBAN with valid check digits (ISO 13616, mod 97), e.g. DE89370400440532013000.
pub fn iban(country: &str, r: &mut Rng) -> String {
    let bban = match country {
        "AT" => digits(r, 16),
        "CH" | "LI" => digits(r, 17),
        "LU" => digits(r, 16),
        "NL" => letters(r, 4) + &digits(r, 10),
        "GB" => letters(r, 4) + &digits(r, 14),
        "BE" => {
            let n = digits(r, 10);
            let v: u64 = n.parse().unwrap_or(0);
            let check = match v % 97 {
                0 => 97,
                c => c,
            };
            format!("{n}{check:02}")
        }
        _ => {
            // DE: bank code (8, not starting with 0) and account number (10)
            let first = char::from(b'1' + r.below(9) as u8);
            format!("{first}{}", digits(r, 17))
        }
    };
    let check = 98 - mod97(&format!("{bban}{country}00"));
    format!("{country}{check:02}{bban}")
}

/// The remainder of a text of digits and letters (A=10 … Z=35) divided by 97.
pub fn mod97(text: &str) -> u32 {
    let mut rem = 0u32;
    for c in text.chars() {
        let v = c.to_digit(36).unwrap_or(0);
        rem = if v >= 10 {
            (rem * 100 + v) % 97
        } else {
            (rem * 10 + v) % 97
        };
    }
    rem
}

/// A BIC: bank (4 letters), country, location (2), sometimes a branch (3).
pub fn bic(country: &str, r: &mut Rng) -> String {
    const LOC: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ23456789";
    let loc: String = (0..2)
        .map(|_| char::from(LOC[r.below(LOC.len() as u64) as usize]))
        .collect();
    let branch = if r.chance(0.5) {
        "XXX".to_string()
    } else {
        String::new()
    };
    format!("{}{country}{loc}{branch}", letters(r, 4))
}

/// A VAT id with valid check digit: DE (ISO 7064 MOD 11,10), AT (ATU), GB (mod 97).
pub fn vat_id(country: &str, r: &mut Rng) -> String {
    match country {
        "AT" => {
            let d: Vec<u32> = (0..7).map(|_| r.below(10) as u32).collect();
            let s: u32 = d
                .iter()
                .enumerate()
                .map(|(i, &x)| if i % 2 == 1 { (2 * x) / 10 + (2 * x) % 10 } else { x })
                .sum();
            let check = (10 - (s + 4) % 10) % 10;
            format!("ATU{}{check}", d.iter().map(|x| x.to_string()).collect::<String>())
        }
        "GB" => {
            let d: Vec<u32> = (0..7).map(|_| r.below(10) as u32).collect();
            let s: u32 = d.iter().zip((2..=8).rev()).map(|(x, w)| x * w).sum();
            let check = (97 - s % 97) % 97;
            format!("GB{}{check:02}", d.iter().map(|x| x.to_string()).collect::<String>())
        }
        _ => {
            let mut d: Vec<u32> = (0..8).map(|_| r.below(10) as u32).collect();
            d[0] = 1 + r.below(9) as u32;
            let mut product = 10;
            for &x in &d {
                let mut sum = (x + product) % 10;
                if sum == 0 {
                    sum = 10;
                }
                product = (2 * sum) % 11;
            }
            let check = (11 - product) % 10;
            format!("DE{}{check}", d.iter().map(|x| x.to_string()).collect::<String>())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_values() {
        // examples of the standards
        assert_eq!(mod97("370400440532013000DE89"), 1);
        assert_eq!(
            mod97(
                "GB82WEST12345698765432"
                    .chars()
                    .cycle()
                    .skip(4)
                    .take(22)
                    .collect::<String>()
                    .as_str()
            ),
            1
        );
    }

    #[test]
    fn generated_values_are_valid() {
        let mut r = Rng::new(5);
        for c in IBAN_COUNTRIES {
            for _ in 0..200 {
                let i = iban(c, &mut r);
                let rotated = format!("{}{}", &i[4..], &i[..4]);
                assert_eq!(mod97(&rotated), 1, "{i}");
                assert!(i.starts_with(c));
            }
        }
        for _ in 0..200 {
            let de = vat_id("DE", &mut r);
            assert_eq!(de.len(), 11);
            let at = vat_id("AT", &mut r);
            assert!(at.starts_with("ATU") && at.len() == 11, "{at}");
            let gb = vat_id("GB", &mut r);
            assert_eq!(gb.len(), 11);
            let s: u32 = gb[2..9]
                .chars()
                .zip((2..=8).rev())
                .map(|(c, w)| c.to_digit(10).unwrap() * w)
                .sum::<u32>()
                + gb[9..].parse::<u32>().unwrap();
            assert_eq!(s % 97, 0, "{gb}");
            let b = bic("DE", &mut r);
            assert!(b.len() == 8 || b.len() == 11, "{b}");
            assert_eq!(&b[4..6], "DE");
        }
        // known valid ids
        assert!(valid_de("DE136695976"));
        assert!(valid_at("ATU13585627"));
    }

    fn valid_de(id: &str) -> bool {
        let d: Vec<u32> = id[2..].chars().map(|c| c.to_digit(10).unwrap()).collect();
        let mut product = 10;
        for &x in &d[..8] {
            let mut sum = (x + product) % 10;
            if sum == 0 {
                sum = 10;
            }
            product = (2 * sum) % 11;
        }
        (11 - product) % 10 == d[8]
    }

    fn valid_at(id: &str) -> bool {
        let d: Vec<u32> = id[3..].chars().map(|c| c.to_digit(10).unwrap()).collect();
        let s: u32 = d[..7]
            .iter()
            .enumerate()
            .map(|(i, &x)| if i % 2 == 1 { (2 * x) / 10 + (2 * x) % 10 } else { x })
            .sum();
        (10 - (s + 4) % 10) % 10 == d[7]
    }
}
