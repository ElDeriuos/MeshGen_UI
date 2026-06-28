//! # structured_mesh — library interface
//!
//! This crate is usable both as a CLI binary (`structured_mesh`) and as a
//! library dependency (e.g. for `mesh_gui`).  All public modules are declared
//! here; the binary entry point lives in `main.rs`.

pub mod error;
pub mod input_parser;
pub mod geometry_core;
pub mod scanline_rasterizer;
pub mod output_text;
pub mod output_tecplot;
pub mod output_vtk;
