//! Input parsing for the project file and polygon geometry file.
//!
//! ## Fortran I/O mapping
//!
//! This module mirrors the exact Fortran read sequence from `polygon3.for` so
//! that any file that worked with the original executable works here unchanged.
//!
//! ### Project file (`Polygon_project.txt`)
//!
//! ```text
//! Fortran:  read(24,'(80a)') ch1   → geometry input path
//!           read(24,'(80a)') ch2   → secondary text output path
//!           read(24,'(80a)') ch3   → Tecplot FEPOINT output path
//!           read(24,'(80a)') ch4   → Tecplot structured-zone output path
//! ```
//!
//! Each line is truncated to 80 characters (matching `'(80a)'`) and then
//! ASCII-whitespace trimmed.  An empty trimmed line is a hard error.
//!
//! ### Geometry file (list-directed `read(35,*)`)
//!
//! ```text
//! dx  dy          ← cell spacing (both must be > 0)
//! nnode           ← number of boundary nodes
//! x  y            ← repeated nnode times
//! nele            ← number of directed edges
//! k1  k2  kp      ← repeated nele times
//!                    k1, k2: 1-based node indices (converted to 0-based internally)
//!                    kp: region / boundary tag (must be ≥ 1)
//! ```
//!
//! Blank lines are silently skipped, matching Fortran list-directed behaviour.
//!
//! ## Public types
//!
//! - [`ProjectConfig`] — four output-path strings parsed from the project file.
//! - [`MeshInput`] — validated geometry: `dx`, `dy`, node coordinates, edge list.
//! - [`Edge`] — a single directed boundary edge with a region tag.
//!
//! ## Public functions
//!
//! - [`parse_project`] — read `Polygon_project.txt`.
//! - [`parse_geometry`] — read the geometry file named in [`ProjectConfig`].

use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::error::MeshError;

// ---------------------------------------------------------------------------
// Public structs
// ---------------------------------------------------------------------------

/// Paths extracted from the four-line Fortran project file.
#[derive(Debug, Clone)]
pub struct ProjectConfig {
    /// ch1 – geometry input file path
    pub geometry_file: String,
    /// ch2 – secondary text output path
    pub ch2_file: String,
    /// ch3 – Tecplot FEPOINT output path
    pub ch3_file: String,
    /// ch4 – Tecplot structured-zone output path
    pub ch4_file: String,
}

/// A directed edge in the polygon boundary.
#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    /// Start node index, **0-based** (converted from Fortran 1-based).
    pub k1: usize,
    /// End node index, **0-based** (converted from Fortran 1-based).
    pub k2: usize,
    /// Region / boundary tag (≥ 1).
    pub kp: i32,
}

/// All validated data read from the geometry file.
#[derive(Debug, Clone)]
pub struct MeshInput {
    /// Rasterisation cell width (> 0).
    pub dx: f64,
    /// Rasterisation cell height (> 0).
    pub dy: f64,
    /// Node coordinates, 0-based; Fortran `xp(i)` / `yp(i)`.
    pub nodes: Vec<(f64, f64)>,
    /// Edge connectivity records.
    pub edges: Vec<Edge>,
}

// ---------------------------------------------------------------------------
// Task 3.1: parse_project
// ---------------------------------------------------------------------------

