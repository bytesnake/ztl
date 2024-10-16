# ZTL - Zettel Tools by Lorenz

Ztl provides a unified way to render notes from Markdown, LaTeX and BibTeX. Generated cache provides spanning information for editors like ViM and HTML artifacts to publish notes to Mastodon and for live preview with SIXEL enabled terminals.

The repository provides a terminal tool written in Rust to generate note cache and a Neovim plugin for navigation.

## Features

 - render note source from Markdown with [comrak](https://github.com/kivikakk/comrak), LaTeX with [TeX4ht](https://tug.org/tex4ht/) and BibTeX to standalone HTML and MathML
 - abstract files from individual notes, provides linking capabilities between any file format
 - generate unified representation with TOML files to cache folder at `.ztl/cache/` for downstream plugins
 - use span and note information for note navigation and publishing to Mastodon

Supported subcommands of `ztl` are

```bash
Usage: ztl [OPTIONS] [COMMAND]

Commands:
  watch    Watch files and rebuild
  publish  Publish notes to Mastodon instance
  build    Build all notes from scratch
  help     Print this message or the help of the given subcommand(s)

Options:
  -d, --debug...  Enable debugging
  -h, --help      Print help
  -V, --version   Print version
```

## Example with LaTeX

![Peek 2024-10-16 19-43](https://github.com/user-attachments/assets/ba966a55-9b75-40e9-9249-68ac221ebd0b)

### WIP

 - [x] support for Markdown and LaTeX rendering
 - [x] generate cache for navigation plugins in editor
 - [ ] publish set of notes to Mastodon
 - [ ] write more complete documentation and installation guide
