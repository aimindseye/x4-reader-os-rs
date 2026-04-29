Phase 8 — Extraction Prep for VaachakOS Reader Slice
====================================================

Purpose
-------
This pack keeps the working X4 runtime path intact and prepares the first reader
slice for later extraction into VaachakOS.

It adds a clear, portable model boundary around:

- reader state model
- book identity model
- per-book cache/state layout
- bookmark record model
- theme preset/theme record model

Included replacement files
--------------------------

- src/apps/reader_state.rs
- src/apps/reader/mod.rs
- src/apps/home.rs
- docs/vaachak-reader-slice.md

Apply
-----

From repo root:

    unzip -o /path/to/x4-os-phase8-vaachak-reader-slice-prep.zip

Verify markers:

    rg -n "phase8|BookIdentity|BookStateLayout|ReaderSliceDescriptor|READER_SLICE_SCHEMA" \
      src/apps/reader_state.rs src/apps/reader/mod.rs src/apps/home.rs docs/vaachak-reader-slice.md

Build/flash:

    cargo fmt --all
    cargo build --release
    cargo run --release

Expected runtime markers
------------------------

On opening a book, logs should include:

    phase8: extraction-ready reader slice vaachak-reader-slice-v1 book_id=... format=... meta=state/... progress=state/... theme=state/... bookmarks=state/... index=state/BMIDX.TXT cache=cache/...

Home Continue logs should include:

    phase8: home loaded typed recent book_id=... path=...
    phase8: continue from typed recent book_id=... path=...

Acceptance
----------

- Bookmarks remain unchanged: state/<BOOKID>.BKM and state/BMIDX.TXT.
- Typed meta/progress/theme remain flat: state/<BOOKID>.MTA/.PRG/.THM.
- Existing EPUB cache behavior remains untouched.
- Continue still opens from typed RecentBookRecord, not raw recent text.
- Reader state models now have extraction-facing constructors and descriptors.

Notes
-----

This phase does not move code into a VaachakOS repo yet. It creates the stable
boundary and manifest so a later extraction can copy the model/layout layer
without dragging the X4 app shell, input, display, or KernelHandle APIs with it.
