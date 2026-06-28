# Example 09 — Two Separate Bodies

Two geometrically disconnected outer bodies in a single input file:

1. A 3×4 rectangle (CCW, tag=1) on the left
2. A convex pentagon with a chamfered top (CCW, tag=2) on the right,
   containing a small 1×1.5 rectangular hole (CW, tag=3)

## Geometry

```
(0,4)──(3,4)     (6.5,3.5)─(8,3.5)
  |        |    (5,2.5)         (9,2.5)
  |  body1 |    |   body2            |
  |        |    |  (6.5,1)─(7.5,1)  |
(0,0)──(3,0)    |  | hole3 |        |
                (5,0.5)──────────(9,0.5)
                   (6.5,2.5)─(7.5,2.5)
```

- **Body 1**: nodes 1–4, CCW rectangle, tag = 1
- **Body 2**: nodes 5–10, CCW pentagon (chamfered top), tag = 2
- **Hole in body 2**: nodes 11–14, CW rectangle, tag = 3
- Grid: 0.2×0.2 → 619 nodes, 536 elements

## Key features tested

- **Two disconnected CCW outer boundaries** in one file — `assign_holes` must
  correctly pair the CW hole with body 2, not body 1
- Body 2 is non-rectangular (6-node polygon with a diagonal top edge) — tests
  oblique scanline intersections
- Classic multi-body domain for structural analysis and electromagnetic problems

## Run

```bash
cargo build --release
cd examples/09_two_bodies
../../target/release/structured_mesh --project Polygon_project.txt
```
