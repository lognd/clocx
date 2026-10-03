//! Line totals: walk the tree, count each file with tokei (cached by content hash), group and diff.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use ignore::WalkBuilder;
use jiff::Timestamp;
use rayon::prelude::*;
use tokei::{Config, LanguageType};
use tracing::{debug, info, warn};

use crate::cache::{Counted, Counts, Snapshot, Stamp};
use crate::model::{CacheUse, LineCounts, Totals, TotalsRow};

/// A file the walk found, with its path relative to the root (forward slashes).
struct Found {
    rel: String,
    abs: PathBuf,
}

/// How one file's counts were obtained.
#[derive(Clone, Copy)]
enum Source {
    Unchanged,
    HashHit,
    Counted,
}

/// A file that could not be stat'ed or read; it is logged and left out.
struct Unreadable;

/// One counted file, ready to aggregate and to write back to the cache.
struct FileResult {
    rel: String,
    language: LanguageType,
    counted: Counted,
    stamp: Stamp,
    source: Source,
}

/// The result of one scan: report totals and the snapshot to save as the next baseline.
pub struct Scan {
    /// Totals for the report.
    pub totals: Totals,
    /// Baseline for the next run.
    pub snapshot: Snapshot,
}

/// Groups a relative file path by its first `depth` directories; root files are `.`.
pub fn dir_key(rel: &str, depth: u16) -> String {
    let parts: Vec<&str> = rel.split('/').collect();
    let dirs = &parts[..parts.len().saturating_sub(1)];
    if dirs.is_empty() {
        return ".".to_owned();
    }
    dirs[..dirs.len().min(depth as usize)].join("/")
}

/// The tokei settings every count uses: docstrings count as comments, as in cloc.
///
/// Documentation is comment, wherever it lives: Python docstrings here, and
/// Markdown prose (tokei's default for literate formats).
pub fn tokei_config() -> Config {
    Config {
        treat_doc_strings_as_comments: Some(true),
        ..Config::default()
    }
}

/// Lines that are nothing but a complete triple-quoted string (`"""Doc."""`), the one-line docstrings tokei misses.
fn one_line_docstrings(bytes: &[u8]) -> u64 {
    let text = String::from_utf8_lossy(bytes);
    text.lines()
        .filter(|line| {
            let body = line
                .trim()
                .trim_start_matches(['r', 'R', 'u', 'U', 'b', 'B']);
            ["\"\"\"", "'''"].iter().any(|q| {
                body.len() >= 6
                    && body.starts_with(q)
                    && body.ends_with(q)
                    && !body[3..body.len() - 3].contains(q)
            })
        })
        .count() as u64
}

/// Lists files under `root`, honouring .gitignore and .ignore; hidden files count (like `.github/`), `.git` never.
fn walk(root: &Path) -> Vec<Found> {
    let mut found = Vec::new();
    for entry in WalkBuilder::new(root)
        .hidden(false)
        .filter_entry(|e| e.file_name() != ".git")
        .require_git(false)
        .follow_links(false)
        .build()
    {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                warn!(error = %e, "skipping unwalkable entry");
                continue;
            }
        };
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let abs = entry.into_path();
        let Ok(rel) = abs.strip_prefix(root) else {
            continue;
        };
        let rel = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        found.push(Found { rel, abs });
    }
    found
}

/// Size and mtime of a file, the cheap part of its stamp.
fn stat(path: &Path) -> std::io::Result<(u64, i128)> {
    let meta = fs::metadata(path)?;
    let mtime_ns = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos() as i128);
    Ok((meta.len(), mtime_ns))
}

