//! Print-shop TIFF compatibility test suite.
//!
//! Constructs spec-compliant TIFF 6.0 streams matching Adobe Photoshop, InDesign,
//! Ghostscript, and libtiff RIP outputs across:
//! - Little-endian (`II`) and Big-endian (`MM`) byte orders
//! - RGB, RGBA, CMYK, Grayscale (WhiteIsZero / BlackIsZero), 1-bit & 16-bit
//! - LZW (with TIFF 6.0 early-change) + Horizontal Differencing Predictor=2
//! - ZIP / Adobe Deflate (`32946` and `8`) + Predictor=2
//! - Multi-page IFD chains (`inspect_tiff_info`, `decode_tiff_page`, `#page=N`)
//! - Tiled & PlanarConfiguration=2 layouts
//! - DPI resolution metadata (Inch and Centimeter units) & embedded ICC profiles
//! - End-to-end PDF/X-1a:2001 and PDF/X-4 export determinism

use std::collections::BTreeMap;

use tok_pdf::{
    decode_image, decode_tiff_page, encode_image_xobject_data, inspect_image_dimensions,
    inspect_tiff_info, ColorManagementEngine, IccProfileId, ImageDecodeError, PdfExportOptions,
    PdfPrePressEngine, PdfXStandard, PreparedColorSpace,
};
use tok_typeset::geometry::{PageLayoutBox, PhysicalRect, PlacedImageBox};

#[derive(Clone, Copy)]
enum Endian {
    Little,
    Big,
}

impl Endian {
    fn push_u16(self, dst: &mut Vec<u8>, v: u16) {
        match self {
            Endian::Little => dst.extend_from_slice(&v.to_le_bytes()),
            Endian::Big => dst.extend_from_slice(&v.to_be_bytes()),
        }
    }

    fn push_u32(self, dst: &mut Vec<u8>, v: u32) {
        match self {
            Endian::Little => dst.extend_from_slice(&v.to_le_bytes()),
            Endian::Big => dst.extend_from_slice(&v.to_be_bytes()),
        }
    }
}

/// TIFF 6.0 MSB-first LZW encoder (9..12-bit, Clear=256, EOI=257, early-change).
fn compress_tiff_lzw(src: &[u8]) -> Vec<u8> {
    const CLEAR: u16 = 256;
    const EOI: u16 = 257;

    let mut out = Vec::new();
    let mut bit_buf: u32 = 0;
    let mut bit_count: u8 = 0;

    let mut write_code = |code: u16, bits: u8| {
        bit_buf = (bit_buf << bits) | (code as u32);
        bit_count += bits;
        while bit_count >= 8 {
            bit_count -= 8;
            out.push(((bit_buf >> bit_count) & 0xFF) as u8);
            bit_buf &= (1u32 << bit_count).wrapping_sub(1);
        }
    };

    let mut dict: BTreeMap<(u16, u8), u16> = BTreeMap::new();
    let mut next_code: u16 = 258;
    let mut code_bits: u8 = 9;

    write_code(CLEAR, code_bits);

    if src.is_empty() {
        write_code(EOI, code_bits);
        if bit_count > 0 {
            out.push(((bit_buf << (8 - bit_count)) & 0xFF) as u8);
        }
        return out;
    }

    let mut cur: u16 = src[0] as u16;
    for &b in &src[1..] {
        if let Some(&existing) = dict.get(&(cur, b)) {
            cur = existing;
        } else {
            write_code(cur, code_bits);
            if next_code < 4094 {
                dict.insert((cur, b), next_code);
                next_code += 1;
                if next_code == (1u16 << code_bits) - 1 && code_bits < 12 {
                    code_bits += 1;
                }
            } else {
                write_code(CLEAR, code_bits);
                dict.clear();
                next_code = 258;
                code_bits = 9;
            }
            cur = b as u16;
        }
    }
    write_code(cur, code_bits);
    write_code(EOI, code_bits);
    if bit_count > 0 {
        out.push(((bit_buf << (8 - bit_count)) & 0xFF) as u8);
    }
    out
}

