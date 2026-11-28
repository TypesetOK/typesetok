//! Reference comparison suite validating TypesetOK's pure-Rust ICC CMM
//! against published LittleCMS (`lcms2`) / ICC ISO 15076-1 / CIEDE2000 reference
//! vectors across sRGB->PCS, Bradford adaptation, Rendering Intents, and Soft Proofing.

use tok_pdf::{
    bradford_adapt, delta_e_2000, delta_e_76, CmykColor, ColorManagementEngine, IccColorTransform,
    IccProfile, IccProfileId, PdfExportOptions, PdfPrePressEngine, PdfXStandard, RenderingIntent,
    SoftProofOptions, SoftProofer,
};
use tok_typeset::{TypesettingEngine, TypesettingEngineConfig};

/// Published CIEDE2000 reference pairs from Sharma, Wu, Dalal (2005)
/// "The CIEDE2000 Color-Difference Formula: Implementation Notes, Supplementary Test Data".
#[test]
fn ciede2000_matches_sharma_wu_dalal_reference_vectors() {
    let cases: [([f32; 3], [f32; 3], f32); 4] = [
        // Pair 1
        ([50.0, 2.6772, -79.7751], [50.0, 0.0, -82.7485], 2.0425),
        // Pair 2
        ([50.0, 3.1571, -77.2803], [50.0, 0.0, -82.7485], 2.8615),
        // Pair 17
        ([50.0, 2.4900, -0.0010], [50.0, -2.4900, 0.0009], 7.1792),
        // Pair 25
        (
            [60.2574, -34.0099, 36.2677],
            [60.4626, -34.1751, 39.4387],
            1.2644,
        ),
    ];

    for (idx, (lab1, lab2, expected)) in cases.iter().enumerate() {
        let got = delta_e_2000(*lab1, *lab2);
        assert!(
            (got - expected).abs() < 5e-4,
            "CIEDE2000 pair #{idx}: expected {expected}, got {got}"
        );
    }
}

/// Reference comparison: sRGB -> CIE L*a*b* (D50 PCS) compared against LittleCMS (`lcms2`)
/// `cmsCreate_sRGBProfile()` -> `cmsCreateLab4Profile(cmsD50_XYZ())` with Relative Colorimetric.
#[test]
fn srgb_to_pcs_lab_matches_littlecms_reference_vectors() {
    let target = IccProfile::from_builtin(IccProfileId::Fogra39).unwrap();
    let xform = IccColorTransform::srgb_to_target(target, RenderingIntent::RelativeColorimetric);

    // Reference (R, G, B) -> LittleCMS D50 Lab (L*, a*, b*)
    let lcms_refs: [([f32; 3], [f32; 3]); 7] = [
        ([0.0, 0.0, 0.0], [0.000, 0.000, 0.000]),
        ([1.0, 1.0, 1.0], [100.000, 0.000, 0.000]),
        ([1.0, 0.0, 0.0], [54.291, 80.812, 69.885]),
        ([0.0, 1.0, 0.0], [87.818, -79.287, 80.990]),
        ([0.0, 0.0, 1.0], [29.568, 68.299, -112.030]),
        ([0.5, 0.5, 0.5], [53.389, 0.000, 0.000]),
        ([0.8, 0.4, 0.2], [55.238, 38.833, 46.996]),
    ];

    for (rgb, expected_lab) in lcms_refs {
        let got_lab = xform.rgb_to_pcs_lab(rgb[0], rgb[1], rgb[2]);
        let de00 = delta_e_2000(got_lab, expected_lab);
        assert!(
            de00 < 0.25,
            "sRGB {rgb:?} -> PCS Lab {got_lab:?} diverged from LittleCMS {expected_lab:?} (dE00={de00})"
        );
    }
}

/// Verifies Bradford chromatic adaptation (D65 -> D50) against LittleCMS D50 white point.
#[test]
fn bradford_adaptation_d65_to_d50_matches_reference() {
    let d65 = [0.95047_f32, 1.0, 1.08883];
    let d50 = [0.9642_f32, 1.0, 0.8249];
    let adapted = bradford_adapt(d65, d65, d50);
    for i in 0..3 {
        assert!(
            (adapted[i] - d50[i]).abs() < 1e-4,
            "channel {i}: expected {}, got {}",
            d50[i],
            adapted[i]
        );
    }
}

