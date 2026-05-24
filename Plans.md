# Idep Plans.md

Created: 2026-05-24

---

## Phase 0.1.x: v0.1.5 — File Tree + Multi-Buffer

### Overview

v0.1.5 adds two complementary features to the TUI: a navigable file tree panel and support for multiple open buffers. Together, these enable users to explore project structure and switch between files without closing and reopening. The file tree and buffer management are independent features that can be tested in isolation but compose into a cohesive project navigation UX.

**Gate**: Navigate project files in tree panel; open multiple buffers; switch between them with configurable keys. All keybindings respect TUI conventions (Space+letter for toggles, ]b/[b for navigation).

---

## Phase 1: Multi-Buffer Foundation

### Task 1.1: Original task decomposed into 1.1a + 1.1b + 1.1c (see below)
**Status**: cc:完了 [decomposed-for-delivery]
---

### Task 1.1a: Structural Refactor - BufferState & Helpers
**DoD**: BufferState struct consolidates per-buffer state; App holds Vec<BufferState> + active_buffer_idx; helper methods work; TDD tests pass
**Depends**: -
**Status**: cc:TODO
**Details**:
- Define BufferState struct: consolidates buffer, filename, scroll_offset, modified, highlighter
- Refactor App struct: replace single fields with Vec<BufferState> and active_buffer_idx
- Implement helper methods: active_buffer(), active_buffer_mut(), switch_buffer(idx), add_buffer(path), remove_buffer(idx), get_filename()
- Update constructors: new() and from_file() initialize multi-buffer fields
- Add TDD unit tests: test_multi_buffer_add_switch, test_multi_buffer_remove, test_multi_buffer_isolation, test_active_buffer_methods

[tdd:required]

---

### Task 1.1b: Event Handler Port
**DoD**: All event handling methods route through active_buffer()/active_buffer_mut(); handle_char, handle_backspace, undo, redo, command execution work with multi-buffer
**Depends**: 1.1a
**Details**:
- Port handle_char, handle_backspace, handle_delete to use active_buffer_mut()
- Port undo/redo to active buffer
- Port all command mode handlers to route through active buffer
- Ensure modified flag updates track to modified_flags[active_buffer_idx]
- All existing tests continue to pass

[tdd:skip:event-handling-port]

---

### Task 1.1c: Rendering & Display Port
**DoD**: Rendering code accesses active buffer; status bar shows buffer count [N/M]; scroll and modified state tracked per-buffer correctly
**Depends**: 1.1b
**Details**:
- Port render_buffer to use active_buffer()
- Port render_status_bar to show [N/M] buffer indicator
- Port scroll_offset access to scroll_offsets[active_buffer_idx]
- Port modified flag display to modified_flags[active_buffer_idx]
- Port highlighter access to work with per-buffer cache
- All rendering tests pass

[tdd:skip:rendering-port]

---

### Task 1.2: Unit Tests for Multi-Buffer Operations
**DoD**: Open 3 buffers, close middle one, verify remaining order correct; all buffer operations isolated per active buffer

**Depends**: 1.1a

**Details**:
- Test: create app, add buffer A, add buffer B, add buffer C → all three stored in order
- Test: close buffer B → buffers A and C remain, active index adjusted
- Test: edit buffer A → buffer B and C unaffected
- Test: switch to buffer B, insert text → doesn't appear in buffer A
- Test: undo in buffer A doesn't affect buffer B history
- Test: syntax highlighter caches per buffer (buffer A language ≠ buffer B)

[tdd:required]

---

### Task 1.3: Render Multi-Buffer Status Bar
**DoD**: Status bar shows `[N/M]` format (current buffer index / total) when multiple buffers open

**Depends**: 1.1

**Details**:
- Update status bar render logic to show buffer count
- Format: left side shows `[2/4]` when on buffer 2 of 4
- Single buffer open: show `[1/1]` (consistent behavior)
- Test: verify format with 1, 2, 5, 10 buffers open

[tdd:required]

---

## Phase 2: Buffer Navigation

### Task 2.1: Buffer Switcher Command Mode
**DoD**: `:b [N]` command opens buffer N (1-indexed); `:bn` and `:bp` work for next/previous; test with 3 buffers

**Depends**: 1.1, 1.2, 1.3

