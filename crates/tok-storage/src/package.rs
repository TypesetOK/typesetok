use crate::atomic::{utc_timestamp_now, write_atomic};
use crate::error::StorageError;
use crate::migration::{MigrationPipeline, CURRENT_SCHEMA_VERSION};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use tok_core::model::{DocumentModel, DocumentRoot};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

/// Maximum decompressed size of any single entry (zip-bomb guard).
const MAX_ENTRY_SIZE: u64 = 100 * 1024 * 1024; // 100 MB
/// Maximum total decompressed size of a package.
const MAX_TOTAL_SIZE: u64 = 1024 * 1024 * 1024; // 1 GB
/// Maximum number of entries in a package.
const MAX_ENTRIES: usize = 100_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokManifest {
    pub schema_version: String,
    pub document_id: String,
    pub title: String,
    pub author: String,
    pub created_at: String,
    pub modified_at: String,
    pub master_book_id: Option<String>,
    pub embedded_assets: Vec<String>,
    pub page_count: u32,
    #[serde(default)]
    pub custom_metadata: HashMap<String, String>,
}

impl Default for TokManifest {
    fn default() -> Self {
        let now = utc_timestamp_now();
        Self {
            schema_version: CURRENT_SCHEMA_VERSION.to_string(),
            document_id: tok_core::id::Ulid::new().to_string(),
            title: "מסמך חדש".to_string(),
            author: "מחבר".to_string(),
            created_at: now.clone(),
            modified_at: now,
            master_book_id: None,
            embedded_assets: Vec::new(),
            page_count: 0,
            custom_metadata: HashMap::new(),
        }
    }
}

pub struct TokPackage {
    pub manifest: TokManifest,
    pub assets: HashMap<String, Vec<u8>>,
    pub previews: HashMap<String, Vec<u8>>,
}

impl Default for TokPackage {
    fn default() -> Self {
        Self::new()
    }
}

/// Validates an archive entry name (or an asset name relative to `assets/`):
/// forward-slash separated, relative, no `.`/`..` components, no drive or
/// stream syntax, no backslashes or control characters. This is what makes
/// it safe for callers to later write entries to disk under a directory.
pub(crate) fn validate_entry_name(name: &str) -> Result<(), StorageError> {
    let invalid = || StorageError::CorruptedPackage(format!("Invalid path in zip: {:?}", name));
    if name.is_empty()
        || name.starts_with('/')
        || name.contains('\\')
        || name.contains(':')
        || name.chars().any(char::is_control)
    {
        return Err(invalid());
    }
    let body = name.strip_suffix('/').unwrap_or(name);
    if body
        .split('/')
        .any(|c| c.is_empty() || c == "." || c == "..")
    {
        return Err(invalid());
    }
    Ok(())
}

/// Reads an entry fully, refusing to inflate more than `limit` bytes
/// regardless of the size the archive claims.
fn read_bounded(reader: impl Read, name: &str, limit: u64) -> Result<Vec<u8>, StorageError> {
    let mut data = Vec::new();
    reader.take(limit + 1).read_to_end(&mut data)?;
    if data.len() as u64 > limit {
        return Err(StorageError::CorruptedPackage(format!(
            "Entry {} exceeds max size limit",
            name
        )));
    }
    Ok(data)
}

fn read_text_entry(archive: &mut ZipArchive<File>, name: &str) -> Result<String, StorageError> {
    let entry = archive
        .by_name(name)
        .map_err(|_| StorageError::CorruptedPackage(format!("Missing {}", name)))?;
    let bytes = read_bounded(entry, name, MAX_ENTRY_SIZE)?;
    String::from_utf8(bytes)
        .map_err(|_| StorageError::CorruptedPackage(format!("{} is not valid UTF-8", name)))
}

impl TokPackage {
    pub fn new() -> Self {
        Self {
            manifest: TokManifest::default(),
            assets: HashMap::new(),
            previews: HashMap::new(),
        }
    }

    /// Adds an embedded binary asset (e.g. image, SVG) to the package.
    /// The name must be a relative path; invalid names are rejected by
    /// [`TokPackage::save_atomic`].
    pub fn add_asset(&mut self, relative_path: impl Into<String>, data: Vec<u8>) {
        let path = relative_path.into();
        if !self.manifest.embedded_assets.contains(&path) {
            self.manifest.embedded_assets.push(path.clone());
        }
        self.assets.insert(path, data);
    }

    /// Adds a page preview thumbnail.
    pub fn add_preview(&mut self, page_num: u32, data: Vec<u8>) {
        self.previews.insert(format!("page_{}.png", page_num), data);
    }