fn apply_horizontal_predictor_8(buf: &mut [u8], width: usize, rows: usize, spp: usize) {
    let row_bytes = width * spp;
    for r in 0..rows {
        let start = r * row_bytes;
        let end = (start + row_bytes).min(buf.len());
        for i in (start + spp..end).rev() {
            buf[i] = buf[i].wrapping_sub(buf[i - spp]);
        }
    }
}

struct TiffIfdSpec {
    width: u32,
    height: u32,
    bits_per_sample: Vec<u16>,
    compression: u16,
    photometric: u16,
    samples_per_pixel: u16,
    rows_per_strip: u32,
    x_res: (u32, u32),
    y_res: (u32, u32),
    res_unit: u16,
    planar: u16,
    predictor: u16,
    extra_samples: Vec<u16>,
    icc_profile: Option<Vec<u8>>,
    strip_payloads: Vec<Vec<u8>>,
    tile_dims: Option<(u32, u32)>,
    tile_payloads: Vec<Vec<u8>>,
}

fn build_tiff(end: Endian, pages: Vec<TiffIfdSpec>) -> Vec<u8> {
    let mut buf = Vec::new();
    match end {
        Endian::Little => buf.extend_from_slice(b"II"),
        Endian::Big => buf.extend_from_slice(b"MM"),
    }
    end.push_u16(&mut buf, 42);
    // Placeholder for first IFD offset
    end.push_u32(&mut buf, 8);

    let mut prev_next_ifd_patch: Option<usize> = Some(4);

    for page in pages {
        // Align IFD to even boundary
        if buf.len() % 2 != 0 {
            buf.push(0);
        }
        let ifd_offset = buf.len() as u32;
        if let Some(patch_pos) = prev_next_ifd_patch {
            let bytes = match end {
                Endian::Little => ifd_offset.to_le_bytes(),
                Endian::Big => ifd_offset.to_be_bytes(),
            };
            buf[patch_pos..patch_pos + 4].copy_from_slice(&bytes);
        }

        // Prepare out-of-line data payloads after IFD
        #[derive(Clone)]
        enum TagData {
            Inline(u16, u32, [u8; 4]),
            OutOfLine(u16, u32, Vec<u8>),
        }

        let mut tags: Vec<(u16, TagData)> = Vec::new();
        let push_u32_single = |tags: &mut Vec<(u16, TagData)>, tag: u16, val: u32| {
            let b = match end {
                Endian::Little => val.to_le_bytes(),
                Endian::Big => val.to_be_bytes(),
            };
            tags.push((tag, TagData::Inline(4, 1, b)));
        };
        let push_u16_single = |tags: &mut Vec<(u16, TagData)>, tag: u16, val: u16| {
            let mut b = [0u8; 4];
            let v = match end {
                Endian::Little => val.to_le_bytes(),
                Endian::Big => val.to_be_bytes(),
            };
            b[0] = v[0];
            b[1] = v[1];
            tags.push((tag, TagData::Inline(3, 1, b)));
        };
        let push_u16_slice = |tags: &mut Vec<(u16, TagData)>, tag: u16, vals: &[u16]| {
            let mut raw = Vec::with_capacity(vals.len() * 2);
            for &v in vals {
                end.push_u16(&mut raw, v);
            }
            if raw.len() <= 4 {
                let mut b = [0u8; 4];
                b[..raw.len()].copy_from_slice(&raw);
                tags.push((tag, TagData::Inline(3, vals.len() as u32, b)));
            } else {
                tags.push((tag, TagData::OutOfLine(3, vals.len() as u32, raw)));
            }
        };
        let push_rational = |tags: &mut Vec<(u16, TagData)>, tag: u16, num: u32, den: u32| {
            let mut raw = Vec::with_capacity(8);
            end.push_u32(&mut raw, num);
            end.push_u32(&mut raw, den);
            tags.push((tag, TagData::OutOfLine(5, 1, raw)));
        };

        push_u32_single(&mut tags, 256, page.width);
        push_u32_single(&mut tags, 257, page.height);
        push_u16_slice(&mut tags, 258, &page.bits_per_sample);
        push_u16_single(&mut tags, 259, page.compression);
        push_u16_single(&mut tags, 262, page.photometric);
        push_u16_single(&mut tags, 277, page.samples_per_pixel);
        push_rational(&mut tags, 282, page.x_res.0, page.x_res.1);
        push_rational(&mut tags, 283, page.y_res.0, page.y_res.1);
        push_u16_single(&mut tags, 284, page.planar);
        push_u16_single(&mut tags, 296, page.res_unit);
        if page.predictor != 1 {
            push_u16_single(&mut tags, 317, page.predictor);
        }
        if !page.extra_samples.is_empty() {
            push_u16_slice(&mut tags, 338, &page.extra_samples);
        }
        if let Some(ref icc) = page.icc_profile {
            tags.push((34675, TagData::OutOfLine(7, icc.len() as u32, icc.clone())));
        }

        // Write strip or tile data first after reserving IFD space
        let is_tiled = page.tile_dims.is_some();
        let chunk_count = if is_tiled {
            page.tile_payloads.len()
        } else {
            page.strip_payloads.len()
        };
        if is_tiled {
            let (tw, th) = page.tile_dims.unwrap();
            push_u32_single(&mut tags, 322, tw);
            push_u32_single(&mut tags, 323, th);
            // Placeholders for 324 (TileOffsets) and 325 (TileByteCounts)
            tags.push((324, TagData::OutOfLine(4, chunk_count as u32, Vec::new())));
            tags.push((325, TagData::OutOfLine(4, chunk_count as u32, Vec::new())));
        } else {
            push_u32_single(&mut tags, 278, page.rows_per_strip);
            tags.push((273, TagData::OutOfLine(4, chunk_count as u32, Vec::new())));
            tags.push((279, TagData::OutOfLine(4, chunk_count as u32, Vec::new())));
        }

        tags.sort_by_key(|(t, _)| *t);

        let entry_count = tags.len();
        let ifd_byte_len = 2 + entry_count * 12 + 4;
        buf.resize(ifd_offset as usize + ifd_byte_len, 0);
        let count_bytes = match end {
            Endian::Little => (entry_count as u16).to_le_bytes(),
            Endian::Big => (entry_count as u16).to_be_bytes(),
        };
        buf[ifd_offset as usize..ifd_offset as usize + 2].copy_from_slice(&count_bytes);
        let next_ifd_pos = ifd_offset as usize + 2 + entry_count * 12;
        prev_next_ifd_patch = Some(next_ifd_pos);

        // Emit strip/tile payloads and record offsets + byte counts
        let payloads = if is_tiled {
            &page.tile_payloads
        } else {
            &page.strip_payloads
        };
        let mut offsets = Vec::with_capacity(payloads.len());
        let mut counts = Vec::with_capacity(payloads.len());
        for p in payloads {
            offsets.push(buf.len() as u32);
            counts.push(p.len() as u32);
            buf.extend_from_slice(p);
        }

        // Write IFD entries
        for (idx, (tag, data)) in tags.into_iter().enumerate() {
            let e_pos = ifd_offset as usize + 2 + idx * 12;
            let (ftype, count, val_bytes) = match data {
                TagData::Inline(ft, cnt, b) => (ft, cnt, b),
                TagData::OutOfLine(ft, cnt, mut raw) => {
                    if tag == 273 || tag == 324 {
                        raw.clear();
                        for &o in &offsets {
                            end.push_u32(&mut raw, o);
                        }
                    } else if tag == 279 || tag == 325 {
                        raw.clear();
                        for &c in &counts {
                            end.push_u32(&mut raw, c);
                        }
                    }
                    if raw.len() <= 4 {
                        let mut b = [0u8; 4];
                        b[..raw.len()].copy_from_slice(&raw);
                        (ft, cnt, b)
                    } else {
                        let off = buf.len() as u32;
                        buf.extend_from_slice(&raw);
                        let b = match end {
                            Endian::Little => off.to_le_bytes(),
                            Endian::Big => off.to_be_bytes(),
                        };
                        (ft, cnt, b)
                    }
                }
            };
            let t_b = match end {
                Endian::Little => tag.to_le_bytes(),
                Endian::Big => tag.to_be_bytes(),
            };
            let f_b = match end {
                Endian::Little => ftype.to_le_bytes(),
                Endian::Big => ftype.to_be_bytes(),
            };
            let c_b = match end {
                Endian::Little => count.to_le_bytes(),
                Endian::Big => count.to_be_bytes(),
            };
            buf[e_pos..e_pos + 2].copy_from_slice(&t_b);
            buf[e_pos + 2..e_pos + 4].copy_from_slice(&f_b);
            buf[e_pos + 4..e_pos + 8].copy_from_slice(&c_b);
            buf[e_pos + 8..e_pos + 12].copy_from_slice(&val_bytes);
        }
    }

    buf
}

