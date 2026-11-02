//! Interface texts in nine languages (Fluent files in `app/locales`, one
//! folder per locale tag, English as the fallback for missing texts).

use archeion_core::Language;
use fluent_templates::{LanguageIdentifier, Loader as _, fluent_bundle::FluentValue, static_loader};
use std::{borrow::Cow, collections::HashMap};

static_loader! {
    static LOCALES = {
        locales: "./locales",
        fallback_language: "en-US",
        // egui would draw Unicode isolation marks around arguments as boxes.
        customise: |bundle| bundle.set_use_isolating(false),
    };
}

/// Translator for the chosen language.
#[derive(Debug, Clone)]
pub struct Tr {
    lang: LanguageIdentifier,
    /// The language actually shown: never `Language::System`.
    language: Language,
}

impl Tr {
    /// Translator for `choice`; `Language::System` follows the Windows display
    /// language, or English when Archeion does not have it.
    pub fn new(choice: Language) -> Self {
        let language = match choice {
            Language::System => sys_locale::get_locale().map_or(Language::English, |l| from_locale(&l)),
            chosen => chosen,
        };
        Self { lang: tag(language).parse().expect("valid language tag"), language }
    }

    /// Text for `id`.
    pub fn t(&self, id: &str) -> String {
        LOCALES.lookup(&self.lang, id)
    }

    /// Text for `id` with named arguments.
    pub fn t_args(&self, id: &str, args: &[(&'static str, FluentValue<'static>)]) -> String {
        let map: HashMap<Cow<'static, str>, FluentValue<'static>> =
            args.iter().map(|(k, v)| (Cow::Borrowed(*k), v.clone())).collect();
        LOCALES.lookup_with_args(&self.lang, id, &map)
    }

    /// Byte count with a binary unit, decimal separator of the language.
    pub fn bytes(&self, n: u64) -> String {
        const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
        #[expect(clippy::cast_precision_loss, reason = "display only")]
        let mut v = n as f64;
        let mut unit = 0;
        while v >= 1024.0 && unit + 1 < UNITS.len() {
            v /= 1024.0;
            unit += 1;
        }
        let text = if unit == 0 { format!("{n}") } else { format!("{v:.1}") };
        let text = if decimal_comma(self.language) { text.replace('.', ",") } else { text };
        format!("{text} {}", UNITS.get(unit).copied().unwrap_or("B"))
    }
}

/// Locale tag of a language: the name of its folder in `app/locales`.
const fn tag(language: Language) -> &'static str {
    match language {
        Language::System | Language::English => "en-US",
        Language::Italian => "it-IT",
        Language::Chinese => "zh-CN",
        Language::Hindi => "hi-IN",
        Language::Spanish => "es-ES",
        Language::French => "fr-FR",
        Language::Japanese => "ja-JP",
        Language::Portuguese => "pt-BR",
        Language::Russian => "ru-RU",
    }
}

/// The language matching a system locale such as `it-IT` or `zh-Hans-CN`,
/// compared by primary language only; English when none matches.
fn from_locale(locale: &str) -> Language {
    let primary = locale.split(['-', '_']).next().unwrap_or_default().to_ascii_lowercase();
    Language::ALL
        .into_iter()
        .find(|&l| tag(l).split('-').next() == Some(primary.as_str()))
        .unwrap_or(Language::English)
}

/// Whether the language writes 1,5 rather than 1.5.
const fn decimal_comma(language: Language) -> bool {
    matches!(
        language,
        Language::Italian | Language::Spanish | Language::French | Language::Portuguese | Language::Russian
    )
}