    /// Registers a font binary inside `assets/fonts/`.
    pub fn add_font_asset(&mut self, file_name: &str, data: Vec<u8>) {
        let trimmed = file_name
            .trim_start_matches('/')
            .trim_start_matches("assets/");
        let rel = if trimmed.starts_with("fonts/") {
            trimmed.to_string()
        } else {
            format!("fonts/{}", trimmed)
        };
        self.add_asset(rel, data);
    }

    /// Returns all embedded font assets (`fonts/...`, bytes) in deterministic sorted order.
    pub fn font_assets(&self) -> Vec<(&str, &[u8])> {
        let mut out: Vec<(&str, &[u8])> = self
            .assets
            .iter()
            .filter(|(k, _)| k.starts_with("fonts/"))
            .map(|(k, v)| (k.as_str(), v.as_slice()))
            .collect();
        out.sort_by(|a, b| a.0.cmp(b.0));
        out
    }

    /// Registers an image binary inside `assets/images/`.
    pub fn add_image_asset(&mut self, asset_rel_path: &str, data: Vec<u8>) {
        let trimmed = asset_rel_path
            .trim_start_matches('/')
            .trim_start_matches("assets/");
        let rel = if trimmed.starts_with("images/") {
            trimmed.to_string()
        } else {
            format!("images/{}", trimmed)
        };
        self.add_asset(rel, data);
    }

    /// Returns all embedded image assets (`images/...`, bytes) in deterministic sorted order.
    pub fn image_assets(&self) -> Vec<(&str, &[u8])> {
        let mut out: Vec<(&str, &[u8])> = self
            .assets
            .iter()
            .filter(|(k, _)| k.starts_with("images/"))
            .map(|(k, v)| (k.as_str(), v.as_slice()))
            .collect();
        out.sort_by(|a, b| a.0.cmp(b.0));
        out
    }

    /// Registers an ornament vector asset inside `assets/ornaments/`.
    pub fn add_ornament_asset(&mut self, file_name: &str, data: Vec<u8>) {
        let trimmed = file_name
            .trim_start_matches('/')
            .trim_start_matches("assets/");
        let rel = if trimmed.starts_with("ornaments/") {
            trimmed.to_string()
        } else {
            format!("ornaments/{}", trimmed)
        };
        self.add_asset(rel, data);
    }

    /// Returns all embedded ornament assets (`ornaments/...`, bytes) in deterministic sorted order.
    pub fn ornament_assets(&self) -> Vec<(&str, &[u8])> {
        let mut out: Vec<(&str, &[u8])> = self
            .assets
            .iter()
            .filter(|(k, _)| k.starts_with("ornaments/"))
            .map(|(k, v)| (k.as_str(), v.as_slice()))
            .collect();
        out.sort_by(|a, b| a.0.cmp(b.0));
        out
    }

    /// Saves the DocumentModel and package assets atomically to a `.tok` file.
    /// Implements Section 11.2: write to a temporary file, flush to disk,
    /// atomic rename/replace. Entries are written in a stable order.
    pub fn save_atomic(
        &mut self,
        doc: &DocumentModel,
        path: impl AsRef<Path>,
    ) -> Result<(), StorageError> {
        // Refuse to write a package that `open` would reject.
        for name in self.assets.keys() {
            validate_entry_name(&format!("assets/{}", name))?;
        }
        for name in self.previews.keys() {
            validate_entry_name(&format!("previews/{}", name))?;
        }

        // Sync manifest with document root
        let root = doc.root();
        self.manifest.title = root.metadata.title.clone();
        self.manifest.author = root.metadata.author.clone();
        self.manifest.document_id = root.id.to_string();
        self.manifest.modified_at = utc_timestamp_now();

        let manifest = &self.manifest;
        let mut assets: Vec<_> = self.assets.iter().collect();
        assets.sort_by(|a, b| a.0.cmp(b.0));
        let mut previews: Vec<_> = self.previews.iter().collect();
        previews.sort_by(|a, b| a.0.cmp(b.0));

        write_atomic(path.as_ref(), |file| {
            let mut zip = ZipWriter::new(file);
            let options =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

            zip.start_file("manifest.json", options)?;
            zip.write_all(&serde_json::to_vec_pretty(manifest)?)?;

            zip.start_file("document.json", options)?;
            zip.write_all(&serde_json::to_vec_pretty(root)?)?;

            for (asset_name, asset_bytes) in assets {
                zip.start_file(format!("assets/{}", asset_name), options)?;
                zip.write_all(asset_bytes)?;
            }

            for (preview_name, preview_bytes) in previews {
                zip.start_file(format!("previews/{}", preview_name), options)?;
                zip.write_all(preview_bytes)?;
            }

            let mut file = zip.finish()?;
            file.flush()?;
            Ok(file)
        })
    }

