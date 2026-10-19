//! One inline answer for what was typed.

use crate::{calc, units};

/// An answer shown under the address bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    /// What the row says: "= 4" or "5 km = 3.10686 mi".
    pub text: String,
    /// What Enter copies: "4" or "3.10686".
    pub value: String,
}

/// The answer to `input` if it is a sum or a unit conversion.
pub fn answer(input: &str) -> Option<Answer> {
    if let Some(conversion) = units::convert(input) {
        return Some(Answer {
            text: conversion.text(),
            value: conversion.result_text(),
        });
    }
    let value = calc::format(calc::evaluate(input)?);
    Some(Answer {
        text: format!("= {value}"),
        value,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_sums_and_conversions() {
        assert_eq!(
            answer("2 + 2"),
            Some(Answer {
                text: "= 4".into(),
                value: "4".into()
            })
        );
        assert_eq!(answer("1 km in m").unwrap().value, "1000");
        assert_eq!(answer("rust book"), None);
    }
}
