use super::{validate_manifest, validate_relative_path, Error, Manifest, PreparedPackage};
use std::{
    collections::{BTreeMap, HashSet},
    io::{Cursor, Read, Write},
};
use zip::{
    write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter,
};

const MAX_ARCHIVE_BYTES: u64 = super::MAX_ARCHIVE_BYTES;
const MAX_FILES: usize = super::MAX_FILES;
const MAX_ENTRIES: usize = super::MAX_ENTRIES;
const MAX_FILE_BYTES: u64 = super::MAX_FILE_BYTES;
const MAX_TOTAL_BYTES: u64 = super::MAX_TOTAL_BYTES;
const MAX_COMPRESSION_RATIO: u64 = super::MAX_COMPRESSION_RATIO;
const MAX_MANIFEST_BYTES: u64 = super::MAX_MANIFEST_BYTES;

pub(super) fn read(bytes: &[u8]) -> Result<PreparedPackage, Error> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_ARCHIVE_BYTES {
        return Err(Error::new(
            "validation",
            format!("A plugin archive must be between 1 byte and {MAX_ARCHIVE_BYTES} bytes."),
        ));
    }
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| Error::new("validation", format!("Invalid ZIP archive: {error}")))?;
    if archive.len() > MAX_ENTRIES {
        return Err(Error::new(
            "validation",
            format!("A plugin archive can contain at most {MAX_ENTRIES} entries."),
        ));
    }
    let mut entries = Vec::new();
    let mut names = Vec::new();
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|error| Error::new("validation", format!("Invalid ZIP entry: {error}")))?;
        if entry.is_symlink() {
            return Err(Error::new(
                "validation",
                "Plugin archives cannot contain symbolic links.",
            ));
        }
        let name = entry
            .name()
            .map_err(|error| Error::new("validation", format!("Invalid ZIP entry name: {error}")))?
            .to_string();
        if name.contains('\\') {
            return Err(Error::new(
                "validation",
                "Plugin archive paths must use forward slashes.",
            ));
        }
        let is_dir = entry.is_dir();
        let normalized = name.trim_end_matches('/').to_string();
        if normalized.is_empty() {
            return Err(Error::new("validation", "Plugin archive contains an empty path."));
        }
        validate_relative_path(&normalized)?;
        let size = entry.size();
        let compressed = entry.compressed_size();
        if !is_dir {
            if size > MAX_FILE_BYTES {
                return Err(Error::new(
                    "validation",
                    format!("{normalized} is larger than {MAX_FILE_BYTES} bytes."),
                ));
            }
            if size > 0 && compressed == 0 {
                return Err(Error::new(
                    "validation",
                    format!("{normalized} has an invalid compressed size."),
                ));
            }
            if compressed > 0 && size / compressed > MAX_COMPRESSION_RATIO {
                return Err(Error::new(
                    "validation",
                    format!("{normalized} expands too far and may be a ZIP bomb."),
                ));
            }
        }
        names.push(normalized.clone());
        entries.push((index, normalized, is_dir, size, compressed));
    }
    let prefix = root_prefix(&names)?;
    let mut files = BTreeMap::new();
    let mut seen = HashSet::new();
    let mut total = 0u64;
    for (index, name, is_dir, _, _) in entries {
        if is_dir {
            continue;
        }
        if !name.starts_with(&prefix) {
            return Err(Error::new(
                "validation",
                "Every plugin file must be inside the package root.",
            ));
        }
        let relative = name[prefix.len()..].to_string();
        if relative.is_empty() {
            continue;
        }
        let lower = relative.to_lowercase();
        if !seen.insert(lower) {
            return Err(Error::new(
                "validation",
                format!("Plugin archive contains a duplicate or case-colliding path: {relative}."),
            ));
        }
        let mut entry = archive
            .by_index(index)
            .map_err(|error| Error::new("validation", format!("Invalid ZIP entry: {error}")))?;
        let limit = if relative == "luma-extension.json" {
            MAX_MANIFEST_BYTES
        } else {
            MAX_FILE_BYTES
        };
        let mut bytes = Vec::new();
        entry
            .by_ref()
            .take(limit + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| Error::new("validation", format!("Could not read {relative}: {error}")))?;
        if bytes.len() as u64 > limit {
            return Err(Error::new(
                "validation",
                format!("{relative} is larger than its allowed size."),
            ));
        }
        if bytes.len() as u64 != entry.size() {
            return Err(Error::new(
                "validation",
                format!("{relative} does not match its declared size."),
            ));
        }
        total = total.saturating_add(bytes.len() as u64);
        if total > MAX_TOTAL_BYTES {
            return Err(Error::new(
                "validation",
                format!("A plugin can unpack to at most {MAX_TOTAL_BYTES} bytes."),
            ));
        }
        if files.insert(relative, bytes).is_some() {
            return Err(Error::new(
                "validation",
                "Plugin archive contains a duplicate path.",
            ));
        }
    }
    if files.len() > MAX_FILES {
        return Err(Error::new(
            "validation",
            format!("A plugin can contain at most {MAX_FILES} files."),
        ));
    }
    for path in files.keys() {
        let mut parent = path.as_str();
        while let Some(index) = parent.rfind('/') {
            parent = &parent[..index];
            if files.contains_key(parent) {
                return Err(Error::new(
                    "validation",
                    format!("Plugin archive uses {parent} as both a file and a directory."),
                ));
            }
        }
    }
    let manifest_bytes = files.remove("luma-extension.json").ok_or_else(|| {
        Error::new(
            "validation",
            "Plugin archive must contain luma-extension.json at its root.",
        )
    })?;
    let raw = std::str::from_utf8(&manifest_bytes).map_err(|error| {
        Error::new("validation", format!("luma-extension.json must be UTF-8: {error}"))
    })?;
    let manifest: Manifest = serde_json::from_str(raw)
        .map_err(|error| Error::new("validation", format!("Invalid luma-extension.json: {error}")))?;
    validate_manifest(&manifest)?;
    super::finalize(manifest, files)
}

fn root_prefix(names: &[String]) -> Result<String, Error> {
    if names.iter().any(|name| name == "luma-extension.json") {
        return Ok(String::new());
    }
    let mut roots = HashSet::new();
    for name in names {
        if let Some((root, _)) = name.split_once('/') {
            roots.insert(root.to_string());
        }
    }
    if roots.len() == 1 {
        let root = roots.into_iter().next().expect("one root");
        let prefix = format!("{root}/");
        if names.iter().any(|name| name == &format!("{prefix}luma-extension.json"))
            && names.iter().all(|name| name.starts_with(&prefix))
        {
            return Ok(prefix);
        }
    }
    Err(Error::new(
        "validation",
        "Put luma-extension.json at the archive root or inside one top-level folder.",
    ))
}

pub(super) fn write(manifest: &Manifest, files: &BTreeMap<String, Vec<u8>>) -> Result<Vec<u8>, Error> {
    validate_manifest(manifest)?;
    let manifest_json = serde_json::to_string(manifest)
        .map_err(|error| Error::new("validation", error))?;
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);
    writer
        .start_file("luma-extension.json", options)
        .map_err(|error| Error::new("storage", error))?;
    writer
        .write_all(manifest_json.as_bytes())
        .map_err(|error| Error::new("storage", error))?;
    for (path, bytes) in files {
        validate_relative_path(path)?;
        writer
            .start_file(path, options)
            .map_err(|error| Error::new("storage", error))?;
        writer.write_all(bytes).map_err(|error| Error::new("storage", error))?;
    }
    let cursor = writer.finish().map_err(|error| Error::new("storage", error))?;
    Ok(cursor.into_inner())
}