    /// Opens and extracts a `.tok` archive into a DocumentModel and TokPackage container.
    ///
    /// Rejects archives with unsafe entry names (absolute, `..`, drive or
    /// backslash paths), symbolic links, or entries that inflate beyond the
    /// size limits.
    pub fn open(path: impl AsRef<Path>) -> Result<(DocumentModel, Self), StorageError> {
        let file = File::open(path)?;
        let mut archive = ZipArchive::new(file)?;

        if archive.len() > MAX_ENTRIES {
            return Err(StorageError::CorruptedPackage(format!(
                "Package has too many entries ({})",
                archive.len()
            )));
        }

        // 1. Validate every entry before trusting any of them.
        for i in 0..archive.len() {
            let entry = archive.by_index_raw(i)?;
            let name = entry.name()?.to_string();
            validate_entry_name(&name)?;
            if entry.is_symlink() {
                return Err(StorageError::CorruptedPackage(format!(
                    "Symbolic link in zip: {}",
                    name
                )));
            }
            if entry.size() > MAX_ENTRY_SIZE {
                return Err(StorageError::CorruptedPackage(format!(
                    "Entry {} exceeds max size limit",
                    name
                )));
            }
        }

        // 2. Read and validate manifest.json
        let manifest: TokManifest =
            serde_json::from_str(&read_text_entry(&mut archive, "manifest.json")?)?;
        MigrationPipeline::validate_version(&manifest.schema_version)?;

        // 3. Read document.json (bypass intermediate serde_json::Value when schema is current)
        let doc_text = read_text_entry(&mut archive, "document.json")?;
        let root: DocumentRoot = if manifest.schema_version
            == crate::migration::CURRENT_SCHEMA_VERSION
        {
            match serde_json::from_str::<DocumentRoot>(&doc_text) {
                Ok(parsed) => {
                    MigrationPipeline::validate_version(&parsed.metadata.schema_version)?;
                    if parsed.metadata.schema_version == crate::migration::CURRENT_SCHEMA_VERSION {
                        parsed
                    } else {
                        let doc_json: serde_json::Value = serde_json::from_str(&doc_text)?;
                        let migrated_doc_json = MigrationPipeline::migrate_document_json(doc_json)?;
                        serde_json::from_value(migrated_doc_json)?
                    }
                }
                Err(_) => {
                    let doc_json: serde_json::Value = serde_json::from_str(&doc_text)?;
                    let migrated_doc_json = MigrationPipeline::migrate_document_json(doc_json)?;
                    serde_json::from_value(migrated_doc_json)?
                }
            }
        } else {
            let doc_json: serde_json::Value = serde_json::from_str(&doc_text)?;
            let migrated_doc_json = MigrationPipeline::migrate_document_json(doc_json)?;
            serde_json::from_value(migrated_doc_json)?
        };

        // 4. Read assets and previews
        let mut assets = HashMap::new();
        let mut previews = HashMap::new();
        let mut total: u64 = 0;

        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            let name = file.name()?.to_string();
            if name.ends_with('/') {
                continue;
            }
            let target = if let Some(key) = name.strip_prefix("assets/") {
                Some((&mut assets, key.to_string()))
            } else {
                name.strip_prefix("previews/")
                    .map(|key| (&mut previews, key.to_string()))
            };
            if let Some((map, key)) = target {
                let data = read_bounded(file, &name, MAX_ENTRY_SIZE)?;
                total += data.len() as u64;
                if total > MAX_TOTAL_SIZE {
                    return Err(StorageError::CorruptedPackage(
                        "Package exceeds total size limit".to_string(),
                    ));
                }
                map.insert(key, data);
            }
        }

        let package = Self {
            manifest,
            assets,
            previews,
        };

        Ok((DocumentModel::new(root), package))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn minimal_doc() -> DocumentModel {
        DocumentModel::new(DocumentRoot::new("בדיקה"))
    }

    /// Builds a raw zip with the given (name, content) entries.
    fn raw_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        for (name, data) in entries {
            zip.start_file(*name, options).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap().into_inner()
    }

    fn valid_entries() -> (Vec<u8>, Vec<u8>) {
        let manifest = serde_json::to_vec(&TokManifest::default()).unwrap();
        let doc = serde_json::to_vec(minimal_doc().root()).unwrap();
        (manifest, doc)
    }

    fn open_bytes(bytes: &[u8]) -> Result<(DocumentModel, TokPackage), StorageError> {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.tok");
        std::fs::write(&path, bytes).unwrap();
        TokPackage::open(&path)
    }

    #[test]
    fn entry_name_validation() {
        for ok in [
            "manifest.json",
            "assets/cover.svg",
            "assets/a..b.png",
            "assets/dir/",
        ] {
            assert!(validate_entry_name(ok).is_ok(), "{ok}");
        }
        for bad in [
            "",
            "/etc/passwd",
            "../evil",
            "assets/../../evil",
            "assets/./x",
            "assets\\..\\evil",
            "C:/Windows/evil",
            "C:evil",
            "assets//x",
            "assets/x\u{0}y",
            "file.txt:stream",
        ] {
            assert!(validate_entry_name(bad).is_err(), "{bad:?}");
        }
    }