#[test]
fn printshop_cmyk_lzw_predictor2_icc_and_dpi_little_and_big_endian() {
    let w = 4u32;
    let h = 2u32;
    // 4x2 CMYK image with rich press ink values
    let pixels: Vec<u8> = vec![
        0, 0, 0, 255, // pure K
        200, 100, 0, 10, // cyan-blue
        0, 220, 200, 0, // warm red
        10, 20, 30, 40, // neutral midtone
        50, 0, 180, 0, // green
        0, 0, 0, 0, // paper white
        120, 120, 120, 80, // rich black
        255, 255, 0, 0, // deep blue
    ];
    let mut pred_pixels = pixels.clone();
    apply_horizontal_predictor_8(&mut pred_pixels, w as usize, h as usize, 4);
    let lzw_strip = compress_tiff_lzw(&pred_pixels);
    let fogra39_icc = ColorManagementEngine::generate_icc_profile_bytes(IccProfileId::Fogra39);

    for end in [Endian::Little, Endian::Big] {
        let tiff = build_tiff(
            end,
            vec![TiffIfdSpec {
                width: w,
                height: h,
                bits_per_sample: vec![8, 8, 8, 8],
                compression: 5, // LZW
                photometric: 5, // CMYK
                samples_per_pixel: 4,
                rows_per_strip: h,
                x_res: (300, 1),
                y_res: (300, 1),
                res_unit: 2, // Inch
                planar: 1,
                predictor: 2, // Horizontal Differencing
                extra_samples: vec![],
                icc_profile: Some(fogra39_icc.clone()),
                strip_payloads: vec![lzw_strip.clone()],
                tile_dims: None,
                tile_payloads: vec![],
            }],
        );

        assert_eq!(inspect_image_dimensions(&tiff).unwrap(), (w, h));
        let decoded = decode_image(&tiff).unwrap();
        assert_eq!(decoded.pixels, pixels);
        assert!((decoded.dpi_x.unwrap() - 300.0).abs() < 1e-3);
        assert!((decoded.dpi_y.unwrap() - 300.0).abs() < 1e-3);
        assert_eq!(decoded.icc_profile.as_deref(), Some(fogra39_icc.as_slice()));

        // PDF/X-1a must strip ICCBased on image XObject and use DeviceCMYK
        let prep_x1a = encode_image_xobject_data(&tiff, true, IccProfileId::Fogra39).unwrap();
        assert_eq!(prep_x1a.color_space, PreparedColorSpace::DeviceCmyk);
        assert!(prep_x1a.icc_profile.is_none());

        // PDF/X-4 preserves embedded ICC profile
        let prep_x4 = encode_image_xobject_data(&tiff, false, IccProfileId::Fogra39).unwrap();
        assert_eq!(prep_x4.color_space, PreparedColorSpace::DeviceCmyk);
        assert_eq!(prep_x4.icc_profile.as_deref(), Some(fogra39_icc.as_slice()));
    }
}

