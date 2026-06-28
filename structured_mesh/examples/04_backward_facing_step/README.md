# Example 04 — Backward-Facing Step

The backward-facing step (BFS) is one of the most widely used benchmark
geometries in computational fluid dynamics. It appears in hundreds of
validation studies for Navier-Stokes solvers, turbulence models, and mesh
generation algorithms.

## Geometry

```
(0,2) ────────────────────── (6,2)
  |                                |
(0,1) ── (3,1)                    |
               |    channel        |
           (3,0) ─────────── (6,0)
```

The domain is a single CCW polygon — an upstream channel (height=1, x∈[0,3])
that opens into a wider downstream channel (height=2, x∈[3,6]) at the step.

- 6 nodes, 6 edges, single region tag = 1
- Step height: 1.0 unit  (expansion ratio = 2:1)
- Step location: x = 3.0
- Grid: 0.25×0.25 cells → 24×8 bounding box → ~192 active elements

## Why this is a useful benchmark

- **Re-entrant corner** at (3,1): interior angle = 270°, identical character to
  the L-shaped domain but embedded in a flow channel context
- **Non-rectangular active region**: the upper-left quadrant (x∈[0,3], y∈[1,2])
  is exterior — tests that the rasteriser leaves those cells as `0`
- Reference case for Kim & Moin (1985) DNS, Armaly et al. (1983) experiments,
  and the NASA CFL3D validation suite
- Step expansion ratio of 2:1 (ER=2) is the most-cited configuration

## Reference

Armaly, B.F., Durst, F., Pereira, J.C.F., Schönung, B. (1983). Experimental
and theoretical investigation of backward-facing step flow.
*Journal of Fluid Mechanics*, 127, 473–496.

Kim, J., Moin, P. (1985). Application of a fractional-step method to
incompressible Navier-Stokes equations.
*Journal of Computational Physics*, 59(2), 308–323.

## Run

```bash
cargo build --release
cd examples/04_backward_facing_step
../../target/release/structured_mesh --project Polygon_project.txt
```