/// Read a Fortran-style project file and return the four channel path strings.
///
/// The file must contain at least four lines; each line is read up to 80
/// characters (matching Fortran `read(24,'(80a)')`), then ASCII-whitespace
/// trimmed.  An empty trimmed line is a [`MeshError::Parse`] error.
pub fn parse_project(project_path: &Path) -> Result<ProjectConfig, MeshError> {
    let file = std::fs::File::open(project_path).map_err(MeshError::Io)?;
    let reader = BufReader::new(file);

    let mut paths: Vec<String> = Vec::with_capacity(4);
    let mut raw_line = String::new();

    // We need exactly four lines (the Fortran unit reads with format '(80a)',
    // so we replicate that by taking the first 80 chars of each raw line).
    let mut line_iter = reader.lines();
    for line_no in 1..=4_usize {
        let raw = line_iter
            .next()
            .ok_or_else(|| MeshError::Parse {
                line: line_no,
                msg: format!("file has fewer than 4 lines; missing line {line_no}"),
            })?
            .map_err(MeshError::Io)?;

        // Replicate Fortran '(80a)': take at most 80 characters.
        let truncated: String = raw.chars().take(80).collect();
        let trimmed = truncated.trim_ascii().to_string();

        if trimmed.is_empty() {
            return Err(MeshError::Parse {
                line: line_no,
                msg: "empty path".to_string(),
            });
        }

        paths.push(trimmed);
        raw_line.clear();
    }

    // `paths` is guaranteed to have exactly 4 elements here.
    Ok(ProjectConfig {
        geometry_file: paths.remove(0),
        ch2_file: paths.remove(0),
        ch3_file: paths.remove(0),
        ch4_file: paths.remove(0),
    })
}

// ---------------------------------------------------------------------------
// Task 3.2: parse_geometry
// ---------------------------------------------------------------------------

