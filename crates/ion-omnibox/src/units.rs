//! Unit conversion for the address bar: `5 km to mi`, `100f in c`,
//! `2.5 cups in ml`, `1 gib to mb`, `60 mph in km/h`.
//!
//! The input is `<number> <unit> <to|in|as|=> <unit>`; units of different
//! kinds (metres to kilograms) don't convert.

use crate::calc;

/// A finished conversion.
#[derive(Debug, Clone, PartialEq)]
pub struct Conversion {
    pub value: f64,
    pub from: &'static str,
    pub result: f64,
    pub to: &'static str,
}

impl Conversion {
    /// "5 km = 3.10686 mi".
    pub fn text(&self) -> String {
        format!(
            "{} = {}",
            with_unit(&calc::format(self.value), self.from),
            with_unit(&self.result_text(), self.to)
        )
    }

    /// The converted number alone, rounded for reading: "3.10686".
    pub fn result_text(&self) -> String {
        calc::format(round_significant(self.result, 6))
    }
}

/// "5 km", but "-40°C".
fn with_unit(number: &str, symbol: &str) -> String {
    if symbol.starts_with('°') {
        format!("{number}{symbol}")
    } else {
        format!("{number} {symbol}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dimension {
    Length,
    Mass,
    Temperature,
    Volume,
    Area,
    Speed,
    Time,
    Data,
    Energy,
    Pressure,
    Angle,
}

/// One unit: the symbol shown, names people type, and how many base units
/// (metre, kilogram, litre, …) it is. Temperatures convert separately.
struct Unit {
    symbol: &'static str,
    names: &'static [&'static str],
    dimension: Dimension,
    factor: f64,
}

const fn unit(
    symbol: &'static str,
    names: &'static [&'static str],
    dimension: Dimension,
    factor: f64,
) -> Unit {
    Unit {
        symbol,
        names,
        dimension,
        factor,
    }
}

use Dimension::*;

