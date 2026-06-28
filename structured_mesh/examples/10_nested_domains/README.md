# Example 10 — Three-Level Nested Domains

The most topologically complex example: three concentric square boundaries
creating two separate material regions separated by a void ring.

```
Level 0 (outer CCW, tag=1):  12×10 rectangle
Level 1 (hole  CW,  tag=2):   8×6  rectangle  → creates void ring
Level 2 (inner CCW, tag=3):   4×3  rectangle  → island inside the void
```

## Geometry

```
(0,10) ───────────────────────── (12,10)
  |   (2,8)───────────(10,8)           |
  |   |  void ring (exterior)  |       |
  |   |   (4,7)───(8,7)        |       |
  |   |   | island (tag=3) |   |       |
  |   |   (4,4)───(8,4)        |       |
  |   (2,2)───────────(10,2)   |       |
  |                             |       |
(0,0) ───────────────────────── (12,0)
```

- **Outer** (tag=1): 12×10 CCW — main domain
- **Void ring** (tag=2): 8×6 CW — creates the annular void region
- **Island** (tag=3): 4×3 CCW — floating solid inside the void
- Grid: 0.25×0.25 → 1517 nodes, 1344 elements

## Key features tested

- **Three-level topology**: CCW outer → CW hole → CCW island inside hole
- The island (tag=3) must be rasterised as interior (`matr≥1`) even though it
  sits inside a CW boundary — requires correct nested `assign_holes` logic
- Two distinct active regions that are geometrically disconnected in the mesh
- Models composite material problems: e.g. concrete with a steel rebar island
  surrounded by an air gap

## Reference

Dörsek, P., Melenk, J.M. (2011). Hp-FEM for the fractional diffusion equation.
*Numerical Mathematics*, 107(4). (Uses nested annular test domain.)

## Run

```bash
cargo build --release
cd examples/10_nested_domains
../../target/release/structured_mesh --project Polygon_project.txt
```
