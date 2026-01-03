# Markdown Annotator

A VS Code extension for adding annotations to markdown files with AI assistance.

## Features

- Add annotations to selected text
- AI-powered explanations (Claude, Gemini, Codex)
- Manual note input
- Auto-generates `<mark data-note="">` tags for web display

## Installation

### Option 1: Symlink (Development)

```bash
ln -s /path/to/MD_RedPen ~/.vscode/extensions/markdown-annotator
```

### Option 2: Copy

```bash
cp -r /path/to/MD_RedPen ~/.vscode/extensions/markdown-annotator
```

Restart VS Code after installation.

## Usage

### Keyboard Shortcuts

| OS | Shortcut |
|---|---|
| macOS | `Cmd + Shift + M` |
| Windows/Linux | `Ctrl + Shift + M` |

### Steps

1. Select text in a markdown file
2. Press `Cmd + Shift + M` (or `Ctrl + Shift + M`)
3. Choose an option:
   - **Direct Input**: Write your own note
   - **Claude**: Request explanation from Claude AI
   - **Gemini**: Request explanation from Gemini AI
   - **Codex**: Request explanation from Codex AI
4. For AI options:
   - Review/edit the prompt, then press Enter
   - Wait for AI response
   - Review/edit the response, then press Enter
5. The text is wrapped with `<mark data-note="note">selected text</mark>`

### Example

**Before:**
```markdown
TransferEngine uses RDMA for data transfer.
```

**After:**
```markdown
<mark data-note="RDMA enables direct memory access between computers without CPU involvement">TransferEngine uses RDMA for data transfer.</mark>
```

## Requirements for AI Features

To use AI features, you need to install and authenticate the CLI tools yourself.

### Supported CLI Tools

| Model | CLI | Installation |
|---|---|---|
| Claude | `claude` | [Claude Code](https://docs.anthropic.com/en/docs/claude-code) |
| Gemini | `gemini` | [Gemini CLI](https://github.com/google-gemini/gemini-cli) |
| Codex | `codex` | [Codex CLI](https://github.com/openai/codex) |

### Setup Steps

1. Install the CLI tool for your preferred AI model
2. Authenticate with your own account/API key
3. Verify the CLI works by running it in terminal (e.g., `claude --help`)

**Note:** If CLI tools are not installed, AI features will show an error. You can still use "Direct Input" to write notes manually.

## Privacy & Security

| Item | Risk | Description |
|---|---|---|
| Data Collection | ✅ None | This extension does not collect or transmit any data |
| External Connections | ✅ None | Only calls locally installed CLI tools |
| Selected Text | ⚠️ User Responsibility | Text is sent to AI through your own authenticated CLI |

**Summary:** This extension is safe. It only acts as a bridge to your locally installed CLI tools. Any data sent to AI services goes through your own authenticated CLI, under your control and responsibility.

## Web Display

Annotations added with this extension are displayed as highlights on Jekyll blogs.
Visitors can click the highlight to view the note in a popup.

## License

MIT License
