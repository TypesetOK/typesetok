//! Minimal example demonstrating `tok-layout-analyzer` on a file path.
//!
//! Run with: `cargo run -p tok-layout-analyzer --example analyze_sample -- <path/to/file.pdf>`

use std::env;
use std::path::PathBuf;
use tok_layout_analyzer::{analyze_document_pages, AnalysisInput, AnalysisOptions};

fn main() {
    let Some(path_str) = env::args().nth(1) else {
        eprintln!("Usage: analyze_sample <path/to/input.pdf|input.pgm>");
        std::process::exit(1);
    };

    let options = AnalysisOptions::default();
    match analyze_document_pages(AnalysisInput::FilePath(PathBuf::from(path_str)), &options) {
        Ok(rec) => {
            println!(
                "Detected Form: {} ({:?})",
                rec.display_name_he, rec.page_form
            );
            println!("Template Type: {:?}", rec.recommended_template_type);
            println!("Confidence: {:.1}%", rec.confidence * 100.0);
            println!("Recto/Verso Verified: {}", rec.recto_verso_verified);
        }
        Err(err) => {
            eprintln!("Analysis failed: {err}");
            std::process::exit(1);
        }
    }
}