**Details**:
- Extend command mode parser to recognize `:b N`, `:bn`, `:bp`, `:bN` (alias for `:b N`)
- `:b 1` → switch to buffer 1
- `:bn` → switch to next buffer (wrap to first if at end)
- `:bp` → switch to previous buffer (wrap to last if at beginning)
- Render active buffer file path in status bar when switching
- Test: switch through all buffers, verify correct content displayed

[tdd:required]

---

### Task 2.2: Keybinding-Based Buffer Navigation
**DoD**: `]b` and `[b` keys switch next/previous buffer in Normal mode; configurable via env vars (IDEP_NEXT_BUFFER_KEY, IDEP_PREV_BUFFER_KEY)

**Depends**: 2.1

**Details**:
- Add mode to key handling for `]b` (next) and `[b` (previous) in Normal mode
- Reuse existing sequence logic (pending_space flag analog, or new state for bracket pairs)
- Env var fallback to default `]b` and `[b` if not specified
- Test: `]b` three times in 4-buffer app → cycles through all, wraps to first
- Test: `[b` goes backward

[tdd:required]

---

### Task 2.3: File Dialog for New Buffer
**DoD**: `:e path/to/file` command opens file in new buffer or switches if already open; `:enew` opens empty buffer

**Depends**: 1.1

**Details**:
- Extend command mode to parse `:e path` and `:enew`
- `:e path/to/file` → if buffer for path already open, switch to it; else load file and create new buffer
- `:enew` → create empty buffer with no file path
- Handle relative paths (relative to workspace root or current file directory)
- Test: `:e foo.rs`, `:e bar.rs`, `:e foo.rs` again → switch to first, not duplicate

[tdd:required]

---

## Phase 3: Buffer Lifecycle

### Task 3.1: Close Buffer with Unsaved Check
**DoD**: `:q` on modified buffer prompts "unsaved changes, close anyway?"; `:q!` closes without prompt

**Depends**: 1.1, 1.2

**Details**:
- When closing a buffer with `modified=true`, show status message prompt
- `:q` with unsaved → set status "Unsaved changes in buffer X, type :q! to close"
- `:q!` → close regardless
- If closing last buffer, exit editor (not leave empty editor state)
- Test: open file, edit, `:q` → prompt shown; `:q!` → closed

[tdd:required]

---

### Task 3.2: Configurable Buffer Close Key
**DoD**: `Space+q` closes active buffer; configurable via IDEP_CLOSE_BUFFER_KEY env var

**Depends**: 3.1

**Details**:
- Add key handling for Space+q (default) to close buffer
- If only buffer open and modified, prompt as in 3.1
- If only buffer open and unmodified, close it (exit app)
- Env var override: `IDEP_CLOSE_BUFFER_KEY=w` → Space+w closes instead
- Test: close middle buffer in 3-buffer app → correct buffer removed, active index adjusted

[tdd:required]

---

### Task 3.3: Buffer Save All with `:wa`
**DoD**: `:wa` saves all modified buffers; `:waq` saves all and quits; test with 3 modified buffers

**Depends**: 2.3, 3.1

**Details**:
- Extend command mode to recognize `:wa` and `:waq`
- `:wa` → iterate through all buffers, write modified ones to disk
- `:waq` → same, then quit editor
- Show status "Saved N buffers"
- Test: modify buffers A, B, C; `:wa` → all three written, status shown

[tdd:required]

---

## Phase 4: File Tree Panel

### Task 4.1: File Tree Data Structure
**DoD**: Tree node type holds path, name, children; recursive walk of project directory produces correct tree structure; unit test on test fixtures

**Depends**: -

