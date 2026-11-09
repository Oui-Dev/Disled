// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! The files of the plugins: the fingerprint of a plugin, which a change of
//! any of its files changes, and the import of a plugin from a zip file.
//! Neither follows a link, which could lead out of the folder, nor goes past
//! 5 000 files or 100 MiB.

use std::collections::HashMap;
use std::fs::{self, File, Metadata};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use sha2::{Digest, Sha256};

use super::manifest::{self, Manifest, Reason};

pub const MAX_FILES: usize = 5_000;
pub const MAX_BYTES: u64 = 100 * 1024 * 1024;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// A link, or on Windows a junction: neither is followed.
fn is_link(metadata: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    metadata.file_type().is_symlink()
}

/// The digests of the files read before, by path, size and date, so that
/// a scan reads only the files that changed.
#[derive(Debug, Default)]
pub struct Cache {
    files: HashMap<PathBuf, (u64, SystemTime, String)>,
}

impl Cache {
    fn digest(&mut self, path: &Path, metadata: &Metadata) -> io::Result<String> {
        let modified = metadata.modified()?;
        if let Some((size, date, digest)) = self.files.get(path)
            && *size == metadata.len()
            && *date == modified
        {
            return Ok(digest.clone());
        }
        let mut hasher = Sha256::new();
        io::copy(&mut File::open(path)?, &mut hasher)?;
        let digest = hex(&hasher.finalize());
        self.files
            .insert(path.to_owned(), (metadata.len(), modified, digest.clone()));
        Ok(digest)
    }
}

/// The files of a folder, by their path inside it with `/`.
fn files(folder: &Path) -> Result<Vec<(String, PathBuf, Metadata)>, Reason> {
    let mut found = Vec::new();
    let mut pending = vec![(String::new(), folder.to_owned())];
    let mut bytes = 0;
    while let Some((prefix, dir)) = pending.pop() {
        let entries = fs::read_dir(&dir).map_err(|_| Reason::Missing)?;
        for entry in entries {
            let entry = entry.map_err(|_| Reason::Missing)?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let relative = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            let metadata = fs::symlink_metadata(entry.path()).map_err(|_| Reason::Missing)?;
            if is_link(&metadata) {
                return Err(Reason::Link { path: relative });
            }
            if metadata.is_dir() {
                pending.push((relative, entry.path()));
                continue;
            }
            bytes += metadata.len();
            found.push((relative, entry.path(), metadata));
            if found.len() > MAX_FILES || bytes > MAX_BYTES {
                return Err(Reason::Size);
            }
        }
    }
    found.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(found)
}

/// The fingerprint of a plugin: the SHA-256 of one line per file, sorted by
/// path, made of the path, a NUL, the size in decimal, a NUL, the SHA-256 of
/// the content in hexadecimal, and a newline.
pub fn fingerprint(folder: &Path, cache: &mut Cache) -> Result<String, Reason> {
    let mut list = Sha256::new();
    for (relative, path, metadata) in files(folder)? {
        let digest = cache
            .digest(&path, &metadata)
            .map_err(|_| Reason::Missing)?;
        list.update(format!("{relative}\0{}\0{digest}\n", metadata.len()));
    }
    Ok(hex(&list.finalize()))
}

/// A folder name that no plugin can take, for an import under way.
fn import_folder(plugins: &Path) -> PathBuf {
    let mut random = [0u8; 8];
    let _ = getrandom::fill(&mut random);
    plugins.join(format!(".import-{}", hex(&random)))
}

/// Imports the plugin of a zip file, whose `plugin.json` sits at its root or
/// in its only folder, into `plugins/<id>`, in place of the plugin of that
/// id. Returns the id.
#[cfg(test)]
pub fn import(zip: &Path, plugins: &Path) -> Result<String, Reason> {
    unpack(zip, plugins)?.install(plugins)
}

/// A plugin that came out of its zip file, in a folder that no plugin can
/// take until `install`.
pub struct Unpacked {
    folder: PathBuf,
    id: String,
}

impl Unpacked {
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Puts the plugin in `plugins/<id>`, in place of the plugin of that id.
    pub fn install(self, plugins: &Path) -> Result<String, Reason> {
        let destination = plugins.join(&self.id);
        if destination.exists() {
            fs::remove_dir_all(&destination).map_err(|_| Reason::Missing)?;
        }
        fs::rename(&self.folder, &destination).map_err(|_| {
            let _ = fs::remove_dir_all(&self.folder);
            Reason::Missing
        })?;
        Ok(self.id)
    }
}

