//! Requirements for a **new** password of an encrypted profile.
//!
//! [`assess`] checks a candidate password and returns one [`Check`] per
//! [`Rule`], in the order the window lists them, plus an estimated
//! [`Strength`]. The rules apply only when a password is chosen (first setup
//! or change), never when the password of existing backups is typed.
//!
//! The rules stay humane: a long passphrase is always accepted and there are
//! no composition tricks that push towards "Password1!".
//!
//! - at least [`MIN_CHARS`] characters, counted as Unicode characters;
//! - at least [`MIN_KINDS`] of the four kinds of characters (lowercase,
//!   uppercase, digits, symbols), unless the password is a passphrase of at
//!   least [`PASSPHRASE_CHARS`] characters;
//! - not one of the most common passwords, also with digits or symbols
//!   around it ("Password2024!");
//! - no run of more than [`MAX_RUN`] identical or consecutive characters
//!   ("aaaa", "1234", "dcba");
//! - not containing the profile name (when it has at least
//!   [`MIN_NAME_CHARS`] characters);
//! - an estimated strength of at least [`Strength::Fair`].
//!
//! The strength is a rough entropy estimate: the length times the bits of
//! the character pool in use, where characters inside runs, common
//! passwords or the profile name count for (almost) nothing. It is a guide,
//! not a guarantee.

/// Fewest characters of a new password.
pub const MIN_CHARS: usize = 12;
/// From this length the kinds of characters are not required.
pub const PASSPHRASE_CHARS: usize = 20;
/// Kinds of characters (out of four) required below [`PASSPHRASE_CHARS`].
pub const MIN_KINDS: usize = 3;
/// Longest accepted run of identical or consecutive characters.
pub const MAX_RUN: usize = 3;
/// Shorter profile names are not looked for in the password.
pub const MIN_NAME_CHARS: usize = 3;
/// Weakest strength accepted.
pub const MIN_STRENGTH: Strength = Strength::Fair;

/// Estimated bits from which a password is [`Strength::Fair`].
const FAIR_BITS: f64 = 60.0;
/// Estimated bits from which a password is [`Strength::Strong`].
const STRONG_BITS: f64 = 80.0;
/// Estimated bits from which a password is [`Strength::VeryStrong`].
const VERY_STRONG_BITS: f64 = 100.0;

/// Pool sizes of the kinds of characters, for the estimate.
const LOWER_POOL: f64 = 26.0;
const UPPER_POOL: f64 = 26.0;
const DIGIT_POOL: f64 = 10.0;
const SYMBOL_POOL: f64 = 33.0;
/// Spaces alone add little: they separate the words of a passphrase.
const SPACE_POOL: f64 = 1.0;
/// Added once when a character outside ASCII is used (accents, other scripts).
const NON_ASCII_POOL: f64 = 32.0;
/// What a character continuing a run is worth.
const RUN_CHAR_BITS: f64 = 1.0;

/// The most common passwords and keyboard walks, lowercase, at least four
/// characters each: English lists plus a few frequent Italian ones.
#[rustfmt::skip]
const COMMON: &[&str] = &[
    "123456", "123456789", "12345678", "1234567890", "1234567", "12345", "1234", "111111", "000000",
    "123123", "654321", "666666", "121212", "112233", "696969", "987654321", "123321", "7777777",
    "password", "passw0rd", "p@ssw0rd", "p@ssword", "pass", "admin", "administrator", "root", "user",
    "login", "welcome", "qwerty", "qwertz", "azerty", "qwertyuiop", "asdfgh", "asdfghjkl", "asdf",
    "zxcvbn", "zxcvbnm", "zxcv", "qazwsx", "1qaz2wsx", "1q2w3e4r", "1q2w3e", "abc123", "abcdef",
    "letmein", "iloveyou", "love", "princess", "monkey", "dragon", "master", "sunshine", "shadow",
    "football", "baseball", "soccer", "superman", "batman", "trustno1", "starwars", "whatever",
    "freedom", "secret", "michael", "jordan", "charlie", "donald", "jennifer", "hunter", "ashley",
    "thomas", "computer", "internet", "samsung", "google", "summer", "winter", "spring", "autumn",
    "hello", "flower", "cookie", "chocolate", "pokemon", "matrix", "killer", "access", "ninja",
    "mustang", "harley", "ranger", "buster", "tigger", "pepper", "ginger", "orange", "banana",
    "backup", "archeion", "changeme", "default", "guest", "test", "test123", "ciao", "ciaociao",
    "amore", "tiamo", "juventus", "napoli", "milan", "inter", "roma", "lazio", "forzainter",
    "forzajuve", "francesco", "alessandro", "andrea", "giuseppe", "giovanni", "antonio", 
    "tesoro", "stella", "martina", "giulia", "chiara", "federica", "lorenzo", "matteo", "italia",
    "mamma", "papa",
];

