//! Command-line contract.

use std::path::PathBuf;

use clap::Parser;

const KEY_HELP: &str = "\
Keys:
  v  start visual selection
  a  add manual highlighted endnote
  c  ask Codex for an endnote
  w  select current word
  Up/Down or j/k  move by paragraph
  Left/Right or h/l  move by grapheme
  mouse drag  select rendered text
  Enter  follow annotation link
  b  return from endnote
  q  quit";

/// MD RedPen command-line options.
#[derive(Debug, Parser)]
#[command(
    name = "md-redpen",
    about = "Read Markdown and attach highlighted AI-assisted endnotes",
    after_help = KEY_HELP
)]
pub struct Cli {
    /// Markdown file to open.
    pub markdown: PathBuf,
}

impl Cli {
    /// Parses process arguments, including Clap's help and error exits.
    #[must_use]
    pub fn from_env() -> Self {
        Self::parse()
    }
}
