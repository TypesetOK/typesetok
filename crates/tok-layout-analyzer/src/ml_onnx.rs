//! Optional ONNX Runtime (`ort`) inference wrapper for Apache-2.0 `PP-DocLayout-S`.
//!
//! Compiled with full ONNX inference when `--features ml-onnx` is enabled;
//! provides a clean, zero-dependency fallback stub otherwise.

use crate::error::{AnalyzerError, AnalyzerResult};
use crate::geometry::NormalizedBBox;
use crate::ml_labels::map_pp_doclayout_class;
use crate::profile::DetectedRegion;
use crate::raster::GrayImage;
use std::path::Path;

/// Input resolution expected by `PP-DocLayout-S` (RT-DETR).
pub const PP_DOCLAYOUT_INPUT_SIZE: u32 = 640;

/// Minimum detection confidence threshold for ML region proposals.
pub const DEFAULT_ML_SCORE_THRESHOLD: f32 = 0.42;

/// Local ONNX Runtime session wrapper for `PP-DocLayout-S`.
pub struct OnnxLayoutDetector {
    #[cfg(feature = "ml-onnx")]
    session: ort::session::Session,
    score_threshold: f32,
}

impl OnnxLayoutDetector {
    /// Loads a `PP-DocLayout-S` ONNX model from `model_path`.
    #[cfg(feature = "ml-onnx")]
    pub fn load(model_path: &Path) -> AnalyzerResult<Self> {
        let session = ort::session::Session::builder()
            .map_err(|e| AnalyzerError::MlInference(e.to_string()))?
            .with_intra_threads(4)
            .map_err(|e| AnalyzerError::MlInference(e.to_string()))?
            .commit_from_file(model_path)
            .map_err(|e| AnalyzerError::MlInference(e.to_string()))?;

        Ok(Self {
            session,
            score_threshold: DEFAULT_ML_SCORE_THRESHOLD,
        })
    }

    /// Stub loader when `ml-onnx` feature is not enabled.
    #[cfg(not(feature = "ml-onnx"))]
    pub fn load(_model_path: &Path) -> AnalyzerResult<Self> {
        Err(AnalyzerError::MlInference(
            "tok-layout-analyzer compiled without `ml-onnx` feature".into(),
        ))
    }

    /// Runs `PP-DocLayout-S` object detection on a grayscale page image.
    #[cfg(feature = "ml-onnx")]
    pub fn detect(&mut self, img: &GrayImage) -> AnalyzerResult<Vec<DetectedRegion>> {
        let tensor = prepare_rtdetr_chw_tensor(img, PP_DOCLAYOUT_INPUT_SIZE);
        let input_shape = [
            1usize,
            3usize,
            PP_DOCLAYOUT_INPUT_SIZE as usize,
            PP_DOCLAYOUT_INPUT_SIZE as usize,
        ];
        let input_value = ort::value::Tensor::from_array((input_shape, tensor))
            .map_err(|e| AnalyzerError::MlInference(e.to_string()))?;

        let outputs = self
            .session
            .run(ort::inputs![input_value])
            .map_err(|e| AnalyzerError::MlInference(e.to_string()))?;

        let (_, raw_boxes) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| AnalyzerError::MlInference(e.to_string()))?;

        let mut regions = Vec::new();
        let sz = PP_DOCLAYOUT_INPUT_SIZE as f32;
        for row in raw_boxes.chunks_exact(6) {
            let class_id = row[0].max(0.0) as usize;
            let score = row[1];
            if score < self.score_threshold {
                continue;
            }
            let bbox = NormalizedBBox::new(row[2] / sz, row[3] / sz, row[4] / sz, row[5] / sz);
            regions.push(DetectedRegion {
                bbox,
                region_class: map_pp_doclayout_class(class_id),
                confidence: score,
                font_size_pt: None,
                font_name: None,
                median_line_height: None,
                is_rashi_script: None,
            });
        }
        Ok(regions)
    }

    /// Stub detector when `ml-onnx` feature is not enabled.
    #[cfg(not(feature = "ml-onnx"))]
    pub fn detect(&mut self, _img: &GrayImage) -> AnalyzerResult<Vec<DetectedRegion>> {
        Err(AnalyzerError::MlInference(
            "tok-layout-analyzer compiled without `ml-onnx` feature".into(),
        ))
    }
}

/// Resizes `img` to `target x target` and normalizes into `[1, 3, H, W]` RGB-replicated f32 tensor.
pub fn prepare_rtdetr_chw_tensor(img: &GrayImage, target: u32) -> Vec<f32> {
    let t = target as usize;
    let plane_len = t * t;
    let mut tensor = vec![0.0_f32; 3 * plane_len];
    let sx = img.width as f32 / target as f32;
    let sy = img.height as f32 / target as f32;

    for dy in 0..t {
        let src_y = ((dy as f32 * sy) as u32).min(img.height.saturating_sub(1));
        for dx in 0..t {
            let src_x = ((dx as f32 * sx) as u32).min(img.width.saturating_sub(1));
            let val = img.get_pixel(src_x, src_y) as f32 / 255.0;
            let idx = dy * t + dx;
            tensor[idx] = val;
            tensor[plane_len + idx] = val;
            tensor[2 * plane_len + idx] = val;
        }
    }
    tensor
}

/// Suppresses `#[allow(dead_code)]` when `ml-onnx` is disabled.
#[allow(dead_code)]
fn _keep_label_mapper_linked(id: usize) -> crate::profile::RegionClass {
    map_pp_doclayout_class(id)
}
