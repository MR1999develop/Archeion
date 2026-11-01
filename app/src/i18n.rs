//! Interface texts in English and Italian (Fluent files in `app/locales`).

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
    italian: bool,
}

impl Tr {
    pub fn new(choice: Language) -> Self {
        let italian = match choice {
            Language::Italian => true,
            Language::English => false,
            Language::System => sys_locale::get_locale().is_some_and(|l| l.starts_with("it")),
        };
        let tag = if italian { "it-IT" } else { "en-US" };
        Self { lang: tag.parse().expect("valid language tag"), italian }
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
        let text = if self.italian { text.replace('.', ",") } else { text };
        format!("{text} {}", UNITS.get(unit).copied().unwrap_or("B"))
    }
}

/// Shorthand for a Fluent argument.
pub fn arg(v: impl Into<FluentValue<'static>>) -> FluentValue<'static> {
    v.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_english_key_exists_in_italian() {
        let en = include_str!("../locales/en-US/main.ftl");
        let it = include_str!("../locales/it-IT/main.ftl");
        let keys = |s: &'static str| -> Vec<&'static str> {
            s.lines()
                .filter(|l| !l.starts_with([' ', '#', '\t']) && l.contains(" ="))
                .filter_map(|l| l.split(" =").next())
                .collect()
        };
        let it_keys = keys(it);
        let missing: Vec<_> = keys(en).into_iter().filter(|k| !it_keys.contains(k)).collect();
        assert!(missing.is_empty(), "missing in it-IT: {missing:?}");
    }

    #[test]
    fn arguments_are_substituted() {
        let tr = Tr::new(Language::English);
        let s = tr.t_args("progress-of", &[("done", arg("1 KB")), ("total", arg("2 KB"))]);
        assert_eq!(s, "1 KB of 2 KB");
    }

    #[test]
    fn bytes_use_the_language_separator() {
        assert_eq!(Tr::new(Language::English).bytes(1536), "1.5 KB");
        assert_eq!(Tr::new(Language::Italian).bytes(1536), "1,5 KB");
        assert_eq!(Tr::new(Language::Italian).bytes(12), "12 B");
    }
}
