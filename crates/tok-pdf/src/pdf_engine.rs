//! Native Pre-Press ISO PDF/X-1a and PDF/X-4 Generation Engine.
//!
//! Replaces browser print-to-PDF by emitting bit-perfect ISO 15930 PDF/X
//! directly from Rust with DeviceCMYK, OutputIntents, page boxes,
//! and TrueType/OpenType font subsetting (Type 0 / CIDFont Type 2).
//!
//! Every font referenced by the layout (`LineBox::fonts`) is subset and
//! embedded separately, so glyph ids are always drawn with the font that
//! produced them.

use crate::boxes::PrePressPageBoxes;
use crate::color::{CmykColor, ColorManagementEngine, IccProfileId};
use crate::font_subsetter::{fnv1a, FontSubsetter, SubsetFontResult, FNV_OFFSET};
use crate::icc::{IccError, IccProfile, RenderingIntent};
use crate::image_xobject::{
    encode_image_xobject_data_for_page, parse_asset_page_suffix, PreparedColorSpace,
};
use crate::ornament::{BuiltinOrnaments, SvgVectorCompiler};
use crate::tounicode::ToUnicodeCMap;
use pdf_writer::types::{
    CidFontType, ColorSpaceOperand, FontFlags, OutputIntentSubtype, SystemInfo, TrappingStatus,
};
use pdf_writer::{Content, Date, Filter, Finish, Name, Pdf, Rect, Ref, Str, TextStr};
use std::collections::BTreeMap;
use std::sync::OnceLock;
use thiserror::Error;
use tok_core::styles::Color;
use tok_typeset::font::{FontManager, EMBEDDED_NOTO_SERIF_HEBREW};
use tok_typeset::geometry::{GlyphBox, LineBox, PageLayoutBox, PhysicalRect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfXStandard {
    PdfX1a2001,
    PdfX4,
    /// Regular PDF 1.6 without any PDF/X conformance claim (screen / office use).
    Plain,
}

pub struct PdfExportOptions {
    pub standard: PdfXStandard,
    pub color_profile: IccProfileId,
    pub title: String,
    pub author: String,
    pub bleed_pt: f32, // default 3mm = 8.504 pt
    pub slug_pt: f32,  // default 10mm = 28.346 pt
    pub draw_crop_marks: bool,
    pub draw_registration_marks: bool,
    pub draw_page_numbers: bool,
    /// Font used for glyphs whose font the layout does not record (layouts
    /// built without font information, or text-only lines). Defaults to the
    /// embedded Noto Serif Hebrew.
    pub custom_font_data: Option<Vec<u8>>,
    /// ISO 8601 / RFC 3339 creation timestamp (e.g. from TokManifest).
    /// If None, a deterministic default is used for bit-for-bit reproducibility.
    pub creation_date: Option<String>,
    /// ISO 8601 / RFC 3339 modification timestamp (e.g. from TokManifest).
    /// If None, creation_date or a deterministic default is used.
    pub mod_date: Option<String>,
    /// Compress streams with FlateDecode (deflate/zlib) for smaller PDF output.
    pub compress_streams: bool,
    /// Raster image assets keyed by asset path (e.g. `images/photo.png` -> PNG/JPEG/TIFF bytes).
    pub image_assets: BTreeMap<String, Vec<u8>>,
    /// Custom ornament SVG documents keyed by asset path (e.g. `ornaments/sep.svg` -> SVG XML).
    pub ornament_assets: BTreeMap<String, String>,
    /// Optional 2-up booklet / signature imposition with creep compensation.
    pub imposition: Option<crate::imposition::ImpositionConfig>,
    /// ICC rendering intent used for RGB-to-CMYK color conversion.
    pub rendering_intent: RenderingIntent,
    /// Optional external `.icc` / `.icm` CMYK output profile bytes overriding `color_profile`.
    pub external_icc_profile: Option<Vec<u8>>,
}

impl Default for PdfExportOptions {
    fn default() -> Self {
        Self {
            standard: PdfXStandard::PdfX1a2001,
            color_profile: IccProfileId::Fogra39,
            title: "מסמך מעומד".to_string(),
            author: "TypesetOK".to_string(),
            bleed_pt: 8.504,
            slug_pt: 28.346,
            draw_crop_marks: true,
            draw_registration_marks: false,
            draw_page_numbers: false,
            custom_font_data: None,
            creation_date: None,
            mod_date: None,
            compress_streams: true,
            image_assets: BTreeMap::new(),
            ornament_assets: BTreeMap::new(),
            imposition: None,
            rendering_intent: RenderingIntent::default(),
            external_icc_profile: None,
        }
    }
}

#[derive(Debug, Error)]
pub enum PdfError {
    #[error("font '{0}' used by the layout is not registered")]
    MissingFont(String),

    #[error("font '{family}' could not be embedded: {reason}")]
    FontEmbedding { family: String, reason: String },

    #[error("invalid ICC profile: {0}")]
    InvalidIccProfile(#[from] IccError),
}

/// Number of addressable TrueType glyph ids.
const GLYPH_SPACE: usize = 1 << 16;
const NO_CID: u32 = u32::MAX;
const LEGACY_LABEL: &str = "custom font";

/// A font that will be embedded, with the glyphs the document uses.
struct FontSlot<'a> {
    label: &'a str,
    data: &'a [u8],
    used: Vec<bool>,
    chars: Vec<Option<char>>,
}

impl<'a> FontSlot<'a> {
    fn new(label: &'a str, data: &'a [u8]) -> Self {
        Self {
            label,
            data,
            used: vec![false; GLYPH_SPACE],
            chars: vec![None; GLYPH_SPACE],
        }
    }

    fn mark(&mut self, gid: u16, ch: Option<char>) {
        if gid == 0 {
            return;
        }
        let i = usize::from(gid);
        self.used[i] = true;
        if self.chars[i].is_none() {
            self.chars[i] = ch;
        }
    }
}

/// Fonts in order of first use (deterministic resource names).
struct FontRegistry<'a> {
    fonts: &'a FontManager,
    legacy_data: &'a [u8],
    slots: Vec<FontSlot<'a>>,
    by_family: BTreeMap<&'a str, usize>,
    legacy: Option<usize>,
}

impl<'a> FontRegistry<'a> {
    fn family(&mut self, name: &'a str) -> Result<usize, PdfError> {
        if let Some(&slot) = self.by_family.get(name) {
            return Ok(slot);
        }
        let data = self
            .fonts
            .font_exact(name)
            .map(|f| f.raw_bytes.as_slice())
            .ok_or_else(|| PdfError::MissingFont(name.to_string()))?;
        self.slots.push(FontSlot::new(name, data));
        self.by_family.insert(name, self.slots.len() - 1);
        Ok(self.slots.len() - 1)
    }

    fn legacy(&mut self) -> usize {
        *self.legacy.get_or_insert_with(|| {
            self.slots
                .push(FontSlot::new(LEGACY_LABEL, self.legacy_data));
            self.slots.len() - 1
        })
    }
}

/// Embedded slot and glyph id of a laid-out glyph. `line_slots` maps the
/// line's font indices to slots; `legacy` provides the fallback slot.
fn glyph_target(
    line: &LineBox,
    g: &GlyphBox,
    line_slots: &[usize],
    mut legacy: impl FnMut() -> Option<usize>,
    legacy_face: &ttf_parser::Face<'_>,
) -> Option<(usize, u16)> {
    if line.fonts.is_empty() {
        // Layout without font information: ids belong to the legacy font.
        return Some((legacy()?, u16::try_from(g.glyph_id).ok()?));
    }
    match line_slots.get(usize::from(g.font_index)) {
        Some(&slot) => Some((slot, u16::try_from(g.glyph_id).ok()?)),
        // Unknown font (heuristic shaping): resolve by character.
        None => {
            let gid = legacy_face.glyph_index(g.character?)?.0;
            Some((legacy()?, gid))
        }
    }
}

struct EmbeddedFont {
    resource: String,
    subset: SubsetFontResult,
    /// Dense glyph id -> CID table (`NO_CID` for unused glyphs).
    cids: Vec<u32>,
}

impl EmbeddedFont {
    fn cid(&self, gid: u16) -> Option<u16> {
        match self.cids[usize::from(gid)] {
            NO_CID => None,
            cid => Some(cid as u16),
        }
    }
}

/// Item of a pending `TJ` array.
enum TjItem {
    Adjust(f32),
    Glyphs(Vec<u8>),
}

/// Builds the `TJ` arrays of one line while tracking where the PDF pen is.
#[derive(Default)]
struct LineTextWriter {
    /// Current x of the text pen in user space (`None` before the first glyph).
    pen: Option<f32>,
    font: Option<(usize, f32)>,
    rise: f32,
    items: Vec<TjItem>,
}

impl LineTextWriter {
    /// Appends a glyph that must start at `x`; `advance_1000` is the font's
    /// own advance (which the viewer applies) in thousandths of an em.
    fn push(&mut self, x: f32, cid: u16, advance_1000: f32, size: f32) {
        let mut pen = self.pen.unwrap_or(x);
        // TJ adjustments are subtracted, in thousandths of the font size.
        // Rounding keeps the stream short; tracking the rounded value keeps
        // the error from accumulating along the line.
        let adjust = ((pen - x) * 1000.0 / size * 100.0).round() / 100.0;
        if adjust != 0.0 {
            self.items.push(TjItem::Adjust(adjust));
            pen -= adjust * size / 1000.0;
        }
        match self.items.last_mut() {
            Some(TjItem::Glyphs(bytes)) => bytes.extend_from_slice(&cid.to_be_bytes()),
            _ => self.items.push(TjItem::Glyphs(cid.to_be_bytes().to_vec())),
        }
        self.pen = Some(pen + advance_1000 * size / 1000.0);
    }

    fn flush(&mut self, content: &mut Content) {
        if self.items.is_empty() {
            return;
        }
        let mut op = content.show_positioned();
        let mut array = op.items();
        for item in self.items.drain(..) {
            match item {
                TjItem::Adjust(amount) => array.adjust(amount),
                TjItem::Glyphs(bytes) => array.show(Str(&bytes)),
            };
        }
    }
}

/// Producing application, written to both the Info dictionary and the XMP packet.
const CREATOR_TOOL: &str = "TypesetOK Native Pre-Press Engine 2026";

/// Escapes text for use inside an XML element (characters XML 1.0 forbids are dropped).
fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c if u32::from(c) < 0x20 && !matches!(c, '\t' | '\n' | '\r') => {}
            c => out.push(c),
        }
    }
    out
}

