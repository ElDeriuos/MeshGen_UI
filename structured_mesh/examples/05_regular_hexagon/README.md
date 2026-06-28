# Example 05 — Regular Hexagon

A unit circumradius regular hexagon (R=1). This is the standard benchmark for
verifying **mesh isotropy** and **area conservation** on non-axis-aligned
boundaries.

## Geometry

```
        (-0.5, 0.866) ── (0.5, 0.866)
       /                              \
(-1, 0)          region 1          (1, 0)
       \                              /
        (-0.5,-0.866) ── (0.5,-0.866)
```

Vertices at angles 0°, 60°, 120°, 180°, 240°, 300° on the unit circle,
traversed CCW.

- 6 nodes, 6 edges, single region tag = 1
- Circumradius R = 1.0, inradius r = √3/2 ≈ 0.866
- Analytic area = (3√3/2) R² ≈ 2.5981
- Grid: 0.1×0.1 cells → ~20×18 bounding box → ~200 active interior elements

## Verification quantities

| Quantity | Analytic value |
|---|---|
| Area | 3√3/2 ≈ 2.5981 |
| Bounding box | [−1, 1] × [−0.866, 0.866] |
| Active cell fraction | π/(2√3) ≈ 0.9069 of bounding box |

The rasterised element count multiplied by `dx·dy` should converge to the
analytic area as the grid is refined.

## Why this is a useful benchmark

- **Non-axis-aligned edges**: 4 of the 6 edges are diagonal — tests the
  `segment_x_intersect` and `segment_y_intersect` routines on oblique segments
- **Symmetry check**: the mesh should be symmetric about both axes and the
  three diagonal axes of the hexagon
- **Area convergence**: element_count × dx × dy → 2.5981 as dx, dy → 0
- Standard domain for testing isotropic diffusion operators in FEM/FVM codes
  (e.g., the honeycomb lattice test in deal.II and FEniCS tutorials)

## Reference

Logg, A., Mardal, K.-A., Wells, G. (2012). *Automated Solution of Differential
Equations by the Finite Element Method (FEniCS Book)*. Springer.
Chapter 2 demo: Poisson equation on a hexagonal domain.

## Run

```bash
cargo build --release
cd examples/05_regular_hexagon
../../target/release/structured_mesh --project Polygon_project.txt
```
