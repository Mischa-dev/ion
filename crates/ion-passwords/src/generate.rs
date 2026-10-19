//! Strong passwords for new accounts, and spotting weak ones.

const LOWER: &[u8] = b"abcdefghijkmnopqrstuvwxyz";
const UPPER: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ";
const DIGITS: &[u8] = b"23456789";
const SYMBOLS: &[u8] = b"-_.!@#$%";

/// The length Ion suggests: about 115 bits, well past any brute force, and
/// short enough for sites with a 20-character limit.
pub const DEFAULT_LENGTH: usize = 20;

/// A random password of `length` (at least 8) characters with lower- and
/// upper-case letters, digits and a few symbols most sites accept. Look-alike
/// characters (l, I, O, 0, 1) are left out so it can be read off a screen.
pub fn generate(length: usize) -> String {
    let length = length.max(8);
    let classes: [&[u8]; 4] = [LOWER, UPPER, DIGITS, SYMBOLS];
    let all: Vec<u8> = classes.concat();
    loop {
        let mut out = Vec::with_capacity(length);
        for _ in 0..length {
            out.push(all[random_below(all.len())]);
        }
        // Retry the rare draw that misses a class rather than patching it,
        // which would skew where each class lands.
        if classes
            .iter()
            .all(|class| out.iter().any(|c| class.contains(c)))
        {
            return String::from_utf8(out).unwrap_or_default();
        }
    }
}

/// A uniformly random index below `n` (n ≤ 256).
fn random_below(n: usize) -> usize {
    debug_assert!(n > 0 && n <= 256);
    // Reject the top of the byte range that would bias the modulo.
    let limit = 256 - 256 % n;
    loop {
        let mut byte = [0u8; 1];
        getrandom::fill(&mut byte).expect("the OS random source failed");
        if (byte[0] as usize) < limit {
            return byte[0] as usize % n;
        }
    }
}

/// Passwords so common that attackers try them first.
const COMMON: &[&str] = &[
    "123456",
    "123456789",
    "12345678",
    "password",
    "qwerty",
    "qwerty123",
    "1q2w3e4r",
    "12345",
    "111111",
    "1234567890",
    "1234567",
    "123123",
    "abc123",
    "password1",
    "iloveyou",
    "admin",
    "welcome",
    "monkey",
    "dragon",
    "letmein",
    "football",
    "baseball",
    "sunshine",
    "princess",
    "000000",
    "654321",
    "superman",
    "trustno1",
    "passw0rd",
    "master",
    "hello123",
    "qwertyuiop",
];

/// Whether `password` is easy to guess: short, all one kind of character
/// below 12 characters, or one of the most common passwords.
pub fn is_weak(password: &str) -> bool {
    let len = password.chars().count();
    if len < 8 {
        return true;
    }
    let lower = password.to_lowercase();
    if COMMON.iter().any(|common| lower == *common) {
        return true;
    }
    let kinds = [
        password.chars().any(|c| c.is_lowercase()),
        password.chars().any(|c| c.is_uppercase()),
        password.chars().any(|c| c.is_ascii_digit()),
        password.chars().any(|c| !c.is_alphanumeric()),
    ]
    .iter()
    .filter(|&&has| has)
    .count();
    len < 12 && kinds < 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_passwords_have_every_class() {
        for _ in 0..200 {
            let password = generate(DEFAULT_LENGTH);
            assert_eq!(password.len(), DEFAULT_LENGTH);
            for class in [LOWER, UPPER, DIGITS, SYMBOLS] {
                assert!(password.bytes().any(|c| class.contains(&c)), "{password}");
            }
            assert!(!password.contains(['l', 'I', 'O', '0', '1']));
            assert!(!is_weak(&password));
        }
        assert_eq!(generate(3).len(), 8);
        assert_ne!(generate(20), generate(20));
    }

    #[test]
    fn weak_passwords_are_spotted() {
        assert!(is_weak("abc"));
        assert!(is_weak("Password"));
        assert!(is_weak("abcdefghij"));
        assert!(is_weak("12345678"));
        assert!(!is_weak("correct horse battery staple"));
        assert!(!is_weak("Tr0ub4dor&3x"));
    }
}
