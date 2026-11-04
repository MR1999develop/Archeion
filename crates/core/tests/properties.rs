//! Property tests of the readers of text that comes from outside the
//! program: manifests (from a destination), keys (from manifests), worker
//! lines, typed exclusions. On any input they answer or refuse, never
//! panic, and what they accept keeps its promises.

use archeion_core::{
    ArchiveKind, Exclusion, Manifest, RunEvent, TypedKind, keys,
    manifest::{Entry, EntryKind},
};
use proptest::prelude::*;
use std::path::{Component, Path};

/// A file or folder name a real path can have.
fn segment() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 _ü!()-][a-zA-Z0-9 ._ü!()-]{0,11}".prop_filter("not . or ..", |s| s != "." && s != "..")
}

fn key() -> impl Strategy<Value = String> {
    (prop::sample::select(vec!["C", "D", "ROOT", "UNC/server/share"]), prop::collection::vec(segment(), 1..5))
        .prop_map(|(root, segs)| format!("{root}/{}", segs.join("/")))
}

/// Keys a doctored manifest could hold: real segments mixed with the ones
/// that climb out or jump elsewhere, or any text at all.
fn hostile_key() -> impl Strategy<Value = String> {
    let piece = prop_oneof![
        segment(),
        prop::sample::select(vec![
            "..",
            ".",
            "",
            "C:",
            "c:",
            "C:\\x",
            "a\\..",
            "\\\\srv\\s",
            "/",
            "a\0b",
            "..\\..",
            "D:..",
            "CON",
            "x:y",
        ])
        .prop_map(str::to_owned),
        "\\PC{0,8}",
    ];
    prop_oneof![prop::collection::vec(piece, 1..6).prop_map(|p| p.join("/")), "\\PC{0,60}",]
}

fn entry() -> impl Strategy<Value = Entry> {
    (key(), any::<u32>(), any::<i64>(), any::<bool>(), any::<bool>()).prop_map(
        |(key, size, mtime_ns, stored, excluded)| Entry {
            key,
            kind: EntryKind::File,
            size: u64::from(size),
            mtime_ns,
            hash: Some("ab".repeat(32)),
            stored,
            blob: None,
            chunks: None,
            excluded,
        },
    )
}

fn config() -> ProptestConfig {
    // A failure prints its smallest input; no regression files in the tree.
    ProptestConfig { cases: 256, failure_persistence: None, ..ProptestConfig::default() }
}

proptest! {
    #![proptest_config(config())]

    /// Any text: a manifest or a refusal, and every key of an accepted one
    /// is one a path gives.
    #[test]
    fn any_text_is_a_manifest_or_refused(text in "\\PC{0,400}") {
        if let Ok(m) = Manifest::from_text(&text) {
            for e in &m.entries {
                prop_assert!(keys::check(&e.key).is_ok(), "{:?}", e.key);
            }
        }
    }

    /// A manifest written and read back is the same, sorted.
    #[test]
    fn manifests_round_trip(entries in prop::collection::vec(entry(), 0..20)) {
        let mut m = Manifest::new(ArchiveKind::Incremental, entries);
        m.normalize();
        let back = Manifest::from_text(&m.to_text()).unwrap();
        prop_assert_eq!(back.entries, m.entries);
    }

    /// A manifest whose keys a doctored destination chose is read only
    /// when every key is one a path gives.
    #[test]
    fn manifests_with_hostile_keys_are_refused(
        entries in prop::collection::vec(entry(), 1..6),
        keys in prop::collection::vec(hostile_key(), 1..6),
    ) {
        let entries: Vec<Entry> = entries.into_iter().zip(keys).map(|(e, key)| Entry { key, ..e }).collect();
        let all_safe = entries.iter().all(|e| keys::check(&e.key).is_ok());
        let text = Manifest::new(ArchiveKind::Complete, entries).to_text();
        prop_assert_eq!(Manifest::from_text(&text).is_ok(), all_safe);
    }

    /// A real manifest with bytes changed, lines dropped or repeated: read
    /// or refused, never a panic, and the keys stay checked.
    #[test]
    fn damaged_manifests_are_refused_or_safe(
        entries in prop::collection::vec(entry(), 1..10),
        cuts in prop::collection::vec((any::<prop::sample::Index>(), any::<u8>()), 1..6),
    ) {
        let text = Manifest::new(ArchiveKind::Complete, entries).to_text();
        let mut bytes = text.into_bytes();
        for (at, b) in cuts {
            let i = at.index(bytes.len());
            bytes[i] = b;
        }
        let text = String::from_utf8_lossy(&bytes);
        if let Ok(m) = Manifest::from_text(&text) {
            for e in &m.entries {
                prop_assert!(keys::check(&e.key).is_ok(), "{:?}", e.key);
            }
        }
    }

    /// Any key that passes the check stays below the folder it is restored
    /// into, one plain name per segment.
    #[test]
    fn checked_keys_stay_below_their_folder(text in hostile_key()) {
        if keys::check(&text).is_ok() {
            let base = Path::new("restore here");
            let under = keys::path_under(base, &text);
            let rest = under.strip_prefix(base).unwrap();
            prop_assert!(rest.components().all(|c| matches!(c, Component::Normal(_))), "{:?}", text);
            prop_assert_eq!(rest.components().count(), text.split('/').count());
        }
    }

    /// Keys of generated paths always pass.
    #[test]
    fn generated_keys_pass(k in key()) {
        prop_assert!(keys::check(&k).is_ok(), "{k:?}");
    }

    /// Lines of a worker's output: an event or an error, never a panic.
    #[test]
    fn any_line_is_an_event_or_refused(line in "\\PC{0,300}") {
        let _ = RunEvent::from_line(&line);
    }

    /// Typed exclusions: a type never holds a separator or a wildcard, a
    /// pattern always has a wildcard or a separator.
    #[test]
    fn typed_rules_keep_their_kind(text in "\\PC{0,40}") {
        if let Ok(Exclusion::Extension { ext }) = Exclusion::typed(TypedKind::Extension, &text) {
            prop_assert!(!ext.is_empty() && !ext.contains(['/', '\\', '*', '?', ' ', '.']), "{ext:?}");
        }
        if let Ok(Exclusion::Pattern { glob }) = Exclusion::typed(TypedKind::Pattern, &text) {
            prop_assert!(glob.contains(['*', '?', '[', '/']), "{glob:?}");
        }
    }
}
