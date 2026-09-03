pub mod consumer;
pub mod error;
pub mod extract;
pub mod model;

pub use consumer::{
    chip_spectra_at, device_spectrum_at, nearest_voltage_index, matches_site, ChipSummary,
    DeviceSummary, SpectraSummary, SpectrumPoint,
};
pub use error::CoreError;
pub use extract::{parse_dir, parse_file, DirOutcome};
pub use model::{Chip, DeviceData};