/// What a requirement asks of a new password.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    /// At least [`MIN_CHARS`] characters.
    Length,
    /// At least [`MIN_KINDS`] kinds of characters, or a passphrase of
    /// [`PASSPHRASE_CHARS`] characters.
    Variety,
    /// Not a common password, even with digits or symbols around it.
    NotCommon,
    /// No run of more than [`MAX_RUN`] identical or consecutive characters.
    NoRuns,
    /// Does not contain the profile name.
    NotName,
    /// Estimated strength at least [`MIN_STRENGTH`].
    Strength,
}

/// One requirement and whether the password meets it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Check {
    pub rule: Rule,
    pub met: bool,
}

/// How hard a password is to guess, from the entropy estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Strength {
    Weak,
    Fair,
    Strong,
    VeryStrong,
}

impl Strength {
    /// Level of an estimate in bits.
    fn of_bits(bits: f64) -> Self {
        if bits >= VERY_STRONG_BITS {
            Self::VeryStrong
        } else if bits >= STRONG_BITS {
            Self::Strong
        } else if bits >= FAIR_BITS {
            Self::Fair
        } else {
            Self::Weak
        }
    }
}

/// The verdict on a candidate password.
#[derive(Debug, Clone, PartialEq)]
pub struct Assessment {
    /// Unicode characters typed.
    pub chars: usize,
    /// Kinds of characters used, 0 to 4.
    pub kinds: usize,
    /// One check per [`Rule`], in display order.
    pub checks: Vec<Check>,
    /// Estimated entropy in bits, after the penalties.
    pub bits: f64,
    pub strength: Strength,
}

impl Assessment {
    /// Every requirement is met: the password can be saved.
    pub fn is_acceptable(&self) -> bool {
        self.checks.iter().all(|c| c.met)
    }

    /// The first requirement not met yet, in display order.
    pub fn first_unmet(&self) -> Option<Rule> {
        self.checks.iter().find(|c| !c.met).map(|c| c.rule)
    }

    /// The estimate as a fraction of [`Strength::VeryStrong`], 0 to 1, for a
    /// strength bar.
    #[expect(clippy::cast_possible_truncation, reason = "clamped to 0..=1: only precision is lost")]
    pub fn fraction(&self) -> f32 {
        (self.bits / VERY_STRONG_BITS).clamp(0.0, 1.0) as f32
    }
}

/// Checks `password` as a new password of the profile called `profile_name`.
pub fn assess(password: &str, profile_name: &str) -> Assessment {
    let chars: Vec<char> = password.chars().collect();
    let folded: Vec<char> = chars.iter().map(|&c| fold(c)).collect();
    let name: Vec<char> = profile_name.trim().chars().map(fold).collect();
    let name = (name.len() >= MIN_NAME_CHARS).then_some(name);

    let kinds = Kinds::of(&chars);
    let longest_run = runs(&folded).map(|(start, end)| end - start + 1).max().unwrap_or(0);
    let contains_name = name.as_deref().is_some_and(|n| find_all(&folded, n).next().is_some());
    let bits = estimate_bits(&folded, &kinds, name.as_deref());
    let strength = Strength::of_bits(bits);

    let met = |rule| match rule {
        Rule::Length => chars.len() >= MIN_CHARS,
        Rule::Variety => kinds.count() >= MIN_KINDS || chars.len() >= PASSPHRASE_CHARS,
        Rule::NotCommon => !is_common(&folded),
        Rule::NoRuns => longest_run <= MAX_RUN,
        Rule::NotName => !contains_name,
        Rule::Strength => strength >= MIN_STRENGTH,
    };
    let checks = [Rule::Length, Rule::Variety, Rule::NotCommon, Rule::NoRuns, Rule::NotName, Rule::Strength]
        .into_iter()
        .map(|rule| Check { rule, met: met(rule) })
        .collect();
    Assessment { chars: chars.len(), kinds: kinds.count(), checks, bits, strength }
}

