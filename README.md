# mq (mail query)

`mq` is a fast, lightweight, read-only terminal application for querying and viewing emails from a `notmuch` database. It is designed around vim-style keybindings and acts as a specialized search tool built to complement existing terminal email clients like `aerc` or `mutt`.

## Features

- **Fast search**: Live search with debouncing using native `notmuch` bindings.
- **HTML previews**: Renders HTML emails to plain text with `html2text`.
- **Multiple views**: Open emails in your pager, view HTML in a browser, or check folder info.
- **Read-only**: Queries without modifying your `notmuch` database or Maildir.

## Installation

Ensure you have Rust, Cargo, `pkg-config`, and `libnotmuch` headers installed.

```bash
git clone https://seed.boers.email/z2AdUML1AaZmUVidUJ4vwQDJhmvKg.git mq
cd mq
cargo install --path .
```

*Note: For NixOS users, a `devenv.nix` is provided to drop into a configured shell environment automatically.*

## Configuration

`mq` leverages your default `notmuch` configuration transparently.

You can optionally override application settings by creating `~/.config/mq/config.toml`:

```toml
# Auto-discovered if omitted. Overrides the active notmuch db.
# database_path = "/home/user/Maildir"

# Overrides $PAGER environment variable
# pager = "less -R"

# Overrides system default browser for opening HTML attachments
# browser = "xdg-open"

# Maximum queries to return (Default: 100)
# max_results = 500
```

## Keybindings

### Global Context

| Key | Action |
| --- | --- |
| `q` / `Esc` | Unfocus search, close popups, or exit `mq` |
| `?` | Toggle help overlay and syntax guide |


### Search Mode (Focus: `/`)

| Key | Action |
| --- | --- |
| `Enter` | Unfocus search and move to Result View |
| `Ctrl+a` / `Ctrl+e` | Go to start/end of line |
| `Ctrl+b` / `Ctrl+f` | Move backward/forward one character |
| `Alt+b` / `Alt+f` | Move backward/forward one word |
| `Alt+d` | Delete forward word |
| `Ctrl+d` | Delete character under cursor |
| `Ctrl+u` | Clear from cursor to start of line |
| `Ctrl+k` | Clear from cursor to end of line |
| `Ctrl+w` | Delete backward word |
| `Ctrl+Left/Right`| Move cursor by word |
| `Home/End` | Move cursor to the start/end of the query |

### Result View

| Key | Action |
| --- | --- |
| `j` / `k` | Navigate the email list down/up |
| `Ctrl+d` / `Ctrl+u`| Jump 10 items down/up in the list |
| `Enter` | Open the selected email with your system `$PAGER` |
| `PgDn/PgUp` | Scroll the preview pane down/up |
| `Ctrl+f/b` | Scroll the preview pane down/up (result view only) |
| `h` / `l` | Scroll the preview pane horizontally (left/right) |
| `o` | Extract the HTML part and open it in the system browser |
| `f` | Show native Maildir folder info (useful for finding the thread in `aerc`) |

## Query Syntax

Standard `notmuch` Xapian syntax applies directly:

- `from:alice@example.com`
- `subject:"weekly report"`
- `date:yesterday..today`
- `tag:unread AND folder:Work`

*(For comprehensive query capabilities, access the `?` menu within `mq`)*