/// Read the polygon geometry file referenced by `cfg.geometry_file` and
/// return a validated [`MeshInput`].
///
/// The parsing mirrors Fortran list-directed `read(35,*)`:
/// - Tokens are whitespace-separated.
/// - Blank lines are skipped (list-directed I/O ignores them).
/// - Lines are consumed token-by-token across physical line boundaries.
pub fn parse_geometry(cfg: &ProjectConfig) -> Result<MeshInput, MeshError> {
    let path = Path::new(&cfg.geometry_file);
    let file = std::fs::File::open(path).map_err(MeshError::Io)?;
    let reader = BufReader::new(file);

    // Collect all tokens together with their 1-based source line numbers for
    // precise error reporting.  Blank lines are skipped.
    let mut tokens: Vec<(String, usize)> = Vec::new();
    for (idx, line_result) in reader.lines().enumerate() {
        let line = line_result.map_err(MeshError::Io)?;
        let line_no = idx + 1;
        for tok in line.split_whitespace() {
            tokens.push((tok.to_string(), line_no));
        }
    }

    let mut pos = 0usize; // current position in the token stream

    // ------------------------------------------------------------------
    // Helper closures that borrow `tokens` immutably and advance `pos`.
    // ------------------------------------------------------------------

    /// Consume the next token or return a parse error referencing the last
    /// seen line number (or line 1 if the stream is empty from the start).
    macro_rules! next_tok {
        ($ctx_line:expr) => {{
            if pos >= tokens.len() {
                return Err(MeshError::Parse {
                    line: $ctx_line,
                    msg: "unexpected end of file".to_string(),
                });
            }
            let tok = &tokens[pos];
            pos += 1;
            tok
        }};
    }

    // ------------------------------------------------------------------
    // Line 1 (list-directed): dx dy
    // ------------------------------------------------------------------
    let (dx_str, dx_line) = {
        let t = next_tok!(1);
        (t.0.clone(), t.1)
    };
    let (dy_str, dy_line) = {
        let t = next_tok!(dx_line);
        (t.0.clone(), t.1)
    };

    let dx = parse_f64(&dx_str, dx_line, "dx")?;
    let dy = parse_f64(&dy_str, dy_line, "dy")?;

    if !dx.is_finite() || dx <= 0.0 {
        return Err(MeshError::Parse {
            line: dx_line,
            msg: "dx must be finite and > 0".to_string(),
        });
    }
    if !dy.is_finite() || dy <= 0.0 {
        return Err(MeshError::Parse {
            line: dy_line,
            msg: "dy must be finite and > 0".to_string(),
        });
    }

    // ------------------------------------------------------------------
    // nnode
    // ------------------------------------------------------------------
    let (nnode_str, nnode_line) = {
        let t = next_tok!(dy_line);
        (t.0.clone(), t.1)
    };
    let nnode = parse_usize(&nnode_str, nnode_line, "nnode")?;

    // ------------------------------------------------------------------
    // Node coordinates
    // ------------------------------------------------------------------
    let mut nodes: Vec<(f64, f64)> = Vec::with_capacity(nnode);
    for _i in 0..nnode {
        let ctx_line = tokens.get(pos).map_or(nnode_line, |t| t.1);

        let (x_str, x_line) = {
            let t = next_tok!(ctx_line);
            (t.0.clone(), t.1)
        };
        let (y_str, y_line) = {
            let t = next_tok!(x_line);
            (t.0.clone(), t.1)
        };

        let x = parse_f64(&x_str, x_line, "node x")?;
        let y = parse_f64(&y_str, y_line, "node y")?;

        if !x.is_finite() {
            return Err(MeshError::Parse {
                line: x_line,
                msg: "node x coordinate is not finite".to_string(),
            });
        }
        if !y.is_finite() {
            return Err(MeshError::Parse {
                line: y_line,
                msg: "node y coordinate is not finite".to_string(),
            });
        }

        nodes.push((x, y));
    }

    // ------------------------------------------------------------------
    // nele
    // ------------------------------------------------------------------
    let ctx_line_nele = tokens.get(pos).map_or(nnode_line, |t| t.1);
    let (nele_str, nele_line) = {
        let t = next_tok!(ctx_line_nele);
        (t.0.clone(), t.1)
    };
    let nele = parse_usize(&nele_str, nele_line, "nele")?;

    // ------------------------------------------------------------------
    // Edge records: k1 k2 kp  (k1, k2 are 1-based in the file)
    // ------------------------------------------------------------------
    let mut edges: Vec<Edge> = Vec::with_capacity(nele);
    for edge_idx in 0..nele {
        let ctx_line = tokens.get(pos).map_or(nele_line, |t| t.1);

        let (k1_str, k1_line) = {
            let t = next_tok!(ctx_line);
            (t.0.clone(), t.1)
        };
        let (k2_str, k2_line) = {
            let t = next_tok!(k1_line);
            (t.0.clone(), t.1)
        };
        let (kp_str, kp_line) = {
            let t = next_tok!(k2_line);
            (t.0.clone(), t.1)
        };

        // k1 / k2: 1-based Fortran → 0-based Rust
        let k1_one = parse_usize(&k1_str, k1_line, "k1")?;
        let k2_one = parse_usize(&k2_str, k2_line, "k2")?;
        let kp = parse_i32(&kp_str, kp_line, "kp")?;

        if k1_one == 0 {
            return Err(MeshError::NodeIndexOutOfRange {
                edge: edge_idx,
                node: 0,
            });
        }
        if k2_one == 0 {
            return Err(MeshError::NodeIndexOutOfRange {
                edge: edge_idx,
                node: 0,
            });
        }

        let k1 = k1_one - 1; // convert to 0-based
        let k2 = k2_one - 1;

        if k1 >= nnode {
            return Err(MeshError::NodeIndexOutOfRange {
                edge: edge_idx,
                node: k1,
            });
        }
        if k2 >= nnode {
            return Err(MeshError::NodeIndexOutOfRange {
                edge: edge_idx,
                node: k2,
            });
        }

        if kp < 1 {
            return Err(MeshError::Parse {
                line: kp_line,
                msg: "kp must be >= 1".to_string(),
            });
        }

        edges.push(Edge { k1, k2, kp });
    }

    Ok(MeshInput {
        dx,
        dy,
        nodes,
        edges,
    })
}

// ---------------------------------------------------------------------------
// Private parsing helpers
// ---------------------------------------------------------------------------