/// Counts one file, reusing the cache by stamp, then by content hash; `None` if not code or unreadable.
fn count_file(
    found: &Found,
    config: &Config,
    cache: &Counts,
) -> Option<Result<FileResult, Unreadable>> {
    let language = LanguageType::from_path(&found.abs, config)?;
    let (size, mtime_ns) = match stat(&found.abs) {
        Ok(s) => s,
        Err(e) => {
            warn!(path = %found.rel, error = %e, "cannot stat; skipped");
            return Some(Err(Unreadable));
        }
    };
    if let Some(stamp) = cache.by_path.get(&found.rel)
        && stamp.size == size
        && stamp.mtime_ns == mtime_ns
        && let Some(counted) = cache.by_key.get(&stamp.key)
    {
        return Some(Ok(FileResult {
            rel: found.rel.clone(),
            language,
            counted: *counted,
            stamp: stamp.clone(),
            source: Source::Unchanged,
        }));
    }
    let bytes = match fs::read(&found.abs) {
        Ok(b) => b,
        Err(e) => {
            warn!(path = %found.rel, error = %e, "cannot read; skipped");
            return Some(Err(Unreadable));
        }
    };
    let key = format!(
        "{:016x}:{}",
        xxhash_rust::xxh3::xxh3_64(&bytes),
        language.name()
    );
    let (counted, source) = match cache.by_key.get(&key) {
        Some(c) => (*c, Source::HashHit),
        None => {
            let stats = language.parse_from_slice(&bytes, config).summarise();
            let mut c = Counted {
                code: stats.code as u64,
                comments: stats.comments as u64,
                blanks: stats.blanks as u64,
            };
            if language == LanguageType::Python {
                // tokei counts a docstring opened and closed on one line as code; move those to comments.
                let n = one_line_docstrings(&bytes).min(c.code);
                c.code -= n;
                c.comments += n;
            }
            (c, Source::Counted)
        }
    };
    Some(Ok(FileResult {
        rel: found.rel.clone(),
        language,
        counted,
        stamp: Stamp {
            size,
            mtime_ns,
            key,
        },
        source,
    }))
}

/// Turns grouped counts into rows sorted by code (then name), with deltas against `before`.
///
/// Groups present before but gone now appear as empty rows with a negative delta.
fn rows(
    groups: BTreeMap<String, LineCounts>,
    before: Option<&BTreeMap<String, u64>>,
) -> Vec<TotalsRow> {
    let mut rows: Vec<TotalsRow> = groups
        .iter()
        .map(|(name, counts)| TotalsRow {
            name: name.clone(),
            counts: *counts,
            code_delta: before
                .map(|b| counts.code as i64 - b.get(name).copied().unwrap_or(0) as i64),
        })
        .collect();
    if let Some(before) = before {
        for (name, code) in before
            .iter()
            .filter(|(n, c)| **c > 0 && !groups.contains_key(*n))
        {
            rows.push(TotalsRow {
                name: name.clone(),
                counts: LineCounts::default(),
                code_delta: Some(-(*code as i64)),
            });
        }
    }
    rows.sort_by(|a, b| {
        b.counts
            .code
            .cmp(&a.counts.code)
            .then_with(|| a.name.cmp(&b.name))
    });
    rows
}

