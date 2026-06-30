# Root Makefile
# Builds EasyMesh and mesh_gui, then moves binaries to this directory.

# Detect Operating System
ifeq ($(OS),Windows_NT)
    RM      = del /f /q
    RMDIR   = rmdir /s /q
    MV      = move /y
    EXE     = .exe
else
    RM      = rm -f
    RMDIR   = rm -rf
    MV      = mv -f
    EXE     =
endif

EASY_DIR    = EasyMesh/Src
CARGO_DIR   = mesh_gui

EASY_BIN    = Easy$(EXE)
CARGO_BIN   = meshgen_ui$(EXE)

.PHONY: all easymesh mesh_gui clean

all: easymesh mesh_gui

# ── EasyMesh ──────────────────────────────────────────────────────────────────
easymesh:
	$(MAKE) -C $(EASY_DIR)
	$(MV) $(EASY_DIR)/$(EASY_BIN) $(EASY_BIN)

# ── mesh_gui (Rust / Cargo) ───────────────────────────────────────────────────
mesh_gui:
	cargo build --release --manifest-path $(CARGO_DIR)/Cargo.toml
	$(MV) $(CARGO_DIR)/target/release/$(CARGO_BIN) $(CARGO_BIN)

# ── Clean ─────────────────────────────────────────────────────────────────────
# Removes all build artifacts; keeps the binaries in this directory.
clean:
	$(MAKE) -C $(EASY_DIR) clean
	$(RMDIR) $(CARGO_DIR)/target
