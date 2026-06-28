# Example 06 — Rectangle with Two Rectangular Holes

A 10×6 outer rectangle (CCW, tag=1) containing two non-overlapping rectangular
holes: a small 2×2 hole on the left (CW, tag=2) and a larger 3×3 hole on the
right (CW, tag=3).

## Geometry

```
(0,6) ──────────────────────────── (10,6)
  |                                       |
  |  (1,3)──(3,3)    (6,5)──(9,5)        |
  |    |  hole2 |    |   hole3  |         |
  |  (1,1)──(3,1)    (6,2)──(9,2)        |
  |                                       |
(0,0) ──────────────────────────── (10,0)
```

- **Outer**: nodes 1–4, CCW, tag = 1
- **Left hole**: nodes 5–8, CW, tag = 2  (2×2, centred at (2, 2))
- **Right hole**: nodes 9–12, CW, tag = 3  (3×3, centred at (7.5, 3.5))
- Grid: 0.25×0.25 → 855 nodes, 752 elements

## Key features tested

- Two independent CW holes inside one CCW boundary
- Distinct region tags per hole allow downstream solvers to apply different
  boundary conditions on each hole perimeter
- Holes are non-symmetric (different sizes and positions) — stress test for
  the `assign_holes` pairing logic

## Run

```bash
cargo build --release
cd examples/06_rectangle_two_holes
../../target/release/structured_mesh --project Polygon_project.txt
```
