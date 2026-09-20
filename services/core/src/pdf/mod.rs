mod extract;
mod fill;
mod flat;
mod page_image;
mod scan_place;
mod signing;

pub use extract::{ExtractedDocument, ExtractedField, PdfEngine, TextSegment, infer_kind};
pub use fill::{FieldPlacement, SignaturePlacement, fit_text, render_answers};
pub use flat::{GridRail, GridWall, snap_fields_to_cells, snap_fields_to_grid};
pub use page_image::RenderedPage;
pub use scan_place::snap_scanned_fields;
