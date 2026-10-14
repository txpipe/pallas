//! Genesis fields written under `extraConfig` instead of at the top level.
//!
//! Current tooling writes a chain's starting funds, pools and delegations
//! under `extraConfig` and leaves the old top level fields empty. A reader
//! that only looks at the top level sees an empty chain and no error. The
//! shapes and the rule for choosing between the two places are the same in
//! the shelley and conway files, so they live here.

use pallas_crypto::hash::{Hash, Hasher};
use serde::{Deserialize, Deserializer, de::DeserializeOwned};
use std::{collections::HashMap, path::PathBuf, str::FromStr};

/// The raw keys of one `extraConfig` entry, before it is checked.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
// serde would otherwise require `T: Default` because of the defaulted
// fields, but `Option<T>` defaults to `None` for any `T`.
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
struct InjectionRaw<T> {
    #[serde(default)]
    data: Option<T>,
    #[serde(default)]
    file: Option<Vec<String>>,
    #[serde(default)]
    hash: Option<String>,
}

/// One `extraConfig` entry.
///
/// It is one of three things: the data written inline, a pointer to a file
/// holding the data, or nothing. Confusing them gives a genesis that parses
/// cleanly and is wrong.
#[derive(Debug, Clone)]
pub enum Injection<T> {
    /// The data is written inline under `data`.
    Embedded(T),

    /// The data is in a separate file, given as path segments and the hash
    /// the file must have.
    FromFile { file: Vec<String>, hash: String },

    /// No `data` and no `file`, so this entry injects nothing.
    Absent,
}

impl<T> TryFrom<InjectionRaw<T>> for Injection<T> {
    type Error = String;

    fn try_from(raw: InjectionRaw<T>) -> Result<Self, Self::Error> {
        match (raw.data, raw.file) {
            (Some(_), Some(_)) => {
                Err("an injection names both a data payload and a file".to_string())
            }
            (Some(data), None) => Ok(Self::Embedded(data)),
            (None, Some(file)) => {
                let hash = raw
                    .hash
                    .ok_or_else(|| "an injection file is named without its hash".to_string())?;

                Ok(Self::FromFile { file, hash })
            }
            (None, None) => Ok(Self::Absent),
        }
    }
}

impl<'de, T> Deserialize<'de> for Injection<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = InjectionRaw::<T>::deserialize(deserializer)?;

        Self::try_from(raw).map_err(serde::de::Error::custom)
    }
}

/// Where an injection file may be read from.
#[derive(Debug, Clone)]
pub(crate) enum Source {
    /// No file may be read, so a file injection is refused.
    NoFilesystem,

    /// Injection files are read under the shelley genesis file's directory,
    /// which the node mounts for every era.
    Directory(PathBuf),
}

impl Source {
    /// Read and parse one injection file, hashing its bytes and not the parsed data.
    fn read<K, V>(
        &self,
        injected_name: &str,
        file: &[String],
        hash: &str,
    ) -> Result<HashMap<K, V>, String>
    where
        K: std::hash::Hash + Eq + DeserializeOwned,
        V: DeserializeOwned,
    {
        let directory = match self {
            Self::NoFilesystem => {
                return Err(format!(
                    "extraConfig.{injected_name} names an injection file ({}), which cannot be read while parsing",
                    file.join("/")
                ));
            }
            Self::Directory(directory) => directory,
        };

        for segment in file {
            let names_one_entry = !segment.is_empty()
                && segment != "."
                && segment != ".."
                && !std::path::Path::new(segment).is_absolute()
                && !segment.contains('/')
                && !segment.contains('\\');

            if !names_one_entry {
                return Err(format!(
                    "extraConfig.{injected_name} names the path segment {segment:?}, which is not one file or directory name"
                ));
            }
        }

        let expected = Hash::<32>::from_str(hash).map_err(|_| {
            format!(
                "extraConfig.{injected_name} names the hash {hash}, which is not a blake2b-256 hash"
            )
        })?;

        let path = file
            .iter()
            .fold(directory.clone(), |path, segment| path.join(segment));
        let named = path.display();

        let bytes = std::fs::read(&path).map_err(|err| {
            format!("the injection file {named} that extraConfig.{injected_name} names cannot be read ({err})")
        })?;

        let found = Hasher::<256>::hash(&bytes);
        if found != expected {
            return Err(format!(
                "the injection file {named} hashes to {found}, not the {expected} that extraConfig.{injected_name} names"
            ));
        }

        serde_json::from_slice(&bytes).map_err(|err| {
            format!("the injection file {named} does not hold the map extraConfig.{injected_name} stands for ({err})")
        })
    }
}

