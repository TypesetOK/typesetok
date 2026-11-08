use serde::{Deserialize, Serialize};

/// Form of Hebrew punctuation marks appended to traditional numerals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GematriaPunctuation {
    /// Traditional geresh on single letter (א׳) and gershayim before last letter (תשפ״ו).
    #[default]
    StandardPunctuation,
    /// Pure Hebrew letters without punctuation quotes (א, ב, יא, תשפו).
    PlainLetters,
}

/// Extensible numbering schemes supporting Torah, Hebrew liturgical, and secular documents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum NumberingScheme {
    /// Standard Arabic decimal digits (1, 2, 3...).
    #[default]
    Arabic,
    /// Hebrew Gematria numerals (א׳, ב׳... ט״ו, ט״ז... ק״כ).
    HebrewGematria(GematriaPunctuation),
    /// Torah Siman notation (סימן א׳, סימן ב׳...).
    TorahSiman(GematriaPunctuation),
    /// Torah Seif notation (סעיף א׳, סעיף ב׳...).
    TorahSeif(GematriaPunctuation),
    /// Torah Ot notation (אות א׳, אות ב׳...).
    TorahOt(GematriaPunctuation),
    /// Traditional Talmud Daf and Amud foliation (דף ב׳ ע״א, דף ב׳ ע״ב...).
    /// Value 1 represents 2a (דף ב ע"א), value 2 represents 2b (דף ב ע"ב), etc.
    TorahDafAmud(GematriaPunctuation),
    /// Lowercase Roman numerals (i, ii, iii, iv...).
    RomanLower,
    /// Uppercase Roman numerals (I, II, III, IV...).
    RomanUpper,
    /// Custom wrapper with arbitrary prefix and suffix.
    Custom {
        prefix: String,
        base: Box<NumberingScheme>,
        suffix: String,
    },
}

impl NumberingScheme {
    /// Returns a copy of this scheme configured with the specified Gematria punctuation.
    pub fn with_punctuation(&self, punct: GematriaPunctuation) -> Self {
        match self {
            Self::HebrewGematria(_) => Self::HebrewGematria(punct),
            Self::TorahSiman(_) => Self::TorahSiman(punct),
            Self::TorahSeif(_) => Self::TorahSeif(punct),
            Self::TorahOt(_) => Self::TorahOt(punct),
            Self::TorahDafAmud(_) => Self::TorahDafAmud(punct),
            Self::Custom {
                prefix,
                base,
                suffix,
            } => Self::Custom {
                prefix: prefix.clone(),
                base: Box::new(base.with_punctuation(punct)),
                suffix: suffix.clone(),
            },
            other => other.clone(),
        }
    }
}

/// Scope at which counter increments or resets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum NumberingScope {
    /// Continuous numbering across the entire book.
    #[default]
    ContinuousBook,
    /// Resets at every section or chapter break.
    PerSection,
    /// Resets at every page boundary (e.g. footnotes resetting per page).
    PerPage,
}

/// Formatter for numbering schemes.
pub struct NumberingFormatter;

impl NumberingFormatter {
    /// Formats an ordinal value (1-based) according to the scheme.
    pub fn format(scheme: &NumberingScheme, value: usize) -> String {
        if value == 0 {
            return "0".to_string();
        }

        match scheme {
            NumberingScheme::Arabic => value.to_string(),
            NumberingScheme::HebrewGematria(punct) => Self::to_gematria(value, *punct),
            NumberingScheme::TorahSiman(punct) => {
                format!("סימן {}", Self::to_gematria(value, *punct))
            }
            NumberingScheme::TorahSeif(punct) => {
                format!("סעיף {}", Self::to_gematria(value, *punct))
            }
            NumberingScheme::TorahOt(punct) => {
                format!("אות {}", Self::to_gematria(value, *punct))
            }
            NumberingScheme::TorahDafAmud(punct) => {
                // Value 1 -> Daf 2 Amud 1; Value 2 -> Daf 2 Amud 2; Value 3 -> Daf 3 Amud 1
                let daf_number = (value - 1) / 2 + 2;
                let is_amud_b = (value - 1) % 2 == 1;
                let daf_str = Self::to_gematria(daf_number, *punct);
                let amud_str = if is_amud_b {
                    match punct {
                        GematriaPunctuation::StandardPunctuation => "ע״ב",
                        GematriaPunctuation::PlainLetters => "עב",
                    }
                } else {
                    match punct {
                        GematriaPunctuation::StandardPunctuation => "ע״א",
                        GematriaPunctuation::PlainLetters => "עא",
                    }
                };
                format!("דף {} {}", daf_str, amud_str)
            }
            NumberingScheme::RomanLower => Self::to_roman(value).to_lowercase(),
            NumberingScheme::RomanUpper => Self::to_roman(value),
            NumberingScheme::Custom {
                prefix,
                base,
                suffix,
            } => {
                format!("{}{}{}", prefix, Self::format(base, value), suffix)
            }
        }
    }

