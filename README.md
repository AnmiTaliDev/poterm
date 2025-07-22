# Poterm

Modern TUI (Terminal User Interface) editor for .po (Portable Object) translation files.

![Poterm Demo](https://img.shields.io/badge/status-in%20development-blue)
![License](https://img.shields.io/badge/license-Apache%202.0-green)
![Rust](https://img.shields.io/badge/rust-1.70+-orange)

## Features

- **Modern TUI Interface**: Clean and intuitive terminal-based interface built with Ratatui
- **Full .po File Support**: Complete parsing and writing of Gettext .po files
- **Real-time Editing**: Edit translations, comments, and metadata inline
- **Smart Filtering**: Filter by translation status (untranslated, fuzzy, all)
- **Powerful Search**: Find entries by original text or translation
- **Progress Tracking**: Visual progress indicators and statistics
- **Syntax Highlighting**: Color-coded entry states (translated, fuzzy, untranslated)
- **Multi-field Editing**: Edit msgid, msgstr, and comments
- **Translation Status Management**: Mark entries as fuzzy or done with hotkeys
- **Metadata Editing**: Edit header metadata (Language, Translator, etc.)
- **.pot Template Support**: Create .po files from .pot templates
- **Keyboard Shortcuts**: Vim-inspired navigation with modern shortcuts

## Installation

### From Source

```bash
git clone https://github.com/AnmiTaliDev/poterm.git
cd poterm
cargo install --path .
```

## Usage

### Basic Usage

```bash
# Edit an existing .po file
poterm translations.po

# Create a new .po file
poterm --create new_translations.po

# Create a new .po file from a .pot template
poterm --from-pot template.pot translations.po
```

### Keyboard Shortcuts

#### Navigation
- `j` / `↓` - Next entry
- `k` / `↑` - Previous entry
- `Page Up` - Page up
- `Page Down` - Page down
- `Home` - First entry
- `End` - Last entry

#### Editing
- `i` / `Enter` - Start editing current field
- `Esc` - Stop editing / Cancel
- `Tab` - Switch to next field (msgid → msgstr → comments)
- `Shift+Tab` - Switch to previous field

#### Search & Filter
- `Ctrl+F` - Start search
- `F3` - Find next
- `Shift+F3` - Find previous
- `Ctrl+U` - Toggle untranslated entries filter
- `Ctrl+Z` - Toggle fuzzy entries filter

#### File Operations
- `Ctrl+S` - Save file
- `Ctrl+Shift+P` - Save current entry
- `Ctrl+Q` - Quit

#### Translation Status
- `F2` / `Ctrl+T` - Toggle fuzzy status of current entry
- `Ctrl+D` - Mark current entry as done (remove fuzzy flag)

#### Metadata
- `F9` - Toggle metadata editing mode

#### Help
- `F1` - Show help overlay

## Metadata Editing

To edit .po file metadata (header fields):

1. **Enter Metadata Mode**: Press `Ctrl+M`
2. **Navigate**: Use `↑`/`↓` or `j`/`k` to select metadata field
3. **Edit Field**: Press `Enter` or `i` to start editing
4. **Save Changes**: Press `Enter` to save, `Esc` to cancel
5. **Exit Metadata Mode**: Press `Ctrl+M` again

### Supported Metadata Fields

- **Project-Id-Version**: Project name and version
- **Language**: Language code (e.g., "ru", "fr", "de")
- **Language-Team**: Translation team information
- **Last-Translator**: Translator name and email
- **Report-Msgid-Bugs-To**: Bug report contact
- **POT-Creation-Date**: Template creation date
- **PO-Revision-Date**: Last modification date (auto-updated)
- **MIME-Version**: MIME version (usually "1.0")
- **Content-Type**: Content type and charset
- **Content-Transfer-Encoding**: Transfer encoding
- **Plural-Forms**: Plural form rules for the language

## .po File Format Support

Poterm supports the complete Gettext .po file format including:

- **msgid/msgstr**: Original and translated text
- **msgctxt**: Message context
- **Comments**: Translator comments (`# comment`)
- **Extracted Comments**: Developer comments (`#. comment`)
- **References**: Source file references (`#: file:line`)
- **Flags**: Translation flags (`#, fuzzy`, `#, c-format`, etc.)
- **Multiline strings**: Proper handling of multi-line translations
- **Escape sequences**: Support for `\n`, `\t`, `\"`, etc.
- **Header metadata**: Project information and translation metadata

## Interface Overview

```
┌─ Poterm - translations.po ─────────────────────────────────────────────┐
│ Total: 150 | Translated: 120 (80.0%) | Fuzzy: 10 | Untranslated: 20   │
├────────────────────────────────────────────────────────────────────────┤
│ ┌─ Entries [All] ────────────────┐ ┌─ Original Text (msgid) ─────────┐ │
│ │ ✓   1 Hello World              │ │ Hello World                     │ │
│ │ ~   2 Welcome to the app       │ │                                 │ │
│ │ ○   3 Please enter your name   │ └─────────────────────────────────┘ │
│ │ ✓   4 Submit                   │ ┌─ Translation (msgstr) ──────────┐ │
│ │ ►   5 Cancel                   │ │ Hola Mundo                      │ │
│ │                                │ │                                 │ │
│ └────────────────────────────────┘ └─────────────────────────────────┘ │
│                                    ┌─ Comments ──────────────────────┐ │
│                                    │ This is a greeting message      │ │
│                                    │                                 │ │
│                                    └─────────────────────────────────┘ │
│                                    ┌─ Information ───────────────────┐ │
│                                    │ References: main.c:15           │ │
│                                    │ Flags: c-format                 │ │
│                                    └─────────────────────────────────┘ │
├────────────────────────────────────────────────────────────────────────┤
│ Ctrl+Q: Quit | Ctrl+S: Save | Enter: Edit | Tab: Switch | Ctrl+F: Search│
└────────────────────────────────────────────────────────────────────────┘
```

### Status Icons
- `✓` - Translated entry
- `~` - Fuzzy translation (needs review)
- `○` - Untranslated entry

## Development

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

### Development Dependencies

- Rust 1.70+
- Cargo

### Project Structure

```
src/
├── main.rs        # Application entry point and CLI
├── ui.rs          # TUI interface and event handling
└── gettext.rs     # .po file parsing and manipulation

Cargo.toml         # Project configuration
README.md          # This file
```

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

## Author

**AnmiTaliDev** - [anmitali198@gmail.com](mailto:anmitali198@gmail.com)

GitHub: [AnmiTaliDev/poterm](https://github.com/AnmiTaliDev/poterm)

## Acknowledgments

- Built with [Ratatui](https://github.com/ratatui-org/ratatui) for the terminal interface
- Inspired by modern text editors and translation tools
- Thanks to the Rust community for excellent crates and documentation

---

*Poterm aims to make .po file translation faster and more enjoyable for developers and translators alike.*
