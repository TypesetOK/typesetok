use crate::error::EngineServiceError;
use crate::io::sanitize_asset_filename;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use tok_core::id::{FractionalIndex, NodeId};
use tok_core::model::{DocumentModel, DocumentRoot, ParagraphNode};
use tok_core::numbering::GematriaPunctuation;
use tok_core::outline::OutlineNumberingConfig;
use tok_core::transaction::CompoundTransaction;
use tok_core::ui_state::{convert_multi_flow_state_to_root, RawMultiFlowState};
use tok_storage::package::{TokManifest, TokPackage};
use tok_typeset::font::{FontData, FontLoadError, FontManager};

/// Summary metadata returned by [`inspect_package`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageInspectionSummary {
    pub title: String,
    pub author: String,
    pub document_id: String,
    pub schema_version: String,
    pub sections: usize,
    pub total_paragraphs: usize,
    pub embedded_assets: Vec<String>,
    pub page_previews: usize,
}

/// Generates a deterministic Hebrew Mishnah Berakhot demo document with full niqqud.
pub fn create_sample_hebrew_document(num_paragraphs: usize) -> DocumentModel {
    let mut root = DocumentRoot::new("תלמוד בבלי - מסכת ברכות");
    let sec = &mut root.sections[0];
    let flow = sec.main_flow_mut().unwrap();

    let sample_texts = [
        "מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בְּעַרְבִית? מִשָּׁעָה שֶׁהַכֹּהֲנִים נִכְנָסִים לֶאֱכֹל בִּתְרוּמָתָן, עַד סוֹף הָאַשְׁמוּרָה הָרִאשׁוֹנָה, דִּבְרֵי רַבִּי אֱלִיעֶזֶר. וַחֲכָמִים אוֹמְרִים: עַד חֲצוֹת. רַבָּן גַּמְלִיאֵל אוֹמֵר: עַד שֶׁיַּעֲלֶה עַמּוּד הַשַּׁחַר.",
        "מַעֲשֶׂה שֶׁבָּאוּ בָנָיו מִבֵּית הַמִּשְׁתֶּה, אָמְרוּ לוֹ: לֹא קָרִינוּ אֶת שְׁמַע! אָמַר לָהֶם: אִם לֹא עָלָה עַמּוּד הַשַּׁחַר, חַיָּיבִין אַתֶּם לִקְרוֹת.",
        "וְלֹא זוֹ בִּלְבַד, אֶלָּא כָּל מַה שֶּׁאָמְרוּ חֲכָמִים עַד חֲצוֹת, מִצְוָתָן עַד שֶׁיַּעֲלֶה עַמּוּד הַשַּׁחַר. הֶקְטֵר חֲלָבִים וְאֵבָרִים מִצְוָתָן עַד שֶׁיַּעֲלֶה עַמּוּד הַשַּׁחַר, וְכָל הַנֶּאֱכָלִים לְיוֹם אֶחָד מִצְוָתָן עַד שֶׁיַּעֲלֶה עַמּוּד הַשַּׁחַר.",
        "אִם כֵּן, לָמָּה אָמְרוּ חֲכָמִים עַד חֲצוֹת? כְּדֵי לְהַרְחִיק אֶת הָאָדָם מִן הָעֲבֵרָה, שֶׁלֹּא יֹאמַר אָדָם: יֵשׁ לִי עוֹד זְמַן, וְנִמְצָא יָשֵׁן וְעוֹבֵר עַל דִּבְרֵי תוֹרָה.",
        "תַּנָּא הֵיכָא קָאֵי דְּקָתָנֵי מֵאֵימָתַי? וְתוּ, מַאי שְׁנָא דְּתָנֵי בְּעַרְבִית בְּרֵישָׁא, לִתְנֵי דְּשַׁחֲרִית בְּרֵישָׁא? תַּנָּא אַקְּרָא קָאֵי, דִּכְתִיב: בְּשָׁכְבְּךָ וּבְקוּמֶךָ.",
    ];

    let mut prev_idx = FractionalIndex::initial();
    for i in 0..num_paragraphs {
        let text = sample_texts[i % sample_texts.len()];
        let idx = FractionalIndex::between(Some(&prev_idx), None).unwrap();
        flow.paragraphs
            .push(ParagraphNode::new(idx.clone(), "normal", text));
        prev_idx = idx;
    }

    DocumentModel::new(root)
}

/// Builds package manifest metadata (title, author, document id) from a [`DocumentRoot`].
pub fn manifest_for_root(root: &DocumentRoot) -> TokManifest {
    TokManifest {
        title: root.metadata.title.clone(),
        author: root.metadata.author.clone(),
        document_id: root.id.to_string(),
        ..TokManifest::default()
    }
}