    /// Regression: `..\\` and drive-letter names passed the old check.
    #[test]
    fn open_rejects_unsafe_entry_names() {
        let (manifest, doc) = valid_entries();
        for bad in ["assets\\..\\..\\evil.dll", "C:/evil.dll", "assets/./x"] {
            let bytes = raw_zip(&[
                ("manifest.json", &manifest),
                ("document.json", &doc),
                (bad, b"x"),
            ]);
            assert!(open_bytes(&bytes).is_err(), "{bad:?} accepted");
        }
    }

    #[test]
    fn open_rejects_symlinks() {
        let (manifest, doc) = valid_entries();
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        zip.start_file("manifest.json", options).unwrap();
        zip.write_all(&manifest).unwrap();
        zip.start_file("document.json", options).unwrap();
        zip.write_all(&doc).unwrap();
        zip.add_symlink("assets/link", "/etc/passwd", options)
            .unwrap();
        let bytes = zip.finish().unwrap().into_inner();
        assert!(open_bytes(&bytes).is_err());
    }

    /// Regression: manifest.json / document.json were inflated with
    /// `read_to_string` and no size limit; reads are now capped no matter what
    /// size the archive declares.
    #[test]
    fn bounded_reads_stop_at_the_limit() {
        let data = [7u8; 11];
        assert!(read_bounded(&data[..], "x", 10).is_err());
        assert_eq!(read_bounded(&data[..], "x", 11).unwrap().len(), 11);
        // An endless reader is cut off after limit + 1 bytes.
        assert!(read_bounded(std::io::repeat(b' '), "bomb", 1024).is_err());
    }

    #[test]
    fn missing_required_entries_are_reported() {
        let (manifest, doc) = valid_entries();
        assert!(open_bytes(&raw_zip(&[("manifest.json", &manifest)])).is_err());
        assert!(open_bytes(&raw_zip(&[("document.json", &doc)])).is_err());
        assert!(open_bytes(b"not a zip").is_err());
        let (model, _) = open_bytes(&raw_zip(&[
            ("manifest.json", &manifest),
            ("document.json", &doc),
        ]))
        .unwrap();
        assert_eq!(model.root().metadata.title, "בדיקה");
    }

    #[test]
    fn asset_names_round_trip_exactly() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.tok");
        let mut pkg = TokPackage::new();
        // Regression: trim_start_matches stripped a repeated "assets/" prefix.
        pkg.add_asset("assets/nested.png", b"1".to_vec());
        pkg.add_asset("fonts/David.ttf", b"2".to_vec());
        pkg.save_atomic(&minimal_doc(), &path).unwrap();
        let (_, loaded) = TokPackage::open(&path).unwrap();
        assert_eq!(loaded.assets.get("assets/nested.png").unwrap(), b"1");
        assert_eq!(loaded.assets.get("fonts/David.ttf").unwrap(), b"2");
    }

    #[test]
    fn invalid_asset_name_is_rejected_at_save_time() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.tok");
        let mut pkg = TokPackage::new();
        pkg.add_asset("../outside.png", b"1".to_vec());
        assert!(pkg.save_atomic(&minimal_doc(), &path).is_err());
        assert!(!path.exists());
    }

    /// Regression: on Windows, saving over an existing package first renamed it
    /// to `<name>.bak`, deleting any unrelated file of that name, and left a
    /// window with no package on disk.
    #[test]
    fn resave_replaces_file_and_leaves_siblings_alone() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("book.tok");
        let sibling = dir.path().join("book.bak");
        std::fs::write(&sibling, b"user data").unwrap();

        let mut pkg = TokPackage::new();
        pkg.save_atomic(&minimal_doc(), &path).unwrap();
        let mut doc2 = DocumentRoot::new("גרסה שנייה");
        doc2.metadata.author = "שני".to_string();
        pkg.save_atomic(&DocumentModel::new(doc2), &path).unwrap();

        let (loaded, manifest) = TokPackage::open(&path).unwrap();
        assert_eq!(loaded.root().metadata.title, "גרסה שנייה");
        assert_eq!(manifest.manifest.author, "שני");
        assert_eq!(std::fs::read(&sibling).unwrap(), b"user data");
        let names: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names.len(), 2, "temporary files left behind: {names:?}");
    }

    #[test]
    fn manifest_timestamps_are_current() {
        let m = TokManifest::default();
        assert_ne!(m.created_at, "2026-09-30T12:00:00Z");
        assert!(m.created_at.ends_with('Z') && m.created_at.len() == 20);
    }
}