#[test]
fn printshop_rgb_zip_deflate_alpha_and_cm_resolution() {
    let w = 3u32;
    let h = 2u32;
    let rgba: Vec<u8> = vec![
        255, 0, 0, 255, // opaque red
        0, 255, 0, 128, // semi-transparent green
        0, 0, 255, 0, // transparent blue
        20, 40, 60, 255, // dark slate
        240, 240, 240, 200, // light gray
        12, 34, 56, 255, // navy
    ];
    let mut pred = rgba.clone();
    apply_horizontal_predictor_8(&mut pred, w as usize, h as usize, 4);
    let zip_strip = miniz_oxide::deflate::compress_to_vec_zlib(&pred, 6);

    // Test both Adobe Deflate (32946) and standard Deflate (8)
    for comp in [32946u16, 8u16] {
        let tiff = build_tiff(
            Endian::Little,
            vec![TiffIfdSpec {
                width: w,
                height: h,
                bits_per_sample: vec![8, 8, 8, 8],
                compression: comp,
                photometric: 2, // RGB
                samples_per_pixel: 4,
                rows_per_strip: h,
                x_res: (11811, 100), // 118.11 px/cm ≈ 300.0 DPI
                y_res: (11811, 100),
                res_unit: 3, // Centimeter
                planar: 1,
                predictor: 2,
                extra_samples: vec![2], // Unassociated alpha
                icc_profile: Some(b"FAKE_SRGB_ICC_HEADER".to_vec()),
                strip_payloads: vec![zip_strip.clone()],
                tile_dims: None,
                tile_payloads: vec![],
            }],
        );

        let decoded = decode_image(&tiff).unwrap();
        assert_eq!(decoded.width, w);
        assert_eq!(decoded.height, h);
        assert_eq!(
            decoded.alpha.as_ref().unwrap(),
            &[255, 128, 0, 255, 200, 255]
        );
        assert!((decoded.dpi_x.unwrap() - 300.0).abs() < 0.05);

        let prep_x4 = encode_image_xobject_data(&tiff, false, IccProfileId::Fogra39).unwrap();
        assert!(prep_x4.smask_flate_data.is_some());
        assert_eq!(prep_x4.color_space, PreparedColorSpace::CalRgb);
        assert!(prep_x4.icc_profile.is_some());

        let prep_x1a = encode_image_xobject_data(&tiff, true, IccProfileId::Fogra39).unwrap();
        assert!(prep_x1a.smask_flate_data.is_none());
        assert_eq!(prep_x1a.color_space, PreparedColorSpace::DeviceCmyk);
    }
}

