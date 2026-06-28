# Example 07 — C-Shaped Channel

A C-shaped (or U-channel) domain — a 6×5 rectangle with a 4.5×2 rectangular
notch cut from the right side, forming a channel open on the right.

## Geometry

```
(0,5) ──────────── (6,5)
  |                      |
  |              (6,3.5)─┘
  |              (inside notch — exterior)
  |              (6,1.5)─┐
  |                      |
(0,0) ──────────── (6,0)
```

The 8-node single CCW polygon visits the outer perimeter and the two inner
step corners of the notch:

```
(0,0)→(6,0)→(6,1.5)→(1.5,1.5)→(1.5,3.5)→(6,3.5)→(6,5)→(0,5)→(0,0)
```

- 8 nodes, 8 edges, single region tag = 1
- Two re-entrant corners at (6,1.5) and (6,3.5) — interior angle = 270°
- Grid: 0.25×0.25 → 399 nodes, 336 elements

## Key features tested

- **Two simultaneous re-entrant corners** on the same edge of the domain
- The notch region (x∈[1.5,6], y∈[1.5,3.5]) must be fully exterior (`matr=0`)
- Classic domain for lid-driven cavity variants and channel flow with obstacles
- Tests that the scanline fill correctly handles a concave polygon with two
  interior angles > 180°

## Reference

Moffatt, H.K. (1964). Viscous and resistive eddies near a sharp corner.
*Journal of Fluid Mechanics*, 18(1), 1–18.

## Run

```bash
cargo build --release
cd examples/07_c_channel
../../target/release/structured_mesh --project Polygon_project.txt
```
