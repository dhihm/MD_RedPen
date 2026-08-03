# MD RedPen Terminal Design System

## 0. Research Log

- **Product reference:** the user defined the signature interaction as a physical
  yellow highlighter stroke over selected prose. The highlight is content
  semantics, not decoration.
- **Layer A:** the neutral operational guidance from `taste-skill.md` keeps the
  reading surface quiet, keyboard-first, and accessibility-led.
- **Layer B:** Notion's warm-paper minimalism informs the restrained chrome,
  warm neutrals, thin separators, and focus-first reading hierarchy. Brand
  assets and web typography are not copied.
- **Terminal reference:** Ratatui semantic spans and real xterm.js rendering are
  the implementation and QA surfaces. Browser-only layout, motion, and image
  research do not apply to a terminal application.

### Direction

MD RedPen should feel like reading a clean sheet of warm paper under a desk
lamp. The one memorable moment is a selected phrase turning into a persistent
yellow marker stroke that remains visibly linked to its endnote.

## 1. Atmosphere

- Quiet reading canvas with minimal chrome.
- Warm neutral text rather than cold blue-gray.
- One saturated semantic accent: marker yellow.
- Blue is reserved for transient keyboard focus and active link navigation.
- Red is reserved for errors that prevented a file mutation.
- The document remains the dominant surface at every terminal size.

## 2. Color Tokens

Colors are semantic Ratatui `Color::Rgb` values. No UI module may introduce a
new literal color without first adding a token here.

| Token | RGB | Purpose |
|---|---:|---|
| `CANVAS` | `24, 23, 21` | Warm near-black terminal background |
| `TEXT_PRIMARY` | `235, 232, 225` | Markdown body and primary labels |
| `TEXT_MUTED` | `166, 160, 151` | Metadata, inactive help, line numbers |
| `BORDER_QUIET` | `76, 72, 66` | Pane borders and separators |
| `MARKER_YELLOW` | `246, 211, 101` | Persisted RedPen annotation background |
| `MARKER_TEXT` | `34, 30, 20` | Text on persisted marker yellow |
| `SELECTION_BLUE` | `55, 110, 166` | Active, unsaved visual selection; 5.0:1 with `SELECTION_TEXT` |
| `SELECTION_TEXT` | `245, 248, 252` | Text on active selection |
| `FOCUS_BLUE` | `78, 161, 230` | Focused control and followed link |
| `SUCCESS_GREEN` | `91, 181, 124` | Successful atomic save status |
| `ERROR_RED` | `224, 98, 98` | Rejected selection or failed operation |
| `NOTE_SURFACE` | `40, 38, 34` | Endnote/review panel background |

### Color fallbacks

- With color disabled, persisted annotations use `REVERSED`.
- Active selection uses `REVERSED + BOLD`.
- Focus uses `UNDERLINED`.
- Errors use `BOLD` and the `Error:` prefix.

## 3. Typography and Text Hierarchy

The user's terminal owns the typeface. Hierarchy uses weight, color, and
spacing rather than font substitution.

| Role | Style |
|---|---|
| Document title | `BOLD`, `TEXT_PRIMARY` |
| Markdown heading | `BOLD`, one blank line before |
| Body | `TEXT_PRIMARY` |
| Inline code | `NOTE_SURFACE` background, `TEXT_PRIMARY` |
| Persisted annotation | `MARKER_YELLOW` background, `MARKER_TEXT` |
| Active selection | `SELECTION_BLUE` background, `SELECTION_TEXT` |
| Focused annotation | persisted style plus `UNDERLINED` |
| Numbered endnote title | `BOLD`, `TEXT_PRIMARY` foreground |
| Status/help | `TEXT_MUTED`; current mode is `BOLD` |
| Error | `ERROR_RED`, `BOLD` |

Korean, CJK, emoji, and combining sequences are measured as graphemes and
terminal-cell widths. No styling operation may split a grapheme.

## 4. Spacing and Layout

Terminal spacing is expressed in cells.

| Token | Cells | Usage |
|---|---:|---|
| `SPACE_INLINE` | 1 | Label/value separation |
| `SPACE_PANEL_X` | 2 | Horizontal panel padding |
| `SPACE_PANEL_Y` | 1 | Vertical panel padding |
| `STATUS_HEIGHT` | 2 | Mode/status and key help |
| `REVIEW_MIN_HEIGHT` | 5 | Manual/Codex review editor |

### Responsive terminal layout

- **Width >= 80, height >= 20:** document viewport plus two-line status bar.
  Input/review opens as a bordered bottom panel up to one third of the height.
- **Width 50-79 or height 12-19:** compact one-line status; review replaces the
  bottom half without a secondary help row.
- **Below 50x12:** render a typed terminal-size error and keep quit/help keys
  active. Never clip document bytes silently.