const UNITS: &[Unit] = &[
    unit("mm", &["millimeter", "millimetre"], Length, 0.001),
    unit("cm", &["centimeter", "centimetre"], Length, 0.01),
    unit("m", &["meter", "metre"], Length, 1.0),
    unit("km", &["kilometer", "kilometre", "kms"], Length, 1000.0),
    unit("in", &["inch", "inches", "\""], Length, 0.0254),
    unit("ft", &["foot", "feet", "'"], Length, 0.3048),
    unit("yd", &["yard"], Length, 0.9144),
    unit("mi", &["mile"], Length, 1609.344),
    unit("nmi", &["nautical mile"], Length, 1852.0),
    unit("mg", &["milligram"], Mass, 1e-6),
    unit("g", &["gram", "gramme"], Mass, 0.001),
    unit("kg", &["kilogram", "kilo", "kgs"], Mass, 1.0),
    unit("t", &["tonne", "metric ton"], Mass, 1000.0),
    unit("oz", &["ounce"], Mass, 0.028_349_523_125),
    unit("lb", &["pound", "lbs"], Mass, 0.453_592_37),
    unit("st", &["stone"], Mass, 6.350_293_18),
    unit(
        "°C",
        &["c", "celsius", "degc", "centigrade"],
        Temperature,
        0.0,
    ),
    unit("°F", &["f", "fahrenheit", "degf"], Temperature, 0.0),
    unit("K", &["k", "kelvin"], Temperature, 0.0),
    unit("ml", &["milliliter", "millilitre"], Volume, 0.001),
    unit("cl", &["centiliter", "centilitre"], Volume, 0.01),
    unit("dl", &["deciliter", "decilitre"], Volume, 0.1),
    unit("l", &["liter", "litre", "ltr"], Volume, 1.0),
    unit("m³", &["m3", "cubic meter", "cubic metre"], Volume, 1000.0),
    unit("tsp", &["teaspoon"], Volume, 0.004_928_921_593_75),
    unit("tbsp", &["tablespoon"], Volume, 0.014_786_764_781_25),
    unit(
        "fl oz",
        &["floz", "fluid ounce"],
        Volume,
        0.029_573_529_562_5,
    ),
    unit("cup", &["cups"], Volume, 0.236_588_236_5),
    unit("pt", &["pint"], Volume, 0.473_176_473),
    unit("qt", &["quart"], Volume, 0.946_352_946),
    unit("gal", &["gallon"], Volume, 3.785_411_784),
    unit(
        "m²",
        &["m2", "sqm", "square meter", "square metre"],
        Area,
        1.0,
    ),
    unit(
        "km²",
        &["km2", "square kilometer", "square kilometre"],
        Area,
        1e6,
    ),
    unit(
        "ft²",
        &["ft2", "sqft", "square foot", "square feet"],
        Area,
        0.092_903_04,
    ),
    unit("mi²", &["mi2", "square mile"], Area, 2_589_988.110_336),
    unit("ha", &["hectare"], Area, 10_000.0),
    unit("acre", &["acres", "ac"], Area, 4_046.856_422_4),
    unit(
        "m/s",
        &["mps", "meters per second", "metres per second"],
        Speed,
        1.0,
    ),
    unit(
        "km/h",
        &[
            "kmh",
            "kph",
            "kmph",
            "kilometers per hour",
            "kilometres per hour",
        ],
        Speed,
        1.0 / 3.6,
    ),
    unit("mph", &["mi/h", "miles per hour"], Speed, 0.447_04),
    unit("kn", &["knot", "knots", "kt"], Speed, 0.514_444_444),
    unit("ms", &["millisecond", "msec"], Time, 0.001),
    unit("s", &["sec", "second", "secs"], Time, 1.0),
    unit("min", &["minute", "mins"], Time, 60.0),
    unit("h", &["hr", "hour", "hrs"], Time, 3600.0),
    unit("day", &["d", "days"], Time, 86_400.0),
    unit("week", &["wk", "weeks"], Time, 604_800.0),
    unit("year", &["yr", "years", "yrs"], Time, 31_557_600.0),
    unit("bit", &["bits"], Data, 0.125),
    unit("B", &["b", "byte"], Data, 1.0),
    unit("kB", &["kb", "kilobyte"], Data, 1e3),
    unit("MB", &["mb", "megabyte"], Data, 1e6),
    unit("GB", &["gb", "gigabyte"], Data, 1e9),
    unit("TB", &["tb", "terabyte"], Data, 1e12),
    unit("KiB", &["kib", "kibibyte"], Data, 1024.0),
    unit("MiB", &["mib", "mebibyte"], Data, 1_048_576.0),
    unit("GiB", &["gib", "gibibyte"], Data, 1_073_741_824.0),
    unit("TiB", &["tib", "tebibyte"], Data, 1_099_511_627_776.0),
    unit("Mbit", &["mbit", "megabit", "mbps"], Data, 125_000.0),
    unit("Gbit", &["gbit", "gigabit", "gbps"], Data, 125_000_000.0),
    unit("J", &["j", "joule"], Energy, 1.0),
    unit("kJ", &["kj", "kilojoule"], Energy, 1000.0),
    unit("cal", &["calorie"], Energy, 4.184),
    unit("kcal", &["kilocalorie", "calories"], Energy, 4184.0),
    unit("kWh", &["kwh", "kilowatt hour"], Energy, 3_600_000.0),
    unit("Pa", &["pa", "pascal"], Pressure, 1.0),
    unit("kPa", &["kpa", "kilopascal"], Pressure, 1000.0),
    unit("bar", &["bars"], Pressure, 100_000.0),
    unit("psi", &[], Pressure, 6_894.757_293_168),
    unit("atm", &["atmosphere"], Pressure, 101_325.0),
    unit(
        "°",
        &["deg", "degree", "degrees"],
        Angle,
        std::f64::consts::PI / 180.0,
    ),
    unit("rad", &["radian", "radians"], Angle, 1.0),
];

/// Words between the two units.
const SEPARATORS: &[&str] = &[" to ", " in ", " as ", " into ", "=", "->", "→"];

/// Convert `input` when it reads as `<number> <unit> to <unit>`.
pub fn convert(input: &str) -> Option<Conversion> {
    let input = input.trim().to_lowercase();
    let (left, right) = SEPARATORS.iter().find_map(|sep| {
        let at = input.rfind(sep)?;
        Some((&input[..at], &input[at + sep.len()..]))
    })?;
    let (value, from) = quantity(left)?;
    let to = find(right.trim())?;
    if from.dimension != to.dimension || std::ptr::eq(from, to) {
        return None;
    }
    let result = if from.dimension == Temperature {
        from_kelvin(to.symbol, to_kelvin(from.symbol, value))
    } else {
        value * from.factor / to.factor
    };
    result.is_finite().then_some(Conversion {
        value,
        from: from.symbol,
        result,
        to: to.symbol,
    })
}