/// Date for the XMP packet: ISO 8601 with a time zone, matching what `parse_pdf_date` reads for
/// the Info dictionary. Unrecognised input falls back to the deterministic default date.
fn xmp_date(s: &str) -> String {
    const FALLBACK: &str = "2026-01-01T00:00:00Z";
    let s = s.trim();
    let s = s.strip_prefix("D:").unwrap_or(s);
    let bytes = s.as_bytes();
    if bytes.len() >= 10 && bytes[4] == b'-' && bytes[7] == b'-' {
        return match bytes.len() {
            10 => format!("{s}T00:00:00Z"),
            19 => format!("{}Z", s.replacen(' ', "T", 1)),
            _ => s.replacen(' ', "T", 1),
        };
    }
    if bytes.len() >= 14 && bytes[..14].iter().all(u8::is_ascii_digit) {
        return format!(
            "{}-{}-{}T{}:{}:{}Z",
            &s[0..4],
            &s[4..6],
            &s[6..8],
            &s[8..10],
            &s[10..12],
            &s[12..14]
        );
    }
    FALLBACK.to_string()
}

/// `uuid:` URN built from the first 16 bytes of the file identifier.
fn uuid_urn(id: &[u8]) -> String {
    let hex: String = id.iter().take(16).map(|b| format!("{b:02x}")).collect();
    format!(
        "uuid:{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// XMP packet required by PDF/X-4 (ISO 15930-7): it repeats the Info dictionary values and adds
/// the PDF/X version and the document identity.
fn build_xmp(
    title: &str,
    author: &str,
    created: &str,
    modified: &str,
    document_id: &str,
) -> String {
    format!(
        r#"<?xpacket begin="{bom}" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:dc="http://purl.org/dc/elements/1.1/"
    xmlns:xmp="http://ns.adobe.com/xap/1.0/"
    xmlns:pdf="http://ns.adobe.com/pdf/1.3/"
    xmlns:pdfxid="http://www.npes.org/pdfx/ns/id/"
    xmlns:xmpMM="http://ns.adobe.com/xap/1.0/mm/">
   <dc:format>application/pdf</dc:format>
   <dc:title><rdf:Alt><rdf:li xml:lang="x-default">{title}</rdf:li></rdf:Alt></dc:title>
   <dc:creator><rdf:Seq><rdf:li>{author}</rdf:li></rdf:Seq></dc:creator>
   <xmp:CreatorTool>{tool}</xmp:CreatorTool>
   <xmp:CreateDate>{created}</xmp:CreateDate>
   <xmp:ModifyDate>{modified}</xmp:ModifyDate>
   <xmp:MetadataDate>{modified}</xmp:MetadataDate>
   <pdf:Trapped>False</pdf:Trapped>
   <pdfxid:GTS_PDFXVersion>PDF/X-4</pdfxid:GTS_PDFXVersion>
   <xmpMM:DocumentID>{document_id}</xmpMM:DocumentID>
   <xmpMM:VersionID>1</xmpMM:VersionID>
   <xmpMM:RenditionClass>default</xmpMM:RenditionClass>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
<?xpacket end="r"?>"#,
        bom = '\u{feff}',
        title = xml_escape(title),
        author = xml_escape(author),
        tool = xml_escape(CREATOR_TOOL),
    )
}

/// Parses an RFC 3339 / ISO 8601 or PDF formatted date string into a [`pdf_writer::Date`].
pub fn parse_pdf_date(s: &str) -> Option<Date> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let s = s.strip_prefix("D:").unwrap_or(s);

    // Formats like YYYY-MM-DD or YYYY-MM-DDTHH:MM:SSZ
    if s.len() >= 10 && s.as_bytes()[4] == b'-' && s.as_bytes()[7] == b'-' {
        let year: u16 = s[0..4].parse().ok()?;
        let month: u8 = s[5..7].parse().ok()?;
        let day: u8 = s[8..10].parse().ok()?;
        let mut date = Date::new(year).month(month).day(day);

        let rem = &s[10..];
        if rem.starts_with('T') || rem.starts_with(' ') {
            let time_part = &rem[1..];
            if time_part.len() >= 8
                && time_part.as_bytes()[2] == b':'
                && time_part.as_bytes()[5] == b':'
            {
                let hour: u8 = time_part[0..2].parse().ok()?;
                let minute: u8 = time_part[3..5].parse().ok()?;
                let second: u8 = time_part[6..8].parse().ok()?;
                date = date.hour(hour).minute(minute).second(second);

                let tz_part = &time_part[8..];
                if tz_part.starts_with('Z') || tz_part.starts_with('z') {
                    date = date.utc_offset_hour(0).utc_offset_minute(0);
                } else if tz_part.starts_with('+') || tz_part.starts_with('-') {
                    let sign: i8 = if tz_part.starts_with('+') { 1 } else { -1 };
                    let tz_rest = &tz_part[1..];
                    if let Some(colon_pos) = tz_rest.find(':') {
                        let tz_h: i8 = tz_rest[..colon_pos].parse().ok()?;
                        let tz_m: u8 = tz_rest
                            [colon_pos + 1..colon_pos + 1 + 2.min(tz_rest.len() - colon_pos - 1)]
                            .parse()
                            .ok()?;
                        date = date.utc_offset_hour(sign * tz_h).utc_offset_minute(tz_m);
                    } else if tz_rest.len() >= 2 {
                        let tz_h: i8 = tz_rest[..2].parse().ok()?;
                        date = date.utc_offset_hour(sign * tz_h).utc_offset_minute(0);
                    }
                } else {
                    date = date.utc_offset_hour(0).utc_offset_minute(0);
                }
            }
        } else {
            date = date
                .hour(0)
                .minute(0)
                .second(0)
                .utc_offset_hour(0)
                .utc_offset_minute(0);
        }
        return Some(date);
    }

    // Compact format YYYYMMDDHHmmSS...
    if s.len() >= 8 {
        let year: u16 = s[0..4].parse().ok()?;
        let month: u8 = s[4..6].parse().ok()?;
        let day: u8 = s[6..8].parse().ok()?;
        let mut date = Date::new(year).month(month).day(day);
        let rem = &s[8..];
        if rem.len() >= 4 {
            let hour: u8 = rem[0..2].parse().ok()?;
            let minute: u8 = rem[2..4].parse().ok()?;
            let second: u8 = if rem.len() >= 6 {
                rem[4..6].parse().unwrap_or(0)
            } else {
                0
            };
            date = date.hour(hour).minute(minute).second(second);
        }
        date = date.utc_offset_hour(0).utc_offset_minute(0);
        return Some(date);
    }

    None
}

fn default_font_manager() -> &'static FontManager {
    static FONTS: OnceLock<FontManager> = OnceLock::new();
    FONTS.get_or_init(FontManager::default)
}

pub struct PdfPrePressEngine;

impl PdfPrePressEngine {
    /// Generates a complete, compliant PDF/X document from typeset page layout
    /// boxes, resolving fonts with the default [`FontManager`].
    pub fn export_pdf(
        pages: &[PageLayoutBox],
        options: &PdfExportOptions,
    ) -> Result<Vec<u8>, PdfError> {
        Self::export_pdf_with_fonts(pages, options, default_font_manager())
    }

