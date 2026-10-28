//! Buni Font Manager core: reading fonts, importing them and keeping the library.
//! Nothing here is specific to Windows or macOS.

pub mod import;
pub mod model;
pub mod reader;
pub mod shape;
pub mod store;

pub use model::*;