/// Parses document JSON across direct `DocumentRoot`, migration pipeline, and UI `RawMultiFlowState`.
pub fn parse_document_json(
    content: &str,
) -> Result<(DocumentModel, TokManifest), EngineServiceError> {
    if let Ok(root) = DocumentRoot::from_json(content) {
        let manifest = manifest_for_root(&root);
        return Ok((DocumentModel::new(root), manifest));
    }

    if let Ok(val) = serde_json::from_str::<serde_json::Value>(content) {
        if let Ok(migrated) =
            tok_storage::migration::MigrationPipeline::migrate_document_json(val.clone())
        {
            if let Ok(root) = serde_json::from_value::<DocumentRoot>(migrated) {
                let manifest = manifest_for_root(&root);
                return Ok((DocumentModel::new(root), manifest));
            }
        }
        if let Ok(raw_state) = serde_json::from_value::<RawMultiFlowState>(val) {
            let root = convert_multi_flow_state_to_root(raw_state);
            let manifest = manifest_for_root(&root);
            return Ok((DocumentModel::new(root), manifest));
        }
    }

    Err(EngineServiceError::Parse(
        "Invalid document JSON: could not parse as DocumentRoot or MultiFlowDocumentState"
            .to_string(),
    ))
}

/// Loads a document from `--demo`, a `.json` file, or a `.tok` zip package.
pub fn load_package_or_doc(
    input: &str,
    demo_paragraphs: usize,
) -> Result<(DocumentModel, TokManifest, Option<TokPackage>), EngineServiceError> {
    if input == "--demo" {
        return Ok((
            create_sample_hebrew_document(demo_paragraphs),
            TokManifest::default(),
            None,
        ));
    }

    let path = Path::new(input);
    if !path.exists() {
        return Err(EngineServiceError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Input file does not exist: {}", input),
        )));
    }

    if input.ends_with(".json") {
        let content = fs::read_to_string(path)?;
        let (doc, manifest) = parse_document_json(&content)?;
        return Ok((doc, manifest, None));
    }

    match TokPackage::open(input) {
        Ok((model, pkg)) => {
            let manifest = pkg.manifest.clone();
            Ok((model, manifest, Some(pkg)))
        }
        Err(tok_err) => {
            if let Ok(content) = fs::read_to_string(path) {
                if content.trim_start().starts_with('{') {
                    let (doc, manifest) = parse_document_json(&content)?;
                    return Ok((doc, manifest, None));
                }
            }
            Err(EngineServiceError::Storage(format!(
                "Failed to open package or document ({}): {}",
                input, tok_err
            )))
        }
    }
}

/// Loads a document without returning its outer `.tok` container.
pub fn load_document_input(
    input: &str,
    demo_paragraphs: usize,
) -> Result<(DocumentModel, TokManifest), EngineServiceError> {
    let (doc, manifest, _) = load_package_or_doc(input, demo_paragraphs)?;
    Ok((doc, manifest))
}

/// Collects all font family names referenced across styles, patches, and spans.
pub fn collect_referenced_font_families(doc: &DocumentRoot) -> HashSet<String> {
    let mut out = HashSet::new();
    for s in &doc.paragraph_styles {
        if !s.font_family.trim().is_empty() {
            out.insert(s.font_family.clone());
        }
    }
    for cs in &doc.character_styles {
        if let Some(fam) = &cs.font_family {
            if !fam.trim().is_empty() {
                out.insert(fam.clone());
            }
        }
    }
    for sec in &doc.sections {
        for flow in &sec.flows {
            for p in &flow.paragraphs {
                let eff = p.effective_style(&doc.paragraph_styles);
                if !eff.font_family.trim().is_empty() {
                    out.insert(eff.font_family);
                }
                for patch in &p.style_patches {
                    if let Some(fam) = &patch.font_family {
                        if !fam.trim().is_empty() {
                            out.insert(fam.clone());
                        }
                    }
                }
                for sp in &p.spans {
                    if let Some(fam) = &sp.format.font_family {
                        if !fam.trim().is_empty() {
                            out.insert(fam.clone());
                        }
                    }
                }
            }
        }
    }
    out
}

