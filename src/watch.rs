use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime};

type Fingerprint = BTreeMap<PathBuf, (SystemTime, u64)>;

fn should_skip(name: &str) -> bool {
    name == ".git" || name == "target" || name == "node_modules"
}

fn scan(dir: &Path, out: &mut Fingerprint) -> io::Result<()> {
    for entry in fs::read_dir(dir)?.filter_map(Result::ok) {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if should_skip(&name) {
            continue;
        }
        if path.is_dir() {
            scan(&path, out)?;
        } else if path.is_file() {
            let meta = fs::metadata(&path)?;
            out.insert(path, (meta.modified()?, meta.len()));
        }
    }
    Ok(())
}

/// Take a snapshot of all files below `root` (path -> mtime, size).
pub fn fingerprint(root: &Path) -> io::Result<Fingerprint> {
    let mut out = Fingerprint::new();
    scan(root, &mut out)?;
    Ok(out)
}

/// Re-run `check` whenever a file under `root` changes.
///
/// Polling is used instead of a filesystem-watching dependency to keep the
/// binary dependency-free. `check` is called once immediately, then again
/// after each detected change. The loop runs until the process is stopped.
pub fn watch<F>(root: &Path, interval: Duration, mut check: F) -> io::Result<()>
where
    F: FnMut(),
{
    let mut previous = fingerprint(root)?;
    check();
    loop {
        thread::sleep(interval);
        let current = fingerprint(root)?;
        if current != previous {
            previous = current;
            check();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_tracks_file_changes() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("plugin.json");
        fs::write(&file, "{}").unwrap();
        let first = fingerprint(tmp.path()).unwrap();
        assert_eq!(first.len(), 1);
        fs::write(&file, "{ } ").unwrap();
        let second = fingerprint(tmp.path()).unwrap();
        assert_ne!(first, second);
    }

    #[test]
    fn fingerprint_ignores_git_and_target() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join(".git")).unwrap();
        fs::write(tmp.path().join(".git/config"), "x").unwrap();
        fs::create_dir_all(tmp.path().join("target")).unwrap();
        fs::write(tmp.path().join("target/out"), "x").unwrap();
        fs::write(tmp.path().join("plugin.json"), "{}").unwrap();
        assert_eq!(fingerprint(tmp.path()).unwrap().len(), 1);
    }
}
