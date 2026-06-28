//! # structured_mesh
//!
//! A structured polygon mesh generator — idiomatic Rust rewrite of the legacy
//! Fortran 77 program `polygon3.for`.
//!
//! ## What it does
//!
//! Given a polygon description (boundary nodes + directed edges with integer
//! region tags), the tool:
//!
//! 1. Reads a four-line **project file** (`Polygon_project.txt`) that names the
//!    geometry input and three output files.
//! 2. Parses the **geometry file**: cell spacing `(dx, dy)`, node coordinates,
//!    and edge records `(k1, k2, kp)`.
//! 3. Computes a rectangular **rasterisation grid** from the polygon bounding box.
//! 4. Runs a two-pass **scanline fill** (horizontal X-pass then vertical Y-pass)
//!    to classify every cell as exterior (`0`), interior (`1`), or boundary (`≥2`).
//! 5. Assembles **quad-mesh connectivity** (`PlotData`) from active cells.
//! 6. Writes four output files:
//!    - `<ch2>` — legacy text format (Fortran-compatible, spans + connectivity)
//!    - `<ch3>` — Tecplot FEPOINT quadrilateral mesh
//!    - `<ch4>` — Tecplot structured-zone blocks (full grid + row/col spans)
//!    - `mesh.vtu` — VTK XML Unstructured Grid (new; for ParaView / VisIt)
//!
//! ## Module map
//!
//! | Module | Purpose |
//! |---|---|
//! | [`error`] | [`MeshError`](error::MeshError) — all error variants |
//! | [`input_parser`] | Read project file and geometry file |
//! | [`geometry_core`] | Winding order, signed area, point-in-polygon, intersections |
//! | [`scanline_rasterizer`] | Grid sizing, X/Y-pass fill, plot-data assembly |
//! | [`output_text`] | Write `geom3.dat` and `ch2` in Fortran-compatible format |
//! | [`output_tecplot`] | Write `ch3` (FEPOINT) and `ch4` (structured zones) |
//! | [`output_vtk`] | Write `mesh.vtu` VTK XML |
//!
//! ## Quick start
//!
//! ```bash
//! # Build
//! cargo build --release
//!
//! # Run with explicit project file
//! ./target/release/structured_mesh --project path/to/Polygon_project.txt
//!
//! # Run from the directory that contains Polygon_project.txt
//! cd my_mesh_dir
//! structured_mesh
//! ```
//!
//! ## Input file format
//!
//! **`Polygon_project.txt`** — four lines, one path per line:
//! ```text
//! geometry_input.txt    ← ch1: polygon geometry
//! output_text.txt       ← ch2: secondary text output
//! mesh_fepoint.dat      ← ch3: Tecplot FEPOINT output
//! mesh_zones.dat        ← ch4: Tecplot structured zones
//! ```
//!
//! **Geometry file** (list-directed, blank lines ignored):
//! ```text
//! dx  dy              ← cell width and height (floats, both > 0)
//! nnode               ← number of boundary nodes
//! x1 y1               ← node coordinates (one per line)
//! ...
//! xN yN
//! nele                ← number of directed edges
//! k1 k2 kp            ← edge: start node (1-based), end node (1-based), region tag (≥1)
//! ...
//! ```
//!
//! ## Differences from the Fortran original
//!
//! | Aspect | Fortran `polygon3.for` | This crate |
//! |---|---|---|
//! | Grid size limit | Hard-coded `matr(1500,1200)` | **No limit** — heap-allocated |
//! | Node limit | `np = 5000`, `ns = 50` | **Unlimited** — `Vec`-based |
//! | Temporary files | Writes/reads `unit 16 ('t')` | **No temp files** — all in memory |
//! | Hole handling | Ad-hoc polygon-index convention | Winding-order CCW/CW classification |
//! | VTK output | Not present | New `mesh.vtu` output |
//! | Error handling | Silent/abort | Typed `MeshError` with clear messages |

use std::path::{Path, PathBuf};
use std::process;

use clap::Parser;

use structured_mesh::error::MeshError;
use structured_mesh::input_parser::{parse_geometry, parse_project};
use structured_mesh::output_tecplot::{write_ch3, write_ch4};
use structured_mesh::output_text::write_ch2;
use structured_mesh::output_vtk::write_vtu;
use structured_mesh::scanline_rasterizer::{build_plot_data, compute_grid, rasterize};

// ---------------------------------------------------------------------------
// Task 12.1 — CLI struct
// ---------------------------------------------------------------------------

/// Structured polygon mesh generator — Rust port of polygon3.for
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the project file (default: Polygon_project.txt in current directory)
    #[arg(short, long, value_name = "FILE")]
    project: Option<PathBuf>,
}

// ---------------------------------------------------------------------------
// Task 12.2 — full pipeline
// ---------------------------------------------------------------------------

fn run(project_path: &Path) -> Result<(), MeshError> {
    // 1. Parse project file → four output paths
    let cfg = parse_project(project_path)?;

    // 2. Parse geometry file → validated MeshInput
    let input = parse_geometry(&cfg)?;

    // 3. Compute grid sizing
    let geo = compute_grid(&input)?;

    // 4. Rasterize (X-pass + Y-pass)
    let state = rasterize(&input, &geo)?;

    // 5. Build plot / connectivity data
    let plot = build_plot_data(&state);

    // Resolve output paths relative to the project file's directory.
    let base = project_path.parent().unwrap_or(Path::new("."));

    let ch2_path = base.join(&cfg.ch2_file);
    let ch3_path = base.join(&cfg.ch3_file);
    let ch4_path = base.join(&cfg.ch4_file);
    let vtu_path = base.join("mesh.vtu");

    // 6. Write output files
    write_ch2(&state, &plot, &ch2_path)?;
    write_ch3(&plot, &ch3_path)?;
    write_ch4(&state, &plot, &ch4_path)?;
    write_vtu(&plot, &vtu_path)?;

    // 7. Success summary
    println!(
        "mesh_generator: grid {}×{}, {} nodes, {} elements",
        state.nx, state.ny,
        plot.nodes.len(),
        plot.elements.len(),
    );
    println!("  wrote: {}", ch2_path.display());
    println!("  wrote: {}", ch3_path.display());
    println!("  wrote: {}", ch4_path.display());
    println!("  wrote: {}", vtu_path.display());

    Ok(())
}

fn main() {
    let cli = Cli::parse();

    // Resolve project path: explicit flag or default Polygon_project.txt in CWD.
    let project_path = match cli.project {
        Some(p) => p,
        None => {
            let default = PathBuf::from("Polygon_project.txt");
            if !default.exists() {
                eprintln!(
                    "error: no --project flag supplied and \
                     'Polygon_project.txt' not found in the current directory"
                );
                process::exit(1);
            }
            default
        }
    };

    if let Err(e) = run(&project_path) {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_explicit_project_flag() {
        let cli = Cli::try_parse_from(["mesh_generator", "--project", "my_project.txt"])
            .expect("should parse");
        assert_eq!(cli.project, Some(PathBuf::from("my_project.txt")));
    }

    #[test]
    fn cli_short_flag() {
        let cli = Cli::try_parse_from(["mesh_generator", "-p", "other.txt"])
            .expect("should parse short flag");
        assert_eq!(cli.project, Some(PathBuf::from("other.txt")));
    }

    #[test]
    fn cli_no_flag_gives_none() {
        let cli = Cli::try_parse_from(["mesh_generator"]).expect("should parse with no args");
        assert!(cli.project.is_none());
    }
}