/// Lowercase of a character when it is a single character, else itself.
fn fold(c: char) -> char {
    let mut lower = c.to_lowercase();
    match (lower.next(), lower.next()) {
        (Some(l), None) => l,
        _ => c,
    }
}

/// Which kinds of characters a password uses.
#[derive(Debug, Default)]
struct Kinds {
    lower: bool,
    upper: bool,
    digit: bool,
    symbol: bool,
    /// A symbol other than white space.
    punctuation: bool,
    non_ascii: bool,
}

impl Kinds {
    /// Letters without case (as in Chinese) count as lowercase; spaces,
    /// punctuation and emoji count as symbols.
    fn of(chars: &[char]) -> Self {
        let mut k = Self::default();
        for &c in chars {
            if c.is_uppercase() {
                k.upper = true;
            } else if c.is_alphabetic() {
                k.lower = true;
            } else if c.is_numeric() {
                k.digit = true;
            } else {
                k.symbol = true;
                k.punctuation |= !c.is_whitespace();
            }
            k.non_ascii |= !c.is_ascii();
        }
        k
    }

    fn count(&self) -> usize {
        [self.lower, self.upper, self.digit, self.symbol].into_iter().filter(|&b| b).count()
    }

    /// Bits of one character drawn from the pool in use.
    fn bits_per_char(&self) -> f64 {
        let pool = [
            (self.lower, LOWER_POOL),
            (self.upper, UPPER_POOL),
            (self.digit, DIGIT_POOL),
            (self.punctuation, SYMBOL_POOL),
            (self.symbol && !self.punctuation, SPACE_POOL),
            (self.non_ascii, NON_ASCII_POOL),
        ]
        .into_iter()
        .filter_map(|(used, size)| used.then_some(size))
        .sum::<f64>();
        if pool > 1.0 { pool.log2() } else { 0.0 }
    }
}

/// How one character follows the previous one in a run.
fn step(a: char, b: char) -> Option<i8> {
    if a == b {
        return Some(0);
    }
    let same_class =
        (a.is_ascii_digit() && b.is_ascii_digit()) || (a.is_ascii_alphabetic() && b.is_ascii_alphabetic());
    if !same_class {
        return None;
    }
    let (a, b) = (u32::from(a), u32::from(b));
    if b == a + 1 {
        Some(1)
    } else if a == b + 1 {
        Some(-1)
    } else {
        None
    }
}

/// Runs of at least two identical or consecutive characters, as
/// `(first, last)` indices. Neighbouring runs share their boundary
/// character.
fn runs(chars: &[char]) -> impl Iterator<Item = (usize, usize)> + '_ {
    let mut i = 0;
    std::iter::from_fn(move || {
        while i + 1 < chars.len() {
            let start = i;
            let Some(dir) = step(chars[i], chars[i + 1]) else {
                i += 1;
                continue;
            };
            let mut end = i + 1;
            while end + 1 < chars.len() && step(chars[end], chars[end + 1]) == Some(dir) {
                end += 1;
            }
            i = end;
            return Some((start, end));
        }
        None
    })
}

/// Start indices of the occurrences of `needle` in `hay`, not overlapping.
fn find_all<'a>(hay: &'a [char], needle: &'a [char]) -> impl Iterator<Item = usize> + 'a {
    let mut i = 0;
    std::iter::from_fn(move || {
        if needle.is_empty() {
            return None;
        }
        while i + needle.len() <= hay.len() {
            let at = i;
            if hay[at..].starts_with(needle) {
                i += needle.len();
                return Some(at);
            }
            i += 1;
        }
        None
    })
}

