# Example 02 — L-Shaped Domain

The L-shaped domain is one of the most cited benchmark geometries in finite
element analysis. It appears in the NIST Adaptive Mesh Refinement benchmarks
and in virtually every FEM textbook as the canonical re-entrant corner test.

## Geometry

```
(0,2) ── (1,2)
  |          |
  |          |
(0,1)   (1,1) ── (2,1)
  |                    |
  |      region 1      |
  |                    |
(0,0) ──────────── (2,0)
```

- 6 nodes, 6 edges, single region tag = 1
- The re-entrant corner at (1,1) is the key feature: interior angle = 270°
- Grid: 0.25×0.25 cells → 8×8 bounding box → ~48 active interior cells

## Why this is a useful benchmark

The L-domain is used to verify:
- Non-convex polygon rasterisation — the re-entrant corner must NOT be filled
- The upper-right quadrant `(1,2)×(1,2)` must be entirely exterior (`matr == 0`)
- Boundary tagging along the step edges (nodes 3–4–5)
- Classic test for Laplacian eigenvalue problems and stress singularities

## Reference

NIST "Benchmark Problems for Adaptive Mesh Refinement", Problem 1 (L-domain).
Grisvard, P. (1985). *Elliptic Problems in Nonsmooth Domains*. Pitman.

## Run

```bash
cargo build --release
cd examples/02_l_shaped_domain
../../target/release/structured_mesh --project Polygon_project.txt
```