/// "5 km", "5km", "-40°f", "2.5 cups", "1,000 ft".
fn quantity(text: &str) -> Option<(f64, &'static Unit)> {
    let text = text.trim();
    let split = text
        .char_indices()
        .find(|&(i, c)| {
            !(c.is_ascii_digit() || c == '.' || c == ',' || (i == 0 && (c == '-' || c == '+')))
        })
        .map(|(i, _)| i)?;
    let number: String = text[..split].chars().filter(|c| *c != ',').collect();
    let value: f64 = number.parse().ok()?;
    Some((value, find(text[split..].trim())?))
}

fn find(name: &str) -> Option<&'static Unit> {
    let name = name
        .trim()
        .trim_start_matches("degrees ")
        .trim_start_matches("degree ")
        .trim_start_matches('°')
        .trim();
    if name.is_empty() {
        return None;
    }
    let named = |name: &str| {
        UNITS.iter().find(|unit| {
            let symbol = unit.symbol.trim_start_matches('°');
            symbol.eq_ignore_ascii_case(name) || unit.names.contains(&name)
        })
    };
    // An exact name first, so "ms" is milliseconds rather than metres.
    named(name).or_else(|| {
        name.strip_suffix('s')
            .filter(|s| s.len() > 1)
            .and_then(named)
    })
}

fn to_kelvin(symbol: &str, value: f64) -> f64 {
    match symbol {
        "°C" => value + 273.15,
        "°F" => (value - 32.0) * 5.0 / 9.0 + 273.15,
        _ => value,
    }
}

fn from_kelvin(symbol: &str, kelvin: f64) -> f64 {
    match symbol {
        "°C" => kelvin - 273.15,
        "°F" => (kelvin - 273.15) * 9.0 / 5.0 + 32.0,
        _ => kelvin,
    }
}

fn round_significant(value: f64, digits: i32) -> f64 {
    if value == 0.0 || !value.is_finite() {
        return value;
    }
    let scale = 10f64.powi(digits - 1 - value.abs().log10().floor() as i32);
    (value * scale).round() / scale
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(input: &str) -> Option<String> {
        convert(input).map(|c| c.text())
    }

    #[test]
    fn converts_common_units() {
        assert_eq!(text("5 km to mi").as_deref(), Some("5 km = 3.10686 mi"));
        assert_eq!(text("5km in miles").as_deref(), Some("5 km = 3.10686 mi"));
        assert_eq!(text("6 feet in cm").as_deref(), Some("6 ft = 182.88 cm"));
        assert_eq!(text("10 kg to lbs").as_deref(), Some("10 kg = 22.0462 lb"));
        assert_eq!(text("2 cups in ml").as_deref(), Some("2 cup = 473.176 ml"));
        assert_eq!(text("1 GiB to MB").as_deref(), Some("1 GiB = 1073.74 MB"));
        assert_eq!(
            text("60 mph in km/h").as_deref(),
            Some("60 mph = 96.5606 km/h")
        );
        assert_eq!(text("1 hour in seconds").as_deref(), Some("1 h = 3600 s"));
        assert_eq!(text("1500 ms to s").as_deref(), Some("1500 ms = 1.5 s"));
        assert_eq!(text("1,000 ft = m").as_deref(), Some("1000 ft = 304.8 m"));
        assert_eq!(
            text("180 deg to rad").as_deref(),
            Some("180° = 3.14159 rad")
        );
    }

    #[test]
    fn converts_temperatures() {
        assert_eq!(text("100f to c").as_deref(), Some("100°F = 37.7778°C"));
        assert_eq!(text("-40 °C in °F").as_deref(), Some("-40°C = -40°F"));
        assert_eq!(
            text("0 kelvin to celsius").as_deref(),
            Some("0 K = -273.15°C")
        );
        assert_eq!(text("20 degrees c in f").as_deref(), Some("20°C = 68°F"));
    }

    #[test]
    fn refuses_mismatched_or_unknown_units() {
        for input in [
            "5 km to kg",
            "5 km to km",
            "5 parsecs to m",
            "km to mi",
            "how to cook rice",
            "weather in paris",
            "5 km",
        ] {
            assert_eq!(convert(input), None, "{input}");
        }
    }
}