/// The whole password is a common one, possibly with digits, symbols or
/// spaces before or after it ("Password2024!", "123qwerty").
fn is_common(folded: &[char]) -> bool {
    let whole: String = folded.iter().collect();
    let core = whole.trim_matches(|c: char| !c.is_alphabetic());
    COMMON.iter().any(|&w| w == whole || (!core.is_empty() && w == core))
}

/// Longest common password starting at `at`, in characters.
fn common_at(folded: &[char], at: usize) -> Option<usize> {
    COMMON
        .iter()
        .map(|w| w.chars().collect::<Vec<char>>())
        .filter(|w| folded[at..].starts_with(w))
        .map(|w| w.len())
        .max()
}

/// Entropy estimate: bits per character of the pool in use, with characters
/// continuing runs worth [`RUN_CHAR_BITS`], those of a common password worth
/// nothing (each common password adds the bits of picking it from the
/// list), and those of the profile name worth nothing.
fn estimate_bits(folded: &[char], kinds: &Kinds, name: Option<&[char]>) -> f64 {
    let mut weight = vec![kinds.bits_per_char(); folded.len()];
    for (start, end) in runs(folded) {
        if end - start + 1 >= 3 {
            for w in &mut weight[start + 1..=end] {
                *w = w.min(RUN_CHAR_BITS);
            }
        }
    }
    let mut extra = 0.0;
    let mut i = 0;
    while i < folded.len() {
        match common_at(folded, i) {
            Some(len) => {
                weight[i..i + len].fill(0.0);
                extra += (COMMON.len() as f64).log2();
                i += len;
            }
            None => i += 1,
        }
    }
    if let Some(name) = name {
        for at in find_all(folded, name) {
            weight[at..at + name.len()].fill(0.0);
        }
    }
    weight.iter().sum::<f64>() + extra
}

#[cfg(test)]
mod tests {
    use super::*;

    fn met(password: &str, name: &str, rule: Rule) -> bool {
        assess(password, name).checks.iter().find(|c| c.rule == rule).is_some_and(|c| c.met)
    }

    #[test]
    fn checks_follow_the_display_order() {
        let rules: Vec<Rule> = assess("x", "").checks.iter().map(|c| c.rule).collect();
        assert_eq!(
            rules,
            [Rule::Length, Rule::Variety, Rule::NotCommon, Rule::NoRuns, Rule::NotName, Rule::Strength]
        );
    }

    #[test]
    fn length_counts_characters_not_bytes() {
        assert!(!met("Kx7#mQ2!vR9", "", Rule::Length), "11 characters");
        assert!(met("Kx7#mQ2!vR9z", "", Rule::Length), "12 characters");
        assert!(met("àèìòùéçñüöäß", "", Rule::Length), "12 characters, 24 bytes");
        assert!(!met("àèìòùéçñüöä", "", Rule::Length), "11 characters, 22 bytes");
    }

    #[test]
    fn variety_needs_three_kinds_below_twenty_characters() {
        assert_eq!(assess("onlylowercase", "").kinds, 1);
        assert!(!met("onlylowercase", "", Rule::Variety));
        assert!(!met("lower and spaces", "", Rule::Variety), "two kinds");
        assert!(met("Lower and spaces", "", Rule::Variety), "three kinds");
        assert!(met("Kx7mQ2vR9zpw", "", Rule::Variety), "lower, upper, digits");
    }

    #[test]
    fn a_passphrase_of_twenty_characters_needs_no_variety() {
        let nineteen = "gentle river stones";
        let twenty = "gentle river stonesx";
        assert_eq!(nineteen.chars().count(), 19);
        assert_eq!(twenty.chars().count(), 20);
        assert!(!met(nineteen, "", Rule::Variety));
        assert!(met(twenty, "", Rule::Variety));
        let a = assess("violet kettle on the windy hill", "");
        assert!(a.is_acceptable(), "{a:?}");
    }

