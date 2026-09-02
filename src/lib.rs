pub mod adapter;
pub mod conversion;
pub mod diagnostic;
pub mod model;
pub mod redact;

pub use adapter::{discover_project_config, parse_file, parse_source, render_canonical};
pub use conversion::{ConversionReport, convert};
pub use diagnostic::{Diagnostic, Severity};
pub use model::{CanonicalConfig, Client, ConversionStatus, ParsedConfig, Transport};