fn parse_f64(s: &str, line: usize, field: &str) -> Result<f64, MeshError> {
    s.parse::<f64>().map_err(|_| MeshError::Parse {
        line,
        msg: format!("expected f64 for {field}, got {s:?}"),
    })
}

fn parse_usize(s: &str, line: usize, field: &str) -> Result<usize, MeshError> {
    s.parse::<usize>().map_err(|_| MeshError::Parse {
        line,
        msg: format!("expected non-negative integer for {field}, got {s:?}"),
    })
}

fn parse_i32(s: &str, line: usize, field: &str) -> Result<i32, MeshError> {
    s.parse::<i32>().map_err(|_| MeshError::Parse {
        line,
        msg: format!("expected integer for {field}, got {s:?}"),
    })
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // ---- helpers ----

    fn write_temp(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().expect("tempfile");
        f.write_all(content.as_bytes()).expect("write");
        f
    }

    // ---- parse_project ----

    #[test]
    fn test_parse_project_happy_path() {
        let f = write_temp("test_poly.txt\nfortran_reference.txt\noutplot1.plt\noutplot2.plt\n");
        let cfg = parse_project(f.path()).expect("should parse");
        assert_eq!(cfg.geometry_file, "test_poly.txt");
        assert_eq!(cfg.ch2_file, "fortran_reference.txt");
        assert_eq!(cfg.ch3_file, "outplot1.plt");
        assert_eq!(cfg.ch4_file, "outplot2.plt");
    }

    #[test]
    fn test_parse_project_trims_whitespace() {
        let f = write_temp("  file_a.txt  \n  file_b.txt  \n  file_c.txt  \n  file_d.txt  \n");
        let cfg = parse_project(f.path()).expect("should parse");
        assert_eq!(cfg.geometry_file, "file_a.txt");
        assert_eq!(cfg.ch4_file, "file_d.txt");
    }

    #[test]
    fn test_parse_project_truncates_at_80_chars() {
        // A line of 90 'a' chars: Fortran reads only 80, so trimmed = 80 'a's
        let long_line = "a".repeat(90);
        let content = format!("{long_line}\nb.txt\nc.txt\nd.txt\n");
        let f = write_temp(&content);
        let cfg = parse_project(f.path()).expect("should parse");
        assert_eq!(cfg.geometry_file.len(), 80);
    }

    #[test]
    fn test_parse_project_empty_line_is_error() {
        let f = write_temp("file_a.txt\n\nfile_c.txt\nfile_d.txt\n");
        let err = parse_project(f.path()).expect_err("should fail on empty line");
        match err {
            MeshError::Parse { line, msg } => {
                assert_eq!(line, 2);
                assert!(msg.contains("empty path"), "msg was: {msg}");
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn test_parse_project_missing_file() {
        let err = parse_project(Path::new("/nonexistent/path/project.txt"))
            .expect_err("should fail for missing file");
        assert!(matches!(err, MeshError::Io(_)));
    }

    #[test]
    fn test_parse_project_too_few_lines() {
        let f = write_temp("file_a.txt\nfile_b.txt\n");
        let err = parse_project(f.path()).expect_err("should fail with too few lines");
        assert!(matches!(err, MeshError::Parse { line: 3, .. }));
    }

    // ---- parse_geometry ----

    fn make_geometry_cfg(geometry_path: &str) -> ProjectConfig {
        ProjectConfig {
            geometry_file: geometry_path.to_string(),
            ch2_file: String::new(),
            ch3_file: String::new(),
            ch4_file: String::new(),
        }
    }

    const SAMPLE_GEOM: &str = "\
1. 1.
10
0. 0.
10. 0.
10. 8.
12. 11.
3.0 8.
2. 2.
4. 2.
4. 4.
3. 5.
2. 5.
10
1 2 1
2 3 1
3 4 1
4 5 1
5 1 1
6 7 2
7 8 2
8 9 2
9 10 2
10 6 2
";

    #[test]
    fn test_parse_geometry_happy_path() {
        let f = write_temp(SAMPLE_GEOM);
        let cfg = make_geometry_cfg(f.path().to_str().unwrap());
        let mesh = parse_geometry(&cfg).expect("should parse");

        assert_eq!(mesh.dx, 1.0);
        assert_eq!(mesh.dy, 1.0);
        assert_eq!(mesh.nodes.len(), 10);
        assert_eq!(mesh.edges.len(), 10);

        // First and last nodes
        assert_eq!(mesh.nodes[0], (0.0, 0.0));
        assert_eq!(mesh.nodes[9], (2.0, 5.0));

        // First edge: Fortran "1 2 1" → k1=0, k2=1, kp=1
        assert_eq!(
            mesh.edges[0],
            Edge {
                k1: 0,
                k2: 1,
                kp: 1
            }
        );

        // Last edge: Fortran "10 6 2" → k1=9, k2=5, kp=2
        assert_eq!(
            mesh.edges[9],
            Edge {
                k1: 9,
                k2: 5,
                kp: 2
            }
        );
    }

    #[test]
    fn test_parse_geometry_dx_dy_positive() {
        let bad = "0. 1.\n1\n0. 0.\n1\n1 1 1\n";
        let f = write_temp(bad);
        let err = parse_geometry(&make_geometry_cfg(f.path().to_str().unwrap()))
            .expect_err("dx=0 should fail");
        assert!(matches!(err, MeshError::Parse { line: 1, .. }));
    }

    #[test]
    fn test_parse_geometry_negative_dy_is_error() {
        let bad = "1. -1.\n1\n0. 0.\n1\n1 1 1\n";
        let f = write_temp(bad);
        let err = parse_geometry(&make_geometry_cfg(f.path().to_str().unwrap()))
            .expect_err("dy<0 should fail");
        assert!(matches!(err, MeshError::Parse { .. }));
    }

    #[test]
    fn test_parse_geometry_node_index_out_of_range() {
        // nnode=2, but edge references node index 5 (1-based), which is ≥ nnode
        let bad = "1. 1.\n2\n0. 0.\n1. 1.\n1\n1 5 1\n";
        let f = write_temp(bad);
        let err = parse_geometry(&make_geometry_cfg(f.path().to_str().unwrap()))
            .expect_err("out-of-range index should fail");
        assert!(matches!(
            err,
            MeshError::NodeIndexOutOfRange { edge: 0, node: 4 }
        ));
    }

    #[test]
    fn test_parse_geometry_kp_must_be_positive() {
        let bad = "1. 1.\n2\n0. 0.\n1. 1.\n1\n1 2 0\n";
        let f = write_temp(bad);
        let err = parse_geometry(&make_geometry_cfg(f.path().to_str().unwrap()))
            .expect_err("kp=0 should fail");
        match err {
            MeshError::Parse { msg, .. } => assert!(msg.contains("kp must be >= 1")),
            other => panic!("unexpected: {other}"),
        }
    }

    #[test]
    fn test_parse_geometry_blank_lines_skipped() {
        // Blank lines between tokens should be transparent (list-directed I/O)
        let with_blanks = "1. 1.\n\n2\n\n0. 0.\n1. 1.\n\n1\n1 2 1\n";
        let f = write_temp(with_blanks);
        let cfg = make_geometry_cfg(f.path().to_str().unwrap());
        let mesh = parse_geometry(&cfg).expect("blank lines should be skipped");
        assert_eq!(mesh.nodes.len(), 2);
        assert_eq!(mesh.edges.len(), 1);
    }

    #[test]
    fn test_parse_geometry_missing_file() {
        let cfg = make_geometry_cfg("/nonexistent/geom.txt");
        let err = parse_geometry(&cfg).expect_err("missing file should fail");
        assert!(matches!(err, MeshError::Io(_)));
    }
}