    /// Converts a positive integer to traditional Hebrew gematria representation.
    pub fn to_gematria(num: usize, punct: GematriaPunctuation) -> String {
        if num == 0 {
            return String::new();
        }

        let thousands = num / 1000;
        let remainder = num % 1000;

        match punct {
            GematriaPunctuation::PlainLetters => {
                let mut s = String::new();
                if thousands > 0 {
                    s.push_str(&Self::to_gematria_sub_1000(thousands));
                }
                if remainder > 0 {
                    s.push_str(&Self::to_gematria_sub_1000(remainder));
                }
                s
            }
            GematriaPunctuation::StandardPunctuation => {
                let mut s = String::new();
                if thousands > 0 {
                    let th_str = Self::to_gematria_sub_1000(thousands);
                    s.push_str(&th_str);
                    s.push('׳');
                }
                if remainder > 0 {
                    let rem_chars: Vec<char> =
                        Self::to_gematria_sub_1000(remainder).chars().collect();
                    if rem_chars.len() == 1 {
                        s.push(rem_chars[0]);
                        s.push('׳');
                    } else {
                        for (i, c) in rem_chars.iter().enumerate() {
                            if i == rem_chars.len() - 1 {
                                s.push('״');
                            }
                            s.push(*c);
                        }
                    }
                }
                s
            }
        }
    }

    fn to_gematria_sub_1000(mut n: usize) -> String {
        let mut result = String::new();

        // Hundreds: 400 = ת, 300 = ש, 200 = ר, 100 = ק
        while n >= 400 {
            result.push('ת');
            n -= 400;
        }
        if n >= 300 {
            result.push('ש');
            n -= 300;
        } else if n >= 200 {
            result.push('ר');
            n -= 200;
        } else if n >= 100 {
            result.push('ק');
            n -= 100;
        }

        // Traditional special cases for 15 and 16 to avoid God's name
        if n == 15 {
            result.push('ט');
            result.push('ו');
            return result;
        }
        if n == 16 {
            result.push('ט');
            result.push('ז');
            return result;
        }

        // Tens
        let tens_table = [
            (90, 'צ'),
            (80, 'פ'),
            (70, 'ע'),
            (60, 'ס'),
            (50, 'נ'),
            (40, 'מ'),
            (30, 'ל'),
            (20, 'כ'),
            (10, 'י'),
        ];
        for (val, ch) in tens_table {
            if n >= val {
                result.push(ch);
                n -= val;
                break;
            }
        }

        // Units
        let units_table = [
            (9, 'ט'),
            (8, 'ח'),
            (7, 'ז'),
            (6, 'ו'),
            (5, 'ה'),
            (4, 'ד'),
            (3, 'ג'),
            (2, 'ב'),
            (1, 'א'),
        ];
        for (val, ch) in units_table {
            if n >= val {
                result.push(ch);
                break;
            }
        }

        result
    }

