//! Label mapping for Apache-2.0 `PP-DocLayout-S` (23 classes) to [`RegionClass`].

use crate::profile::RegionClass;

/// Maps a `PP-DocLayout-S` class index (`0..23`) to TypesetOK's [`RegionClass`].
pub fn map_pp_doclayout_class(class_id: usize) -> RegionClass {
    match class_id {
        0 => RegionClass::Title,       // paragraph_title
        1 => RegionClass::Figure,      // image
        2 => RegionClass::Text,        // text
        3 => RegionClass::Footer,      // number
        4 => RegionClass::Text,        // abstract
        5 => RegionClass::Text,        // content
        6 => RegionClass::Title,       // figure_title
        7 => RegionClass::Header,      // header
        8 => RegionClass::Footer,      // footer
        9 => RegionClass::Text,        // reference
        10 => RegionClass::Title,      // doc_title
        11 => RegionClass::Footnote,   // footnote
        12 => RegionClass::Figure,     // formula
        13 => RegionClass::Figure,     // algorithm
        14 => RegionClass::Figure,     // table
        15 => RegionClass::Figure,     // seal
        16 => RegionClass::Figure,     // chart
        17 => RegionClass::Text,       // formula_number
        18 => RegionClass::Title,      // table_title
        19 => RegionClass::Text,       // header_image / caption
        20 => RegionClass::Text,       // aside_text
        21 => RegionClass::Marginalia, // reference_content
        22 => RegionClass::Ornament,   // ornament / stamp
        _ => RegionClass::Text,
    }
}