    /// Generates a PDF/X document, resolving the fonts recorded in the layout
    /// through `fonts` (use the manager the layout was typeset with).
    pub fn export_pdf_with_fonts(
        pages: &[PageLayoutBox],
        options: &PdfExportOptions,
        fonts: &FontManager,
    ) -> Result<Vec<u8>, PdfError> {
        let dummy_page;
        let imposed_pages_owned;
        let raw_pages: &[PageLayoutBox] = if pages.is_empty() {
            dummy_page = [PageLayoutBox {
                page_index: 0,
                page_number_gematria: String::new(),
                dimensions: PhysicalRect::a4_portrait(),
                frames: Vec::new(),
                break_token: None,
                is_blank: false,
                ..Default::default()
            }];
            &dummy_page
        } else {
            pages
        };
        let iter_pages: &[PageLayoutBox] = match &options.imposition {
            Some(imp) if imp.enabled => {
                let (_plan, imposed) = crate::imposition::impose_layout_pages(raw_pages, imp);
                imposed_pages_owned = imposed;
                &imposed_pages_owned
            }
            _ => raw_pages,
        };

        let legacy_data: &[u8] = options
            .custom_font_data
            .as_deref()
            .unwrap_or(EMBEDDED_NOTO_SERIF_HEBREW);
        let legacy_face =
            ttf_parser::Face::parse(legacy_data, 0).map_err(|e| PdfError::FontEmbedding {
                family: LEGACY_LABEL.to_string(),
                reason: e.to_string(),
            })?;

        // 1. Collect used glyphs per font, in order of first use.
        let mut registry = FontRegistry {
            fonts,
            legacy_data,
            slots: Vec::new(),
            by_family: BTreeMap::new(),
            legacy: None,
        };
        for line in iter_pages
            .iter()
            .flat_map(|p| &p.frames)
            .flat_map(|f| &f.lines)
        {
            if line.glyphs.is_empty() {
                // Text-only line: map characters through the legacy font.
                for ch in line.text.chars() {
                    if let Some(gid) = legacy_face.glyph_index(ch) {
                        let slot = registry.legacy();
                        registry.slots[slot].mark(gid.0, Some(ch));
                    }
                }
                continue;
            }
            let line_slots = line
                .fonts
                .iter()
                .map(|name| registry.family(name))
                .collect::<Result<Vec<_>, _>>()?;
            for g in &line.glyphs {
                let mut legacy_slot = None;
                let target = glyph_target(
                    line,
                    g,
                    &line_slots,
                    || Some(*legacy_slot.get_or_insert_with(|| registry.legacy())),
                    &legacy_face,
                );
                if let Some((slot, gid)) = target {
                    registry.slots[slot].mark(gid, g.character);
                }
            }
        }
        if options.draw_page_numbers {
            for p in iter_pages {
                if !p.page_number_gematria.is_empty() {
                    for ch in p.page_number_gematria.chars() {
                        if let Some(gid) = legacy_face.glyph_index(ch) {
                            let slot = registry.legacy();
                            registry.slots[slot].mark(gid.0, Some(ch));
                        }
                    }
                }
            }
        }

        // 2. Subset every font.
        let mut embedded = Vec::with_capacity(registry.slots.len());
        for (i, slot) in registry.slots.iter().enumerate() {
            let pairs: Vec<(u16, Option<char>)> = (1..GLYPH_SPACE)
                .filter(|&g| slot.used[g])
                .map(|g| (g as u16, slot.chars[g]))
                .collect();
            let subset = FontSubsetter::create_subset(slot.data, &pairs).map_err(|reason| {
                PdfError::FontEmbedding {
                    family: slot.label.to_string(),
                    reason,
                }
            })?;
            let mut cids = vec![NO_CID; GLYPH_SPACE];
            for (&gid, &cid) in &subset.gid_to_cid {
                cids[usize::from(gid)] = u32::from(cid);
            }
            embedded.push(EmbeddedFont {
                resource: format!("F{}", i + 1),
                subset,
                cids,
            });
        }

        let mut pdf = Pdf::new();
        let mut next_id = 1;
        let mut alloc = || {
            let id = Ref::new(next_id);
            next_id += 1;
            id
        };

        let catalog_id = alloc();
        let pages_id = alloc();
        let info_id = alloc();

        // 3. Info dictionary with PDF/X conformance keys
        let effective_title = if options.title.trim().is_empty() {
            "מסמך מעומד"
        } else {
            options.title.trim()
        };
        let creation_str = options
            .creation_date
            .as_deref()
            .unwrap_or("2026-01-01T00:00:00Z");
        let mod_str = options.mod_date.as_deref().unwrap_or(creation_str);

        {
            let mut info = pdf.document_info(info_id);
            info.title(TextStr(effective_title));
            info.author(TextStr(&options.author));
            info.creator(TextStr(CREATOR_TOOL));
            info.trapped(TrappingStatus::NotTrapped);

            if let Some(cdate) = parse_pdf_date(creation_str) {
                info.creation_date(cdate);
            }
            if let Some(mdate) = parse_pdf_date(mod_str) {
                info.modified_date(mdate);
            }

            match options.standard {
                PdfXStandard::PdfX1a2001 => {
                    info.pair(Name(b"GTS_PDFXVersion"), TextStr("PDF/X-1:2001"));
                    info.pair(Name(b"GTS_PDFXConformance"), TextStr("PDF/X-1a:2001"));
                }
                PdfXStandard::PdfX4 => {
                    info.pair(Name(b"GTS_PDFXVersion"), TextStr("PDF/X-4"));
                }
                PdfXStandard::Plain => {}
            }
        }
        // PDF/X-1a:2001 is based on PDF 1.3, PDF/X-4 on PDF 1.6.
        match options.standard {
            PdfXStandard::PdfX1a2001 => pdf.set_version(1, 3),
            PdfXStandard::PdfX4 | PdfXStandard::Plain => pdf.set_version(1, 6),
        }

        // 4. Font objects
        let mut type0_ids = Vec::with_capacity(embedded.len());
        let mut id_hash = FNV_OFFSET;
        for font in &embedded {
            let ids = [alloc(), alloc(), alloc(), alloc(), alloc()];
            type0_ids.push(ids[0]);
            Self::write_font(&mut pdf, &font.subset, ids, options.compress_streams);
            id_hash = fnv1a(id_hash, font.subset.font_name.as_bytes());
        }

        // 4b. Collect Spot colors across all pages and emit /Separation color spaces
        let mut spot_colors: BTreeMap<String, (String, Ref, CmykColor)> = BTreeMap::new();
        let register_spot = |color: &Color,
                             map: &mut BTreeMap<String, (String, Ref, CmykColor)>,
                             alloc: &mut dyn FnMut() -> Ref| {
            if let Color::Spot {
                name,
                fallback_cmyk,
                ..
            } = color
            {
                if !name.is_empty() && !map.contains_key(name) {
                    let idx = map.len() + 1;
                    let res_name = format!("CS_Spot{}", idx);
                    let cs_ref = alloc();
                    let fallback = ColorManagementEngine::enforce_tac_limit(
                        CmykColor::new(
                            fallback_cmyk.0,
                            fallback_cmyk.1,
                            fallback_cmyk.2,
                            fallback_cmyk.3,
                        ),
                        options.color_profile.max_tac_percent(),
                    );
                    map.insert(name.clone(), (res_name, cs_ref, fallback));
                }
            }
        };
        for p in iter_pages {
            for f in &p.frames {
                for l in &f.lines {
                    for c in &l.colors {
                        register_spot(c, &mut spot_colors, &mut alloc);
                    }
                }
            }
            for orn in &p.ornaments {
                register_spot(&orn.color, &mut spot_colors, &mut alloc);
            }
        }
        for (spot_name, (_, cs_ref, fallback)) in &spot_colors {
            let mut sep = pdf
                .color_space(*cs_ref)
                .separation(Name(spot_name.as_bytes()));
            sep.alternate_color_space().device_cmyk();
            sep.tint_exponential()
                .domain([0.0, 1.0])
                .c0([0.0, 0.0, 0.0, 0.0])
                .c1([fallback.c, fallback.m, fallback.y, fallback.k])
                .n(1.0);
            sep.finish();
        }

        // 4c. Collect and emit Image XObjects (+ /SMask in PDF/X-4)
        let is_pdf_x1a = matches!(options.standard, PdfXStandard::PdfX1a2001);
        let lookup_image_bytes = |asset_path: &str| -> Option<&[u8]> {
            let norm = asset_path
                .trim_start_matches('/')
                .trim_start_matches("assets/");
            options
                .image_assets
                .get(asset_path)
                .or_else(|| options.image_assets.get(norm))
                .or_else(|| options.image_assets.get(&format!("images/{}", norm)))
                .map(Vec::as_slice)
        };
        let mut image_xobjects: BTreeMap<String, (String, Ref)> = BTreeMap::new();
        for p_box in iter_pages {
            if p_box.is_blank {
                continue;
            }
            for placed_img in &p_box.images {
                if image_xobjects.contains_key(&placed_img.asset_path) {
                    continue;
                }
                let (base_asset_path, page_index) = parse_asset_page_suffix(&placed_img.asset_path);
                if let Some(raw_bytes) = lookup_image_bytes(base_asset_path) {
                    if let Ok(prep) = encode_image_xobject_data_for_page(
                        raw_bytes,
                        page_index,
                        is_pdf_x1a,
                        options.color_profile,
                    ) {
                        let smask_ref = prep.smask_flate_data.as_ref().map(|smask_bytes| {
                            let s_ref = alloc();
                            let mut sm = pdf.image_xobject(s_ref, smask_bytes);
                            sm.width(prep.width as i32);
                            sm.height(prep.height as i32);
                            sm.color_space().device_gray();
                            sm.bits_per_component(8);
                            sm.filter(Filter::FlateDecode);
                            sm.finish();
                            s_ref
                        });

                        let icc_ref = if !is_pdf_x1a {
                            prep.icc_profile.as_ref().map(|icc_raw| {
                                let c_ref = alloc();
                                let n_components = match prep.color_space {
                                    PreparedColorSpace::DeviceCmyk => 4i32,
                                    PreparedColorSpace::DeviceGray => 1i32,
                                    PreparedColorSpace::CalRgb => 3i32,
                                };
                                let compressed_icc =
                                    miniz_oxide::deflate::compress_to_vec_zlib(icc_raw, 6);
                                let mut icc_stream = pdf.stream(c_ref, &compressed_icc);
                                icc_stream.filter(Filter::FlateDecode);
                                icc_stream.pair(Name(b"N"), n_components);
                                icc_stream.finish();
                                c_ref
                            })
                        } else {
                            None
                        };

                        let img_ref = alloc();
                        let res_name = format!("Im{}", image_xobjects.len() + 1);
                        let mut xobj = pdf.image_xobject(img_ref, &prep.data);
                        xobj.width(prep.width as i32);
                        xobj.height(prep.height as i32);
                        xobj.bits_per_component(prep.bits_per_component);
                        if let Some(c_ref) = icc_ref {
                            xobj.color_space().icc_based(c_ref);
                        } else {
                            match prep.color_space {
                                PreparedColorSpace::DeviceCmyk => {
                                    xobj.color_space().device_cmyk();
                                }
                                PreparedColorSpace::DeviceGray => {
                                    xobj.color_space().device_gray();
                                }
                                PreparedColorSpace::CalRgb => {
                                    xobj.color_space().srgb();
                                }
                            }
                        }
                        if prep.cmyk_decode_inverted {
                            xobj.decode([1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0]);
                        }
                        if let Some(s_ref) = smask_ref {
                            xobj.s_mask(s_ref);
                        }
                        xobj.filter(if prep.is_dct_jpeg {
                            Filter::DctDecode
                        } else {
                            Filter::FlateDecode
                        });
                        xobj.finish();
                        image_xobjects.insert(placed_img.asset_path.clone(), (res_name, img_ref));
                    }
                }
            }
        }

        // 5. Pages and Content Streams
        let mut page_ids = Vec::with_capacity(iter_pages.len());
        for p_box in iter_pages {
            let page_id = alloc();
            let content_id = alloc();
            page_ids.push(page_id);

            let page_boxes = PrePressPageBoxes::new(
                p_box.dimensions.width,
                p_box.dimensions.height,
                options.bleed_pt,
                options.slug_pt,
            );

            let content = Self::page_content(
                p_box,
                &page_boxes,
                options.draw_crop_marks,
                options.draw_registration_marks,
                options.draw_page_numbers,
                options.color_profile,
                &spot_colors,
                &image_xobjects,
                &options.ornament_assets,
                &embedded,
                &registry,
                &legacy_face,
            );
            // Fingerprint (not a full hash: content streams can be huge).
            id_hash = fnv1a(id_hash, &(content.len() as u64).to_le_bytes());
            id_hash = fnv1a(id_hash, &content[..content.len().min(256)]);
            id_hash = fnv1a(id_hash, &content[content.len().saturating_sub(256)..]);
            if options.compress_streams {
                let compressed = miniz_oxide::deflate::compress_to_vec_zlib(&content, 6);
                pdf.stream(content_id, &compressed)
                    .filter(Filter::FlateDecode);
            } else {
                pdf.stream(content_id, &content);
            }

            let mut page = pdf.page(page_id);
            page.parent(pages_id);
            let rect = |b: [f32; 4]| Rect::new(b[0], b[1], b[2], b[3]);
            page.media_box(rect(page_boxes.media_box));
            page.bleed_box(rect(page_boxes.bleed_box));
            page.trim_box(rect(page_boxes.trim_box));
            page.crop_box(rect(page_boxes.crop_box));

            let mut resources = page.resources();
            let mut font_dict = resources.fonts();
            for (font, id) in embedded.iter().zip(&type0_ids) {
                font_dict.pair(Name(font.resource.as_bytes()), *id);
            }
            font_dict.finish();
            if !spot_colors.is_empty() {
                let mut cs_dict = resources.color_spaces();
                for (res_name, cs_ref, _) in spot_colors.values() {
                    cs_dict.pair(Name(res_name.as_bytes()), *cs_ref);
                }
                cs_dict.finish();
            }
            if !image_xobjects.is_empty() {
                let mut xo_dict = resources.x_objects();
                for (res_name, img_ref) in image_xobjects.values() {
                    xo_dict.pair(Name(res_name.as_bytes()), *img_ref);
                }
                xo_dict.finish();
            }
            resources.finish();

            page.contents(content_id);
            page.finish();
        }

        // 6. Pages tree
        let mut pages_node = pdf.pages(pages_id);
        pages_node.count(page_ids.len() as i32);
        pages_node.kids(page_ids.iter().copied());
        pages_node.finish();

        // 7. Embedded ICC DestOutputProfile Stream (N = 4 CMYK components) & Catalog OutputIntent
        let icc_profile_id = alloc();
        let (intent_info, icc_raw) = if let Some(ext_bytes) = options.external_icc_profile.as_ref()
        {
            let parsed = IccProfile::from_bytes(ext_bytes.clone())?;
            let desc = parsed.description().to_string();
            (desc, parsed.raw_bytes().to_vec())
        } else {
            let profile_info = options.color_profile.info();
            (
                format!("TypesetOK approximation of {profile_info}"),
                ColorManagementEngine::generate_icc_profile_bytes(options.color_profile),
            )
        };
        if options.compress_streams {
            let compressed_icc = miniz_oxide::deflate::compress_to_vec_zlib(&icc_raw, 6);
            let mut icc_stream = pdf.stream(icc_profile_id, &compressed_icc);
            icc_stream.filter(Filter::FlateDecode);
            icc_stream.pair(Name(b"N"), 4i32);
            icc_stream.finish();
        } else {
            let mut icc_stream = pdf.stream(icc_profile_id, &icc_raw);
            icc_stream.pair(Name(b"N"), 4i32);
            icc_stream.finish();
        }

        // 8. Deterministic file identifier (required by PDF/X); also the XMP DocumentID.
        id_hash = fnv1a(id_hash, effective_title.as_bytes());
        id_hash = fnv1a(id_hash, creation_str.as_bytes());
        id_hash = fnv1a(id_hash, mod_str.as_bytes());
        let file_id = [id_hash.to_be_bytes(), fnv1a(id_hash, b"tok").to_be_bytes()].concat();

        // XMP metadata: required by PDF/X-4; PDF 1.3 (PDF/X-1a) has no /Metadata entry.
        let xmp_id = if matches!(options.standard, PdfXStandard::PdfX4) {
            let id = alloc();
            let xmp = build_xmp(
                effective_title,
                &options.author,
                &xmp_date(creation_str),
                &xmp_date(mod_str),
                &uuid_urn(&file_id),
            );
            let mut xmp_stream = pdf.stream(id, xmp.as_bytes());
            xmp_stream.pair(Name(b"Type"), Name(b"Metadata"));
            xmp_stream.pair(Name(b"Subtype"), Name(b"XML"));
            xmp_stream.finish();
            Some(id)
        } else {
            None
        };

        let mut catalog = pdf.catalog(catalog_id);
        catalog.pages(pages_id);
        if let Some(id) = xmp_id {
            catalog.pair(Name(b"Metadata"), id);
        }
        let mut catalog_intents = catalog.output_intents();
        let mut intent = catalog_intents.push();
        intent.subtype(OutputIntentSubtype::PDFX);
        // The embedded profile is a generic approximation, not the characterisation of the
        // registered press condition it is modelled on. PDF/X allows a custom output condition
        // when the profile is embedded, so declare one instead of a registered identifier.
        intent.output_condition_identifier(TextStr("Custom"));
        intent.info(TextStr(&intent_info));
        intent.dest_output_profile(icc_profile_id);
        intent.finish();
        catalog_intents.finish();
        catalog.finish();

        // 9. File identifier (computed earlier, it is also the XMP DocumentID).
        pdf.set_file_id((file_id.clone(), file_id));

        Ok(pdf.finish())
    }

