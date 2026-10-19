//! A calculator for the address bar: `2+2`, `sqrt(2)/2`, `15% of 80`,
//! `2^10`, `5!`, `sin(pi/6)`.
//!
//! Only input that is plainly arithmetic counts. A bare number, a word, or
//! something shaped like a date, phone number or version (`2026-10-19`,
//! `555-1234`) is left alone so it can still be searched.

/// The value of `input` when it reads as a calculation.
pub fn evaluate(input: &str) -> Option<f64> {
    let input = input.trim().trim_end_matches('=').trim();
    if input.is_empty() || looks_like_an_identifier(input) {
        return None;
    }
    let tokens = tokenize(input)?;
    let mut parser = Parser {
        tokens: &tokens,
        pos: 0,
        operations: 0,
    };
    let value = parser.expr()?;
    if parser.pos != tokens.len() || parser.operations == 0 || !value.is_finite() {
        return None;
    }
    Some(value)
}

/// `value` written for people: whole numbers without a decimal point, others
/// to 12 significant digits, very large or small ones in `e` notation.
pub fn format(value: f64) -> String {
    if value == 0.0 {
        return "0".to_owned();
    }
    let magnitude = value.abs();
    if !(1e-6..1e15).contains(&magnitude) {
        let text = format!("{value:.10e}");
        let (mantissa, exponent) = text.split_once('e').unwrap_or((&text, "0"));
        return format!("{}e{exponent}", trim_zeros(mantissa));
    }
    // Within a few units in the last place of a whole number: rounding
    // error, as in 0.1 * 3 * 10, not a real fraction.
    if (value - value.round()).abs() <= 8.0 * f64::EPSILON * magnitude.max(1.0) {
        return format!("{}", value.round() as i64);
    }
    let decimals = (11 - magnitude.log10().floor() as i32).clamp(0, 17) as usize;
    trim_zeros(&format!("{value:.decimals$}")).to_owned()
}

fn trim_zeros(text: &str) -> &str {
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.')
    } else {
        text
    }
}

/// Dashes and dots between bare digit groups read as dates, phone numbers,
/// IP addresses or versions, not sums.
fn looks_like_an_identifier(input: &str) -> bool {
    const SEPARATORS: [char; 3] = ['-', '.', '/'];
    // A country code's "+" doesn't make "+1-555-1234" a sum.
    let input = input.strip_prefix('+').unwrap_or(input);
    let groups: Vec<&str> = input.split(SEPARATORS).collect();
    if groups.len() < 2
        || !groups
            .iter()
            .all(|group| !group.is_empty() && group.chars().all(|c| c.is_ascii_digit()))
    {
        return false;
    }
    let separators = groups.len() - 1;
    let one_kind = SEPARATORS
        .iter()
        .any(|kind| input.matches(*kind).count() == separators);
    // One dash reads as a phone number or a month only in their shapes:
    // three digits then four ("555-1234"), or a year then a month ("2026-10").
    let pair = |first: usize, second: usize| {
        groups.len() == 2 && groups[0].len() == first && groups[1].len() == second
    };
    let month = pair(4, 2) && matches!(groups[1].parse::<u8>(), Ok(1..=12));
    // "1/2", "10-3" and "100-50" are sums; "555-1234", "2026-10", "1.2.3"
    // and "10/19/2026" aren't.
    one_kind && (separators >= 2 || (input.contains('-') && (pair(3, 4) || month)))
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    Word(String),
    Plus,
    Minus,
    Times,
    Divide,
    Power,
    Percent,
    Factorial,
    Open,
    Close,
}

fn tokenize(input: &str) -> Option<Vec<Token>> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let token = match c {
            ' ' | '\t' => {
                i += 1;
                continue;
            }
            '0'..='9' | '.' => {
                let (value, end) = number(&chars, i)?;
                i = end;
                tokens.push(Token::Number(value));
                continue;
            }
            '+' => Token::Plus,
            '-' | '−' | '–' => Token::Minus,
            '*' if chars.get(i + 1) == Some(&'*') => {
                i += 1;
                Token::Power
            }
            '*' | '×' | '·' | 'x' if c != 'x' || is_times_x(&chars, i) => Token::Times,
            '/' | '÷' => Token::Divide,
            '^' => Token::Power,
            '%' => Token::Percent,
            '!' => Token::Factorial,
            '(' => Token::Open,
            ')' => Token::Close,
            'π' => Token::Word("pi".to_owned()),
            c if c.is_alphabetic() => {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect::<String>().to_lowercase();
                tokens.push(Token::Word(word));
                continue;
            }
            _ => return None,
        };
        tokens.push(token);
        i += 1;
    }
    Some(tokens)
}

