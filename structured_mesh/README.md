# structured_mesh

A structured polygon mesh generator written in idiomatic Rust — a complete
rewrite of the legacy Fortran 77 program `polygon3.for`.

Given a polygon description (boundary nodes + directed edges with region tags),
the tool rasterises the domain onto a rectangular quad grid and writes the mesh
in Tecplot ASCII and VTK XML formats.

---

## Table of contents

1. [Quick start](#quick-start)
2. [Input format](#input-format)
3. [Output files](#output-files)
4. [Examples](#examples)
5. [Algorithm](#algorithm)
6. [Architecture](#architecture)
7. [Building and testing](#building-and-testing)
8. [Error reference](#error-reference)
9. [Differences from the Fortran original](#differences-from-the-fortran-original)

---

## Quick start

```bash
git clone <repo-url>
cd structured_mesh
cargo build --release

# Run an example (geometry paths resolve relative to CWD)
cd examples/01_unit_square
../../target/release/structured_mesh --project Polygon_project.txt
```

---

## Input format

### Project file (`Polygon_project.txt`)

Four lines, one path per line (up to 80 chars each):

```
geometry.txt          ← polygon geometry input
output_text.txt       ← legacy text output (spans + connectivity)
mesh_fepoint.plt      ← Tecplot FEPOINT output
mesh_zones.plt        ← Tecplot structured-zone output
```

> Geometry paths are resolved relative to the **working directory** when the
> tool is run, matching the Fortran original's behaviour. Run from inside the
> example directory or supply absolute paths.

### Geometry file

List-directed format — blank lines are ignored, tokens are whitespace-separated.

```
dx  dy
```
Cell width and height (both must be `> 0`). Smaller values produce finer grids.

```
nnode
x1  y1
...
xN  yN
nele
k1  k2  kp
...
```

| Field | Description |
|---|---|
| `nnode` | Number of boundary nodes |
| `x y` | Node coordinates |
| `nele` | Number of directed edges |
| `k1 k2` | Start / end node indices, **1-based** |
| `kp` | Region / boundary tag, must be `≥ 1` |

**Winding convention**

| Boundary type | Winding | Signed area |
|---|---|---|
| Outer domain | CCW | positive |
| Hole | CW | negative |
| Island inside hole | CCW | positive |

**Minimal example** — outer rectangle with one hole:

```
1.0 1.0
8
0.0  0.0
10.0 0.0
10.0 8.0
0.0  8.0
2.0  2.0
4.0  2.0
4.0  4.0
2.0  4.0
8
1 2 1
2 3 1
3 4 1
4 1 1
5 8 2
8 7 2
7 6 2
6 5 2
```

---

## Output files

| File | Format | Opens in |
|---|---|---|
| `output_text.txt` (ch2) | Legacy text — spans + quad connectivity | Any text editor; legacy Fortran tools |
| `mesh_fepoint.plt` (ch3) | Tecplot FEPOINT quadrilateral zone | Tecplot 360, FieldView |
| `mesh_zones.plt` (ch4) | Tecplot POINT + structured zones | Tecplot 360 |
| `mesh.vtu` | VTK XML Unstructured Grid | ParaView, VisIt |

The legacy text format (`output_text.txt`) replicates Fortran FORMAT statements
byte-for-byte: `'(5i6)'` for span records, `'(2f11.1)'` for node coordinates,
`'(6i9)'` for element connectivity with the Fortran `n4/n3` swap.

### Cell classification

After rasterisation each cell in the grid holds one of:

| Value | Meaning |
|---|---|
| `0` | Exterior (outside all domains or inside a hole) |
| `1` | Interior |
| `≥ 2` | Boundary (equals the `kp` region tag of the bounding edge) |

---

## Examples

All examples live under `examples/`. Run from inside the example directory.

```bash
cd examples/<name>
../../target/release/structured_mesh --project Polygon_project.txt
```

### Simple benchmarks

| # | Name | Nodes | Elements | Tests |
|---|---|---|---|---|
| 01 | `01_unit_square` | 121 | 100 | Baseline; grid origin offsets |
| 02 | `02_l_shaped_domain` | 65 | 48 | Re-entrant 270° corner (NIST AMR) |
| 03 | `03_square_with_hole` | 425 | 375 | Single hole; winding-order detection |
| 04 | `04_backward_facing_step` | 177 | 144 | CFD benchmark (Armaly et al. 1983) |
| 05 | `05_regular_hexagon` | 295 | 257 | Oblique edges; area convergence |

### Complex benchmarks

| # | Name | Nodes | Elements | Tests |
|---|---|---|---|---|
| 06 | `06_rectangle_two_holes` | 855 | 752 | Two holes with distinct region tags |
| 07 | `07_c_channel` | 399 | 336 | Two simultaneous re-entrant corners |
| 08 | `08_swiss_cheese` | 1796 | 1636 | Five holes; maximum hole-count stress test |
| 09 | `09_two_bodies` | 619 | 536 | Two disconnected CCW bodies; hole pairing |
| 10 | `10_nested_domains` | 1517 | 1344 | Three-level nesting: outer → void → island |

Each example directory contains its own `README.md` with geometry diagrams,
winding-order tables, verification quantities, and references.

---

## Algorithm

### Grid sizing

The rasterisation grid is derived from the polygon bounding box using the same
magic offsets as the Fortran original (prevents boundary nodes landing on cell
edges):

```
xm0 = xmin - dx/2 - 0.012549
ym0 = ymin - dy/2 - 0.05497
nx  = floor((xmax + dx/2 - xm0) / dx + 0.5)
ny  = floor((ymax + dy/2 - ym0) / dy + 0.5)
```

Cell centre for column `i`, row `j` (1-based):

```
x = (i-1)*dx + xm0 + dx/2
y = (j-1)*dy + ym0 + dy/2
```

### Scanline fill (two passes)

**X-pass** — for each horizontal scanline `y0`, collect edge intersections
where `y0 ∈ (min(y1,y2), max(y1,y2)]`, sort by x, process in pairs:
left-boundary tag → interior fill → right-boundary tag.

**Y-pass** — same vertically with `x0 ∈ (min(x1,x2), max(x1,x2)]`. Only
writes `1` (interior) if the cell currently holds `< 2`, preserving boundary
tags set by the X-pass.

The half-open interval convention guarantees an even intersection count for
any well-formed (non-self-intersecting) polygon.

### Hole handling

`assign_holes` classifies boundaries by winding order (shoelace formula):
CCW boundaries are outer domains, CW boundaries are holes. Each CW hole is
paired with the smallest CCW boundary that contains its centroid via a
ray-cast point-in-polygon test.

---

## Architecture

```
Polygon_project.txt
    │
    ▼
input_parser::parse_project()   →  ProjectConfig
    │
    ▼
input_parser::parse_geometry()  →  MeshInput { dx, dy, nodes, edges }
    │
    ▼
scanline_rasterizer::compute_grid()  →  GeometryData { xm0, ym0, nx, ny }
    │
    ▼
scanline_rasterizer::rasterize()     →  MeshState { matr[nx][ny], row/col records }
    │
    ▼
scanline_rasterizer::build_plot_data()  →  PlotData { nodes, elements, region_tags }
    │
    ├──► output_text::write_ch2()     →  output_text.txt
    ├──► output_tecplot::write_ch3()  →  mesh_fepoint.plt
    ├──► output_tecplot::write_ch4()  →  mesh_zones.plt
    └──► output_vtk::write_vtu()      →  mesh.vtu
```

### Module summary

| Module | Responsibility |
|---|---|
| `error` | `MeshError` — all typed error variants via `thiserror` |
| `input_parser` | Project file (4×80-char lines) and geometry file (list-directed) |
| `geometry_core` | Signed area, winding order, centroid, point-in-polygon, segment intersections, hole assignment |
| `scanline_rasterizer` | Grid sizing, X/Y-pass fill, quad connectivity assembly |
| `output_text` | Legacy text format; Fortran `'(5i6)'`, `'(2f11.1)'`, `'(6i9)'` helpers |
| `output_tecplot` | Tecplot FEPOINT (`ch3`) and structured-zone (`ch4`) writers |
| `output_vtk` | VTK XML (`mesh.vtu`) via `quick-xml` |

---

## Building and testing

```bash
# Debug build
cargo build

# Release build (recommended for large meshes)
cargo build --release

# Run all tests
cargo test

# Check without building
cargo check

# Generate and open HTML docs
cargo doc --open
```

### Dependencies

| Crate | Use |
|---|---|
| `thiserror 1` | Typed error enum |
| `clap 4` | CLI argument parsing |
| `quick-xml 0.31` | Well-formed VTK XML output |
| `proptest 1` *(dev)* | Property-based tests |
| `tempfile 3` *(dev)* | Temporary files in tests |

---

## Error reference

| Error message | Cause |
|---|---|
| `I/O error: No such file or directory` | Project or geometry file not found |
| `parse error at line N: dx must be finite and > 0` | Cell spacing ≤ 0 or non-numeric |
| `node index out of range: edge E references node N` | 1-based index exceeds `nnode` |
| `orphaned hole: CW boundary with region tag T has no enclosing CCW outer boundary` | CW polygon has no containing CCW polygon — check winding order |
| `odd intersection count at scanline …` | Self-intersecting polygon |
| `grid too large: requested NX×NY cells` | Only from `compute_grid_capped`; increase `dx`/`dy` or use `compute_grid` |

---

## Differences from the Fortran original

| Aspect | Fortran `polygon3.for` | This crate |
|---|---|---|
| Grid size | Hard-coded `matr(1500,1200)` | Unlimited — heap `Vec<Vec<i32>>` |
| Node count | `np = 5000` | Unlimited — `Vec<(f64,f64)>` |
| Intersection buffer | `ns = 50` per scanline | Unlimited — `Vec<(f64,i32)>` |
| Temp files | Unit 16 (`t`) | None — all state in `MeshState` |
| Duplicate output | `geom3.dat` + `ch2` (identical) | Single `ch2` output |
| Hole handling | Ad-hoc polygon-index convention | Winding-order + point-in-polygon |
| VTK output | Not present | `mesh.vtu` for ParaView/VisIt |
| Error handling | Silent / `stop` | Typed `MeshError`, stderr, exit 1 |

---

## License

MIT or Apache-2.0 — your choice.