    fn write_font(pdf: &mut Pdf, subset: &SubsetFontResult, ids: [Ref; 5], compress: bool) {
        let [type0_font_id, cid_font_id, descriptor_id, font_file_id, tounicode_id] = ids;
        let base_font = Name(subset.font_name.as_bytes());

        // FontFile2 Stream
        if compress {
            let compressed_font =
                miniz_oxide::deflate::compress_to_vec_zlib(&subset.subset_bytes, 6);
            let mut font_stream = pdf.stream(font_file_id, &compressed_font);
            font_stream.filter(Filter::FlateDecode);
            font_stream.pair(Name(b"Length1"), subset.subset_bytes.len() as i32);
            font_stream.finish();
        } else {
            let mut font_stream = pdf.stream(font_file_id, &subset.subset_bytes);
            font_stream.pair(Name(b"Length1"), subset.subset_bytes.len() as i32);
            font_stream.finish();
        }

        // FontDescriptor
        let mut descriptor = pdf.font_descriptor(descriptor_id);
        descriptor.name(base_font);
        descriptor.flags(FontFlags::SYMBOLIC);
        descriptor.bbox(Rect::new(
            subset.bbox_1000[0],
            subset.bbox_1000[1],
            subset.bbox_1000[2],
            subset.bbox_1000[3],
        ));
        descriptor.italic_angle(subset.italic_angle);
        descriptor.ascent(subset.ascender_1000);
        descriptor.descent(subset.descender_1000);
        descriptor.cap_height(subset.cap_height_1000);
        descriptor.stem_v(80.0);
        descriptor.font_file2(font_file_id);
        descriptor.finish();

        // CIDFont (Descendant Font)
        let mut cid_font = pdf.cid_font(cid_font_id);
        cid_font.subtype(CidFontType::Type2);
        cid_font.base_font(base_font);
        cid_font.system_info(SystemInfo {
            registry: Str(b"Adobe"),
            ordering: Str(b"Identity"),
            supplement: 0,
        });
        cid_font.font_descriptor(descriptor_id);
        cid_font.default_width(1000.0);
        cid_font
            .widths()
            .consecutive(0, subset.cid_widths_1000.iter().copied());
        cid_font.cid_to_gid_map_predefined(Name(b"Identity"));
        cid_font.finish();

        // /ToUnicode CMap Stream
        let cmap_bytes = ToUnicodeCMap::generate(&subset.to_unicode_map);
        if compress {
            let compressed_cmap = miniz_oxide::deflate::compress_to_vec_zlib(&cmap_bytes, 6);
            pdf.stream(tounicode_id, &compressed_cmap)
                .filter(Filter::FlateDecode);
        } else {
            pdf.stream(tounicode_id, &cmap_bytes);
        }

        // Type 0 Font
        let mut type0 = pdf.type0_font(type0_font_id);
        type0.base_font(base_font);
        type0.encoding_predefined(Name(b"Identity-H"));
        type0.descendant_font(cid_font_id);
        type0.to_unicode(tounicode_id);
        type0.finish();
    }

    fn apply_fill_color(
        content: &mut Content,
        color: &Color,
        profile: IccProfileId,
        spot_colors: &BTreeMap<String, (String, Ref, CmykColor)>,
    ) {
        if let Color::Spot { name, tint, .. } = color {
            if let Some((res_name, _, _)) = spot_colors.get(name) {
                content.set_fill_color_space(ColorSpaceOperand::Named(Name(res_name.as_bytes())));
                let safe_tint = if tint.is_finite() {
                    tint.clamp(0.0, 1.0)
                } else {
                    1.0
                };
                content.set_fill_color([safe_tint]);
                return;
            }
        }
        let cmyk = ColorManagementEngine::convert_color(color, profile);
        content.set_fill_cmyk(cmyk.c, cmyk.m, cmyk.y, cmyk.k);
    }

    fn apply_stroke_color(
        content: &mut Content,
        color: &Color,
        profile: IccProfileId,
        spot_colors: &BTreeMap<String, (String, Ref, CmykColor)>,
    ) {
        if let Color::Spot { name, tint, .. } = color {
            if let Some((res_name, _, _)) = spot_colors.get(name) {
                content.set_stroke_color_space(ColorSpaceOperand::Named(Name(res_name.as_bytes())));
                let safe_tint = if tint.is_finite() {
                    tint.clamp(0.0, 1.0)
                } else {
                    1.0
                };
                content.set_stroke_color([safe_tint]);
                return;
            }
        }
        let cmyk = ColorManagementEngine::convert_color(color, profile);
        content.set_stroke_cmyk(cmyk.c, cmyk.m, cmyk.y, cmyk.k);
    }