/// "3 x 4" and "3x4" multiply; the x in "exp" or "max" doesn't.
fn is_times_x(chars: &[char], i: usize) -> bool {
    let before = chars[..i].iter().rev().find(|c| **c != ' ');
    let after = chars[i + 1..].iter().find(|c| **c != ' ');
    let operand_before = before.is_some_and(|c| c.is_ascii_digit() || *c == ')' || *c == '.');
    let operand_after = after.is_some_and(|c| c.is_ascii_digit() || *c == '(' || *c == '.');
    operand_before && operand_after
}

/// A decimal number at `start`: `12`, `1.5`, `.5`, `1e-3`, `1,000,000`.
fn number(chars: &[char], start: usize) -> Option<(f64, usize)> {
    let mut text = String::new();
    let mut i = start;
    let mut seen_dot = false;
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_digit() {
            text.push(c);
        } else if c == '.' && !seen_dot {
            seen_dot = true;
            text.push(c);
        } else if c == ','
            && !seen_dot
            && chars.len() >= i + 4
            && chars[i + 1..i + 4].iter().all(char::is_ascii_digit)
            && chars.get(i + 4).is_none_or(|c| !c.is_ascii_digit())
        {
            // A thousands separator: exactly three digits follow.
        } else {
            break;
        }
        i += 1;
    }
    // An exponent only when digits follow, so "2e" stays 2 × e.
    if matches!(chars.get(i), Some('e' | 'E')) {
        let mut j = i + 1;
        if matches!(chars.get(j), Some('+' | '-')) {
            j += 1;
        }
        if chars.get(j).is_some_and(char::is_ascii_digit) {
            text.push('e');
            text.extend(&chars[i + 1..j]);
            i = j;
            while i < chars.len() && chars[i].is_ascii_digit() {
                text.push(chars[i]);
                i += 1;
            }
        }
    }
    if text == "." {
        return None;
    }
    text.parse().ok().map(|value| (value, i))
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    /// Operators and functions applied; a lone number isn't a calculation.
    operations: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn eat(&mut self, token: &Token) -> bool {
        if self.peek() == Some(token) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn eat_word(&mut self, word: &str) -> bool {
        if matches!(self.peek(), Some(Token::Word(w)) if w == word) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expr(&mut self) -> Option<f64> {
        let mut value = self.term()?;
        loop {
            if self.eat(&Token::Plus) {
                value += self.term()?;
            } else if self.eat(&Token::Minus) {
                value -= self.term()?;
            } else {
                return Some(value);
            }
            self.operations += 1;
        }
    }

    fn term(&mut self) -> Option<f64> {
        let mut value = self.unary()?;
        loop {
            if self.eat(&Token::Times) || self.eat_word("of") || self.eat_word("times") {
                value *= self.unary()?;
            } else if self.eat(&Token::Divide) {
                value /= self.unary()?;
            } else if self.eat_word("mod") {
                value = value.rem_euclid(self.unary()?);
            } else if matches!(self.peek(), Some(Token::Open | Token::Word(_)))
                && !matches!(self.peek(), Some(Token::Word(w)) if w == "mod" || w == "of" || w == "times")
            {
                // "2pi", "3(4+5)", "2 sqrt 2".
                value *= self.unary()?;
            } else {
                return Some(value);
            }
            self.operations += 1;
        }
    }

    fn unary(&mut self) -> Option<f64> {
        if self.eat(&Token::Minus) {
            return Some(-self.unary()?);
        }
        if self.eat(&Token::Plus) {
            return self.unary();
        }
        self.power()
    }

    fn power(&mut self) -> Option<f64> {
        let base = self.postfix()?;
        if self.eat(&Token::Power) {
            self.operations += 1;
            return Some(base.powf(self.unary()?));
        }
        Some(base)
    }

    fn postfix(&mut self) -> Option<f64> {
        let mut value = self.primary()?;
        loop {
            if self.eat(&Token::Percent) {
                value /= 100.0;
            } else if self.eat(&Token::Factorial) {
                value = factorial(value)?;
            } else {
                return Some(value);
            }
            self.operations += 1;
        }
    }

    fn primary(&mut self) -> Option<f64> {
        match self.peek()?.clone() {
            Token::Number(value) => {
                self.pos += 1;
                Some(value)
            }
            Token::Open => {
                self.pos += 1;
                let value = self.expr()?;
                // A missing closing bracket at the very end is forgiven.
                if !self.eat(&Token::Close) && self.pos != self.tokens.len() {
                    return None;
                }
                Some(value)
            }
            Token::Word(word) => {
                self.pos += 1;
                if let Some(value) = constant(&word) {
                    return Some(value);
                }
                let function = function(&word)?;
                self.operations += 1;
                // "sin(x)^2" squares the sine; "sqrt 2^2" takes the root of 4.
                let argument = if self.peek() == Some(&Token::Open) {
                    self.postfix()?
                } else {
                    self.power()?
                };
                Some(function(argument))
            }
            _ => None,
        }
    }
}

fn constant(word: &str) -> Option<f64> {
    Some(match word {
        "pi" => std::f64::consts::PI,
        "tau" => std::f64::consts::TAU,
        "e" => std::f64::consts::E,
        "phi" => (1.0 + 5f64.sqrt()) / 2.0,
        _ => return None,
    })
}

fn function(word: &str) -> Option<fn(f64) -> f64> {
    Some(match word {
        "sqrt" => f64::sqrt,
        "cbrt" => f64::cbrt,
        "abs" => f64::abs,
        "ln" => f64::ln,
        "log" | "lg" => f64::log10,
        "log2" => f64::log2,
        "exp" => f64::exp,
        "sin" => f64::sin,
        "cos" => f64::cos,
        "tan" => f64::tan,
        "asin" | "arcsin" => f64::asin,
        "acos" | "arccos" => f64::acos,
        "atan" | "arctan" => f64::atan,
        "sinh" => f64::sinh,
        "cosh" => f64::cosh,
        "tanh" => f64::tanh,
        "round" => f64::round,
        "floor" => f64::floor,
        "ceil" => f64::ceil,
        _ => return None,
    })
}

/// `n!` for whole numbers up to 170 (above that it overflows).
fn factorial(n: f64) -> Option<f64> {
    if n < 0.0 || n.fract() != 0.0 || n > 170.0 {
        return None;
    }
    Some((1..=n as u32).map(f64::from).product())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calc(input: &str) -> Option<String> {
        evaluate(input).map(format)
    }

    #[test]
    fn arithmetic_follows_precedence() {
        assert_eq!(calc("2+2").as_deref(), Some("4"));
        assert_eq!(calc("2 + 3 * 4").as_deref(), Some("14"));
        assert_eq!(calc("(2 + 3) * 4").as_deref(), Some("20"));
        assert_eq!(calc("2^10").as_deref(), Some("1024"));
        assert_eq!(calc("2**3**2").as_deref(), Some("512"));
        assert_eq!(calc("-2^2").as_deref(), Some("-4"));
        assert_eq!(calc("10 / 4").as_deref(), Some("2.5"));
        assert_eq!(calc("7 mod 3").as_deref(), Some("1"));
        assert_eq!(calc("1/3").as_deref(), Some("0.333333333333"));
        assert_eq!(calc("3 x 4").as_deref(), Some("12"));
        assert_eq!(calc("6 × 7 =").as_deref(), Some("42"));
        assert_eq!(calc("(1+2").as_deref(), Some("3"));
    }

    #[test]
    fn functions_constants_and_postfix() {
        assert_eq!(calc("sqrt(16)").as_deref(), Some("4"));
        assert_eq!(calc("sqrt 2").as_deref(), Some("1.41421356237"));
        assert_eq!(calc("2pi").as_deref(), Some("6.28318530718"));
        assert_eq!(calc("sin(pi/6)").as_deref(), Some("0.5"));
        assert_eq!(calc("5!").as_deref(), Some("120"));
        assert_eq!(calc("15% of 80").as_deref(), Some("12"));
        assert_eq!(calc("50%").as_deref(), Some("0.5"));
        assert_eq!(calc("log(1000)").as_deref(), Some("3"));
        assert_eq!(calc("1,000 * 3").as_deref(), Some("3000"));
        assert_eq!(calc("1e3 + 1").as_deref(), Some("1001"));
    }

    #[test]
    fn very_large_and_small_results_use_e_notation() {
        assert_eq!(calc("10^20").as_deref(), Some("1e20"));
        assert_eq!(calc("2^-30").as_deref(), Some("9.3132257462e-10"));
        assert_eq!(calc("0.1 + 0.2").as_deref(), Some("0.3"));
        assert_eq!(calc("0.1 * 3 * 10").as_deref(), Some("3"));
        assert_eq!(calc("1000000000 + 0.5").as_deref(), Some("1000000000.5"));
    }

    #[test]
    fn leaves_non_calculations_alone() {
        for input in [
            "",
            "42",
            "-5",
            "pi",
            "hello",
            "covid-19",
            "2026-10-19",
            "2026-10",
            "555-1234",
            "+1-555-1234",
            "+44-20-7946-0958",
            "1.2.3",
            "10/19/2026",
            "192.168.1.1",
            "example.com",
            "1/0",
            "x",
            "max 3",
            "sqrt",
            "2 +",
            "(",
            "171!",
            "rust 2024",
        ] {
            assert_eq!(calc(input), None, "{input}");
        }
    }

    #[test]
    fn short_dashed_sums_still_count() {
        assert_eq!(calc("10-3").as_deref(), Some("7"));
        assert_eq!(calc("100-50").as_deref(), Some("50"));
        assert_eq!(calc("1000-1").as_deref(), Some("999"));
        assert_eq!(calc("2026-13").as_deref(), Some("2013"));
        assert_eq!(calc("10 - 3 - 2").as_deref(), Some("5"));
    }
}