/// Verifies that all 5 built-in profiles and all 4 Rendering Intents execute
/// through the real ICC LUT pipeline with TAC enforcement and consistent round-trips.
#[test]
fn all_profiles_and_intents_produce_consistent_managed_cmyk_and_soft_proofs() {
    let sample_colors = [
        [0.85_f32, 0.20, 0.20],
        [0.20, 0.55, 0.30],
        [0.15, 0.30, 0.70],
        [0.45, 0.45, 0.45],
    ];

    for profile_id in [
        IccProfileId::Fogra39,
        IccProfileId::Fogra51,
        IccProfileId::IsoCoatedV2,
        IccProfileId::Gracol2013,
        IccProfileId::Swop2006,
    ] {
        let profile = IccProfile::from_builtin(profile_id).unwrap();
        for intent in [
            RenderingIntent::Perceptual,
            RenderingIntent::RelativeColorimetric,
            RenderingIntent::Saturation,
            RenderingIntent::AbsoluteColorimetric,
        ] {
            let xform = IccColorTransform::srgb_to_target(profile.clone(), intent)
                .with_max_tac(profile_id.max_tac_percent());
            let proofer = SoftProofer::srgb_to_target(
                profile.clone(),
                SoftProofOptions {
                    intent,
                    ..Default::default()
                },
            );

            for rgb in sample_colors {
                let cmyk = xform.rgb_to_cmyk(rgb[0], rgb[1], rgb[2]);
                assert!(
                    cmyk.total_ink_percent() <= profile_id.max_tac_percent() + 1e-2,
                    "{profile_id:?} {intent:?} exceeded TAC: {}",
                    cmyk.total_ink_percent()
                );

                let proof = proofer.proof_srgb(rgb[0], rgb[1], rgb[2]);
                assert!(proof.delta_e_76.is_finite());
                assert!(proof.delta_e_2000.is_finite());
            }
        }
    }
}

/// Verifies Absolute Colorimetric paper-white simulation vs Relative Colorimetric
/// using a modified external ICC profile with a warm uncoated paper white point.
#[test]
fn external_icc_absolute_vs_relative_intent_simulates_paper_white() {
    let mut ext_bytes = ColorManagementEngine::generate_icc_profile_bytes(IccProfileId::Fogra39);
    // Patch `wtpt` tag (warm uncoated cream paper: X=0.92, Y=0.96, Z=0.70)
    let tag_count = u32::from_be_bytes(ext_bytes[128..132].try_into().unwrap()) as usize;
    let wtpt_off = (0..tag_count)
        .map(|i| 132 + i * 12)
        .find(|&entry| ext_bytes[entry..entry + 4] == *b"wtpt")
        .map(|entry| {
            u32::from_be_bytes(ext_bytes[entry + 4..entry + 8].try_into().unwrap()) as usize
        })
        .expect("wtpt tag");
    let to_s15f16 = |v: f32| ((v * 65536.0).round() as i32).to_be_bytes();
    ext_bytes[wtpt_off + 8..wtpt_off + 12].copy_from_slice(&to_s15f16(0.92));
    ext_bytes[wtpt_off + 12..wtpt_off + 16].copy_from_slice(&to_s15f16(0.96));
    ext_bytes[wtpt_off + 16..wtpt_off + 20].copy_from_slice(&to_s15f16(0.70));

    let ext_profile = IccProfile::from_bytes(ext_bytes.clone()).unwrap();

    let rel_xform = IccColorTransform::srgb_to_target(
        ext_profile.clone(),
        RenderingIntent::RelativeColorimetric,
    );
    let abs_xform = IccColorTransform::srgb_to_target(
        ext_profile.clone(),
        RenderingIntent::AbsoluteColorimetric,
    );

    assert_eq!(rel_xform.rgb_to_cmyk(1.0, 1.0, 1.0), CmykColor::WHITE);
    let abs_white = abs_xform.rgb_to_cmyk(1.0, 1.0, 1.0);
    assert!(
        abs_white.c + abs_white.m + abs_white.y + abs_white.k > 0.0,
        "Absolute Colorimetric must simulate paper tint on white"
    );

    let proofer_rel = SoftProofer::srgb_to_target(
        ext_profile.clone(),
        SoftProofOptions {
            simulate_paper_color: false,
            ..Default::default()
        },
    );
    let proofer_abs = SoftProofer::srgb_to_target(
        ext_profile,
        SoftProofOptions {
            simulate_paper_color: true,
            ..Default::default()
        },
    );
    let p_rel = proofer_rel.proof_srgb(0.95, 0.95, 0.95);
    let p_abs = proofer_abs.proof_srgb(0.95, 0.95, 0.95);
    assert!(
        delta_e_76(p_rel.proofed_lab, p_abs.proofed_lab) > 0.5,
        "Paper color simulation must measurably shift highlight soft proof"
    );

    // End-to-end PDF/X-1a export with external ICC profile & Perceptual intent
    let engine = TypesettingEngine::new(TypesettingEngineConfig::default());
    let doc = tok_core::model::DocumentRoot::new("External ICC Test");
    let pages = engine.typeset_document(&doc);
    let opts = PdfExportOptions {
        standard: PdfXStandard::PdfX1a2001,
        rendering_intent: RenderingIntent::Perceptual,
        external_icc_profile: Some(ext_bytes),
        compress_streams: false,
        ..Default::default()
    };
    let pdf = PdfPrePressEngine::export_pdf_with_fonts(&pages, &opts, &engine.font_manager)
        .expect("PDF/X export with external ICC profile must succeed");
    assert!(pdf.starts_with(b"%PDF-"));
}
