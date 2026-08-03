# MD RedPen

MD RedPen is a keyboard-first terminal UI for reviewing Markdown. It turns
selected prose into a **yellow highlighter-style link** and stores explanations,
revision suggestions, or supporting context as endnotes at the end of the same
document.

The source text and its notes remain together in one Markdown file. MD RedPen
does not require a database or sidecar file, and readers who do not use the
application can still follow the links and read the notes in an ordinary
Markdown renderer.

## How it works

1. Open a Markdown file.
2. Move between paragraphs with `Up` / `Down`, then select text with `v` and
   `Left` / `Right`, drag over rendered prose with the mouse, or press `w` to
   select the current word.
3. Choose a note workflow:
   - `a` writes a manual endnote.
   - `c` asks a Codex CLI authenticated through a ChatGPT subscription for an
     explanatory draft.
4. Review and edit the draft.
5. Press Enter to save the highlighted body link and its endnote together.

The saved structure remains human-readable Markdown. This English-only example
shows the link, anchor, and managed note block:

```markdown
<mark>[RDMA][rp-019fc502-c508-7750-ae68-dc807f695d5a]</mark> enables direct memory access.

<!-- md-redpen:notes:start v=1 -->
## MD RedPen Notes

<a id="rp-note-019fc502-c508-7750-ae68-dc807f695d5a"></a>
### 1) RDMA

A device can access memory without routing data through the CPU.

[rp-019fc502-c508-7750-ae68-dc807f695d5a]: #rp-note-019fc502-c508-7750-ae68-dc807f695d5a
<!-- md-redpen:notes:end -->
```

The `<mark>` element preserves the highlighter meaning, while the selected prose
becomes a link to the matching `#rp-note-...` anchor.

Each endnote heading is numbered in document order and uses at most 24 Unicode
graphemes from the selected prose. Longer selections end with an ellipsis.

## Requirements

- Rust 1.94 or later
- A terminal with true-color support is recommended
- To use Codex:
  - The Codex CLI must be available on `PATH`.
  - `codex login` must be authenticated with a ChatGPT subscription.

Check the current authentication state with:

```bash
env -u CODEX_API_KEY -u OPENAI_API_KEY codex login status
```

A valid subscription login prints `Logged in using ChatGPT`.

## Install and run

```bash
git clone https://github.com/dhihm/MD_RedPen.git
cd MD_RedPen
cargo install --path .
md-redpen path/to/document.md
```

You can also run the application directly from the repository:

```bash
cargo run -- path/to/document.md
```

Show the built-in help:

```bash
md-redpen --help
```

## Key bindings

| Key | Mode | Action |
|---|---|---|
| `Up` / `Down`, `k` / `j` | Browse | Move to the previous or next Markdown paragraph |
| `Left` / `Right`, `h` / `l` | Browse, Visual | Move or extend the selection by one selectable grapheme |
| Left-button drag | Browse, Visual | Select the exact rendered grapheme cells under the pointer |
| Mouse wheel | Browse, Visual | Scroll the rendered document by three terminal rows |
| `v` | Browse | Start a visual selection at the cursor |
| `w` | Browse | Select the current word |
| `a` | Visual | Write a manual endnote |
| `c` | Visual | Request a Codex explanation |
| Enter | Input, Review | Atomically save the reviewed endnote |
| Enter | Browse | Follow a highlighted link to its endnote |
| `b` | Browse | Return from an endnote to the body |
| Escape | Visual, Input, Review, Codex | Cancel the current operation |
| `q`, Ctrl-C | Browse | Restore the terminal and quit |

Korean, CJK, emoji, and combining characters are handled by Unicode grapheme
boundaries and their actual terminal cell widths.

## Codex subscription integration

MD RedPen never copies or stores an API key. It invokes the current user's
`codex` executable with this contract:

```text
codex exec
  --ephemeral
  --ignore-user-config
  --ignore-rules
  --skip-git-repo-check
  --sandbox read-only
  --color never
  -
```

- `codex login status` must explicitly confirm a ChatGPT login.
- `CODEX_API_KEY` and `OPENAI_API_KEY` are removed from the child process.
- Codex runs in an empty temporary working directory with a read-only sandbox.
- Only the selected text and the source line containing it are sent to Codex.
- Standard output and standard error are each limited to 64 KiB.
- The default timeout is 120 seconds.
- Escape, Ctrl-C, errors, and normal shutdown all terminate the Codex process
  group.
- Codex returns text only. MD RedPen remains the only process that modifies the
  document.

Set a model only when the subscription requires an explicit model name:

```bash
MD_REDPEN_CODEX_MODEL=<subscription-available-model> md-redpen document.md
```

Tests and internal wrappers can override the executable path with
`MD_REDPEN_CODEX_BIN`.

> The selected text and the rest of its source line are sent to the model
> provider. Do not select a line that contains confidential information.

## Data safety

- The body link and endnote are created in one save transaction.
- Saving stops if another editor changes the file bytes after MD RedPen opens
  it.
- Saving never truncates the original file in place. A temporary file in the
  same directory is synchronized and atomically renamed.
- Selections that overlap an existing Markdown link or MD RedPen highlight are
  rejected.
- A Codex draft is not written until it is accepted with Enter on the Review
  screen.
- Parsing, selection, Codex, and save failures preserve the original file
  bytes.
- Managed-note markers shown inside fenced or inline code are treated as
  examples, not as real note boundaries.

## Selection limits

MD RedPen rejects ranges that could damage Markdown structure:

- A range spanning multiple lines or Markdown blocks
- A range overlapping an existing link, image, or inline-code span
- A range overlapping an existing MD RedPen highlight
- A range inside the managed `md-redpen:notes` block at the end of the document

Nested highlights are not supported. Mouse selection passes through the same
source-range safety checks as keyboard selection.

## Terminal accessibility

- Modes, status text, and underlines communicate state without relying on color
  alone.
- Every selection, navigation, and save action is available without a mouse.
- `NO_COLOR=1` replaces highlighter and selection colors with reverse and bold
  styles.
- Terminals smaller than 50x12 show a size error while keeping the quit key
  available.
- Persisted highlights use a marker-yellow background with dark foreground
  text.

Visual rules and color tokens are defined in [`DESIGN.md`](DESIGN.md).

## Development and verification

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
```

The test suite covers:

- Source-byte mapping for Korean, combining characters, and emoji
- Exact CJK mouse hit testing and paragraph navigation
- Clickable `<mark>` links and managed-endnote serialization
- Rejection of existing links, inline code, images, and nested highlights
- Parser-aware handling of literal managed-note markers in code examples
- External file conflict detection and byte preservation
- CLI help and typed path errors
- Safe Codex arguments, standard-input prompts, and API-key removal
- The invariant that no file changes occur before Codex review

## Migration from the VS Code prototype

The repository originally contained an experimental VS Code extension. It ran
a user-configurable shell command and replaced selected text with
`<mark data-note>` directly.

The current application is a standalone Rust TUI. The old `extension.js` and
`package.json` files have been removed, and the project no longer depends on VS
Code command registration or a Node.js runtime.

Documents created by the prototype are not converted automatically. Commit or
back up those files first, then migrate each note manually to the current
link-and-endnote format.

## License

[MIT](LICENSE)