    #[allow(clippy::too_many_arguments)]
    fn page_content(
        p_box: &PageLayoutBox,
        page_boxes: &PrePressPageBoxes,
        draw_crop_marks: bool,
        draw_registration_marks: bool,
        draw_page_numbers: bool,
        color_profile: IccProfileId,
        spot_colors: &BTreeMap<String, (String, Ref, CmykColor)>,
        image_xobjects: &BTreeMap<String, (String, Ref)>,
        ornament_assets: &BTreeMap<String, String>,
        embedded: &[EmbeddedFont],
        registry: &FontRegistry<'_>,
        legacy_face: &ttf_parser::Face<'_>,
    ) -> Vec<u8> {
        let mut content = Content::new();

        if draw_crop_marks {
            page_boxes.draw_crop_marks(&mut content);
        }
        if draw_registration_marks {
            page_boxes.draw_registration_marks(&mut content);
        }

        if p_box.is_blank {
            // A blank verso page suppresses all content, footnotes, running headers, and page numbers,
            // while preserving prepress crop marks and registration marks for sheet imposition.
            return content.finish();
        }

        // Set fill color to pure DeviceCMYK black (0 0 0 1 k)
        let default_black = Color::default();
        content.set_fill_cmyk(0.0, 0.0, 0.0, 1.0);
        let mut active_fill: Color = default_black.clone();

        // Origin offset by slug_pt to align inside TrimBox
        let origin_x = page_boxes.trim_box[0];
        let origin_y = page_boxes.trim_box[1];
        let page_h = p_box.dimensions.height;

        // Draw footnote apparatus separator rules (main rule above first apparatus, shorter rule between apparatuses)
        let mut fn_frames: Vec<&tok_typeset::geometry::TextFrameBox> = p_box
            .frames
            .iter()
            .filter(|f| {
                !f.lines.is_empty()
                    && (f.flow_id == "notes"
                        || f.flow_id == "footnote"
                        || f.flow_id == "sources"
                        || f.flow_id == "biurim"
                        || f.flow_id.contains("note")
                        || f.flow_id.contains("apparatus"))
            })
            .collect();
        fn_frames.sort_by(|a, b| a.rect.y.total_cmp(&b.rect.y));
        for (app_idx, fn_frame) in fn_frames.iter().enumerate() {
            let ratio = if app_idx == 0 { 0.33_f32 } else { 0.18_f32 };
            let line_w = if app_idx == 0 { 0.5_f32 } else { 0.35_f32 };
            let rule_len = (fn_frame.rect.width * ratio).max(24.0);
            let rule_x_end = origin_x + fn_frame.rect.x + fn_frame.rect.width;
            let rule_x_start = rule_x_end - rule_len;
            let rule_y = origin_y + (page_h - fn_frame.rect.y) + 2.5;
            content.set_line_width(line_w);
            content.set_stroke_cmyk(0.0, 0.0, 0.0, 1.0);
            content.move_to(rule_x_start, rule_y);
            content.line_to(rule_x_end, rule_y);
            content.stroke();
        }

        // Draw vertical column separator rules computed by the layout engine
        for &(x1, y1, x2, y2) in &p_box.column_rules {
            content.set_line_width(0.4);
            content.set_stroke_cmyk(0.0, 0.0, 0.0, 0.65);
            content.move_to(origin_x + x1, origin_y + (page_h - y1));
            content.line_to(origin_x + x2, origin_y + (page_h - y2));
            content.stroke();
        }

        for frame in &p_box.frames {
            for line in &frame.lines {
                let base_y = origin_y + (page_h - (frame.rect.y + line.baseline_y));

                if line.glyphs.is_empty() {
                    // Text-only line: a single run at the line origin.
                    let Some(font) = registry.legacy.map(|slot| &embedded[slot]) else {
                        continue;
                    };
                    let cids: Vec<u8> = line
                        .text
                        .chars()
                        .filter_map(|ch| legacy_face.glyph_index(ch))
                        .filter_map(|gid| font.cid(gid.0))
                        .flat_map(u16::to_be_bytes)
                        .collect();
                    if cids.is_empty() {
                        continue;
                    }
                    let line_color = line.colors.first().unwrap_or(&default_black);
                    if *line_color != active_fill {
                        Self::apply_fill_color(
                            &mut content,
                            line_color,
                            color_profile,
                            spot_colors,
                        );
                        active_fill = line_color.clone();
                    }
                    content.begin_text();
                    content.set_font(Name(font.resource.as_bytes()), line.height);
                    content.set_text_matrix([1.0, 0.0, 0.0, 1.0, origin_x + frame.rect.x, base_y]);
                    content.show(Str(&cids));
                    content.end_text();
                    continue;
                }

                // Every family was registered in the collection pass.
                let line_slots: Vec<usize> = line
                    .fonts
                    .iter()
                    .map_while(|name| registry.by_family.get(name.as_str()).copied())
                    .collect();

                // One text object and one text matrix per line. Glyphs are
                // placed with TJ adjustments relative to the running pen, and
                // vertical mark offsets with the text rise, so the stream does
                // not need a matrix per glyph.
                let mut underlines: Vec<(f32, f32, f32, f32, Color)> = Vec::new();
                content.begin_text();
                let mut writer = LineTextWriter::default();
                for g in &line.glyphs {
                    let Some((slot, gid)) =
                        glyph_target(line, g, &line_slots, || registry.legacy, legacy_face)
                    else {
                        continue;
                    };
                    let Some(font) = embedded.get(slot) else {
                        continue;
                    };
                    let Some(cid) = font.cid(gid) else {
                        continue;
                    };
                    let size = g.height;
                    if !(size.is_finite() && size > 0.0) {
                        continue;
                    }
                    let gx = origin_x + frame.rect.x + g.x;
                    if writer.pen.is_none() {
                        content.set_text_matrix([1.0, 0.0, 0.0, 1.0, gx, base_y]);
                        writer.pen = Some(gx);
                    }
                    let glyph_color = line.color_of(g).unwrap_or(&default_black);
                    if *glyph_color != active_fill {
                        writer.flush(&mut content);
                        Self::apply_fill_color(
                            &mut content,
                            glyph_color,
                            color_profile,
                            spot_colors,
                        );
                        active_fill = glyph_color.clone();
                    }
                    if writer.font != Some((slot, size)) {
                        writer.flush(&mut content);
                        content.set_font(Name(font.resource.as_bytes()), size);
                        writer.font = Some((slot, size));
                    }
                    if writer.rise != g.y {
                        writer.flush(&mut content);
                        content.set_rise(g.y);
                        writer.rise = g.y;
                    }
                    if g.underline && g.width > 0.0 {
                        let uy = base_y - size * 0.12;
                        let thickness = (size * 0.06).max(0.4);
                        if let Some(last) = underlines.last_mut() {
                            if (last.1 - gx).abs() <= 0.5
                                && (last.2 - uy).abs() <= 0.2
                                && (last.3 - thickness).abs() <= 0.1
                                && last.4 == *glyph_color
                            {
                                last.1 = gx + g.width;
                            } else {
                                underlines.push((
                                    gx,
                                    gx + g.width,
                                    uy,
                                    thickness,
                                    glyph_color.clone(),
                                ));
                            }
                        } else {
                            underlines.push((gx, gx + g.width, uy, thickness, glyph_color.clone()));
                        }
                    }
                    let advance_1000 = font
                        .subset
                        .cid_widths_1000
                        .get(usize::from(cid))
                        .copied()
                        .unwrap_or(0.0);
                    if g.stretch > 0.0 && g.stretch.is_finite() {
                        // Justification Tier 2: draw the Ahalterm letter horizontally widened.
                        // The widening is the text matrix's x scale, applied from the glyph
                        // origin, so the layout advance (already in `g.width`) is untouched;
                        // the next glyph starts a fresh, unscaled matrix.
                        writer.flush(&mut content);
                        content.set_text_matrix([1.0 + g.stretch, 0.0, 0.0, 1.0, gx, base_y]);
                        writer.pen = Some(gx);
                        writer.push(gx, cid, advance_1000, size);
                        writer.flush(&mut content);
                        writer.pen = None;
                    } else {
                        writer.push(gx, cid, advance_1000, size);
                    }
                }
                writer.flush(&mut content);
                if writer.rise != 0.0 {
                    // Text rise is graphics state: do not leak it to the next line.
                    content.set_rise(0.0);
                }
                content.end_text();

                for (x0, x1, uy, thickness, ucolor) in underlines {
                    content.set_line_width(thickness);
                    Self::apply_stroke_color(&mut content, &ucolor, color_profile, spot_colors);
                    content.move_to(x0, uy);
                    content.line_to(x1, uy);
                    content.stroke();
                }
            }
        }

        // Draw page number folio if enabled (either via PdfExportOptions or PageNumberRenderSpec)
        let should_draw_folio = (draw_page_numbers || p_box.page_number_style.is_some())
            && !p_box.page_number_gematria.is_empty();
        if should_draw_folio {
            if active_fill != default_black {
                content.set_fill_cmyk(0.0, 0.0, 0.0, 1.0);
            }
            if let Some(slot) = registry.legacy {
                let font = &embedded[slot];
                let style_spec = p_box.page_number_style.as_ref();
                let folio_font_size = style_spec.map(|s| s.font_size_pt.max(6.0)).unwrap_or(10.0);
                // Each glyph with its advance, in logical (reading) order.
                let glyphs: Vec<(u16, f32)> = p_box
                    .page_number_gematria
                    .chars()
                    .filter_map(|ch| legacy_face.glyph_index(ch))
                    .filter_map(|gid| font.cid(gid.0))
                    .map(|cid| {
                        let adv = font
                            .subset
                            .cid_widths_1000
                            .get(usize::from(cid))
                            .copied()
                            .unwrap_or(500.0);
                        (cid, adv / 1000.0 * folio_font_size)
                    })
                    .collect();
                if !glyphs.is_empty() {
                    let total_width: f32 = glyphs.iter().map(|(_, adv)| *adv).sum();
                    let (left_x, folio_y) = if let Some(spec) = style_spec {
                        if spec.embed_in_ornament {
                            if let Some(ref orn_rect) = spec.ornament_rect {
                                let cx =
                                    origin_x + orn_rect.x + (orn_rect.width - total_width) / 2.0;
                                // Convert top-down orn_rect.y to PDF bottom-up y, centering text vertically.
                                let cy = origin_y + (page_h - (orn_rect.y + orn_rect.height / 2.0))
                                    - folio_font_size * 0.35;
                                (cx, cy)
                            } else {
                                (
                                    origin_x + (p_box.dimensions.width - total_width) / 2.0,
                                    origin_y + 25.0,
                                )
                            }
                        } else {
                            let is_top =
                                spec.placement == "top-center" || spec.placement == "top-outer";
                            let is_outer =
                                spec.placement == "top-outer" || spec.placement == "bottom-outer";
                            let is_right_page = p_box.page_index % 2 == 1;
                            let x = if !is_outer {
                                origin_x + (p_box.dimensions.width - total_width) / 2.0
                            } else if is_right_page {
                                origin_x + (p_box.dimensions.width - 22.0 - total_width).max(0.0)
                            } else {
                                origin_x + 22.0
                            };
                            let y = if is_top {
                                origin_y + page_h - 16.0 - folio_font_size
                            } else {
                                origin_y + 25.0
                            };
                            (x, y)
                        }
                    } else {
                        (
                            origin_x + (p_box.dimensions.width - total_width) / 2.0,
                            origin_y + 25.0,
                        )
                    };
                    // A Hebrew numeral (e.g. "י״א") reads right to left. The glyphs stay in
                    // logical order in the stream, so copy and search give the right text, and
                    // each one is placed explicitly so the geresh and gershayim land on the left.
                    let is_rtl = p_box
                        .page_number_gematria
                        .chars()
                        .any(|ch| ('\u{0590}'..='\u{05FF}').contains(&ch));
                    let mut pen_x = if is_rtl { left_x + total_width } else { left_x };
                    content.begin_text();
                    content.set_font(Name(font.resource.as_bytes()), folio_font_size);
                    for (cid, adv) in &glyphs {
                        if is_rtl {
                            pen_x -= *adv;
                        }
                        content.set_text_matrix([1.0, 0.0, 0.0, 1.0, pen_x, folio_y]);
                        content.show(Str(&cid.to_be_bytes()));
                        if !is_rtl {
                            pen_x += *adv;
                        }
                    }
                    content.end_text();
                }
            }
        }

        // Draw placed raster images using emitted XObject resources
        for placed_img in &p_box.images {
            if let Some((res_name, _)) = image_xobjects.get(&placed_img.asset_path) {
                let w = placed_img.rect.width.max(1.0);
                let h = placed_img.rect.height.max(1.0);
                let pdf_x = origin_x + placed_img.rect.x;
                let pdf_y = origin_y + (page_h - (placed_img.rect.y + h));
                content
                    .save_state()
                    .transform([w, 0.0, 0.0, h, pdf_x, pdf_y])
                    .x_object(Name(res_name.as_bytes()))
                    .restore_state();
            }
        }

        // Compile and emit placed vector ornaments (custom asset SVG or built-in sacred preset)
        for orn in &p_box.ornaments {
            let custom_svg = orn.asset_path.as_ref().and_then(|ap| {
                let norm = ap.trim_start_matches('/').trim_start_matches("assets/");
                ornament_assets
                    .get(ap)
                    .or_else(|| ornament_assets.get(norm))
                    .or_else(|| ornament_assets.get(&format!("ornaments/{}", norm)))
                    .map(String::as_str)
            });
            let svg_opt = custom_svg.or_else(|| BuiltinOrnaments::get_svg(&orn.ornament_id));
            if let Some(svg) = svg_opt {
                let w = orn.rect.width.max(1.0);
                let h = orn.rect.height.max(1.0);
                let pdf_x = origin_x + orn.rect.x;
                let pdf_y = origin_y + (page_h - (orn.rect.y + h));
                let _ = SvgVectorCompiler::compile_into_content(
                    svg,
                    pdf_x,
                    pdf_y,
                    w,
                    h,
                    &orn.color,
                    color_profile,
                    spot_colors,
                    &mut content,
                );
            }
        }

        content.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tok_core::id::FractionalIndex;
    use tok_core::model::ParagraphNode;
    use tok_typeset::engine::{TypesettingEngine, TypesettingEngineConfig};
    use tok_typeset::geometry::TextFrameBox;

    fn page_with(lines: Vec<LineBox>) -> PageLayoutBox {
        PageLayoutBox {
            page_index: 0,
            page_number_gematria: "א׳".to_string(),
            dimensions: PhysicalRect::a4_portrait(),
            frames: vec![TextFrameBox {
                frame_id: "frame_1".to_string(),
                flow_id: "main".to_string(),
                rect: PhysicalRect::new(50.0, 50.0, 400.0, 600.0),
                lines,
            }],
            break_token: None,
            is_blank: false,
            ..Default::default()
        }
    }

    fn typeset(text: &str, family: &str) -> Vec<LineBox> {
        let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
        let para = ParagraphNode::new(FractionalIndex::initial(), "default-body", text);
        engine.typeset_paragraph_with_font(&para, family, 400.0, 12.0, 16.0)
    }

    fn count(haystack: &[u8], needle: &[u8]) -> usize {
        haystack
            .windows(needle.len())
            .filter(|w| *w == needle)
            .count()
    }

    fn decompress_flate_streams(pdf_bytes: &[u8]) -> Vec<Vec<u8>> {
        let mut results = Vec::new();
        let stream_tag = b"stream\n";
        let endstream_tag = b"\nendstream";
        let mut offset = 0;
        while let Some(start_rel) = pdf_bytes[offset..]
            .windows(stream_tag.len())
            .position(|w| w == stream_tag)
        {
            let start = offset + start_rel + stream_tag.len();
            if let Some(end_rel) = pdf_bytes[start..]
                .windows(endstream_tag.len())
                .position(|w| w == endstream_tag)
            {
                let stream_data = &pdf_bytes[start..start + end_rel];
                if let Ok(decomp) = miniz_oxide::inflate::decompress_to_vec_zlib(stream_data) {
                    results.push(decomp);
                } else {
                    results.push(stream_data.to_vec());
                }
                offset = start + end_rel + endstream_tag.len();
            } else {
                break;
            }
        }
        results
    }

    #[test]
    fn test_pdf_export_structure() {
        let line = LineBox {
            line_index: 0,
            paragraph_id: None,
            baseline_y: 20.0,
            height: 14.5,
            width: 300.0,
            x_offset: 0.0,
            is_drop_cap: false,
            glyphs: Vec::new(),
            text: "שלום עולם".to_string(),
            is_rtl: true,
            fonts: Vec::new(),
            ..Default::default()
        };

        let bytes =
            PdfPrePressEngine::export_pdf(&[page_with(vec![line])], &PdfExportOptions::default())
                .unwrap();

        assert!(!bytes.is_empty());
        let s = String::from_utf8_lossy(&bytes);
        assert!(s.starts_with("%PDF-1.3"));
        assert!(s.contains("GTS_PDFX"));
        assert!(s.contains("/GTS_PDFXConformance (PDF/X-1a:2001)"));
        assert!(s.contains("/Trapped /False"));
        assert!(s.contains("/ID ["));
        assert!(s.contains("/TrimBox"));
        assert!(s.contains("/BleedBox"));
        assert!(s.contains("FOGRA39"));
        assert!(s.contains("/Type0"), "Must embed Type 0 font");
        assert!(s.contains("/CIDFontType2"), "Must embed CIDFont Type 2");
        assert!(s.contains("/FontDescriptor"), "Must embed FontDescriptor");
        assert!(s.contains("/Identity-H"), "Must use Identity-H encoding");
        assert!(
            s.contains("/Filter /FlateDecode"),
            "Must use FlateDecode compression"
        );
        let decompressed = decompress_flate_streams(&bytes);
        let bt_count: usize = decompressed.iter().map(|st| count(st, b"BT\n")).sum();
        assert_eq!(bt_count, 1);
    }

    #[test]
    fn test_pdf_export_with_typeset_glyphs() {
        let lines = typeset("מֵאֵימָתַי קוֹרִין אֶת שְׁמַע בְּעַרְבִית?", "Noto Serif Hebrew");
        assert!(!lines.is_empty());
        assert!(!lines[0].glyphs.is_empty());

        let bytes =
            PdfPrePressEngine::export_pdf(&[page_with(lines)], &PdfExportOptions::default())
                .unwrap();
        assert!(!bytes.is_empty());
        let s = String::from_utf8_lossy(&bytes);
        assert!(s.contains("/CIDToGIDMap"));
        assert!(s.contains("/ToUnicode"));
    }

    /// Regression: glyph ids shaped with David Libre (the style default, and
    /// the fallback for punctuation/Latin) were drawn with Noto Serif Hebrew.
    #[test]
    fn each_layout_font_is_embedded() {
        let mut lines = typeset("שלום עולם", "David CLM");
        lines.extend(typeset("שלום עולם", "Noto Serif Hebrew"));
        let bytes =
            PdfPrePressEngine::export_pdf(&[page_with(lines)], &PdfExportOptions::default())
                .unwrap();
        assert_eq!(count(&bytes, b"/Subtype /Type0"), 2);
        let s = String::from_utf8_lossy(&bytes);
        assert!(s.contains("+DavidLibre"), "David Libre must be embedded");
        assert!(s.contains("+NotoSerifHebrew"), "Noto must be embedded");
        assert!(s.contains("/F1 ") && s.contains("/F2 "));
    }

    #[test]
    fn ui_fonts_are_embedded_with_cantillation_fallback() {
        let mut lines = typeset("מֵאֵימָתַי קוֹרִין", "Frank Ruhl Libre");
        lines.extend(typeset("רַשִׁ״י", "Rashi"));
        lines.extend(typeset("בְּרֵאשִׁ֖ית", "Frank Ruhl Libre"));
        let bytes =
            PdfPrePressEngine::export_pdf(&[page_with(lines)], &PdfExportOptions::default())
                .unwrap();
        let s = String::from_utf8_lossy(&bytes);
        assert!(
            s.contains("+FrankRuhlLibre"),
            "Frank Ruhl Libre must be embedded"
        );
        assert!(
            s.contains("+NotoRashiHebrew"),
            "Noto Rashi Hebrew must be embedded"
        );
        assert!(
            s.contains("+NotoSerifHebrew"),
            "cantillation must fall back to Noto Serif Hebrew"
        );
        assert_eq!(count(&bytes, b"/Subtype /Type0"), 3);
    }

    #[test]
    fn bold_face_is_embedded_as_its_own_font() {
        let mut lines = typeset("מֵאֵימָתַי קוֹרִין", "Frank Ruhl Libre");
        lines.extend(typeset("מֵאֵימָתַי קוֹרִין", "Frank Ruhl Libre Bold"));
        let bytes =
            PdfPrePressEngine::export_pdf(&[page_with(lines)], &PdfExportOptions::default())
                .unwrap();
        let s = String::from_utf8_lossy(&bytes);
        assert!(s.contains("+FrankRuhlLibre-Regular"));
        assert!(s.contains("+FrankRuhlLibre-Bold"));
        assert_eq!(count(&bytes, b"/Subtype /Type0"), 2);
    }

    #[test]
    fn unknown_layout_font_is_an_error() {
        let mut lines = typeset("שלום", "Noto Serif Hebrew");
        lines[0].fonts = vec!["No Such Font".to_string()];
        let result =
            PdfPrePressEngine::export_pdf(&[page_with(lines)], &PdfExportOptions::default());
        assert!(matches!(result, Err(PdfError::MissingFont(_))));
    }

    /// Regression: invalid `custom_font_data` panicked inside the exporter.
    #[test]
    fn invalid_custom_font_is_an_error_not_a_panic() {
        let options = PdfExportOptions {
            custom_font_data: Some(b"definitely not a font".to_vec()),
            ..Default::default()
        };
        let result = PdfPrePressEngine::export_pdf(&[], &options);
        assert!(matches!(result, Err(PdfError::FontEmbedding { .. })));
    }

    #[test]
    fn oversized_and_unknown_glyphs_are_skipped() {
        let glyph = |id: u32, font_index: u16, ch: char| GlyphBox {
            glyph_id: id,
            cluster: 0,
            x: 0.0,
            y: 0.0,
            width: 5.0,
            height: 12.0,
            character: Some(ch),
            font_index,
            ..Default::default()
        };
        let line = LineBox {
            line_index: 0,
            paragraph_id: None,
            baseline_y: 20.0,
            height: 14.5,
            width: 10.0,
            x_offset: 0.0,
            is_drop_cap: false,
            glyphs: vec![
                glyph(70_000, 0, 'x'),
                glyph(0x05D0, tok_typeset::geometry::UNKNOWN_FONT, 'א'),
            ],
            text: "xא".to_string(),
            is_rtl: true,
            fonts: vec!["Noto Serif Hebrew".to_string()],
            ..Default::default()
        };
        let bytes =
            PdfPrePressEngine::export_pdf(&[page_with(vec![line])], &PdfExportOptions::default())
                .unwrap();
        assert!(bytes.starts_with(b"%PDF-"));
        // The heuristic glyph is drawn through the legacy font by character.
        // Ignore embedded font/resource streams: their binary bytes may contain
        // the byte sequence "TJ" even though it is not a PDF text operator.
        let decompressed = decompress_flate_streams(&bytes);
        // A PDF content stream starts its text object with the BT operator.
        // Do not require a particular newline after ET: pdf-writer may serialize
        // the closing operator with different whitespace.
        let text_streams = decompressed
            .iter()
            .filter(|stream| stream.windows(3).any(|window| window == b"BT\n"));
        let tj_count: usize = text_streams.map(|stream| count(stream, b"TJ")).sum();
        assert_eq!(
            tj_count, 1,
            "Expected exactly one text-show operator for the supported fallback glyph"
        );
    }

    #[test]
    fn export_is_deterministic_and_text_objects_are_per_line() {
        let lines = typeset(
            "בְּרֵאשִׁית בָּרָא אֱלֹהִים אֵת הַשָּׁמַיִם וְאֵת הָאָרֶץ TypesetOK 2026",
            "Noto Serif Hebrew",
        );
        let n_lines = lines.len();
        let pages = [page_with(lines)];
        let a = PdfPrePressEngine::export_pdf(&pages, &PdfExportOptions::default()).unwrap();
        let b = PdfPrePressEngine::export_pdf(&pages, &PdfExportOptions::default()).unwrap();
        assert_eq!(a, b);
        let decompressed = decompress_flate_streams(&a);
        let bt_count: usize = decompressed.iter().map(|st| count(st, b"BT\n")).sum();
        assert_eq!(bt_count, n_lines);
    }

    #[test]
    fn pdf_x4_header() {
        let options = PdfExportOptions {
            standard: PdfXStandard::PdfX4,
            ..Default::default()
        };
        let bytes = PdfPrePressEngine::export_pdf(&[], &options).unwrap();
        assert!(bytes.starts_with(b"%PDF-1.6"));
        assert!(String::from_utf8_lossy(&bytes).contains("/GTS_PDFXVersion (PDF/X-4)"));
    }

    #[test]
    fn test_parse_pdf_date() {
        let d = parse_pdf_date("2026-10-04T20:14:37Z").unwrap();
        let mut buf = Vec::new();
        pdf_writer::Primitive::write(d, &mut buf);
        assert_eq!(std::str::from_utf8(&buf).unwrap(), "(D:20261004201437Z)");

        let d_tz = parse_pdf_date("2026-10-04T20:14:37+02:00").unwrap();
        let mut buf_tz = Vec::new();
        pdf_writer::Primitive::write(d_tz, &mut buf_tz);
        assert_eq!(
            std::str::from_utf8(&buf_tz).unwrap(),
            "(D:20261004201437+02'00)"
        );

        let d_pdf = parse_pdf_date("D:20261004201437Z").unwrap();
        assert_eq!(d, d_pdf);
    }

    #[test]
    fn test_pdf_export_manifest_dates() {
        let options = PdfExportOptions {
            creation_date: Some("2026-10-04T20:14:37Z".to_string()),
            mod_date: Some("2026-10-04T22:30:00Z".to_string()),
            ..Default::default()
        };
        let bytes = PdfPrePressEngine::export_pdf(&[], &options).unwrap();
        let s = String::from_utf8_lossy(&bytes);
        assert!(s.contains("/CreationDate (D:20261004201437Z)"));
        assert!(s.contains("/ModDate (D:20261004223000Z)"));
    }

    #[test]
    fn test_flate_compression_reduces_size() {
        let lines = typeset(
            "בְּרֵאשִׁית בָּרָא אֱלֹהִים אֵת הַשָּׁמַיִם וְאֵת הָאָרֶץ TypesetOK 2026",
            "Noto Serif Hebrew",
        );
        let pages = [page_with(lines)];
        let uncompressed = PdfPrePressEngine::export_pdf(
            &pages,
            &PdfExportOptions {
                compress_streams: false,
                ..Default::default()
            },
        )
        .unwrap();
        let compressed = PdfPrePressEngine::export_pdf(
            &pages,
            &PdfExportOptions {
                compress_streams: true,
                ..Default::default()
            },
        )
        .unwrap();

        assert!(
            compressed.len() < uncompressed.len(),
            "Compressed PDF ({} bytes) must be smaller than uncompressed ({} bytes)",
            compressed.len(),
            uncompressed.len()
        );
        let s_comp = String::from_utf8_lossy(&compressed);
        let s_uncomp = String::from_utf8_lossy(&uncompressed);
        assert!(s_comp.contains("/Filter /FlateDecode"));
        assert!(!s_uncomp.contains("/Filter /FlateDecode"));
    }

    #[test]
    fn test_pdf_export_with_gematria_folio() {
        let lines = typeset("בדיקת מספר עמוד עברי", "Noto Serif Hebrew");
        let mut p = page_with(lines);
        p.page_number_gematria = "קמ״ה".to_string();

        let options = PdfExportOptions {
            draw_page_numbers: true,
            ..Default::default()
        };

        let bytes = PdfPrePressEngine::export_pdf(&[p], &options).unwrap();
        assert!(!bytes.is_empty());
        let decompressed = decompress_flate_streams(&bytes);
        // Expect at least 2 text objects: 1 for content line, 1 for folio
        let bt_count: usize = decompressed.iter().map(|st| count(st, b"BT\n")).sum();
        assert!(
            bt_count >= 2,
            "Must contain at least 2 text objects including folio, got {}",
            bt_count
        );
    }

    #[test]
    fn test_pdf_icc_output_intent_and_multi_apparatus_separator_rules() {
        use tok_typeset::geometry::{PhysicalRect, TextFrameBox};

        let line_main = typeset(
            "טקסט ראשי בעמוד עם שני מדורי הערות שוליים",
            "Noto Serif Hebrew",
        );
        let line_app1 = typeset("א. מקור ראשון במדור מקורות וציונים", "Noto Serif Hebrew");
        let line_app2 = typeset("1. ביאור ארוך במדור ביאורים והרחבות", "Noto Serif Hebrew");

        let page = PageLayoutBox {
            page_index: 0,
            page_number_gematria: "א".to_string(),
            dimensions: PhysicalRect::new(0.0, 0.0, 481.89, 680.31),
            frames: vec![
                TextFrameBox {
                    frame_id: "frame_0_main".to_string(),
                    flow_id: "main".to_string(),
                    rect: PhysicalRect::new(42.52, 51.02, 382.68, 420.0),
                    lines: line_main,
                },
                TextFrameBox {
                    frame_id: "frame_0_footnote_notes".to_string(),
                    flow_id: "notes".to_string(),
                    rect: PhysicalRect::new(42.52, 480.0, 382.68, 55.0),
                    lines: line_app1,
                },
                TextFrameBox {
                    frame_id: "frame_0_footnote_biurim".to_string(),
                    flow_id: "biurim".to_string(),
                    rect: PhysicalRect::new(42.52, 545.0, 382.68, 70.0),
                    lines: line_app2,
                },
            ],
            break_token: None,
            is_blank: false,
            ..Default::default()
        };

        let options = PdfExportOptions {
            standard: PdfXStandard::PdfX4,
            color_profile: IccProfileId::Gracol2013,
            draw_registration_marks: true,
            compress_streams: false,
            ..Default::default()
        };

        let bytes = PdfPrePressEngine::export_pdf(&[page], &options).unwrap();
        let s = String::from_utf8_lossy(&bytes);
        assert!(
            s.contains("/DestOutputProfile"),
            "Must embed DestOutputProfile ICC stream"
        );
        assert!(
            s.contains("/N 4"),
            "ICC stream must specify N=4 CMYK components"
        );
        assert!(
            s.contains("GRACoL 2013"),
            "Must embed GRACoL 2013 OutputIntent info"
        );
        assert!(
            s.contains("(Custom)") && !s.contains("CGATS TR 016") && !s.contains("color.org"),
            "The approximation profile must declare a custom, not a registered, condition"
        );
        assert!(
            s.contains("/Metadata") && s.contains("<pdfxid:GTS_PDFXVersion>PDF/X-4<"),
            "PDF/X-4 must carry an XMP packet with the PDF/X version"
        );
        assert!(
            s.contains("<xmpMM:DocumentID>uuid:"),
            "PDF/X-4 XMP must carry a DocumentID"
        );
        // Both separator line widths (0.5 w for primary apparatus, 0.35 w for secondary apparatus)
        assert!(
            s.contains("0.5 w"),
            "Must draw primary footnote separator rule"
        );
        assert!(
            s.contains("0.35 w"),
            "Must draw inter-apparatus separator rule"
        );
    }

    #[test]
    fn test_pdf_export_blank_verso_page_has_no_text_streams_but_retains_crop_marks() {
        let lines = typeset("שלום עולם", "Noto Serif Hebrew");
        let page1 = PageLayoutBox {
            page_index: 0,
            page_number_gematria: "א׳".to_string(),
            dimensions: PhysicalRect::a4_portrait(),
            frames: vec![TextFrameBox {
                frame_id: "main_0".to_string(),
                flow_id: "main".to_string(),
                rect: PhysicalRect::new(42.52, 42.52, 382.68, 500.0),
                lines,
            }],
            break_token: None,
            is_blank: false,
            ..Default::default()
        };
        let page2_blank = PageLayoutBox {
            page_index: 1,
            page_number_gematria: "ב׳".to_string(),
            dimensions: PhysicalRect::a4_portrait(),
            frames: Vec::new(),
            break_token: None,
            is_blank: true,
            ..Default::default()
        };

        let options = PdfExportOptions {
            standard: PdfXStandard::PdfX4,
            draw_registration_marks: true,
            compress_streams: false,
            ..Default::default()
        };

        let bytes = PdfPrePressEngine::export_pdf(&[page1, page2_blank], &options).unwrap();
        let s = String::from_utf8_lossy(&bytes);
        // Both pages must be present in the PDF page tree
        let page_count = s.matches("/Type /Page\n").count() + s.matches("/Type /Page\r").count();
        assert_eq!(
            page_count, 2,
            "PDF page tree must contain exactly 2 pages (recto + blank verso)"
        );
        // Only page 1 has text stream (BT ... ET)
        let bt_count = s.matches("BT\n").count() + s.matches("BT\r").count();
        assert_eq!(bt_count, 1, "Only non-blank page must have a text block");
        // Registration marks (e.g. crop mark path commands 'm' and 'l') should be drawn
        assert!(
            s.contains(" m\n") || s.contains(" l\n"),
            "Must contain crop mark vector paths"
        );
    }

    #[test]
    fn xmp_helpers_escape_text_and_normalise_dates_and_ids() {
        assert_eq!(
            xml_escape("a & b <c> \"d\" 'e'"),
            "a &amp; b &lt;c&gt; &quot;d&quot; &apos;e&apos;"
        );
        assert_eq!(xml_escape("a\u{1}b"), "ab");
        assert_eq!(xmp_date("2026-03-04"), "2026-03-04T00:00:00Z");
        assert_eq!(xmp_date("2026-03-04T05:06:07Z"), "2026-03-04T05:06:07Z");
        assert_eq!(xmp_date("2026-03-04 05:06:07"), "2026-03-04T05:06:07Z");
        assert_eq!(
            xmp_date("2026-03-04T05:06:07+02:00"),
            "2026-03-04T05:06:07+02:00"
        );
        assert_eq!(xmp_date("D:20260304050607"), "2026-03-04T05:06:07Z");
        assert_eq!(xmp_date("not a date"), "2026-01-01T00:00:00Z");
        let id: Vec<u8> = (0u8..16).collect();
        assert_eq!(uuid_urn(&id), "uuid:00010203-0405-0607-0809-0a0b0c0d0e0f");
    }

    #[test]
    fn pdf_x4_xmp_matches_the_info_dictionary_and_escapes_the_title() {
        let options = PdfExportOptions {
            standard: PdfXStandard::PdfX4,
            title: "A & B <ספר>".to_string(),
            compress_streams: false,
            creation_date: Some("2026-05-06T07:08:09Z".to_string()),
            ..Default::default()
        };
        let bytes = PdfPrePressEngine::export_pdf(&[], &options).unwrap();
        let s = String::from_utf8_lossy(&bytes);
        let title_xml = "<rdf:li xml:lang=\"x-default\">A &amp; B &lt;ספר&gt;</rdf:li>";
        let created_xml = "<xmp:CreateDate>2026-05-06T07:08:09Z</xmp:CreateDate>";
        assert!(s.contains(title_xml));
        assert!(s.contains(created_xml));
        assert!(s.contains("<pdf:Trapped>False</pdf:Trapped>"));
        assert!(s.contains("/Subtype /XML"));
    }

    #[test]
    fn pdf_x1a_has_no_xmp_metadata() {
        let options = PdfExportOptions {
            standard: PdfXStandard::PdfX1a2001,
            compress_streams: false,
            ..Default::default()
        };
        let bytes = PdfPrePressEngine::export_pdf(&[], &options).unwrap();
        let s = String::from_utf8_lossy(&bytes);
        assert!(!s.contains("xpacket") && !s.contains("/Metadata"));
    }

    #[test]
    fn blank_title_falls_back_to_non_empty_pdfx_title() {
        for standard in [PdfXStandard::PdfX1a2001, PdfXStandard::PdfX4] {
            let options = PdfExportOptions {
                standard,
                title: "   ".to_string(),
                compress_streams: false,
                ..Default::default()
            };
            let bytes = PdfPrePressEngine::export_pdf(&[], &options).unwrap();
            let s = String::from_utf8_lossy(&bytes);
            assert!(
                s.contains("/Title") && !s.contains("/Title ()"),
                "PDF/X Info /Title must not be empty"
            );
        }
    }

    #[test]
    fn typeset_and_pdf_emit_managed_cmyk_and_visual_runs() {
        use tok_core::id::FractionalIndex;
        use tok_core::model::{DocumentRoot, ParagraphNode};
        use tok_core::spans::TextSpan;
        use tok_core::styles::CharacterFormat;
        use tok_typeset::engine::{TypesettingEngine, TypesettingEngineConfig};

        let mut doc = DocumentRoot::new("Color Pipeline Test");
        let mut para = ParagraphNode::new(FractionalIndex::initial(), "body", "שלום עולם");
        para.spans = vec![
            TextSpan::new(
                0,
                4,
                CharacterFormat {
                    color: Some(Color::Rgb {
                        r: 0.86,
                        g: 0.08,
                        b: 0.08,
                    }),
                    underline: Some(true),
                    ..Default::default()
                },
            ),
            TextSpan::new(
                5,
                9,
                CharacterFormat {
                    color: Some(Color::DeviceCmyk {
                        c: 0.0,
                        m: 0.5,
                        y: 0.5,
                        k: 0.2,
                    }),
                    ..Default::default()
                },
            ),
        ];
        doc.sections[0].flows[0].paragraphs = vec![para];

        let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
        let pages = engine.typeset_document(&doc);
        assert!(!pages.is_empty());
        let first_line = &pages[0].frames[0].lines[0];
        assert!(
            first_line.colors.len() >= 2,
            "LineBox should carry distinct span colors: {:?}",
            first_line.colors
        );
        assert!(
            !first_line.visual_runs.is_empty(),
            "LineBox must include explicit VisualLineRun entries"
        );
        assert!(first_line.visual_runs.iter().any(|r| r.underline));
        assert!(first_line.glyphs.iter().any(|g| g.underline));

        let pdf_bytes = PdfPrePressEngine::export_pdf(
            &pages,
            &PdfExportOptions {
                compress_streams: false,
                ..Default::default()
            },
        )
        .unwrap();
        let pdf_str = String::from_utf8_lossy(&pdf_bytes);
        assert!(
            pdf_str.contains(" k\n") || pdf_str.contains(" k "),
            "Expected DeviceCMYK fill color operator 'k' in PDF stream"
        );
        assert!(
            !pdf_str.contains(" rg\n") && !pdf_str.contains(" RG\n"),
            "PDF/X must never emit DeviceRGB fill/stroke operators"
        );
    }

    #[test]
    fn spot_color_emits_separation_colorspace_and_scn_operators() {
        let spot = Color::Spot {
            name: "PANTONE 186 C".to_string(),
            tint: 1.0,
            fallback_cmyk: (0.0, 1.0, 0.81, 0.04),
        };
        let glyph = GlyphBox {
            glyph_id: 2,
            cluster: 0,
            x: 10.0,
            y: 0.0,
            width: 8.0,
            height: 12.0,
            character: Some('א'),
            font_index: 0,
            color_index: 1,
            underline: true,
            italic: false,
            stretch: 0.0,
        };
        let line = LineBox {
            line_index: 0,
            paragraph_id: None,
            baseline_y: 24.0,
            height: 16.0,
            width: 120.0,
            x_offset: 0.0,
            is_drop_cap: false,
            glyphs: vec![glyph],
            text: "א".to_string(),
            is_rtl: true,
            fonts: vec!["Noto Serif Hebrew".to_string()],
            colors: vec![Color::default(), spot],
            ..Default::default()
        };
        let page = page_with(vec![line]);
        let pdf_bytes = PdfPrePressEngine::export_pdf(
            &[page],
            &PdfExportOptions {
                compress_streams: false,
                ..Default::default()
            },
        )
        .unwrap();
        let pdf_str = String::from_utf8_lossy(&pdf_bytes);
        assert!(
            pdf_str.contains("/Separation"),
            "Expected /Separation color space for Spot ink"
        );
        assert!(
            pdf_str.contains(" cs") && pdf_str.contains(" scn"),
            "Expected 'cs' and 'scn' Spot color selection operators"
        );
        assert!(
            pdf_str.contains(" CS") && pdf_str.contains(" SCN"),
            "Expected 'CS' and 'SCN' Spot stroke color selection operators for underline"
        );
    }

    #[test]
    fn embedded_png_and_builtin_ornament_render_in_pdfx() {
        use tok_typeset::geometry::{PlacedImageBox, PlacedOrnamentBox};

        // Valid minimal 1x1 RGBA PNG
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        let crc = |d: &[u8]| -> u32 {
            let mut c = 0xFFFFFFFFu32;
            for &b in d {
                c ^= b as u32;
                for _ in 0..8 {
                    c = (c >> 1) ^ (0xEDB88320 & (c & 1).wrapping_neg());
                }
            }
            !c
        };
        let chunk = |dst: &mut Vec<u8>, tag: &[u8; 4], data: &[u8]| {
            dst.extend_from_slice(&(data.len() as u32).to_be_bytes());
            dst.extend_from_slice(tag);
            dst.extend_from_slice(data);
            let mut buf = tag.to_vec();
            buf.extend_from_slice(data);
            dst.extend_from_slice(&crc(&buf).to_be_bytes());
        };
        chunk(&mut png, b"IHDR", &[0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0]);
        let idat = miniz_oxide::deflate::compress_to_vec_zlib(&[0, 200, 40, 20, 128], 6);
        chunk(&mut png, b"IDAT", &idat);
        chunk(&mut png, b"IEND", &[]);

        let mut page = page_with(Vec::new());
        page.images.push(PlacedImageBox {
            asset_path: "images/cover.png".to_string(),
            rect: PhysicalRect::new(80.0, 100.0, 100.0, 100.0),
            intrinsic_width_px: 1,
            intrinsic_height_px: 1,
            effective_dpi_x: 0.72,
            effective_dpi_y: 0.72,
            paragraph_id: None,
        });
        page.ornaments.push(PlacedOrnamentBox {
            ornament_id: "crown_torah".to_string(),
            asset_path: None,
            rect: PhysicalRect::new(100.0, 220.0, 96.0, 32.0),
            color: Color::default(),
        });

        let mut assets = BTreeMap::new();
        assets.insert("images/cover.png".to_string(), png);

        let opts_x1a = PdfExportOptions {
            standard: PdfXStandard::PdfX1a2001,
            compress_streams: false,
            image_assets: assets.clone(),
            ..Default::default()
        };
        let bytes_x1a = PdfPrePressEngine::export_pdf(&[page.clone()], &opts_x1a).unwrap();
        let s_x1a = String::from_utf8_lossy(&bytes_x1a);
        assert!(s_x1a.contains("/Subtype /Image"));
        assert!(s_x1a.contains("/Im1 Do"));
        assert!(s_x1a.contains("/DeviceCMYK"));
        assert!(!s_x1a.contains("/SMask"));
        assert!(
            s_x1a.contains("\nf\n") || s_x1a.contains(" f\n"),
            "Ornament vector fill missing"
        );

        let opts_x4 = PdfExportOptions {
            standard: PdfXStandard::PdfX4,
            compress_streams: false,
            image_assets: assets,
            ..Default::default()
        };
        let bytes_x4 = PdfPrePressEngine::export_pdf(&[page], &opts_x4).unwrap();
        let s_x4 = String::from_utf8_lossy(&bytes_x4);
        assert!(
            s_x4.contains("/SMask"),
            "PDF/X-4 should keep alpha as /SMask"
        );
    }
}