/// Builds a [`FontManager`] containing embedded fonts, optional user fonts directory, and `.tok` fonts.
pub fn build_font_manager(fonts_dir: Option<&Path>, pkg: Option<&TokPackage>) -> FontManager {
    let mut mgr = FontManager::default();
    if let Some(dir) = fonts_dir {
        mgr.load_fonts_from_dir(dir);
    }
    if let Some(package) = pkg {
        for (rel_path, bytes) in package.font_assets() {
            let fallback = Path::new(rel_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("EmbeddedFont");
            let _ = mgr.register_external_font_bytes(bytes.to_vec(), fallback);
        }
    }
    mgr
}

/// Validates and inspects raw font bytes into the canonical JSON report payload.
pub fn inspect_font_bytes(bytes: &[u8], fallback_family: &str) -> serde_json::Value {
    match FontData::validate_and_inspect(bytes, fallback_family) {
        Ok((_, report)) => serde_json::json!({
            "valid": true,
            "report": report,
        }),
        Err(err) => {
            let partial = FontData::inspect_font_bytes(bytes, fallback_family)
                .ok()
                .map(|(_, rep)| rep);
            let error_code = match err {
                FontLoadError::RejectWoff => "RejectWoff",
                FontLoadError::RejectCff => "RejectCff",
                FontLoadError::RestrictedLicense => "RestrictedLicense",
                FontLoadError::BitmapOnlyEmbedding => "BitmapOnlyEmbedding",
                FontLoadError::InvalidFontBinary(_) => "InvalidFontBinary",
            };
            serde_json::json!({
                "valid": false,
                "error": error_code,
                "message": err.to_string(),
                "report": partial,
            })
        }
    }
}

/// Reads a font file from disk and returns its validation/coverage JSON report.
pub fn inspect_font_file(font_path: &Path) -> Result<serde_json::Value, EngineServiceError> {
    let bytes = fs::read(font_path)?;
    let fallback = font_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("CustomFont");
    Ok(inspect_font_bytes(&bytes, fallback))
}

/// Reads and summarizes a `.tok` package's manifest, sections, paragraphs, and assets.
pub fn inspect_package(input: &Path) -> Result<PackageInspectionSummary, EngineServiceError> {
    let (model, pkg) = TokPackage::open(input)?;
    let total_paragraphs: usize = model
        .root()
        .sections
        .iter()
        .map(|s| s.flows.iter().map(|f| f.paragraphs.len()).sum::<usize>())
        .sum();
    let mut embedded_assets: Vec<String> = pkg.assets.keys().cloned().collect();
    embedded_assets.sort();

    Ok(PackageInspectionSummary {
        title: pkg.manifest.title,
        author: pkg.manifest.author,
        document_id: pkg.manifest.document_id,
        schema_version: pkg.manifest.schema_version,
        sections: model.root().sections.len(),
        total_paragraphs,
        embedded_assets,
        page_previews: pkg.previews.len(),
    })
}

/// Atomically saves a [`DocumentModel`] into a `.tok` zip package, preserving existing assets
/// and embedding any referenced external fonts from `fonts_dir` or `existing_pkg`.
pub fn save_package(
    doc: &DocumentModel,
    manifest: TokManifest,
    output: &Path,
    existing_pkg: Option<&TokPackage>,
    fonts_dir: Option<&Path>,
) -> Result<(), EngineServiceError> {
    let owned_existing = if existing_pkg.is_none() && output.exists() {
        TokPackage::open(output).ok().map(|(_, pkg)| pkg)
    } else {
        None
    };
    let prev_pkg = existing_pkg.or(owned_existing.as_ref());

    let mut pkg = TokPackage::new();
    pkg.manifest = manifest;
    if let Some(prev) = prev_pkg {
        for (rel, bytes) in &prev.assets {
            pkg.add_asset(rel.clone(), bytes.clone());
        }
        pkg.manifest.created_at = prev.manifest.created_at.clone();
    }

    let font_mgr = build_font_manager(fonts_dir, prev_pkg);
    let referenced = collect_referenced_font_families(doc.root());
    for (face_name, font_data) in font_mgr.external_fonts() {
        let matches_ref = referenced.iter().any(|rf| {
            rf.eq_ignore_ascii_case(&face_name)
                || rf.eq_ignore_ascii_case(&font_data.family_name)
                || face_name.starts_with(rf)
        });
        if matches_ref {
            let file_name = format!("{}.ttf", sanitize_asset_filename(&face_name));
            pkg.add_font_asset(&file_name, (*font_data.raw_bytes).clone());
        }
    }

    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    pkg.save_atomic(doc, output)?;
    Ok(())
}

/// Runs built-in Rust typographic pre-typeset plugins in-place on `root`.
pub fn run_builtin_plugins(root: &mut DocumentRoot) -> Vec<CompoundTransaction> {
    let host = tok_plugin_host::PluginHost::with_builtin_plugins();
    host.run_before_typeset(root)
}

/// Synchronizes outline heading numbering (Hebrew gematria or plain letters) on `root`.
pub fn sync_heading_numbers(root: &mut DocumentRoot, plain: bool) -> Vec<(NodeId, String)> {
    let punctuation = if plain {
        GematriaPunctuation::PlainLetters
    } else {
        GematriaPunctuation::StandardPunctuation
    };
    let config = OutlineNumberingConfig::torah_default(punctuation);
    root.sync_outline_numbering(&config)
}