    fn to_roman(mut n: usize) -> String {
        let lookup = [
            (1000, "M"),
            (900, "CM"),
            (500, "D"),
            (400, "CD"),
            (100, "C"),
            (90, "XC"),
            (50, "L"),
            (40, "XL"),
            (10, "X"),
            (9, "IX"),
            (5, "V"),
            (4, "IV"),
            (1, "I"),
        ];
        let mut out = String::new();
        for (val, sym) in lookup {
            while n >= val {
                out.push_str(sym);
                n -= val;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gematria_standard_numbers() {
        assert_eq!(
            NumberingFormatter::to_gematria(1, GematriaPunctuation::StandardPunctuation),
            "א׳"
        );
        assert_eq!(
            NumberingFormatter::to_gematria(2, GematriaPunctuation::StandardPunctuation),
            "ב׳"
        );
        assert_eq!(
            NumberingFormatter::to_gematria(10, GematriaPunctuation::StandardPunctuation),
            "י׳"
        );
        assert_eq!(
            NumberingFormatter::to_gematria(15, GematriaPunctuation::StandardPunctuation),
            "ט״ו"
        );
        assert_eq!(
            NumberingFormatter::to_gematria(16, GematriaPunctuation::StandardPunctuation),
            "ט״ז"
        );
        assert_eq!(
            NumberingFormatter::to_gematria(42, GematriaPunctuation::StandardPunctuation),
            "מ״ב"
        );
        assert_eq!(
            NumberingFormatter::to_gematria(100, GematriaPunctuation::StandardPunctuation),
            "ק׳"
        );
        assert_eq!(
            NumberingFormatter::to_gematria(5786, GematriaPunctuation::StandardPunctuation),
            "ה׳תשפ״ו"
        );
    }

    #[test]
    fn test_torah_schemes_formatting() {
        let siman_scheme = NumberingScheme::TorahSiman(GematriaPunctuation::StandardPunctuation);
        assert_eq!(NumberingFormatter::format(&siman_scheme, 1), "סימן א׳");
        assert_eq!(NumberingFormatter::format(&siman_scheme, 15), "סימן ט״ו");

        let seif_scheme = NumberingScheme::TorahSeif(GematriaPunctuation::StandardPunctuation);
        assert_eq!(NumberingFormatter::format(&seif_scheme, 3), "סעיף ג׳");

        let siman_plain = NumberingScheme::TorahSiman(GematriaPunctuation::PlainLetters);
        assert_eq!(NumberingFormatter::format(&siman_plain, 1), "סימן א");
        assert_eq!(NumberingFormatter::format(&siman_plain, 15), "סימן טו");
        assert_eq!(NumberingFormatter::format(&siman_plain, 16), "סימן טז");

        let seif_plain = NumberingScheme::TorahSeif(GematriaPunctuation::PlainLetters);
        assert_eq!(NumberingFormatter::format(&seif_plain, 2), "סעיף ב");

        let ot_scheme = NumberingScheme::TorahOt(GematriaPunctuation::StandardPunctuation);
        assert_eq!(NumberingFormatter::format(&ot_scheme, 4), "אות ד׳");

        let daf_scheme = NumberingScheme::TorahDafAmud(GematriaPunctuation::StandardPunctuation);
        assert_eq!(NumberingFormatter::format(&daf_scheme, 1), "דף ב׳ ע״א");
        assert_eq!(NumberingFormatter::format(&daf_scheme, 2), "דף ב׳ ע״ב");
        assert_eq!(NumberingFormatter::format(&daf_scheme, 3), "דף ג׳ ע״א");
    }

    #[test]
    fn test_roman_and_custom_formatting() {
        assert_eq!(
            NumberingFormatter::format(&NumberingScheme::RomanUpper, 14),
            "XIV"
        );
        assert_eq!(
            NumberingFormatter::format(&NumberingScheme::RomanLower, 4),
            "iv"
        );

        let custom = NumberingScheme::Custom {
            prefix: "ס\"ק ".to_string(),
            base: Box::new(NumberingScheme::HebrewGematria(
                GematriaPunctuation::StandardPunctuation,
            )),
            suffix: ")".to_string(),
        };
        assert_eq!(NumberingFormatter::format(&custom, 5), "ס\"ק ה׳)");
    }
}
