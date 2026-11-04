//! What a profile leaves out of its backups.

use serde::{Deserialize, Serialize};
use std::{
    fmt,
    path::{Path, PathBuf},
};

/// One exclusion rule.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", from = "RawExclusion")]
pub enum Exclusion {
    /// A folder and everything in it.
    Folder { path: PathBuf },
    /// One file.
    File { path: PathBuf },
    /// Every file with this extension, without the dot, e.g. `tmp`.
    Extension { ext: String },
    /// A glob matched against the full path, e.g. `**/node_modules`.
    Pattern { glob: String },
}

/// Accepts the plain glob strings of early settings files.
#[derive(Deserialize)]
#[serde(untagged)]
enum RawExclusion {
    Text(String),
    Tagged(Tagged),
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Tagged {
    Folder { path: PathBuf },
    File { path: PathBuf },
    Extension { ext: String },
    Pattern { glob: String },
}

impl From<RawExclusion> for Exclusion {
    fn from(raw: RawExclusion) -> Self {
        match raw {
            RawExclusion::Text(glob) => Self::Pattern { glob },
            RawExclusion::Tagged(Tagged::Folder { path }) => Self::Folder { path },
            RawExclusion::Tagged(Tagged::File { path }) => Self::File { path },
            RawExclusion::Tagged(Tagged::Extension { ext }) => Self::Extension { ext },
            RawExclusion::Tagged(Tagged::Pattern { glob }) => Self::Pattern { glob },
        }
    }
}

/// What typed text is meant to be: the button pressed next to the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypedKind {
    /// "Exclude pattern": a pattern on the whole path.
    Pattern,
    /// "Exclude type": a file extension.
    Extension,
}

/// Why typed text is not the rule of the button pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TypedRuleError {
    #[error("nothing typed")]
    Empty,
    /// `.tmp` given as a pattern: it names a file type (as a pattern it
    /// would match only a path called `.tmp`).
    #[error("a file type, not a pattern")]
    TypeNotPattern,
    /// A pattern (`**/cache`, `*.tmp~`) given as a file type.
    #[error("a pattern, not a file type")]
    PatternNotType,
    /// A name alone (`node_modules`) as a pattern: patterns match the whole
    /// path, so it would match nothing; `**/node_modules` is what is meant.
    #[error("a name alone matches no path")]
    NameAlone,
    /// Not a file type: spaces or dots inside.
    #[error("not a file type")]
    BadType,
}

/// The rules a new profile starts with: temporary files and folders.
#[must_use]
pub fn defaults() -> Vec<Exclusion> {
    vec![
        Exclusion::Extension { ext: "tmp".into() },
        Exclusion::Extension { ext: "temp".into() },
        Exclusion::Pattern { glob: "**/tmp".into() },
        Exclusion::Pattern { glob: "**/temp".into() },
    ]
}

impl Exclusion {
    /// The rule `text` makes as `kind`, or why it does not; the pattern's
    /// syntax is checked where patterns are matched (the engine).
    ///
    /// # Errors
    /// [`TypedRuleError`]: [`TypedRuleError::TypeNotPattern`] and
    /// [`TypedRuleError::PatternNotType`] say the other button fits.
    pub fn typed(kind: TypedKind, text: &str) -> Result<Self, TypedRuleError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(TypedRuleError::Empty);
        }
        let wild = text.contains(['*', '?', '[', '/', '\\']);
        let as_type = Self::extension(text);
        match kind {
            TypedKind::Extension => match as_type {
                Some(rule) => Ok(rule),
                None if wild => Err(TypedRuleError::PatternNotType),
                None => Err(TypedRuleError::BadType),
            },
            TypedKind::Pattern if as_type.is_some() && text.starts_with('.') && !wild => {
                Err(TypedRuleError::TypeNotPattern)
            }
            TypedKind::Pattern if !wild => Err(TypedRuleError::NameAlone),
            TypedKind::Pattern => Ok(Self::Pattern { glob: text.replace('\\', "/") }),
        }
    }

    /// Extension rule from user text: `.TMP`, `tmp` and `*.tmp` all give `tmp`.
    /// Returns `None` for text with no usable extension.
    #[must_use]
    pub fn extension(text: &str) -> Option<Self> {
        let ext = text.trim().trim_start_matches('*').trim_start_matches('.').to_lowercase();
        let valid = !ext.is_empty() && !ext.contains(['/', '\\', '*', '?', ' ', '.']);
        valid.then_some(Self::Extension { ext })
    }

    /// True when the rule is empty and should be ignored.
    #[must_use]
    pub fn is_blank(&self) -> bool {
        match self {
            Self::Folder { path } | Self::File { path } => path.as_os_str().is_empty(),
            Self::Extension { ext } => ext.trim().is_empty(),
            Self::Pattern { glob } => glob.trim().is_empty(),
        }
    }
}

impl fmt::Display for Exclusion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Folder { path } | Self::File { path } => write!(f, "{}", path.display()),
            Self::Extension { ext } => write!(f, ".{ext}"),
            Self::Pattern { glob } => f.write_str(glob),
        }
    }
}

/// Text form used to compare paths: `/` separators, lower case (Windows
/// paths are case-insensitive).
#[must_use]
pub fn normalized(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/").trim_end_matches('/').to_lowercase()
}

/// Matcher for the folder, file and extension rules; patterns are matched
/// by the engine, which owns the glob library.
#[derive(Debug, Default, Clone)]
pub struct PathRules {
    folders: Vec<String>,
    files: Vec<String>,
    extensions: Vec<String>,
}