/// Scans `root`, updating `cache` in place (pruned to the files seen) and diffing against `baseline`.
pub fn scan(
    root: &Path,
    depth: u16,
    cache: &mut Counts,
    baseline: Option<&Snapshot>,
    now: Timestamp,
) -> Scan {
    let config = tokei_config();
    let found = walk(root);
    debug!(files = found.len(), "walked tree");

    let results: Vec<Result<FileResult, Unreadable>> = found
        .par_iter()
        .filter_map(|f| count_file(f, &config, cache))
        .collect();

    let mut usage = CacheUse::default();
    let mut by_lang: BTreeMap<String, LineCounts> = BTreeMap::new();
    let mut by_dir: BTreeMap<String, LineCounts> = BTreeMap::new();
    let mut total = LineCounts::default();
    let mut by_key = HashMap::new();
    let mut by_path = HashMap::new();
    for result in results {
        let Ok(file) = result else {
            usage.unreadable += 1;
            continue;
        };
        match file.source {
            Source::Unchanged => usage.unchanged += 1,
            Source::HashHit => usage.hash_hits += 1,
            Source::Counted => usage.counted += 1,
        }
        let Counted {
            code,
            comments,
            blanks,
        } = file.counted;
        by_lang
            .entry(file.language.name().to_owned())
            .or_default()
            .add_file(code, comments, blanks);
        by_dir
            .entry(dir_key(&file.rel, depth))
            .or_default()
            .add_file(code, comments, blanks);
        total.add_file(code, comments, blanks);
        by_key.insert(file.stamp.key.clone(), file.counted);
        by_path.insert(file.rel, file.stamp);
    }
    cache.by_key = by_key;
    cache.by_path = by_path;
    info!(?usage, files = total.files, code = total.code, "scan done");

    let lang_before = baseline.map(|b| &b.languages);
    let dir_before = baseline
        .filter(|b| b.depth == depth)
        .map(|b| &b.directories);
    let snapshot = Snapshot::new(
        now,
        depth,
        by_lang.iter().map(|(k, v)| (k.clone(), v.code)).collect(),
        by_dir.iter().map(|(k, v)| (k.clone(), v.code)).collect(),
        total.code,
    );
    let totals = Totals {
        depth,
        languages: rows(by_lang, lang_before),
        directories: rows(by_dir, dir_before),
        total: TotalsRow {
            name: "Total".to_owned(),
            counts: total,
            code_delta: baseline.map(|b| total.code as i64 - b.total as i64),
        },
        baseline_at: baseline.and_then(|b| b.taken_at),
        cache: usage,
    };
    Scan { totals, snapshot }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, text: &str) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }

    fn row<'a>(rows: &'a [TotalsRow], name: &str) -> &'a TotalsRow {
        rows.iter()
            .find(|r| r.name == name)
            .unwrap_or_else(|| panic!("no row {name}"))
    }

    fn fixture() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let r = tmp.path();
        write(
            r,
            "src/main.rs",
            "// entry\nfn main() {\n\n    let x = 1;\n}\n",
        );
        write(r, "src/deep/util.rs", "fn util() {}\n");
        write(r, "scripts/run.py", "# run\nprint(1)\n");
        write(r, "README.md", "# Title\n\ntext\n");
        write(r, "image.png", "\u{1}\u{2}");
        write(r, "target/out.rs", "fn ignored() {}\n");
        write(r, ".gitignore", "target/\n");
        write(r, ".github/workflows/ci.yml", "on: push\njobs: {}\n");
        write(r, ".git/config.rs", "fn not_counted() {}\n");
        write(
            r,
            "pkg/mod.py",
            "def f():\n    \"\"\"Doc line one.\n\n    Doc line two.\n    \"\"\"\n    return 1\n",
        );
        tmp
    }

    #[test]
    fn one_line_docstrings_are_found() {
        let src = b"def f():\n    \"\"\"One.\"\"\"\n    x = \"\"\"not a doc\"\"\"\n    r'''Raw.'''\n    \"\"\"a\"\"\" + \"\"\"b\"\"\"\n    \"\"\"\n";
        assert_eq!(one_line_docstrings(src), 2);
    }

    #[test]
    fn python_one_line_docstrings_count_as_comments() {
        let tmp = tempfile::tempdir().unwrap();
        write(
            tmp.path(),
            "a.py",
            "class C:\n    \"\"\"Doc.\"\"\"\n\n    def g(self):\n        \"\"\"Doc.\"\"\"\n        return 1\n",
        );
        let scan = scan(
            tmp.path(),
            1,
            &mut Counts::default(),
            None,
            Timestamp::now(),
        );
        let c = scan.totals.total.counts;
        assert_eq!((c.code, c.comments, c.blanks), (3, 2, 1));
    }

    #[test]
    fn dir_keys_group_by_depth() {
        assert_eq!(dir_key("README.md", 1), ".");
        assert_eq!(dir_key("src/main.rs", 1), "src");
        assert_eq!(dir_key("src/deep/util.rs", 1), "src");
        assert_eq!(dir_key("src/deep/util.rs", 2), "src/deep");
        assert_eq!(dir_key("src/main.rs", 3), "src");
    }

    #[test]
    fn counts_by_language_and_directory() {
        let tmp = fixture();
        let mut cache = Counts::default();
        let scan = scan(tmp.path(), 1, &mut cache, None, Timestamp::now());
        let t = &scan.totals;

        let rust = row(&t.languages, "Rust");
        assert_eq!(
            rust.counts,
            LineCounts {
                files: 2,
                code: 4,
                comments: 1,
                blanks: 1
            }
        );
        assert_eq!(rust.code_delta, None);
        let py = row(&t.languages, "Python");
        assert_eq!(
            (py.counts.files, py.counts.code),
            (2, 3),
            "docstrings are not code"
        );
        assert_eq!(
            py.counts.comments,
            1 + 4,
            "the # comment plus four docstring lines"
        );
        assert_eq!(
            row(&t.languages, "YAML").counts.files,
            1,
            "hidden .github is counted"
        );
        assert!(t.directories.iter().any(|r| r.name == ".github"));
        assert!(
            t.directories.iter().all(|r| r.name != ".git"),
            ".git is never counted"
        );
        assert!(
            t.languages.iter().all(|r| r.name != "PNG"),
            "binary files are not code"
        );

        assert_eq!(row(&t.directories, "src").counts.files, 2);
        assert_eq!(row(&t.directories, "scripts").counts.files, 1);
        assert!(
            t.directories.iter().all(|r| r.name != "target"),
            ".gitignore is honoured"
        );
        assert_eq!(t.languages[0].name, "Rust", "most code first");
        assert_eq!(
            t.total.counts.files,
            t.languages.iter().map(|r| r.counts.files).sum::<u64>()
        );
    }

    #[test]
    fn deltas_compare_with_the_previous_snapshot() {
        let tmp = fixture();
        let mut cache = Counts::default();
        let first = scan(tmp.path(), 1, &mut cache, None, Timestamp::now());

        write(tmp.path(), "src/new.rs", "fn a() {}\nfn b() {}\n");
        fs::remove_file(tmp.path().join("scripts/run.py")).unwrap();
        fs::remove_file(tmp.path().join(".github/workflows/ci.yml")).unwrap();
        let second = scan(
            tmp.path(),
            1,
            &mut cache,
            Some(&first.snapshot),
            Timestamp::now(),
        );
        let t = &second.totals;

        assert_eq!(row(&t.languages, "Rust").code_delta, Some(2));
        assert_eq!(row(&t.directories, "src").code_delta, Some(2));
        let py = row(&t.languages, "Python");
        assert_eq!((py.counts.files, py.code_delta), (1, Some(-1)));
        let gone = row(&t.languages, "YAML");
        assert_eq!((gone.counts.files, gone.code_delta), (0, Some(-2)));
        assert_eq!(t.total.code_delta, Some(-1));
        assert_eq!(t.baseline_at, first.snapshot.taken_at);
    }

    #[test]
    fn directory_deltas_need_the_same_depth() {
        let tmp = fixture();
        let mut cache = Counts::default();
        let first = scan(tmp.path(), 1, &mut cache, None, Timestamp::now());
        let second = scan(
            tmp.path(),
            2,
            &mut cache,
            Some(&first.snapshot),
            Timestamp::now(),
        );
        assert!(
            second
                .totals
                .directories
                .iter()
                .all(|r| r.code_delta.is_none())
        );
        assert_eq!(row(&second.totals.languages, "Rust").code_delta, Some(0));
    }

    #[test]
    fn unchanged_files_come_from_the_cache() {
        let tmp = fixture();
        let mut cache = Counts::default();
        let first = scan(tmp.path(), 1, &mut cache, None, Timestamp::now());
        let n = first.totals.total.counts.files;
        assert_eq!(first.totals.cache.counted, n);

        let second = scan(tmp.path(), 1, &mut cache, None, Timestamp::now());
        assert_eq!(
            second.totals.cache,
            CacheUse {
                unchanged: n,
                ..CacheUse::default()
            }
        );

        // Same content at a new path is a hash hit, not a recount.
        write(tmp.path(), "src/copy.rs", "fn util() {}\n");
        let third = scan(tmp.path(), 1, &mut cache, None, Timestamp::now());
        assert_eq!(third.totals.cache.hash_hits, 1);
        assert_eq!(third.totals.cache.counted, 0);
        assert_eq!(third.totals.total.counts.files, n + 1);
    }

    #[test]
    fn cache_is_pruned_to_files_seen() {
        let tmp = fixture();
        let mut cache = Counts::default();
        scan(tmp.path(), 1, &mut cache, None, Timestamp::now());
        fs::remove_file(tmp.path().join("src/main.rs")).unwrap();
        scan(tmp.path(), 1, &mut cache, None, Timestamp::now());
        assert!(!cache.by_path.contains_key("src/main.rs"));
        assert_eq!(cache.by_path.len(), cache.by_key.len());
    }
}
