# Example 08 — Swiss Cheese (Five Holes)

A 10×8 outer rectangle (CCW, tag=1) with **five independent rectangular holes**
at varied positions and sizes (CW, tags 2–6, plus a bottom-centre hole tag=7).
This is the maximum-complexity hole test for this format.

## Geometry

```
(0,8) ────────────────────────────── (10,8)
  |  (1.5,6.5)    (6.5,7)──(8,7)          |
  |  (1.5,5)──(2.5,5)  |  hole5  |        |
  |  | hole4 |    (6.5,5)──(8,5)          |
  |  (1.5,5)──(2.5,5)                      |
  |  (4.5,4.5)──(5.5,4.5)                  |
  |  |   hole2   |  (7,2.5)──(8.5,2.5)    |
  |  (4.5,3.5)──(5.5,3.5)  | hole3 |      |
  |  (1.5,2.5)──(2.5,2.5)  (7,1)──(8.5,1) |
  |  |  hole1  |  (3.5,2)──(6.5,2)        |
  |  (1.5,1.5)──(2.5,1.5)  | hole6 |      |
  |                (3.5,0)──(6.5,0)        |
(0,0) ────────────────────────────── (10,0)
```

- **Outer** (tag=1): 10×8 rectangle, CCW
- **Hole 1** (tag=2): 1×1 at bottom-left, CW
- **Hole 2** (tag=3): 1×1 centred, CW
- **Hole 3** (tag=4): 1.5×1.5 at bottom-right, CW
- **Hole 4** (tag=5): 1×1.5 at mid-left, CW
- **Hole 5** (tag=6): 1.5×2 at top-right, CW
- **Hole 6** (tag=7): 3×2 at bottom-centre, CW
- Grid: 0.2×0.2 → 1796 nodes, 1636 elements

## Key features tested

- **Maximum hole count** (5 CW holes) in a single domain
- All holes have distinct region tags — full tag fidelity check
- Non-uniform hole sizes and positions stress-test `assign_holes` pairings
- High element count (~1600) tests output formatter performance

## Run

```bash
cargo build --release
cd examples/08_swiss_cheese
../../target/release/structured_mesh --project Polygon_project.txt
```