impl PathRules {
    #[must_use]
    pub fn new(rules: &[Exclusion]) -> Self {
        let mut r = Self::default();
        for rule in rules.iter().filter(|x| !x.is_blank()) {
            match rule {
                Exclusion::Folder { path } => r.folders.push(normalized(path)),
                Exclusion::File { path } => r.files.push(normalized(path)),
                Exclusion::Extension { ext } => r.extensions.push(ext.to_lowercase()),
                Exclusion::Pattern { .. } => {}
            }
        }
        r
    }

    /// True when `path` is excluded by a folder, file or extension rule.
    #[must_use]
    pub fn excludes(&self, path: &Path, is_dir: bool) -> bool {
        let p = normalized(path);
        let in_folder = self
            .folders
            .iter()
            .any(|f| p == *f || (p.starts_with(f.as_str()) && p.as_bytes().get(f.len()) == Some(&b'/')));
        if in_folder {
            return true;
        }
        if is_dir {
            return false;
        }
        if self.files.contains(&p) {
            return true;
        }
        let name = p.rsplit('/').next().unwrap_or(&p);
        name.rsplit_once('.').is_some_and(|(_, ext)| self.extensions.iter().any(|e| e == ext))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_text_must_be_the_rule_of_the_button_pressed() {
        use TypedKind::{Extension as Type, Pattern};
        assert_eq!(Exclusion::typed(Type, " .TMP "), Ok(Exclusion::Extension { ext: "tmp".into() }));
        assert_eq!(Exclusion::typed(Type, "*.bak"), Ok(Exclusion::Extension { ext: "bak".into() }));
        assert_eq!(Exclusion::typed(Type, "**/cache"), Err(TypedRuleError::PatternNotType));
        assert_eq!(Exclusion::typed(Type, "my file"), Err(TypedRuleError::BadType));
        assert_eq!(Exclusion::typed(Type, "  "), Err(TypedRuleError::Empty));
        assert_eq!(
            Exclusion::typed(Pattern, "**/node_modules"),
            Ok(Exclusion::Pattern { glob: "**/node_modules".into() })
        );
        assert_eq!(
            Exclusion::typed(Pattern, r"**\cache\**"),
            Ok(Exclusion::Pattern { glob: "**/cache/**".into() })
        );
        assert_eq!(Exclusion::typed(Pattern, "*.tmp"), Ok(Exclusion::Pattern { glob: "*.tmp".into() }));
        assert_eq!(Exclusion::typed(Pattern, ".tmp"), Err(TypedRuleError::TypeNotPattern));
        assert_eq!(Exclusion::typed(Pattern, "node_modules"), Err(TypedRuleError::NameAlone));
    }

    #[test]
    fn new_profiles_leave_out_temporary_files_and_folders() {
        let r = PathRules::new(&defaults());
        assert!(r.excludes(Path::new(r"C:\Users\Me\Docs\x.TMP"), false));
        assert!(r.excludes(Path::new(r"C:\Users\Me\Docs\y.temp"), false));
        assert!(defaults().contains(&Exclusion::Pattern { glob: "**/temp".into() }));
    }

    fn rules() -> PathRules {
        PathRules::new(&[
            Exclusion::Folder { path: r"C:\Users\Me\Docs\Temp".into() },
            Exclusion::File { path: r"C:\Users\Me\Docs\big.iso".into() },
            Exclusion::extension(".TMP").unwrap(),
        ])
    }

    #[test]
    fn folder_rule_covers_the_folder_and_its_content_only() {
        let r = rules();
        assert!(r.excludes(Path::new(r"C:\users\me\docs\temp"), true));
        assert!(r.excludes(Path::new(r"C:\Users\Me\Docs\Temp\a\b.txt"), false));
        assert!(!r.excludes(Path::new(r"C:\Users\Me\Docs\Temporary\b.txt"), false));
    }

    #[test]
    fn file_and_extension_rules_ignore_case() {
        let r = rules();
        assert!(r.excludes(Path::new(r"c:\users\me\docs\BIG.ISO"), false));
        assert!(r.excludes(Path::new(r"C:\x\cache.Tmp"), false));
        assert!(!r.excludes(Path::new(r"C:\x\tmp"), false), "no extension");
        assert!(!r.excludes(Path::new(r"C:\x\a.tmp"), true), "folders named like a type stay");
    }

    #[test]
    fn extension_text_is_cleaned() {
        for t in ["tmp", ".tmp", "*.TMP", " .Tmp "] {
            assert_eq!(Exclusion::extension(t), Some(Exclusion::Extension { ext: "tmp".into() }));
        }
        for t in ["", ".", "a b", "tar.gz", "*"] {
            assert_eq!(Exclusion::extension(t), None, "{t:?}");
        }
    }

    #[test]
    fn old_string_rules_load_as_patterns() {
        let v: Vec<Exclusion> =
            serde_json::from_str(r#"["*.bak", {"kind":"extension","ext":"iso"}]"#).unwrap();
        assert_eq!(v[0], Exclusion::Pattern { glob: "*.bak".into() });
        assert_eq!(v[1], Exclusion::Extension { ext: "iso".into() });
    }

    #[test]
    fn rules_round_trip() {
        let v = vec![
            Exclusion::Folder { path: r"C:\a".into() },
            Exclusion::Pattern { glob: "**/node_modules".into() },
        ];
        let text = serde_json::to_string(&v).unwrap();
        assert_eq!(serde_json::from_str::<Vec<Exclusion>>(&text).unwrap(), v);
    }
}