/// Unpacks the plugin of a zip file, whose `plugin.json` sits at its root or
/// in its only folder, after checking every path, and reads its manifest.
pub fn unpack(zip: &Path, plugins: &Path) -> Result<Unpacked, Reason> {
    let mut archive = zip::ZipArchive::new(File::open(zip).map_err(|_| Reason::Missing)?)
        .map_err(|_| Reason::Missing)?;
    // First the names, checked before anything is written.
    let mut names = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(|_| Reason::Missing)?;
        let raw = entry
            .name()
            .map(|name| name.into_owned())
            .unwrap_or_default();
        let Some(path) = entry.enclosed_name() else {
            return Err(Reason::Path { path: raw });
        };
        if entry.is_symlink() {
            return Err(Reason::Link { path: raw });
        }
        names.push((path, entry.is_dir()));
    }
    if names.iter().filter(|(_, dir)| !dir).count() > MAX_FILES {
        return Err(Reason::Size);
    }
    let root = root(&names)?;

    let target = import_folder(plugins);
    let extracted = extract(&mut archive, &names, &root, &target).and_then(|()| {
        let text = fs::read_to_string(target.join(manifest::FILE)).map_err(|_| Reason::Missing)?;
        let value =
            serde_json::from_str(&text).map_err(|error| Reason::Json { line: error.line() })?;
        Manifest::from_value(&value, None)
    });
    match extracted {
        Ok(manifest) => Ok(Unpacked {
            folder: target,
            id: manifest.id,
        }),
        Err(reason) => {
            let _ = fs::remove_dir_all(&target);
            Err(reason)
        }
    }
}

/// Where `plugin.json` is: at the root, or in the one folder that holds
/// everything.
fn root(names: &[(PathBuf, bool)]) -> Result<PathBuf, Reason> {
    let manifest = Path::new(manifest::FILE);
    if names.iter().any(|(path, dir)| !dir && path == manifest) {
        return Ok(PathBuf::new());
    }
    let mut tops = names
        .iter()
        .filter_map(|(path, _)| path.components().next())
        .map(|top| PathBuf::from(top.as_os_str()));
    let first = tops.next().ok_or(Reason::Missing)?;
    let single = tops.all(|top| top == first);
    if single
        && names
            .iter()
            .any(|(path, dir)| !dir && *path == first.join(manifest))
    {
        Ok(first)
    } else {
        Err(Reason::Missing)
    }
}