    #[test]
    fn common_passwords_are_refused_with_digits_or_symbols_around() {
        for p in
            ["password", "Password2024", "PASSWORD2024!", "qwerty123456", "123qwertyuiop", "Juventus1897"]
        {
            assert!(!met(p, "", Rule::NotCommon), "{p}");
        }
        assert!(!met("123456789", "", Rule::NotCommon), "a common number");
        assert!(met("my password is long and silly", "", Rule::NotCommon), "contained, not the whole");
        assert!(met("Kx7#mQ2!vR9z", "", Rule::NotCommon));
    }

    #[test]
    fn common_words_inside_a_password_lower_its_strength() {
        let with = assess("Password12!x", "");
        let without = assess("Kx7#mQ2!vR9z", "");
        assert!(with.strength < without.strength, "{with:?} {without:?}");
        assert!(!with.is_acceptable());
    }

    #[test]
    fn runs_of_four_identical_or_consecutive_characters_are_refused() {
        for p in [
            "Kx7#aaaa!vR9z",
            "Kx7#1234!vRqz",
            "Kx7#abcd!vRqz",
            "Kx7#DCBA!vRqz",
            "Kx7#    !vRqz",
            "Kx7#AbCd!vRqz",
        ] {
            assert!(!met(p, "", Rule::NoRuns), "{p}");
        }
        for p in ["Kx7#aaa!vR9z", "Kx7#123!vRqz", "Kx7#abc!vRqz", "Kx7#a1b2c3!z", "Kx7#9:;<!vRz"] {
            assert!(met(p, "", Rule::NoRuns), "{p}");
        }
    }

    #[test]
    fn the_profile_name_is_not_allowed_inside() {
        assert!(!met("My Documents 2024!", "Documents", Rule::NotName));
        assert!(!met("xx DOCUMENTS yy 7", "  documents ", Rule::NotName), "trimmed, any case");
        assert!(met("My Documents 2024!", "Photos", Rule::NotName));
        assert!(met("Kx7#ab!vR9zq", "ab", Rule::NotName), "names under three characters are ignored");
        assert!(met("Kx7#mQ2!vR9z", "", Rule::NotName));
        assert!(!met("Città di Vacanze 9", "CITTÀ", Rule::NotName), "accents folded too");
    }

    #[test]
    fn accented_letters_are_letters_and_emoji_or_spaces_are_symbols() {
        let a = assess("àèì", "");
        assert_eq!(a.kinds, 1, "lowercase letters");
        assert_eq!(assess("ÀÈÌ", "").kinds, 1, "uppercase letters");
        assert_eq!(assess("àÈ", "").kinds, 2);
        assert_eq!(assess("a 🔑", "").kinds, 2, "space and emoji are symbols");
        assert_eq!(assess("a🔑", "").kinds, 2);
        assert_eq!(assess("a1", "").kinds, 2);
    }

    #[test]
    fn strength_grows_with_length_and_variety() {
        let levels: Vec<Strength> = ["", "abc", "Kx7#mQ2!vR9z", "Kx7#mQ2!vR9zLp", "Kx7#mQ2!vR9zLp5&"]
            .iter()
            .map(|p| assess(p, "").strength)
            .collect();
        assert_eq!(
            levels,
            [Strength::Weak, Strength::Weak, Strength::Fair, Strength::Strong, Strength::VeryStrong]
        );
        assert!(Strength::Weak < Strength::Fair && Strength::Strong < Strength::VeryStrong);
        assert!(assess("aaaaaaaaaaaaaaaaaaaaaaaa", "").strength == Strength::Weak, "a run is worth little");
        assert!(assess("passwordpasswordpassword", "").strength == Strength::Weak);
    }

    #[test]
    fn fraction_is_clamped() {
        assert_eq!(assess("", "").fraction(), 0.0);
        assert_eq!(assess("Kx7#mQ2!vR9zLp5&tW3?hN8Kx7#mQ2!vR9zLp5&tW3?hN8", "").fraction(), 1.0);
    }

    #[test]
    fn a_good_password_meets_everything() {
        let a = assess("Kx7#mQ2!vR9z", "Documents");
        assert!(a.is_acceptable(), "{a:?}");
        assert_eq!(a.first_unmet(), None);
        assert_eq!(assess("short", "").first_unmet(), Some(Rule::Length));
        assert_eq!(assess("onlylowercase", "").first_unmet(), Some(Rule::Variety));
    }
}