/// Shorthand for a Fluent argument.
pub fn arg(v: impl Into<FluentValue<'static>>) -> FluentValue<'static> {
    v.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Every locale file, by tag.
    const LOCALE_FILES: [(&str, &str); 9] = [
        ("en-US", include_str!("../locales/en-US/main.ftl")),
        ("it-IT", include_str!("../locales/it-IT/main.ftl")),
        ("zh-CN", include_str!("../locales/zh-CN/main.ftl")),
        ("hi-IN", include_str!("../locales/hi-IN/main.ftl")),
        ("es-ES", include_str!("../locales/es-ES/main.ftl")),
        ("fr-FR", include_str!("../locales/fr-FR/main.ftl")),
        ("ja-JP", include_str!("../locales/ja-JP/main.ftl")),
        ("pt-BR", include_str!("../locales/pt-BR/main.ftl")),
        ("ru-RU", include_str!("../locales/ru-RU/main.ftl")),
    ];

    /// Message id → the `$variables` its text uses, continuation lines
    /// (select expressions, multiline values) included.
    fn messages(src: &str) -> HashMap<&str, BTreeSet<&str>> {
        let mut out: HashMap<&str, BTreeSet<&str>> = HashMap::new();
        let mut current = None;
        for line in src.lines() {
            if !line.starts_with([' ', '\t']) {
                current = if line.starts_with('#') { None } else { line.split_once(" =").map(|(id, _)| id) };
            }
            let Some(id) = current else { continue };
            let vars = out.entry(id).or_default();
            for after in line.split('$').skip(1) {
                let name = after
                    .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
                    .next()
                    .unwrap_or_default();
                if !name.is_empty() {
                    vars.insert(name);
                }
            }
        }
        out
    }

    #[test]
    fn every_english_message_exists_in_every_locale_with_the_same_variables() {
        let en = messages(LOCALE_FILES[0].1);
        for (tag, src) in &LOCALE_FILES[1..] {
            let other = messages(src);
            let missing: Vec<_> = en.keys().filter(|k| !other.contains_key(*k)).collect();
            assert!(missing.is_empty(), "missing in {tag}: {missing:?}");
            let differing: Vec<_> = en.iter().filter(|(k, vars)| other.get(*k) != Some(*vars)).collect();
            assert!(differing.is_empty(), "variables differ in {tag}: {differing:?}");
        }
    }

    #[test]
    fn every_language_has_its_own_locale_file() {
        let tags: Vec<_> = LOCALE_FILES.iter().map(|(t, _)| *t).collect();
        for language in Language::ALL {
            assert!(tags.contains(&tag(language)), "no locale file for {language:?}");
            let text = Tr::new(language).t("language");
            assert!(!text.is_empty(), "{language:?}");
            // English says "Language": any other text proves the file was loaded and parsed.
            assert_eq!(text == "Language", language == Language::English, "{language:?}: {text}");
        }
    }

    #[test]
    fn system_locale_picks_the_language_by_prefix() {
        assert_eq!(from_locale("it-IT"), Language::Italian);
        assert_eq!(from_locale("zh-Hans-CN"), Language::Chinese);
        assert_eq!(from_locale("pt_PT"), Language::Portuguese);
        assert_eq!(from_locale("JA"), Language::Japanese);
        assert_eq!(from_locale("ru"), Language::Russian);
        assert_eq!(from_locale("de-DE"), Language::English);
        assert_eq!(from_locale(""), Language::English);
    }

    #[test]
    fn arguments_are_substituted() {
        let tr = Tr::new(Language::English);
        let s = tr.t_args("progress-of", &[("done", arg("1 KB")), ("total", arg("2 KB"))]);
        assert_eq!(s, "1 KB of 2 KB");
    }

    #[test]
    fn plurals_follow_the_language() {
        let tr = Tr::new(Language::Russian);
        assert_eq!(tr.t_args("results", &[("count", arg(21))]), "Найден 21 файл");
        assert_eq!(tr.t_args("results", &[("count", arg(3))]), "Найдено 3 файла");
        assert_eq!(tr.t_args("results", &[("count", arg(5))]), "Найдено 5 файлов");
    }

    #[test]
    fn bytes_use_the_language_separator() {
        assert_eq!(Tr::new(Language::English).bytes(1536), "1.5 KB");
        assert_eq!(Tr::new(Language::Italian).bytes(1536), "1,5 KB");
        assert_eq!(Tr::new(Language::Italian).bytes(12), "12 B");
        assert_eq!(Tr::new(Language::French).bytes(1536), "1,5 KB");
        assert_eq!(Tr::new(Language::Japanese).bytes(1536), "1.5 KB");
    }
}