fn extract<R: Read + io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    names: &[(PathBuf, bool)],
    root: &Path,
    target: &Path,
) -> Result<(), Reason> {
    fs::create_dir_all(target).map_err(|_| Reason::Missing)?;
    let mut bytes = 0;
    for (index, (path, dir)) in names.iter().enumerate() {
        let Ok(inside) = path.strip_prefix(root) else {
            continue;
        };
        if inside.as_os_str().is_empty() {
            continue;
        }
        let destination = target.join(inside);
        if *dir {
            fs::create_dir_all(&destination).map_err(|_| Reason::Missing)?;
            continue;
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|_| Reason::Missing)?;
        }
        let mut entry = archive.by_index(index).map_err(|_| Reason::Missing)?;
        let mut file = File::create(&destination).map_err(|_| Reason::Missing)?;
        // The sizes count while the files come out, whatever the archive
        // says of them.
        let left = MAX_BYTES - bytes;
        let copied =
            io::copy(&mut (&mut entry).take(left + 1), &mut file).map_err(|_| Reason::Missing)?;
        bytes += copied;
        if bytes > MAX_BYTES {
            return Err(Reason::Size);
        }
        file.flush().map_err(|_| Reason::Missing)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use zip::write::SimpleFileOptions;

    const MANIFEST: &str = r#"{
        "api": 1, "id": "demo", "name": "Demo", "version": "1.0.0", "author": "test",
        "description": "A test plugin.", "capabilities": ["run"],
        "run": { "default": ["demo"] }
    }"#;

    fn write(path: &Path, text: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("folder");
        }
        fs::write(path, text).expect("write");
    }

    #[test]
    fn the_fingerprint_follows_every_file_and_only_them() {
        let folder = tempfile::tempdir().expect("folder");
        let plugin = folder.path().join("demo");
        write(&plugin.join("plugin.json"), MANIFEST);
        write(&plugin.join("lib/index.js"), "console.log(1)");
        let mut cache = Cache::default();
        let first = fingerprint(&plugin, &mut cache).expect("fingerprint");
        assert_eq!(first.len(), 64);
        // The same files give the same fingerprint, with or without the cache.
        assert_eq!(
            fingerprint(&plugin, &mut Cache::default()),
            Ok(first.clone())
        );
        write(&plugin.join("lib/index.js"), "console.log(2)");
        let changed = fingerprint(&plugin, &mut cache).expect("fingerprint");
        assert_ne!(changed, first);
        write(&plugin.join("lib/index.js"), "console.log(1)");
        write(&plugin.join("extra.txt"), "");
        assert_ne!(fingerprint(&plugin, &mut cache), Ok(first));
    }

    #[test]
    fn the_lines_of_the_fingerprint_are_those_of_annex_d() {
        let folder = tempfile::tempdir().expect("folder");
        write(&folder.path().join("b.txt"), "b");
        write(&folder.path().join("a/c.txt"), "");
        let digest = |bytes: &[u8]| hex(&Sha256::digest(bytes));
        let lines = format!(
            "a/c.txt\x000\x00{}\nb.txt\x001\x00{}\n",
            digest(b""),
            digest(b"b")
        );
        assert_eq!(
            fingerprint(folder.path(), &mut Cache::default()),
            Ok(digest(lines.as_bytes()))
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_link_is_refused() {
        let folder = tempfile::tempdir().expect("folder");
        write(&folder.path().join("plugin.json"), MANIFEST);
        std::os::unix::fs::symlink("/etc/passwd", folder.path().join("x")).expect("link");
        assert_eq!(
            fingerprint(folder.path(), &mut Cache::default()),
            Err(Reason::Link { path: "x".into() })
        );
    }

    fn zip(path: &Path, files: &[(&str, &str)]) {
        let mut writer = zip::ZipWriter::new(File::create(path).expect("zip"));
        for (name, text) in files {
            writer
                .start_file(*name, SimpleFileOptions::default())
                .expect("entry");
            writer.write_all(text.as_bytes()).expect("content");
        }
        writer.finish().expect("finish");
    }

    #[test]
    fn a_zip_imports_from_its_root_or_its_one_folder() {
        let folder = tempfile::tempdir().expect("folder");
        let plugins = folder.path().join("plugins");
        fs::create_dir(&plugins).expect("plugins");
        let archive = folder.path().join("demo.zip");
        zip(
            &archive,
            &[("plugin.json", MANIFEST), ("lib/index.js", "1")],
        );
        assert_eq!(import(&archive, &plugins), Ok("demo".into()));
        assert!(plugins.join("demo/lib/index.js").is_file());

        zip(
            &archive,
            &[
                ("demo-1.0.0/plugin.json", MANIFEST),
                ("demo-1.0.0/index.js", "2"),
            ],
        );
        assert_eq!(import(&archive, &plugins), Ok("demo".into()));
        // The new version took the place of the old one.
        assert!(plugins.join("demo/index.js").is_file());
        assert!(!plugins.join("demo/lib").exists());
        // Nothing of the imports stays behind.
        let left: Vec<_> = fs::read_dir(&plugins)
            .expect("plugins")
            .map(|entry| entry.expect("entry").file_name())
            .collect();
        assert_eq!(left, ["demo"]);
    }

    #[test]
    fn a_path_out_of_the_folder_is_refused_before_anything_is_written() {
        let folder = tempfile::tempdir().expect("folder");
        let plugins = folder.path().join("plugins");
        fs::create_dir(&plugins).expect("plugins");
        let archive = folder.path().join("evil.zip");
        zip(&archive, &[("plugin.json", MANIFEST), ("../x", "pwned")]);
        assert_eq!(
            import(&archive, &plugins),
            Err(Reason::Path {
                path: "../x".into()
            })
        );
        assert!(!folder.path().join("x").exists());
        assert_eq!(fs::read_dir(&plugins).expect("plugins").count(), 0);
    }

    #[test]
    fn a_zip_without_a_manifest_or_with_a_broken_one_leaves_nothing() {
        let folder = tempfile::tempdir().expect("folder");
        let plugins = folder.path().join("plugins");
        fs::create_dir(&plugins).expect("plugins");
        let archive = folder.path().join("empty.zip");
        zip(
            &archive,
            &[("a/plugin.json", MANIFEST), ("b/plugin.json", MANIFEST)],
        );
        assert_eq!(import(&archive, &plugins), Err(Reason::Missing));
        zip(&archive, &[("plugin.json", "{ \"api\": 1 }")]);
        assert_eq!(
            import(&archive, &plugins),
            Err(Reason::Field { field: "id".into() })
        );
        assert_eq!(fs::read_dir(&plugins).expect("plugins").count(), 0);
    }
}