- Cursor movement automatically scrolls the document enough to keep the focused
  paragraph visible.
- The mouse wheel scrolls the viewport by three rendered rows without moving
  the editor cursor.
- Mouse hit testing replays the same Ratatui wrapping and scroll offset used by
  the visible paragraph. Source ranges never derive directly from raw screen
  columns.

## 5. Components and States

### Document viewport

- **Default:** warm canvas, Markdown hierarchy, visible cursor.
- **Visual select:** anchor-to-focus graphemes use active-selection blue.
- **Persisted annotation:** marker-yellow span remains visible after save.
- **Focused annotation:** marker remains yellow and gains underline.
- **Empty:** centered `Empty Markdown document` message with help.
- **Error:** original document remains visible; status explains rejection.

### Cursor

- One grapheme-wide reversed or underlined cell.
- Never occupies synthetic bullet, padding, delimiter, or wrap cells.
- Movement skips non-selectable cells.

### Status bar

- Left: mode (`BROWSE`, `VISUAL`, `INPUT`, `PROMPT`, `CODEX`, `REVIEW`).
- Center: transient success/error message.
- Right: concise keys valid in the current mode.
- No animation except a deterministic spinner while Codex is running.

### Manual note editor

- Bordered `NOTE_SURFACE` panel.
- Label: `Manual endnote`.
- Enter accepts a non-empty note; Escape returns without changing the file.
- Empty submission stays in the editor and reports `Note cannot be empty`.

### Codex progress and review

- Pressing `c` opens a bordered action chooser with revision and automatic
  endnote options; neither option starts until the user selects it.
- Revision mode collects one explicit prose-editing instruction before Codex
  starts.
- Running state identifies whether Codex is drafting a revision or an endnote
  and shows a spinner.
- Escape cancels the child process and returns to the unchanged document.
- Returned text opens in the same review editor. Enter applies a revision to
  the selected source range or saves an endnote, depending on the chosen action.
- No file write occurs before review acceptance.
- Codex errors retain the selection and show a typed error status.

### Endnote navigation

- Enter on a persisted annotation jumps to the linked managed endnote.
- `b` returns to the body anchor.
- The destination receives focus blue; the body highlight remains marker yellow.
- Every endnote title is prefixed with its document-order number and previews at
  most 24 Unicode graphemes from the selected prose.

## 6. Interaction and Feedback

| Key | Mode | Action |
|---|---|---|
| `↑` / `↓`, `k` / `j` | Browse | Jump to the previous/next Markdown paragraph |
| `←` / `→`, `h` / `l` | Browse/Visual | Move by selectable grapheme |
| Left mouse drag | Browse/Visual | Select exact rendered grapheme cells |
| Mouse wheel | Browse/Visual | Scroll the document by three rendered rows |
| `v` | Browse | Start visual selection |
| `w` | Browse | Select the current rendered word |
| `a` | Visual | Open manual endnote editor |
| `c` | Visual | Open the Codex action chooser |
| `r` | Codex choice | Enter a sentence-revision instruction |
| `e` | Codex choice | Generate an automatic explanatory endnote |
| Enter | Revision input | Send the instruction to Codex |
| Enter | Browse | Follow a RedPen annotation link |
| Enter | Revision review | Atomically replace the selected source |
| Enter | Manual input/Endnote review | Accept non-empty note and atomically save |
| Escape | Visual/Choice/Input/Review/Codex | Cancel current transient operation |
| `b` | Browse | Return from endnote to body |
| `q` / Ctrl-C | Browse | Restore terminal and quit |

Feedback is immediate and state-bearing. No decorative transitions or motion.

## 7. Depth and Material

- One quiet border separates document and transient editor.
- Marker yellow is flat and opaque like ink on paper; no glow or gradient.
- Focus is an underline, not a second background that obscures the marker.
- Endnotes use the slightly elevated `NOTE_SURFACE`, not a modal shadow.

## 8. Accessibility Constraints and Accepted Debt

### Constraints

- Every feature is keyboard accessible.
- Color is never the only signal: modes, titles, underline, and prefixes remain.
- Marker text uses a dark foreground on yellow for strong contrast.
- CJK and emoji widths are measured with Unicode-aware libraries and verified in
  a real xterm.js terminal.
- Ctrl-C and all normal exits restore the terminal.
- A failed parse, invalid selection, Codex error, or external file modification
  leaves the original file bytes unchanged.

### Accepted v1 debt

- Selection is limited to one inline block and cannot overlap an existing link,
  code span, image, or RedPen annotation.
- Theme customization is deferred; true-color and no-color fallbacks ship first.
- Screen-reader semantics are limited by terminal capabilities; all information
  remains available as plain text and keyboard state.