**Details**:
- Define `FileTreeNode` struct: `{ path: PathBuf, name: String, children: Vec<FileTreeNode>, is_dir: bool }`
- Implement `FileTree::from_root(root_path)` → walks directory respecting `.gitignore`
- `.gitignore` parsing via `ignore` crate (already used in indexer)
- Sort children: directories first (alphabetically), then files (alphabetically)
- Handle symlinks: follow them (no loop detection; user's responsibility)
- Test: create fixture dir with nested structure, verify tree matches expected layout
- Test: `.gitignore` excludes specified paths

[tdd:required]

---

### Task 4.2: File Tree Rendering
**DoD**: Tree renders in side panel with folder/file icons, indentation, current file highlighted; no flicker on cursor movement in editor

**Depends**: 4.1, 1.1

**Details**:
- Allocate fixed-width panel (default 30 columns, configurable)
- Render tree with indentation (2 spaces per level)
- Folder icon: `[+]` (collapsed) or `[-]` (expanded) (or ► / ▼ if terminal supports)
- File icon: space or dot
- Current active file path highlighted with reverse video or bold
- Render only visible portion of tree (virtual scrolling for deep trees)
- Test: render 50-node tree, verify layout correct, current file highlighted

[tdd:required]

---

### Task 4.3: File Tree Navigation
**DoD**: `j/k` move cursor in tree; `Enter` opens file; `l` expands, `h` collapses; wraps at edges; test with 20-node tree

**Depends**: 4.2

**Details**:
- When tree has focus:
  - `j` → next visible node (wrap to first)
  - `k` → previous visible node (wrap to last)
  - `l` → expand folder (no-op on file)
  - `h` → collapse folder (no-op on file, goes to parent if at root of collapsed tree)
  - `Enter` → open file (call `add_buffer(path)` and switch to it)
- Tree focus state: `tree_focus: bool` on App
- Test: navigate 20-node tree, open file, verify buffer created and active
- Test: expand/collapse changes visible node count correctly

[tdd:required]

---

### Task 4.4: Tree Toggle and Focus Management
**DoD**: `Space+e` toggles tree panel visibility; focus switches between editor and tree with configurable keys; only one has input focus

**Depends**: 4.3, 2.2

**Details**:
- Add `tree_visible: bool` to App
- `Space+e` toggles tree on/off (default key, configurable via IDEP_TREE_TOGGLE_KEY)
- When tree visible and tree has focus:
  - Editor keys (j/k/h/l) route to tree navigation, not editor movement
  - `Tab` or configurable key switches focus to editor
  - `Esc` in tree → focus goes to editor
- When focus on editor:
  - `Space+e` → toggle tree
  - Editor movement (j/k/h/l) works normally
  - `Tab` in editor → focus goes to tree (if visible)
- Status bar shows focus indicator (e.g., `[TREE]` or `[ED]`)

[tdd:required]

---

### Task 4.5: File Tree Integration Tests
**DoD**: Create new test fixtures; open file from tree, verify buffer created; navigate tree with 5+ files; all tests pass

**Depends**: 4.4

**Details**:
- Create `tests/tree_v0_1_5.rs` integration test
- Fixture: small Rust project (3-5 .rs files, nested dirs)
- Test: tree renders correctly on startup
- Test: navigate to file, press Enter, buffer opens and is active
- Test: toggle tree visibility on/off
- Test: focus switches between editor and tree
- Test: expand/collapse directories
- Test: ignore patterns respected in tree

[tdd:required]

---

## Phase 5: LSP and Completion Integration

### Task 5.1: Per-Buffer LSP Document Sync
**DoD**: LSP client tracks document state per buffer; opening/closing buffers sends correct didOpen/didClose; tests pass for 2 buffers

**Depends**: 1.1, 4.4

**Details**:
- Refactor DocumentManager to hold `HashMap<PathBuf, DocumentState>` instead of single file
- `add_buffer(path)` → send `didOpen` for that path
- `remove_buffer(idx)` → send `didClose` for that path
- `switch_buffer(idx)` → no document sync needed (buffer already open; diagnostics already cached)
- Diagnostics storage: per-path in App (hash of path → Vec<Diagnostic>)
- Test: open buffer A, make change → didChange sent; open buffer B, make change → didChange sent for B
- Test: close buffer A → didClose sent; buffer B diagnostics preserved

[tdd:required]

---

### Task 5.2: Per-Buffer Completion and Chat Context
**DoD**: Completion and chat context injection use active buffer's file path and content; test with two .rs files

**Depends**: 5.1

**Details**:
- CompletionEngine trigger uses `active_buffer()` content and active file path
- ContextEngine gathers chunks from active file (not all open files)
- Chat context injection still includes active file + cursor AST chunk
- Test: have buffer A and buffer B open, get completion in B → prompt uses B's path and language
- Test: ask question in chat when B is active → context injection shows B's filename

[tdd:skip:lsp-integration-gated]

---

## Phase 6: Polish and Testing

### Task 6.1: Keybinding Validation
**DoD**: All new keybindings (Space+e, ]b, [b, Space+q) have defaults; env var overrides work; conflicts detected and logged

**Depends**: 2.2, 4.4

**Details**:
- Collect all keybinding env vars: `IDEP_TREE_TOGGLE_KEY`, `IDEP_NEXT_BUFFER_KEY`, `IDEP_PREV_BUFFER_KEY`, `IDEP_CLOSE_BUFFER_KEY`
- Validate on startup: check for conflicts (same key bound to two actions in same mode)
- Log conflicts with both action names
- Test: set all keys to same char → conflict detected and logged
- Test: custom env vars work: `IDEP_TREE_TOGGLE_KEY=f` → Space+f toggles tree

[tdd:skip:config-validation]

---

### Task 6.2: Edge Cases and Robustness
**DoD**: Closing last buffer exits cleanly; opening nonexistent file shows error; very deep tree (100+ nodes) renders without lag

**Depends**: 3.1, 4.2

**Details**:
- Test: with 1 buffer open, `:q!` → editor exits (not left in broken state)
- Test: `:e /nonexistent/path` → shows error "file not found" in status bar, no crash
- Test: tree with 200 nodes, scroll through → no perceptible lag (<100ms per keystroke)
- Test: directory with 1000 files → tree builds in <100ms, renders visible portion only
- Test: symlink to directory in tree → followed without infinite loop (tree depth limit?)

[tdd:required]

---

### Task 6.3: End-to-End Integration Test
**DoD**: Open editor, toggle tree, open 3 files via tree, switch between buffers, edit, save all; no crashes

**Depends**: 6.1, 6.2

**Details**:
- Create comprehensive integration test in `tests/v0_1_5_e2e.rs`
- Fixture: small multi-file Rust project
- Flow:
  1. Open editor with one file
  2. `Space+e` toggle tree on
  3. Navigate to second file, press Enter → buffer created
  4. Navigate to third file, press Enter
  5. `]b` to cycle through all three buffers
  6. In buffer 2, make an edit
  7. `Space+q` to close buffer 2 (it's in focus, unmodified other buffers)
  8. `:wa` to save all remaining
  9. Verify all changes persisted to disk
- Test: complete flow runs without panic, crashes, or hangs

[tdd:required]

---

## Dependencies and Critical Path

```
1.1 (Refactor to multi-buffer)
├─→ 1.2 (Unit tests)
├─→ 1.3 (Status bar render)
├─→ 2.1 (Buffer command mode) ← 1.1, 1.2, 1.3
│   └─→ 2.2 (Keybinding nav) ← 2.1
│       └─→ 2.3 (`:e` command) ← 2.2
├─→ 3.1 (Close with check) ← 1.2
│   └─→ 3.2 (Close keybinding) ← 3.1
│       └─→ 3.3 (Save all) ← 2.3, 3.1

4.1 (Tree data structure)
├─→ 4.2 (Tree rendering) ← 1.1
│   └─→ 4.3 (Tree nav) ← 4.2
│       └─→ 4.4 (Tree toggle) ← 4.3, 2.2
│           └─→ 4.5 (Integration tests) ← 4.4

5.1 (Per-buffer LSP) ← 1.1, 4.4
└─→ 5.2 (Per-buffer completion/chat) ← 5.1

6.1 (Keybinding validation) ← 2.2, 4.4
├─→ 6.2 (Edge cases) ← 3.1, 4.2
└─→ 6.3 (E2E test) ← 6.1, 6.2
```

**Critical path (minimum to pass gate)**: 1.1 → 1.2 → 2.1 → 3.1 → 4.1 → 4.2 → 4.3 → 4.4 → 6.3

**Recommended path**: Execute Phase 1 fully, then Phase 2, then Phase 3 (multi-buffer lifecycle complete). In parallel, execute Phase 4 independently. Merge Phase 5 after Phases 2–4 are stable.

---

## Notes

- File tree rebuild: on startup and on directory change (via `notify` watcher). Debounce file system events (500ms window).
- Performance concern: tree with 10k+ files may be slow to render. Consider virtual scrolling or lazy expansion.
- Keybindings: all must be composable (Space+letter or bracket pairs) to avoid conflicts with vi-like navigation.
- Buffer limit: no hard limit, but consider warning if >100 buffers open (UX anti-pattern).
- All new UI panels must respect existing cursor-in-editor invariants (syntax highlight, completion, LSP diagnostics follow active buffer).
