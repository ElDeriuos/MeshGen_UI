# Example 01 — Unit Square

The simplest possible benchmark: a 1×1 square with a uniform 0.1×0.1 grid.

## Geometry

```
(0,1) ──── (1,1)
  |              |
  |    region 1  |
  |              |
(0,0) ──── (1,0)
```

- 4 nodes, 4 edges, single region tag = 1
- Expected grid: ~10×10 cells → ~100 interior elements

## Why this is a useful benchmark

The unit square is the canonical domain for verifying:
- Grid origin offset logic (`xm0`, `ym0` magic constants)
- That all interior cells are tagged `1`
- That boundary cells carry the correct tag `2` (from `kp=1` → rasterized as `kn=2`)
- Element count matches the analytic value: `(1/dx) × (1/dy)` interior cells

## Run

```bash
cargo build --release
cd examples/01_unit_square
../../target/release/structured_mesh --project Polygon_project.txt
```

## Expected output summary

```
mesh_generator: grid 11×12, 121 nodes, 100 elements
```