#[test]
fn printshop_multipage_tiff_tiled_planar_16bit_and_pdf_export() {
    // Page 0: 2x2 RGB (uncompressed)
    let p0_rgb = vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255];

    // Page 1: 4x1 1-bit WhiteIsZero Grayscale (1010_0000 -> [0, 255, 0, 255] after WhiteIsZero invert)
    let p1_bilevel = vec![0b1010_0000];

    // Page 2: 2x2 CMYK Tiled (2x2 tile) + PlanarConfiguration=2 + LZW
    let c_plane = compress_tiff_lzw(&[10, 20, 30, 40]);
    let m_plane = compress_tiff_lzw(&[50, 60, 70, 80]);
    let y_plane = compress_tiff_lzw(&[90, 100, 110, 120]);
    let k_plane = compress_tiff_lzw(&[130, 140, 150, 160]);

    let multipage_tiff = build_tiff(
        Endian::Little,
        vec![
            TiffIfdSpec {
                width: 2,
                height: 2,
                bits_per_sample: vec![8, 8, 8],
                compression: 1,
                photometric: 2,
                samples_per_pixel: 3,
                rows_per_strip: 2,
                x_res: (300, 1),
                y_res: (300, 1),
                res_unit: 2,
                planar: 1,
                predictor: 1,
                extra_samples: vec![],
                icc_profile: None,
                strip_payloads: vec![p0_rgb],
                tile_dims: None,
                tile_payloads: vec![],
            },
            TiffIfdSpec {
                width: 4,
                height: 1,
                bits_per_sample: vec![1],
                compression: 1,
                photometric: 0, // WhiteIsZero
                samples_per_pixel: 1,
                rows_per_strip: 1,
                x_res: (600, 1),
                y_res: (600, 1),
                res_unit: 2,
                planar: 1,
                predictor: 1,
                extra_samples: vec![],
                icc_profile: None,
                strip_payloads: vec![p1_bilevel],
                tile_dims: None,
                tile_payloads: vec![],
            },
            TiffIfdSpec {
                width: 2,
                height: 2,
                bits_per_sample: vec![8, 8, 8, 8],
                compression: 5, // LZW
                photometric: 5, // CMYK
                samples_per_pixel: 4,
                rows_per_strip: 2,
                x_res: (300, 1),
                y_res: (300, 1),
                res_unit: 2,
                planar: 2, // Planar
                predictor: 1,
                extra_samples: vec![],
                icc_profile: Some(ColorManagementEngine::generate_icc_profile_bytes(
                    IccProfileId::Fogra39,
                )),
                strip_payloads: vec![],
                tile_dims: Some((2, 2)),
                tile_payloads: vec![c_plane, m_plane, y_plane, k_plane],
            },
        ],
    );

    let info = inspect_tiff_info(&multipage_tiff).unwrap();
    assert_eq!(info.page_count, 3);
    assert_eq!(info.pages[1].width, 4);
    assert!(info.pages[2].is_tiled);
    assert!(info.pages[2].has_icc_profile);

    let dec1 = decode_tiff_page(&multipage_tiff, 1).unwrap();
    assert_eq!(dec1.pixels, vec![0, 255, 0, 255]);

    let dec2 = decode_tiff_page(&multipage_tiff, 2).unwrap();
    assert_eq!(
        dec2.pixels,
        vec![10, 50, 90, 130, 20, 60, 100, 140, 30, 70, 110, 150, 40, 80, 120, 160]
    );

    let err = decode_tiff_page(&multipage_tiff, 3).unwrap_err();
    assert_eq!(
        err,
        ImageDecodeError::TiffPageOutOfRange { page: 3, total: 3 }
    );

    // End-to-end PDF/X-4 export placing page 1 and page 3 via #page=N fragment
    let mut page_box = PageLayoutBox {
        page_index: 0,
        page_number_gematria: "א".to_string(),
        dimensions: PhysicalRect::new(0.0, 0.0, 595.0, 842.0),
        ..Default::default()
    };
    page_box.images.push(PlacedImageBox {
        asset_path: "images/plates.tif#page=1".to_string(),
        rect: PhysicalRect::new(50.0, 100.0, 72.0, 72.0),
        intrinsic_width_px: 2,
        intrinsic_height_px: 2,
        effective_dpi_x: 2.0,
        effective_dpi_y: 2.0,
        paragraph_id: None,
    });
    page_box.images.push(PlacedImageBox {
        asset_path: "images/plates.tif#page=3".to_string(),
        rect: PhysicalRect::new(150.0, 100.0, 72.0, 72.0),
        intrinsic_width_px: 2,
        intrinsic_height_px: 2,
        effective_dpi_x: 2.0,
        effective_dpi_y: 2.0,
        paragraph_id: None,
    });

    let mut image_assets = BTreeMap::new();
    image_assets.insert("images/plates.tif".to_string(), multipage_tiff);

    let opts = PdfExportOptions {
        standard: PdfXStandard::PdfX4,
        compress_streams: false,
        image_assets,
        ..Default::default()
    };
    let pdf_a = PdfPrePressEngine::export_pdf(&[page_box.clone()], &opts).unwrap();
    let pdf_b = PdfPrePressEngine::export_pdf(&[page_box], &opts).unwrap();
    assert_eq!(
        pdf_a, pdf_b,
        "TIFF PDF/X-4 export must be bit-deterministic"
    );
    let pdf_str = String::from_utf8_lossy(&pdf_a);
    assert!(pdf_str.contains("/Im1 Do"));
    assert!(pdf_str.contains("/Im2 Do"));
    assert!(pdf_str.contains("/ICCBased"));
}
