# Example 03 — Square Domain with a Square Hole

A 4×4 outer square (CCW, tag=1) with a centred 1×1 square hole (CW, tag=2).
This is the standard benchmark for **multiply-connected domains** and hole
detection via winding-order classification.

## Geometry

```
(0,4) ──────────────── (4,4)
  |                          |
  |   (1.5,2.5)─(2.5,2.5)   |
  |       |   hole   |       |
  |   (1.5,1.5)─(2.5,1.5)   |
  |                          |
(0,0) ──────────────── (4,0)
```

- **Outer boundary**: nodes 1–4, CCW (positive signed area), tag = 1
- **Inner hole**:     nodes 5–8, CW  (negative signed area), tag = 2
- Grid: 0.2×0.2 cells → ~380 active elements (annular region)

## Winding orders

| Boundary | Nodes | Direction | Signed area |
|---|---|---|---|
| Outer square | 1→2→3→4→1 | CCW | +16.0 |
| Inner hole   | 5→8→7→6→5 | CW  | −1.0  |

The `assign_holes` function pairs the CW hole with its CCW enclosing outer
boundary. Cells inside the hole region must be tagged `0` (exterior) after
rasterisation.

## Why this is a useful benchmark

- Verifies **hole rasterisation**: cells inside `(1.5,2.5)×(1.5,2.5)` must be `0`
- Verifies **boundary tag propagation**: hole perimeter cells carry tag `2`
- Standard domain for Laplace/Poisson annular problems and heat conduction tests
- Used in CGAL, Triangle, and TetGen test suites as the "square annulus" case

## Reference

Shewchuk, J.R. (1996). *Triangle: Engineering a 2D Quality Mesh Generator*.
Applied Computational Geometry: Towards Geometric Engineering, LNCS 1148.

## Run

```bash
cargo build --release
cd examples/03_square_with_hole
../../target/release/structured_mesh --project Polygon_project.txt
```
