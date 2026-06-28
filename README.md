# Mesh Generator Suite

A cross-platform desktop GUI for generating two-dimensional meshes, combining a
Rust-based structured quad-mesh engine with the EasyMesh unstructured Delaunay
triangulator.

![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-blue)
![Language](https://img.shields.io/badge/language-Rust-orange)
![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-green)

---

## Overview

The suite contains three components that work together:

| Component | Language | Role |
|---|---|---|
| `mesh_gui` | Rust (eframe/egui) | Desktop GUI — the main application |
| `structured_mesh` | Rust | Structured quad-mesh library and CLI |
| `EasyMesh` | C++ | Unstructured Delaunay triangulator (third-party, Bojan Niceno) |

---

## Features

### Structured Mesh engine
- Rasterises arbitrary 2-D polygons onto a rectangular quad grid
- Supports multiple materials, holes, and nested domains
- Outputs: Tecplot FEPOINT `.plt`, Tecplot zones `.plt`, VTK XML `.vtu`, plain text `.txt`
- Both File mode (read geometry file) and Manual mode (enter geometry in the GUI)

### EasyMesh engine (unstructured)
- Constrained Delaunay triangulation with local refinement
- Supports holes, multiple materials, interior open chains
- Outputs: `.n` / `.e` / `.s` node/element/side files, Tecplot `.dat`, ParaView `.vtk`
- Build the C++ source directly from the GUI if the binary is not present

### GUI
- Engine selector — switch between Structured and Unstructured without losing state
- Input validation with inline red-border highlighting and winding-order warnings
- Non-blocking file dialogs (no UI freeze on Linux xdg-portal)
- Save / load the entire project state as JSON
- Live log panel with auto-scroll showing all runner output
- "Build EasyMesh from Source" button with live compiler output in the log
- Cross-platform: Linux, macOS, Windows

---

## Repository Layout

```
mesh_generator/
├── mesh_gui/           Rust eframe/egui GUI application
│   ├── src/
│   │   ├── main.rs
│   │   ├── app.rs          Top-level MeshApp + dialog plumbing
│   │   ├── state/          Serialisable project state
│   │   ├── runner/         Worker thread, structured and EasyMesh runners
│   │   ├── ui/             Per-panel UI code
│   │   └── templates.rs    Built-in example file content
│   └── Cargo.toml
├── structured_mesh/    Rust structured-mesh library + CLI
│   ├── src/
│   ├── examples/       10 built-in benchmark domains
│   └── Cargo.toml
├── EasyMesh/           C++ Delaunay triangulator (original by Bojan Niceno)
│   ├── Src/            C++ source files + Makefile
│   └── Examples/       Sample .d input files
└── README.md           ← you are here
```

---

## Prerequisites

### For the GUI

- Rust toolchain ≥ 1.85 — install from https://rustup.rs
- On Linux: a working OpenGL driver and a desktop compositor (X11 or Wayland)

### For EasyMesh (optional — only needed to generate unstructured meshes)

- A C++ compiler (`g++` or `clang++`) and `make`
- Or: build directly from the GUI with the **Build EasyMesh from Source** button

---

## Building

Clone the repository:

```bash
git clone https://github.com/<your-username>/mesh_generator.git
cd mesh_generator
```

### Build and run the GUI (debug)

```bash
cd mesh_gui
cargo run
```

### Build the GUI (release — recommended for daily use)

```bash
cd mesh_gui
cargo build --release
./target/release/meshgen_ui
```

### Build the structured mesh CLI only

```bash
cd structured_mesh
cargo build --release
./target/release/structured_mesh --help
```

### Build EasyMesh from C++ source

```bash
cd EasyMesh/Src
make
# Binary produced at EasyMesh/Src/Easy (Linux/macOS) or Easy.exe (Windows)
```

Or use the **Build EasyMesh from Source** button in the GUI under the
Unstructured Mesh tab.

---

## Running the GUI

After building, launch the executable:

```bash
# From the repo root
./mesh_gui/target/release/meshgen_ui
```

The GUI automatically detects the `EasyMesh/Src/Easy` binary relative to the
executable location — no manual configuration required as long as the directory
layout above is preserved.

### Structured Mesh workflow

1. Switch to the **Structured Mesh** tab
2. Choose **File Mode** and browse to a `geometry.txt`, or switch to **Manual Mode**
   and enter nodes and edges in the tables
3. In the right panel, tick output formats and choose an output directory
4. Click **▶ Generate Mesh** — progress and results appear in the log at the bottom

### Unstructured Mesh (EasyMesh) workflow

1. Switch to the **Unstructured Mesh (EasyMesh)** tab
2. Choose **File Mode** and browse to a `.d` input file, or switch to **Manual Mode**
3. In the right panel, configure output formats, aggressiveness, and toggles
4. Verify the binary path shows **✔ Binary found** — if not, click
   **🔨 Build EasyMesh from Source**
5. Choose an output directory and click **▶ Generate Mesh**

### Save and load projects

Use **Save Project** / **Load Project** in the top bar to persist the entire
GUI state (both engines, all settings) as a `.json` file.

---

## Input Formats

### Structured mesh geometry file

```
dx  dy           # cell spacing (both > 0)
nnode
x1  y1
...
xN  yN
nele
k1  k2  kp       # 1-based node indices, region tag ≥ 1
...
```

- Outer boundary: **counter-clockwise** winding (positive signed area)
- Holes: **clockwise** winding (negative signed area)

### EasyMesh `.d` file

```
<number of points>
<i>: <x> <y> <spacing> <marker>
...
<number of segments>
<i>: <start_point> <end_point> <marker>
...
```

- `spacing` must be **> 0**; zero causes an infinite loop in the triangulator
- `marker` must be **> 0** for boundary/hole points; 0 is allowed for interior
  refinement chains only
- Outer boundary: **counter-clockwise**; holes: **clockwise**

---

## Running Tests

```bash
# All tests for both crates
cd structured_mesh && cargo test
cd ../mesh_gui && cargo test
```

The EasyMesh integration tests (`integration_easymesh_end_to_end_with_tec_and_vtk`
and `integration_build_easymesh_from_source`) are skipped automatically when the
binary or `make` is not available.

---

## Dependencies

### `mesh_gui`

| Crate | Purpose |
|---|---|
| `eframe 0.31` | Native desktop window + OpenGL |
| `egui_extras 0.31` | `TableBuilder` for editable tables |
| `rfd 0.15` | Native file dialogs (non-blocking via `std::thread`) |
| `serde 1` + `serde_json 1` | Project file serialisation |
| `anyhow 1` | Application-level error handling |
| `thiserror 1` | Typed `AppError` enum |
| `structured_mesh` | In-process structured mesh generation |

### `structured_mesh`

| Crate | Purpose |
|---|---|
| `thiserror 1` | `MeshError` typed errors |
| `clap 4` | CLI argument parsing |
| `quick-xml 0.31` | VTK XML output |

---

## Third-party: EasyMesh

EasyMesh is an open-source C++ program written by **Bojan Niceno**
(bojan.niceno@psi.ch). It is included in this repository as-is under its
original terms. See `EasyMesh/README.md` for full documentation, algorithm
details, and input/output format specifications.

---

## License

The Rust code (`mesh_gui` and `structured_mesh`) is dual-licensed under either:

- **MIT License** — see `LICENSE-MIT`
- **Apache License, Version 2.0** — see `LICENSE-APACHE`

at your option.

EasyMesh is the original work of Bojan Niceno and retains its original terms.