/// Pick which of the two places a genesis field was written in.
///
/// The top level is used when there is no injection, and the injection
/// otherwise. Both sides holding entries is refused before any file is read.
///
/// `injected_name` is the key under `extraConfig` and `top_level_name` is
/// the field it replaces. Both are only used in error messages.
pub(crate) fn resolve<K, V>(
    source: &Source,
    injected_name: &str,
    top_level_name: &str,
    injection: Option<Injection<HashMap<K, V>>>,
    top_level: Option<HashMap<K, V>>,
) -> Result<Option<HashMap<K, V>>, String>
where
    K: std::hash::Hash + Eq + DeserializeOwned,
    V: DeserializeOwned,
{
    match injection {
        None | Some(Injection::Absent) => Ok(top_level),
        Some(Injection::Embedded(injected)) => {
            refuse_two_sources(injected_name, top_level_name, top_level.as_ref())?;

            Ok(Some(injected))
        }
        Some(Injection::FromFile { file, hash }) => {
            refuse_two_sources(injected_name, top_level_name, top_level.as_ref())?;

            source.read(injected_name, &file, &hash).map(Some)
        }
    }
}

fn refuse_two_sources<K, V>(
    injected_name: &str,
    top_level_name: &str,
    top_level: Option<&HashMap<K, V>>,
) -> Result<(), String> {
    match top_level {
        Some(top_level) if !top_level.is_empty() => Err(format!(
            "extraConfig.{injected_name} and {top_level_name} are both populated, so the genesis names two sources for one field"
        )),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Funds = HashMap<String, u64>;

    fn parse(json: &str) -> Result<Injection<Funds>, serde_json::Error> {
        serde_json::from_str(json)
    }

    fn funds(entries: &[(&str, u64)]) -> Funds {
        entries
            .iter()
            .map(|(key, value)| ((*key).to_string(), *value))
            .collect()
    }

    #[test]
    fn an_embedded_payload_is_read() {
        match parse(r#"{ "data": { "aa": 7 } }"#).expect("the injection must parse") {
            Injection::Embedded(payload) => assert_eq!(payload, funds(&[("aa", 7)])),
            other => panic!("expected an embedded payload, got {other:?}"),
        }
    }

    #[test]
    fn an_empty_object_names_no_injection() {
        match parse("{}").expect("the injection must parse") {
            Injection::Absent => {}
            other => panic!("expected no injection, got {other:?}"),
        }
    }

    #[test]
    fn a_file_arm_keeps_path_and_hash() {
        let json = r#"{ "file": ["genesis", "funds.json"], "hash": "abcd" }"#;

        match parse(json).expect("the injection must parse") {
            Injection::FromFile { file, hash } => {
                assert_eq!(file, vec!["genesis".to_string(), "funds.json".to_string()]);
                assert_eq!(hash, "abcd");
            }
            other => panic!("expected a file injection, got {other:?}"),
        }
    }

    #[test]
    fn a_malformed_payload_is_refused() {
        let err = parse(r#"{ "data": { "aa": "not a number" } }"#)
            .expect_err("a payload that does not parse must be refused");

        assert!(err.to_string().contains("invalid type"), "{err}");
    }

    #[test]
    fn both_arms_at_once_is_refused() {
        let json = r#"{ "data": { "aa": 7 }, "file": ["funds.json"], "hash": "abcd" }"#;

        let err = parse(json).expect_err("an injection with two sources must be refused");

        assert!(err.to_string().contains("both a data payload"), "{err}");
    }

    #[test]
    fn an_unknown_key_is_refused() {
        let err = parse(r#"{ "datum": { "aa": 7 } }"#)
            .expect_err("an injection with an unmodelled key must be refused");

        assert!(err.to_string().contains("unknown field"), "{err}");
        assert!(err.to_string().contains("datum"), "{err}");
    }

    #[test]
    fn a_file_arm_needs_its_hash() {
        let err = parse(r#"{ "file": ["funds.json"] }"#)
            .expect_err("a file injection with no hash must be refused");

        assert!(err.to_string().contains("without its hash"), "{err}");
    }

    #[test]
    fn no_injection_reads_the_top_level() {
        let top_level = funds(&[("aa", 7)]);

        let resolved = resolve(
            &Source::NoFilesystem,
            "initialFunds",
            "initialFunds",
            None,
            Some(top_level.clone()),
        )
        .expect("no injection must resolve");
        assert_eq!(resolved, Some(top_level.clone()));

        let resolved = resolve(
            &Source::NoFilesystem,
            "initialFunds",
            "initialFunds",
            Some(Injection::Absent),
            Some(top_level.clone()),
        )
        .expect("an absent injection must resolve");
        assert_eq!(resolved, Some(top_level));
    }

    #[test]
    fn an_injection_beats_an_empty_top_level() {
        let injected = funds(&[("bb", 9)]);

        let resolved = resolve(
            &Source::NoFilesystem,
            "initialFunds",
            "initialFunds",
            Some(Injection::Embedded(injected.clone())),
            Some(Funds::new()),
        )
        .expect("an injection against an empty map must resolve");
        assert_eq!(resolved, Some(injected.clone()));

        let resolved = resolve(
            &Source::NoFilesystem,
            "initialFunds",
            "initialFunds",
            Some(Injection::Embedded(injected.clone())),
            None,
        )
        .expect("an injection against a missing field must resolve");
        assert_eq!(resolved, Some(injected));
    }

    #[test]
    fn an_empty_payload_against_a_populated_top_level_is_refused() {
        let err = resolve(
            &Source::NoFilesystem,
            "initialFunds",
            "initialFunds",
            Some(Injection::Embedded(Funds::new())),
            Some(funds(&[("aa", 7)])),
        )
        .expect_err("an empty payload against a populated top level is two sources");

        assert!(err.contains("both populated"), "{err}");
    }

    #[test]
    fn two_empty_sources_stay_empty() {
        let resolved = resolve(
            &Source::NoFilesystem,
            "stakePools",
            "staking.pools",
            Some(Injection::Embedded(Funds::new())),
            Some(Funds::new()),
        )
        .expect("two empty sources must resolve");

        assert_eq!(resolved, Some(Funds::new()));
    }

    // Disjoint entries on purpose, so the refusal rests on both maps being
    // populated rather than on their keys clashing.
    #[test]
    fn both_sources_populated_is_refused() {
        let err = resolve(
            &Source::NoFilesystem,
            "initialFunds",
            "initialFunds",
            Some(Injection::Embedded(funds(&[("bb", 9)]))),
            Some(funds(&[("aa", 7)])),
        )
        .expect_err("a field with two sources must be refused");

        assert!(err.contains("initialFunds"), "{err}");
        assert!(err.contains("both populated"), "{err}");
    }

    #[test]
    fn the_text_only_path_refuses_a_file_arm() {
        let err = resolve(
            &Source::NoFilesystem,
            "stakePools",
            "staking.pools",
            Some(Injection::FromFile {
                file: vec!["genesis".to_string(), "pools.json".to_string()],
                hash: "abcd".to_string(),
            }),
            Some(Funds::new()),
        )
        .expect_err("an injection this cannot read must be refused");

        assert!(err.contains("stakePools"), "{err}");
        assert!(err.contains("genesis/pools.json"), "{err}");
    }

    const INJECTED_FILE: [&str; 2] = ["file-injection", "initial-funds.json"];
    const INJECTED_FILE_HASH: &str =
        "5f5ef4cb568ce42c470afcf6bfbca574ae345e1323b26be0de40d471db835ad7";

    fn test_data() -> PathBuf {
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("..")
            .join("test_data")
    }

    fn resolve_file(file: &[&str], hash: &str) -> Result<Option<Funds>, String> {
        resolve(
            &Source::Directory(test_data()),
            "initialFunds",
            "initialFunds",
            Some(Injection::FromFile {
                file: file.iter().map(|segment| (*segment).to_string()).collect(),
                hash: hash.to_string(),
            }),
            Some(Funds::new()),
        )
    }

    #[test]
    fn an_injection_file_is_read() {
        let resolved = resolve_file(&INJECTED_FILE, INJECTED_FILE_HASH)
            .expect("the injection file must resolve")
            .expect("the injection file must carry funds");

        assert_eq!(resolved.len(), 3);
        assert_eq!(
            resolved.get("6004d2cf712cfcaafb8bda85dc31baf3a35168d2e28029e0b56c562d37"),
            Some(&2_250_000_000_000),
        );
    }

    #[test]
    fn a_wrong_hash_is_refused() {
        let wrong = "0000000000000000000000000000000000000000000000000000000000000000";

        let err = resolve_file(&INJECTED_FILE, wrong)
            .expect_err("a file that does not hash to what the genesis names must be refused");

        assert!(err.contains("initial-funds.json"), "{err}");
        assert!(err.contains(INJECTED_FILE_HASH), "{err}");
        assert!(err.contains(wrong), "{err}");
    }

    #[test]
    fn a_missing_file_is_refused() {
        let err = resolve_file(&["file-injection", "absent.json"], INJECTED_FILE_HASH)
            .expect_err("a file that is not there must be refused");

        assert!(err.contains("absent.json"), "{err}");
        assert!(
            err.contains("that extraConfig.initialFunds names cannot be read"),
            "{err}"
        );
    }

    #[test]
    fn a_hash_that_is_not_a_hash_is_refused() {
        let err = resolve_file(&INJECTED_FILE, "abcd")
            .expect_err("an injection hash that is not a hash must be refused");

        assert!(err.contains("abcd"), "{err}");
        assert!(err.contains("blake2b-256"), "{err}");
    }

    #[test]
    fn a_file_holding_something_else_is_refused() {
        let file = ["file-injection-shelley-genesis.json"];
        let bytes = std::fs::read(test_data().join(file[0])).expect("the fixture must be there");
        let hash = Hasher::<256>::hash(&bytes).to_string();

        let err = resolve_file(&file, &hash)
            .expect_err("a file that does not hold the field's map must be refused");

        assert!(err.contains("file-injection-shelley-genesis.json"), "{err}");
        assert!(
            err.contains("does not hold the map extraConfig.initialFunds"),
            "{err}"
        );
    }

    #[test]
    fn a_file_arm_against_a_populated_top_level_is_refused() {
        let err = resolve(
            &Source::Directory(test_data()),
            "initialFunds",
            "initialFunds",
            Some(Injection::FromFile {
                file: INJECTED_FILE
                    .iter()
                    .map(|segment| (*segment).to_string())
                    .collect(),
                hash: INJECTED_FILE_HASH.to_string(),
            }),
            Some(funds(&[("aa", 7)])),
        )
        .expect_err("a field with two sources must be refused");

        assert!(err.contains("initialFunds"), "{err}");
        assert!(err.contains("both populated"), "{err}");
    }

    #[test]
    fn a_file_arm_against_a_populated_top_level_is_refused_before_the_read() {
        let err = resolve(
            &Source::Directory(test_data()),
            "initialFunds",
            "initialFunds",
            Some(Injection::FromFile {
                file: vec!["file-injection".to_string(), "absent.json".to_string()],
                hash: INJECTED_FILE_HASH.to_string(),
            }),
            Some(funds(&[("aa", 7)])),
        )
        .expect_err("a field with two sources must be refused");

        assert!(err.contains("both populated"), "{err}");
        assert!(!err.contains("cannot be read"), "{err}");
    }

    #[test]
    fn an_empty_segment_is_refused() {
        let err = resolve_file(
            &["", INJECTED_FILE[0], INJECTED_FILE[1]],
            INJECTED_FILE_HASH,
        )
        .expect_err("an empty segment must be refused");

        assert!(err.contains("initialFunds"), "{err}");
        assert!(err.contains(r#""""#), "{err}");
    }

    #[test]
    fn a_current_directory_segment_is_refused() {
        let err = resolve_file(
            &[".", INJECTED_FILE[0], INJECTED_FILE[1]],
            INJECTED_FILE_HASH,
        )
        .expect_err("a current directory segment must be refused");

        assert!(err.contains("initialFunds"), "{err}");
        assert!(err.contains(r#"".""#), "{err}");
    }

    #[test]
    fn a_parent_segment_is_refused() {
        let err = resolve_file(
            &["..", "test_data", INJECTED_FILE[0], INJECTED_FILE[1]],
            INJECTED_FILE_HASH,
        )
        .expect_err("a parent segment must be refused");

        assert!(err.contains("initialFunds"), "{err}");
        assert!(err.contains(r#""..""#), "{err}");
    }

    #[test]
    fn an_absolute_segment_is_refused() {
        let absolute = test_data()
            .join(INJECTED_FILE[0])
            .join(INJECTED_FILE[1])
            .display()
            .to_string();

        let err = resolve_file(&[&absolute], INJECTED_FILE_HASH)
            .expect_err("an absolute segment must be refused");

        assert!(err.contains("initialFunds"), "{err}");
        assert!(err.contains("not one file or directory name"), "{err}");
    }

    #[test]
    fn a_segment_holding_a_separator_is_refused() {
        let joined = INJECTED_FILE.join("/");

        let err = resolve_file(&[&joined], INJECTED_FILE_HASH)
            .expect_err("a segment holding a path separator must be refused");

        assert!(err.contains("initialFunds"), "{err}");
        assert!(err.contains("file-injection/initial-funds.json"), "{err}");
    }
}
