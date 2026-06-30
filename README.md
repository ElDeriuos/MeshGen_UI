# MeshGen UI

A cross-platform desktop GUI for generating two-dimensional finite element meshes,
combining a Rust-native structured quad-mesh engine with the EasyMesh unstructured
Delaunay triangulator.

[![GitHub](https://img.shields.io/badge/GitHub-ElDeriuos%2FMeshGen__UI-blue)](https://github.com/ElDeriuos/MeshGen_UI)
![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20Windows%20%7C%20MacOS-blue)
![Language](https://img.shields.io/badge/language-Rust%20%2B%20C%2B%2B-orange)
![License](https://img.shields.io/badge/license-MIT%20-green)

![MeshGen UI Main Dashboard](/home/elderiuos/Pictures/Screenshots/ss2.png)
---

## Table of Contents

1. [Overview](#overview)
2. [Pre-compiled Releases](#pre-compiled-releases)
3. [Repository Layout](#repository-layout)
4. [Building from Source](#building-from-source)
5. [Structured Mesh Engine](#structured-mesh-engine)
   - [Input Format](#structured-mesh-input-format)
   - [Output Files](#structured-mesh-output-files)
   - [Algorithm](#structured-mesh-algorithm)
6. [EasyMesh Engine (Unstructured)](#easymesh-engine-unstructured)
   - [Input Format](#easymesh-input-format)
   - [Output Files](#easymesh-output-files)
   - [Mesh Quality Controls](#mesh-quality-controls)
   - [Technical Limitations](#easymesh-technical-limitations)
7. [GUI Reference](#gui-reference)
   - [Layout](#layout)
   - [Input Validation](#input-validation)
   - [Project Save and Load](#project-save-and-load)
8. [Dependencies](#dependencies)
9. [Third-party: EasyMesh](#third-party-easymesh)
10. [License](#license)
11. [Disclaimer](#disclaimer)

---

## Overview

MeshGen UI consists of three components that work together as a single application:

| Component | Language | Role |
|---|---|---|
| `mesh_gui` | Rust — eframe / egui | Desktop GUI, the main executable (`meshgen_ui` / `meshgen_ui.exe`) |
| `structured_mesh` | Rust | Structured quad-mesh library (compiled into the GUI) |
| `EasyMesh` | C++ | Unstructured Delaunay triangulator — separate binary (`Easy` / `Easy.exe`) |

The GUI runs both engines from the same window. The structured engine runs
in-process (no subprocess). The EasyMesh engine is spawned as a subprocess; its
binary must be present alongside `meshgen_ui` or built from source using the
in-app button.

---

## Pre-compiled Releases

Pre-compiled bundles for Linux and Windows are available on the
[Releases page](https://github.com/ElDeriuos/MeshGen_UI/releases).

Each archive contains:

```
meshgen_ui          (Linux) or meshgen_ui.exe  (Windows)
Easy                (Linux) or Easy.exe        (Windows)
outputs/            default output directory (created automatically)
```

**Usage:**

1. Download and unzip the archive for your platform.
2. Run `meshgen_ui` (or `meshgen_ui.exe` on Windows).
3. The GUI automatically detects `Easy` / `Easy.exe` in the same directory.

No installation or configuration is required. On Linux, mark the binaries
executable if needed:

```bash
chmod +x meshgen_ui Easy
./meshgen_ui
```

---

## Repository Layout

```
MeshGen_UI/
├── mesh_gui/                   Rust GUI application (eframe/egui)
│   ├── src/
│   │   ├── main.rs             Entry point — 1200×800 window
│   │   ├── app.rs              MeshApp struct, event loop, dialog plumbing
│   │   ├── templates.rs        Built-in example file content (hardcoded)
│   │   ├── state/
│   │   │   ├── mod.rs
│   │   │   ├── project.rs      ProjectState, Engine enum, InputMode enum
│   │   │   ├── structured.rs   StructuredState — nodes, edges, format flags
│   │   │   └── easymesh.rs     EasyMeshState — points, segments, CLI flags
│   │   ├── runner/
│   │   │   ├── mod.rs          RunRequest, WorkerMsg, AppError, worker thread
│   │   │   ├── structured_runner.rs  In-process structured mesh pipeline
│   │   │   ├── easymesh_runner.rs    EasyMesh subprocess management
│   │   │   └── easymesh_builder.rs   `make`-based build-from-source flow
│   │   └── ui/
│   │       ├── mod.rs
│   │       ├── engine_selector.rs    Top bar — engine tabs, Save/Load
│   │       ├── structured_panel.rs   Structured input (File/Manual mode)
│   │       ├── easymesh_panel.rs     EasyMesh input (File/Manual mode)
│   │       ├── output_panel.rs       Format checkboxes, directory, Generate button
│   │       ├── log_panel.rs          Scrollable log with auto-scroll
│   │       └── validation.rs         Field-level validators, winding checks
│   └── Cargo.toml
├── structured_mesh/            Rust structured-mesh library + standalone CLI
│   ├── src/
│   │   ├── lib.rs
│   │   ├── main.rs
│   │   ├── error.rs
│   │   ├── input_parser.rs
│   │   ├── geometry_core.rs
│   │   ├── scanline_rasterizer.rs
│   │   ├── output_text.rs
│   │   ├── output_tecplot.rs
│   │   └── output_vtk.rs
│   ├── examples/               10 benchmark geometry files
│   └── Cargo.toml
├── EasyMesh/                   C++ Delaunay triangulator (Bojan Niceno)
│   ├── Src/                    C++ source files + Makefile
│   │   ├── Makefile
│   │   ├── easymesh.cpp / .h
│   │   └── *.cpp / *.h
│   ├── Examples/               Sample .d input files
│   └── AUTHORS
├── Makefile                    Root build: builds both binaries and places them here
└── README.md
```

---

## Building from Source

### Prerequisites

- **Rust** ≥ 1.85 — install from [rustup.rs](https://rustup.rs)
- **g++ / clang++** and **make** — for the EasyMesh C++ binary
- **Linux**: a working OpenGL driver and a desktop compositor (X11 or Wayland)
- **Windows**: Visual Studio Build Tools or MinGW; or build under WSL

### Build everything at once (recommended)

From the repository root:

```bash
make
```

This builds both `Easy` (C++) and `meshgen_ui` (Rust release) and places both
binaries in the repository root. Run `meshgen_ui` from there.

To clean all build artifacts (keeps the root binaries):

```bash
make clean
```

### Build components individually

**GUI only (release):**

```bash
cd mesh_gui
cargo build --release
# binary: mesh_gui/target/release/meshgen_ui
```

**EasyMesh C++ binary only:**

```bash
cd EasyMesh/Src
make
# binary: EasyMesh/Src/Easy  (or Easy.exe on Windows)
```

After building individually, place `Easy` / `Easy.exe` next to `meshgen_ui` so
the GUI can find it automatically.

### Binary auto-discovery

On startup `meshgen_ui` searches for the EasyMesh binary in this priority order:

1. Same directory as the `meshgen_ui` executable (`<exe_dir>/Easy`)
2. `<exe_dir>/../EasyMesh/Src/Easy` through `../../../../EasyMesh/Src/Easy`
3. Walk up to 6 levels from the exe directory checking for `EasyMesh/Src/Easy`
4. Fallback: `EasyMesh/Src/Easy` relative to the working directory

If the binary is not found the GUI shows a **✘ Binary not found** indicator and
enables the **Build EasyMesh from Source** button.

---

## Structured Mesh Engine

The structured engine rasterises an arbitrary 2-D polygon onto a rectangular quad
grid. It is a complete Rust rewrite of the legacy Fortran 77 program `polygon3.for`,
compiled directly into the GUI (no subprocess).

### Structured Mesh Input Format

Geometry is described in a plain text file (`geometry.txt`):

```
dx  dy
```

Cell spacing in x and y. Both must be `> 0`. Smaller values produce finer grids.

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
| `kp` | Region tag, must be `≥ 1` |

**Winding convention:**

| Boundary type | Winding | Signed area (shoelace) |
|---|---|---|
| Outer domain | Counter-clockwise (CCW) | positive |
| Hole | Clockwise (CW) | negative |
| Island inside hole | CCW | positive |

**Minimal example — unit square:**

```
0.1 0.1
4
0.0 0.0
1.0 0.0
1.0 1.0
0.0 1.0
4
1 2 1
2 3 1
3 4 1
4 1 1
```

### Structured Mesh Output Files

| File | Format | Opens in |
|---|---|---|
| `mesh.vtu` | VTK XML Unstructured Grid | ParaView, VisIt |
| `mesh_fepoint.plt` | Tecplot FEPOINT quad zone | Tecplot 360, FieldView |
| `mesh_zones.plt` | Tecplot POINT + structured zones | Tecplot 360 |
| `output_text.txt` | Legacy text — span records + quad connectivity | Any text editor |

The legacy text format replicates Fortran FORMAT statements byte-for-byte:
`'(5i6)'` for span records, `'(2f11.1)'` for node coordinates, `'(6i9)'` for
element connectivity with the Fortran `n4/n3` swap — compatible with legacy
Fortran post-processing tools.

### Cell Classification

After rasterisation each cell holds one of:

| Value | Meaning |
|---|---|
| `0` | Exterior (outside all domains or inside a hole) |
| `1` | Interior |
| `≥ 2` | Boundary (equals the `kp` region tag of the bounding edge) |

### Structured Mesh Algorithm

**Grid sizing** — derived from the polygon bounding box using the same magic
offsets as the Fortran original (prevents boundary nodes landing exactly on cell
edges):

```
xm0 = xmin - dx/2 - 0.012549
ym0 = ymin - dy/2 - 0.05497
nx  = floor((xmax + dx/2 - xm0) / dx + 0.5)
ny  = floor((ymax + dy/2 - ym0) / dy + 0.5)
```

**Scanline fill (two passes):**

- X-pass: for each horizontal scanline `y0`, collect edge intersections where
  `y0 ∈ (min(y1,y2), max(y1,y2)]`, sort by x, process in pairs —
  left-boundary tag → interior fill → right-boundary tag.
- Y-pass: same vertically. Writes `1` only if the cell currently holds `< 2`,
  preserving boundary tags from the X-pass.

The half-open interval convention guarantees an even intersection count for any
well-formed (non-self-intersecting) polygon.

**Hole detection** — `assign_holes` classifies boundaries by winding order
(shoelace formula): CCW boundaries are outer domains, CW boundaries are holes.
Each CW hole is paired with the smallest CCW boundary that contains its centroid
via a ray-cast point-in-polygon test.

**Improvements over the Fortran original:**

| Aspect | Fortran `polygon3.for` | This crate |
|---|---|---|
| Grid size | Hard-coded `matr(1500,1200)` | Unlimited heap `Vec<Vec<i32>>` |
| Node count | Up to 5 000 | Unlimited `Vec<(f64,f64)>` |
| Intersection buffer | 50 per scanline | Unlimited `Vec<(f64,i32)>` |
| Temp files | Fortran unit 16 | None — all state in memory |
| Hole handling | Ad-hoc polygon-index convention | Winding-order + point-in-polygon |
| VTK output | Not present | `mesh.vtu` (ParaView/VisIt) |
| Error handling | Silent / `stop` | Typed errors, stderr, non-zero exit |

---

## EasyMesh Engine (Unstructured)

EasyMesh generates two-dimensional unstructured triangular meshes using constrained
Delaunay triangulation. It is an external C++ binary originally written by
**Bojan Niceno** (bojan.niceno@psi.ch, Paul Scherrer Institut). See
[Third-party: EasyMesh](#third-party-easymesh) for full attribution.

The GUI spawns the `Easy` / `Easy.exe` binary as a subprocess, streams its output
line-by-line into the log panel, and determines success by checking whether the
primary output file (`<stem>.n`) was created — because the binary unconditionally
exits with code 1 even on a fully successful run.

### EasyMesh Input Format

The input file uses the `.d` extension. Comments are enclosed in `#` characters
and may appear anywhere.

```
<number of points>
<index>:  <x>  <y>  <spacing>  <marker>
...
<number of segments>
<index>:  <start_point>  <end_point>  <marker>
...
```

| Field | Description |
|---|---|
| `x`, `y` | Point coordinates |
| `spacing` | Desired triangle side length at this point. Must be **> 0** — zero causes an infinite loop in the triangulator. |
| `marker` | Integer boundary condition tag. Must be **> 0** for boundary / hole points. May be `0` for interior refinement chains only. |
| `start_point`, `end_point` | **0-based** point indices of a segment. |

**Chain orientation rules:**

| Chain type | Winding | Notes |
|---|---|---|
| Outer boundary | CCW | Positive signed area |
| Hole | CW | Negative signed area |
| Interior open chain | Any | Must not start/end on a boundary node; used for local refinement/coarsening |
| False hole | CCW | Interior refinement region; `marker` may be `0` |

### EasyMesh Output Files

All output file names are derived from the input file stem.

| File | Content |
|---|---|
| `<stem>.n` | Node file — coordinates and boundary markers |
| `<stem>.e` | Element file — triangle connectivity, neighbour elements, circumcenters, material markers |
| `<stem>.s` | Side file — edge connectivity, boundary markers |
| `<stem>.dat` | Tecplot ASCII FEPOINT (`+tec` flag) — `FETRIANGLE` zone |
| `<stem>.vtk` | VTK legacy ASCII Unstructured Grid (`+vtk` flag) — point data `BoundaryMarker`, cell data `MaterialMarker` |
| `<stem>.eps` | PostScript drawing (`+eps` flag) — Delaunay and/or Voronoi mesh |

**Node file (`.n`):**

```
<Nn>
<index>:  <x>  <y>  <marker>
```

**Element file (`.e`):**

```
<Ne>
<index>:  <i>  <j>  <k>  <ei>  <ej>  <ek>  <si>  <sj>  <sk>  <xV>  <yV>  <marker>
```

`i j k` — vertex node indices; `ei ej ek` — neighbouring element indices
(−1 = boundary face); `si sj sk` — side indices; `xV yV` — circumcenter
(Voronoi vertex); `marker` — material tag.

**Side file (`.s`):**

```
<Ns>
<index>:  <c>  <d>  <ea>  <eb>  <marker>
```

`c d` — start/end node indices; `ea` — left element; `eb` — right element
(−1 = boundary side); `marker` — boundary condition tag.

### Mesh Quality Controls

The GUI exposes the following EasyMesh options:

| Control | Flag | Description |
|---|---|---|
| Tecplot output | `+tec` | Write `<stem>.dat` |
| VTK output | `+vtk` | Write `<stem>.vtk` |
| EPS drawing | `+eps` | Write `<stem>.eps` (Delaunay + Voronoi) |
| Aggressiveness | `+a 0..6` | Higher levels improve convergence on difficult domains at the cost of mesh quality |
| Skip relaxation | `-r` | Disable node relaxation (faster, lower quality) |
| Skip smoothing | `-s` | Disable Laplacian smoothing |
| Boundary only | `-d` | Generate boundary triangulation only; skip interior node insertion |
| Suppress messages | `-m` | No console output from the binary |

### EasyMesh Technical Limitations

- Default maximum node count is **10 000** (compile-time constant in `easymesh.h`).
  To increase it, edit `#define TEMPORARY_MAX_NODES` and rebuild from source.
- Only **one connected domain** per run; multiple disconnected domains are not supported.
- Spacing for every point must be **strictly > 0**. Zero spacing causes an infinite loop.
- Bandwidth renumbering is always applied (cannot be disabled).

---

## GUI Reference

### Layout

The window is divided into four regions:

```
┌─────────────────────────────────────────────────────┐
│  [Structured Mesh] [Unstructured Mesh (EasyMesh)]   │  ← top bar
│  [Save Project]    [Load Project]                   │
├──────────────────────────────┬──────────────────────┤
│                              │  Output              │
│  Input panel                 │  ─ format checkboxes │
│  (File mode or Manual mode)  │  ─ output directory  │
│                              │  ─ Generate button   │
│                              │  ─ status / spinner  │
├──────────────────────────────┴──────────────────────┤
│  Log  ⏬ [Clear Log]                                │  ← bottom log panel
│                                                     │
└─────────────────────────────────────────────────────┘
```

**Top bar** — switches between the two engines without discarding either engine's
state. Also hosts the Save / Load project buttons.

**Input panel (centre)** — engine-specific. Both engines support:
- **File Mode**: browse for an input file on disk.
- **Manual Mode**: enter geometry directly in editable tables (nodes/edges for
  Structured; points/segments for EasyMesh). The **Generate Example Template**
  button saves a built-in example to disk and opens it in the system editor.

**Output panel (right)** — format checkboxes, output directory picker (defaults
to `outputs/` alongside the executable), and the **▶ Generate Mesh** button.
For EasyMesh: also shows the binary path with a green ✔ / red ✘ indicator and
the **🔨 Build EasyMesh from Source** button when the binary is missing.

**Log panel (bottom)** — shows all runner output (stdout + stderr from the
subprocess, or progress messages from the in-process Rust runner). Auto-scrolls
to the latest line; pauses auto-scroll when you scroll up manually, resumes when
you scroll back to the bottom.

### Input Validation

The GUI validates Manual Mode input every frame and shows:

- **Red borders** on invalid table cells.
- **Red error labels** below the tables (non-parseable values, out-of-range
  indices, zero spacing, region tags < 1).
- **Orange warning labels** for advisory issues that do not block generation:
  - CW outer boundary (should be CCW).
  - CCW hole chain (should be CW).
  - Marker `0` on a point or segment (valid only for interior refinement chains).

The **▶ Generate Mesh** button is disabled until:
- An output directory is set.
- In File mode: the selected file exists on disk.
- In Manual mode: at least 3 nodes/points and 3 edges/segments are present.
- For Structured: at least one output format checkbox is ticked.

### Project Save and Load

**Save Project** serialises the complete GUI state (both engines, all settings,
file paths, output directory) to a JSON file. **Load Project** restores it. The
JSON format is versioned through `serde` derives; unknown fields are ignored to
allow forward compatibility.

---

## Dependencies

### `mesh_gui`

| Crate | Version | Purpose |
|---|---|---|
| `eframe` | 0.31 | Native desktop window, OpenGL renderer, event loop |
| `egui_extras` | 0.31 | `TableBuilder` for editable tables |
| `rfd` | 0.15 | Native OS file dialogs (run on background threads to avoid UI freeze) |
| `serde` + `serde_json` | 1 | Project file serialisation / deserialisation |
| `anyhow` | 1 | Application-level error propagation |
| `thiserror` | 1 | Typed `AppError` enum |
| `structured_mesh` | path | In-process structured mesh library |

### `structured_mesh`

| Crate | Version | Purpose |
|---|---|---|
| `thiserror` | 1 | `MeshError` typed error enum |
| `clap` | 4 | CLI argument parsing (standalone binary) |
| `quick-xml` | 0.31 | Well-formed VTK XML output |

---

## Third-party: EasyMesh

EasyMesh is an open-source C++ program originally written by:

> **Bojan Niceno**
> Paul Scherrer Institut (PSI), Switzerland
> bojan.niceno@psi.ch

It is included in this repository in its original form under its original terms.
The source code in `EasyMesh/Src/` is the work of Bojan Niceno. Minor modifications
have been made to the original C++ source.

EasyMesh has been compiled and tested on the following platforms by its original
author:

| Platform | OS | Compiler |
|---|---|---|
| PC | DOS 6.22 / Windows 95 | Watcom C/C++ 10.0a |
| PC | RedHat Linux 3.03 | gcc |
| PC | Caldera Linux | gcc |
| SUN Ultra 1 Model 140 | SunOS 5.5 | cc |
| Iris Indigo xs24 | IRIX 5.2 | cc |
| Cray J916/8-1024 | UNICOS 8.0.3 | cc |

---

## License

The Rust code (`mesh_gui` and `structured_mesh`) is licensed under:

- **MIT License**

EasyMesh (`EasyMesh/`) is the original work of **Bojan Niceno** and is included
under its original terms. Refer to the `EasyMesh/AUTHORS` file and the source
code headers for details.

---

## Disclaimer

THIS SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
FOR A PARTICULAR PURPOSE, AND NON-INFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES, OR OTHER LIABILITY, WHETHER
IN AN ACTION OF CONTRACT, TORT, OR OTHERWISE, ARISING FROM, OUT OF, OR IN
CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

Use at your own risk. The authors make no guarantees about the correctness,
completeness, or suitability of the generated meshes for any particular purpose,
including but not limited to numerical simulation, engineering analysis, or
scientific research.
