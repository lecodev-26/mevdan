# MEVDAN — Editor Specification

`mevdan-editor` is a **programmatic** text editor. It does not
provide a UI; it provides the engine that a UI (or an agent) can
drive.

## Why

`mevdan-tools` already writes files. But it has no concept of:

- A "buffer" (a file open in memory with a version).
- An "edit" (replace range, insert, delete).
- A "version" (increments on each change).
- Unsaved changes (`dirty`).

`mevdan-editor` adds these. It becomes the base for inline editing,
multi-file refactors, and agent-driven patches.

## Concepts

### BufferVersion

Numeric version that increments on every edit.

```rust
let v = BufferVersion::initial();     // 0
let v = v.next();                     // 1
Displays as v0, v1, ...

TextRange
Half-open byte range: [start, end).

rust
let r = TextRange::new(0, 5)?;
let r = TextRange::empty(3);          // start == end
r.len();                               // end - start
r.is_empty();                          // start == end
Ranges must be on char boundaries. Non-boundary ranges are rejected.

Buffer
A file in memory:

rust
pub struct Buffer {
    pub path: PathBuf,
    pub version: BufferVersion,
    pub dirty: bool,
    // private: text, original
}
Operations:

Buffer::from_text(path, text) — create from a string.

Buffer::open(path) — read from disk.

text() / original() — content.

insert(at, text) — insert.

delete(range) — delete.

replace(range, new_text) — replace.

save() — write to disk, clear dirty.

revert() — reset to original, bump version.

Rules:

Range must be within bounds.

Range must respect UTF-8 char boundaries.

Any mutation increments version and sets dirty = true.

TextEdit
An operation:

rust
pub struct TextEdit {
    pub range: TextRange,
    pub new_text: String,
}
Constructors:

TextEdit::insert(at, text).

TextEdit::delete(range).

TextEdit::replace(range, text).

Helpers:

deleted_len() / inserted_len().

EditResult
rust
pub struct EditResult {
    pub edits_applied: usize,
    pub bytes_removed: usize,
    pub bytes_inserted: usize,
    pub final_version: BufferVersion,
}
net_bytes() — inserted - removed.
summary() — textual.

apply_edits
Applies a list of edits to a buffer.

rust
let edits = vec![
    TextEdit::insert(0, "X"),
    TextEdit::insert(5, "Y"),
];
let result = apply_edits(&mut buffer, edits)?;
Rules:

Edits are applied in descending order of range start so
positions remain valid.

Overlapping edits are rejected.

EditResult accumulates counts across all edits.

EditorEngine
Manages multiple buffers.

rust
let mut engine = EditorEngine::new();
engine.open("file.rs")?;
engine.open_from_text("/synthetic/x.rs", "content")?;

engine.apply_edits("file.rs", vec![...])?;
engine.save("file.rs")?;
engine.close("file.rs", false)?;
Operations:

open(path) — open a file.

open_from_text(path, text) — synthetic buffer.

close(path, force) — close; fails if dirty unless force.

get(path) / get_mut(path) / require(path) — access.

apply_edits(path, edits) — apply edits to an open buffer.

save(path) / save_all() — write to disk.

revert(path) — reset to original.

paths() / dirty_paths() — enumerate.

close_all(force) — close every buffer.

versions() — map path → version.

What this is NOT
Not a graphical editor. No UI, no syntax highlighting.

No LSP integration. LSP arrives in V5.9.

No undo/redo stack. revert resets to the original.

No diff/patch format. Edits are byte-range based.

No multi-cursor. Single-cursor edits.

Design notes
Byte offsets. Ranges are bytes, not chars. Users must slice
at char boundaries.

UTF-8 safe. Non-boundary ranges are rejected with a clear
error.

Deterministic. Same edits, same result.

Composable. Applying N edits is one call.

Serializable. Buffers, edits and results roundtrip through JSON.
