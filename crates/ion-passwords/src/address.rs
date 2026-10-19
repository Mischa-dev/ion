//! Saved addresses for filling in forms (shipping, billing, sign-up).

use serde::{Deserialize, Serialize};

/// One saved address. Every field may be empty.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Address {
    pub id: u64,
    pub name: String,
    pub organization: String,
    /// Street lines, newline-separated.
    pub street: String,
    pub city: String,
    /// State, province or county.
    pub region: String,
    pub postal_code: String,
    pub country: String,
    pub email: String,
    pub phone: String,
    /// Milliseconds since the Unix epoch it was last filled in, 0 for never.
    pub used: i64,
}

impl Address {
    /// Whether the form gave enough to be worth saving: a street, or a
    /// name together with an email or phone number.
    pub fn is_worth_saving(&self) -> bool {
        let has = |s: &str| !s.trim().is_empty();
        has(&self.street) || (has(&self.name) && (has(&self.email) || has(&self.phone)))
    }

    /// A one-line summary for menus: "Mischa Nelson, 1 Main St".
    pub fn summary(&self) -> String {
        let street = self.street.lines().next().unwrap_or("");
        [
            self.name.as_str(),
            street,
            self.city.as_str(),
            self.email.as_str(),
        ]
        .into_iter()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .take(2)
        .collect::<Vec<_>>()
        .join(", ")
    }

    /// Whether `other` is the same address, maybe with fewer details.
    pub fn covers(&self, other: &Address) -> bool {
        let norm = |s: &str| {
            s.chars()
                .filter(|c| c.is_alphanumeric())
                .flat_map(char::to_lowercase)
                .collect::<String>()
        };
        let pairs = [
            (&self.name, &other.name),
            (&self.organization, &other.organization),
            (&self.street, &other.street),
            (&self.city, &other.city),
            (&self.region, &other.region),
            (&self.postal_code, &other.postal_code),
            (&self.country, &other.country),
            (&self.email, &other.email),
            (&self.phone, &other.phone),
        ];
        pairs
            .iter()
            .all(|(mine, theirs)| theirs.trim().is_empty() || norm(mine) == norm(theirs))
    }

    /// Fill this address's empty fields from `other`.
    pub fn merge(&mut self, other: &Address) {
        let fields = [
            (&mut self.name, &other.name),
            (&mut self.organization, &other.organization),
            (&mut self.street, &other.street),
            (&mut self.city, &other.city),
            (&mut self.region, &other.region),
            (&mut self.postal_code, &other.postal_code),
            (&mut self.country, &other.country),
            (&mut self.email, &other.email),
            (&mut self.phone, &other.phone),
        ];
        for (mine, theirs) in fields {
            if mine.trim().is_empty() {
                mine.clone_from(theirs);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home() -> Address {
        Address {
            name: "Ada Lovelace".into(),
            street: "12 St James's Square\nFlat 2".into(),
            city: "London".into(),
            postal_code: "SW1Y 4JH".into(),
            email: "ada@example.com".into(),
            ..Address::default()
        }
    }

    #[test]
    fn worth_saving_and_summary() {
        assert!(home().is_worth_saving());
        assert_eq!(home().summary(), "Ada Lovelace, 12 St James's Square");
        let contact = Address {
            name: "Ada".into(),
            phone: "123".into(),
            ..Address::default()
        };
        assert!(contact.is_worth_saving());
        assert!(
            !Address {
                name: "Ada".into(),
                ..Address::default()
            }
            .is_worth_saving()
        );
    }

    #[test]
    fn covering_ignores_case_spacing_and_missing_fields() {
        let partial = Address {
            name: "ada lovelace".into(),
            postal_code: "sw1y4jh".into(),
            ..Address::default()
        };
        assert!(home().covers(&partial));
        let moved = Address {
            city: "Paris".into(),
            ..partial
        };
        assert!(!home().covers(&moved));
        let mut merged = Address {
            phone: "123".into(),
            ..Address::default()
        };
        merged.merge(&home());
        assert_eq!(
            (merged.phone.as_str(), merged.city.as_str()),
            ("123", "London")
        );
    }
}
