use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Result of a successful pack operation.
#[derive(Debug, PartialEq, Eq)]
pub struct Packed {
    pub files: usize,
    pub bytes: u64,
}

#[derive(Debug)]
pub enum PackError {
    Io(io::Error),
    Invalid(String),
}

impl From<io::Error> for PackError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl fmt::Display for PackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "{}", err),
            Self::Invalid(msg) => write!(f, "{}", msg),
        }
    }
}

struct ZipEntry {
    name: Vec<u8>,
    crc: u32,
    size: u32,
    offset: u32,
}

/// Standard CRC-32 (IEEE 802.3, reflected polynomial 0xEDB88320).
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn put16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn skipped(name: &str) -> bool {
    name == ".git" || name == "target" || name == "node_modules" || name.ends_with(".log")
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)?.filter_map(Result::ok).collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if skipped(&name) {
            continue;
        }
        if path.is_dir() {
            collect_files(&path, out)?;
        } else if path.is_file() {
            out.push(path);
        }
    }
    Ok(())
}

fn normalized(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Pack `root` into a stored (uncompressed) ZIP archive at `output`.
///
/// Uses only std so the binary stays dependency-free. `.git`, `target`,
/// `node_modules` and `*.log` are excluded.
pub fn pack_plugin(root: &Path, output: &Path) -> Result<Packed, PackError> {
    if !root.is_dir() {
        return Err(PackError::Invalid(format!(
            "not a directory: {}",
            root.display()
        )));
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }

    let output_norm = normalized(output);
    let mut files = Vec::new();
    collect_files(root, &mut files)?;
    files.retain(|p| normalized(p) != output_norm);

    if files.len() > u16::MAX as usize {
        return Err(PackError::Invalid(format!(
            "too many files ({} > {})",
            files.len(),
            u16::MAX
        )));
    }

    let mut out: Vec<u8> = Vec::new();
    let mut entries: Vec<ZipEntry> = Vec::new();

    for path in &files {
        let rel = path
            .strip_prefix(root)
            .map_err(|_| PackError::Invalid(format!("cannot relativize {}", path.display())))?;
        let name = rel.to_string_lossy().replace('\\', "/");
        let data = fs::read(path)?;
        if data.len() > u32::MAX as usize {
            return Err(PackError::Invalid(format!(
                "file too large: {}",
                path.display()
            )));
        }
        let crc = crc32(&data);
        let size = data.len() as u32;
        let offset = out.len() as u32;
        let name_bytes = name.as_bytes();

        // Local file header (version 2.0, UTF-8 names, stored).
        put32(&mut out, 0x0403_4b50);
        put16(&mut out, 20);
        put16(&mut out, 0x0800);
        put16(&mut out, 0);
        put16(&mut out, 0);
        put16(&mut out, 0x21); // 1980-01-01, fixed for reproducible output
        put32(&mut out, crc);
        put32(&mut out, size);
        put32(&mut out, size);
        put16(&mut out, name_bytes.len() as u16);
        put16(&mut out, 0);
        out.extend_from_slice(name_bytes);
        out.extend_from_slice(&data);

        entries.push(ZipEntry {
            name: name_bytes.to_vec(),
            crc,
            size,
            offset,
        });
    }

    let central_offset = out.len() as u32;
    for entry in &entries {
        // Central directory header.
        put32(&mut out, 0x0201_4b50);
        put16(&mut out, 20);
        put16(&mut out, 20);
        put16(&mut out, 0x0800);
        put16(&mut out, 0);
        put16(&mut out, 0);
        put16(&mut out, 0x21);
        put32(&mut out, entry.crc);
        put32(&mut out, entry.size);
        put32(&mut out, entry.size);
        put16(&mut out, entry.name.len() as u16);
        put16(&mut out, 0);
        put16(&mut out, 0);
        put16(&mut out, 0);
        put16(&mut out, 0);
        put32(&mut out, 0);
        put32(&mut out, entry.offset);
        out.extend_from_slice(&entry.name);
    }
    let central_size = out.len() as u32 - central_offset;

    // End of central directory record.
    put32(&mut out, 0x0605_4b50);
    put16(&mut out, 0);
    put16(&mut out, 0);
    put16(&mut out, entries.len() as u16);
    put16(&mut out, entries.len() as u16);
    put32(&mut out, central_size);
    put32(&mut out, central_offset);
    put16(&mut out, 0);

    fs::write(output, &out)?;
    Ok(Packed {
        files: entries.len(),
        bytes: out.len() as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_reference_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn packs_files_into_valid_zip_structure() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("plugin");
        fs::create_dir_all(root.join("skills/example")).unwrap();
        fs::write(root.join("plugin.json"), "{}").unwrap();
        fs::write(root.join("skills/example/SKILL.md"), "---\nname: x\n---\n").unwrap();
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join(".git/config"), "secret").unwrap();

        let out = tmp.path().join("plugin.zip");
        let packed = pack_plugin(&root, &out).unwrap();
        assert_eq!(packed.files, 2);
        let bytes = fs::read(&out).unwrap();
        assert_eq!(&bytes[0..4], &0x0403_4b50u32.to_le_bytes());
        let eocd = &bytes[bytes.len() - 22..];
        assert_eq!(&eocd[0..4], &0x0605_4b50u32.to_le_bytes());
        let count = u16::from_le_bytes([eocd[8], eocd[9]]);
        assert_eq!(count, 2);
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("plugin.json"));
        assert!(text.contains("skills/example/SKILL.md"));
        assert!(!text.contains(".git/config"));
    }

    #[test]
    fn empty_directory_produces_empty_archive() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("empty");
        fs::create_dir(&root).unwrap();
        let out = tmp.path().join("empty.zip");
        let packed = pack_plugin(&root, &out).unwrap();
        assert_eq!(packed.files, 0);
        assert_eq!(fs::metadata(&out).unwrap().len(), 22);
    }

    #[test]
    fn rejects_non_directory_root() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("file");
        fs::write(&file, "x").unwrap();
        assert!(pack_plugin(&file, &tmp.path().join("out.zip")).is_err());
    }
}
