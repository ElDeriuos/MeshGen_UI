# EasyMesh

**EasyMesh** is a two-dimensional, unstructured triangular mesh generator based on
the Delaunay and constrained Delaunay triangulation algorithms. It takes a simple
ASCII domain description as input and produces meshes suitable for finite element
and finite volume analysis.

---

## Table of Contents

1. [Features](#features)
2. [Building EasyMesh](#building-easymesh)
3. [Input File Format](#input-file-format)
4. [Running EasyMesh](#running-easymesh)
5. [Command Line Options](#command-line-options)
6. [Output Files](#output-files)
   - [Node File (.n)](#node-file-n)
   - [Element File (.e)](#element-file-e)
   - [Side File (.s)](#side-file-s)
   - [TecPlot File (.dat)](#tecplot-file-dat)
   - [ParaView VTK File (.vtk)](#paraview-vtk-file-vtk)
7. [Examples](#examples)
   - [Example 1 – Basic domain with a hole](#example-1--basic-domain-with-a-hole)
   - [Example 2 – Circular hole and local refinement](#example-2--circular-hole-and-local-refinement)
   - [Example 3 – Multi-material domain](#example-3--multi-material-domain)
8. [Design Notes and Limitations](#design-notes-and-limitations)
9. [Authors](#authors)

---

## Features

- Generates two-dimensional, unstructured **Delaunay** and **constrained Delaunay**
  triangulations in general planar domains.
- Handles **holes** in the domain.
- Supports **local mesh refinement and coarsening** through different spacing values
  per node and through interior open chains.
- Handles domains composed of **more than one material**.
- Performs **bandwidth-reducing renumbering** of nodes, elements, and sides to
  decrease the bandwidth of the resulting system of equations.
- Built-in **Laplacian smoothing** and **relaxation** to produce high-quality grids
  (nodes are kept within the range of 5–7 surrounding elements).
- Simple **ASCII input** format with comment support.
- Produces three standard **ASCII output files** (`.n`, `.e`, `.s`) containing all
  data needed for numerical analysis.
- Optional **PostScript drawing** output (`.eps`) showing Delaunay and Voronoi meshes.
- Optional **TecPlot ASCII output** (`.dat`) for direct import into TecPlot 360/Focus.
- Optional **ParaView VTK legacy ASCII output** (`.vtk`) for direct import into
  ParaView or VisIt.

---

## Building EasyMesh

EasyMesh is written in standard C++ and compiles without any external dependencies.

### Linux / macOS (GCC or Clang)

```bash
cd Src
make
```

The resulting executable is named `Easy` and is placed in the `Src/` directory.
You can also compile manually:

```bash
g++ -O3 -o Easy *.cpp -lm
```

### Other UNIX-based systems

```bash
cc -o Easy -O3 *.cpp -lm
```

### Increasing the maximum number of nodes

The default internal limit is 10 000 nodes. To increase it, edit `easymesh.h` and
change the constant:

```cpp
#define TEMPORARY_MAX_NODES 10000
```

to a larger value, e.g. `20000`, then recompile.

---

## Input File Format

The input file must have the extension `.d`.  Comments are enclosed between `#`
characters and can appear anywhere in the file.

```
<number of points>
<index:>  <x>  <y>  <spacing>  <marker>
...
<number of segments>
<index:>  <start_point>  <end_point>  <marker>
...
```

| Field | Description |
|---|---|
| `x`, `y` | Coordinates of the point |
| `spacing` | Desired triangle side length at this point. **Must be non-zero**; a zero value can cause an infinite loop. |
| `marker` | Integer boundary condition tag. Must be **> 0** for points and segments on physical boundaries or holes. Can be **0** for internal refinement chains and false holes. |

### Chain orientation rules

- **Outer boundary chain**: nodes must be inserted in **counter-clockwise** order.
- **Hole chains**: nodes must be inserted in **clockwise** order.
- **Internal open chains** (for local refinement/coarsening): must _not_ start or
  end on a boundary chain point (unless they are a single isolated point).
- **False holes** (interior refinement regions): treated like the outer boundary
  (counter-clockwise), but their `marker` may be `0`.

---

## Running EasyMesh

```
Easy NAME [options]
```

`NAME` is the name of the input file with or without the `.d` extension.
Options and the file name can be specified in any order.

---

## Command Line Options

| Option | Description |
|---|---|
| `+a [0..6]` | Increase aggressiveness level (0 = least aggressive, 6 = most). Higher levels improve the chances of completing a difficult mesh but may reduce quality. |
| `-d` | Generate boundary triangulation only; skip interior node insertion, relaxation, and smoothing. |
| `-m` | Suppress all console messages. |
| `-r` | Skip mesh relaxation. |
| `-s` | Skip Laplacian smoothing. |
| `+eps [D] [V]` | Write PostScript drawing. Optionally append `D` for Delaunay mesh only, `V` for Voronoi only. If neither `D` nor `V` is given, both are drawn. |
| `+tec` | Write TecPlot ASCII output (`NAME.dat`). |
| `+vtk` | Write ParaView VTK legacy ASCII output (`NAME.vtk`). |
| `+example` | Generate example input files and exit. |

**Default behaviour** (no options): full triangulation with relaxation and
smoothing, all messages shown, no graphical or visualisation output.

---

## Output Files

All output file names are derived from the input file base name.

### Node File (`.n`)

```
<Nn>
<index:>  <x>  <y>  <marker>
...
```

| Field | Description |
|---|---|
| `Nn` | Total number of nodes |
| `x`, `y` | Node coordinates |
| `marker` | Boundary condition marker (inherited from the input file) |

### Element File (`.e`)

```
<Ne>
<index:>  <i>  <j>  <k>  <ei>  <ej>  <ek>  <si>  <sj>  <sk>  <xV>  <yV>  <marker>
...
```

| Field | Description |
|---|---|
| `Ne` | Total number of triangular elements |
| `i`, `j`, `k` | Node indices of the triangle vertices |
| `ei`, `ej`, `ek` | Indices of the three neighbouring elements (opposite to nodes `i`, `j`, `k` respectively). `-1` means the face is on the boundary. |
| `si`, `sj`, `sk` | Indices of the three element sides |
| `xV`, `yV` | Coordinates of the circumcenter (Voronoi vertex) |
| `marker` | Material marker (useful for multi-material domains) |

### Side File (`.s`)

```
<Ns>
<index:>  <c>  <d>  <ea>  <eb>  <marker>
...
```

| Field | Description |
|---|---|
| `Ns` | Total number of sides (edges) |
| `c`, `d` | Start and end node indices of the side |
| `ea` | Element on the left of the side |
| `eb` | Element on the right of the side. `-1` means the side is on the boundary. |
| `marker` | Boundary condition marker |

### TecPlot File (`.dat`)

Created with the `+tec` option. Format is **TecPlot ASCII FEPOINT** with
`FETRIANGLE` connectivity.

```
TITLE = "EasyMesh triangulation"
VARIABLES = "X", "Y", "BoundaryMarker"
ZONE T="Mesh", N=<Nn>, E=<Ne>, DATAPACKING=POINT, ZONETYPE=FETRIANGLE
<x>  <y>  <BoundaryMarker>        (one line per node)
...
<i>  <j>  <k>                     (one line per element, 1-based indices)
...
```

The file can be opened directly in **TecPlot 360** or **TecPlot Focus** using the
standard "Tecplot Data" loader. The `BoundaryMarker` scalar field is defined on
nodes and can be used to visualise boundary condition regions.

### ParaView VTK File (`.vtk`)

Created with the `+vtk` option. Format is **VTK legacy ASCII Unstructured Grid**
(version 2.0).

```
# vtk DataFile Version 2.0
EasyMesh triangulation
ASCII
DATASET UNSTRUCTURED_GRID
POINTS <Nn> float
<x>  <y>  0.0            (z = 0 for 2-D mesh; one line per node)
...
CELLS <Ne> <Ne*4>
3  <i>  <j>  <k>         (0-based indices; one line per triangle)
...
CELL_TYPES <Ne>
5                         (VTK_TRIANGLE = 5; one line per element)
...
POINT_DATA <Nn>
SCALARS BoundaryMarker int 1
LOOKUP_TABLE default
<marker>                  (one value per node)
...
CELL_DATA <Ne>
SCALARS MaterialMarker int 1
LOOKUP_TABLE default
<material>                (one value per element)
...
```

The file can be opened directly in **ParaView** (File → Open, select VTK Legacy
reader) or in **VisIt**. Two scalar fields are available:

| Field | Location | Description |
|---|---|---|
| `BoundaryMarker` | Point data | Boundary condition tag per node |
| `MaterialMarker` | Cell data | Material region tag per element |

---

## Examples

All example input files are located in the `Examples/` directory.

### Example 1 – Basic domain with a hole

The domain is a quadrilateral with a rectangular hole.

- Points 0–4 define the outer boundary (counter-clockwise).
- Points 5–8 define the hole (clockwise).

```
EasyMesh example1 +tec +vtk
```

### Example 2 – Circular hole and local refinement

The domain contains a circular hole (12 boundary points) and a rectangular
"false hole" used to locally refine the mesh.

- Points 0–5: outer boundary.
- Points 6–17: circular hole (boundary marker 2, clockwise).
- Points 18–21: false hole for refinement (boundary marker 0, counter-clockwise).

Key observations:
- The desired spacing for the circular hole points is set to `10.0` (exaggerated),
  but EasyMesh clips the value to a sensible maximum automatically.
- The false hole uses marker `0`, which is allowed for interior refinement regions.

```
EasyMesh example2 +tec +vtk
```

### Example 3 – Multi-material domain

Demonstrates multi-material meshes and interior open chains for local coarsening.

- Points 0–11: outer boundary and material frontiers.
- Points 12–13: interior open chain used to coarsen the mesh locally.
- Points 14–16: isolated points acting as **material markers**. Their `marker`
  value is assigned to all elements in the region they fall inside. The `spacing`
  value must be `0` for material marker points.

Frontier rules:
- Frontiers between materials must be **open chains** that begin and end on
  existing boundary or frontier nodes.
- Interior open chains (for refinement/coarsening) must **not** start or end on
  boundary nodes.
- A single isolated node is a valid degenerate open chain (feature from v1.4).

```
EasyMesh example3 +tec +vtk
```

---

## Design Notes and Limitations

- **Memory allocation** is static. The default `TEMPORARY_MAX_NODES` is `10 000`.
  Increase the constant in `easymesh.h` and recompile to handle larger meshes.
- Only **one domain** can be meshed per run; multiple disconnected domains are not
  supported.
- Bandwidth renumbering is always applied and cannot be disabled.
- The spacing value for every node should be strictly **positive and non-zero** to
  avoid infinite loops during node insertion.
- The `marker` for boundary and hole nodes/segments must be **greater than zero**.
  Material marker points use `marker > 0` to identify the target material and
  `spacing = 0` to exclude them from triangulation.

---

## Authors

- **Bojan Niceno** — original author, bojan.niceno@psi.ch

EasyMesh has been compiled and tested on:

| Platform | OS | Compiler |
|---|---|---|
| PC | DOS 6.22 / Windows 95 | Watcom C/C++ 10.0a |
| PC | RedHat Linux 3.03 | gcc |
| PC | Caldera Linux | gcc |
| SUN Ultra 1 Model 140 | SunOS 5.5 | cc |
| Iris Indigo xs24 | IRIX 5.2 | cc |
| Cray J916/8-1024 | UNICOS 8.0.3 | cc |
