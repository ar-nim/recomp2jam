//! Data tables: avatar outfit sets, avatar shop items, the jam gauge
//! thresholds, the `OJNList.dat` song index (records, middle tables,
//! patch index/records), and the key-name string table.
//!
//! Verified on real 7.55 files (docs/file-formats/opi.md,
//! docs/file-formats/ojn.md).

pub mod banners;
pub mod keys;
pub mod mid;
pub mod patch;
pub mod shop;

pub use banners::*;
pub use keys::*;
pub use mid::*;
pub use patch::*;
pub use shop::*;
