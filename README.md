# Classic ASP for Zed

Syntax highlighting and outline support for Classic ASP (VBScript) files in the [Zed](https://zed.dev) editor.

## Features

- Highlighting for `.asp` / `.asa` files:
  - HTML outside `<% %>` blocks
  - VBScript inside `<% %>` and `<%= %>` blocks
  - `<%` / `%>` / `<%=` delimiters
- Case-insensitive VBScript keyword recognition (`Dim`, `dim`, `DIM` all work)
- Outline panel / breadcrumbs for `Sub`, `Function`, `Class`, and `Property` definitions
- Standalone `.vbs` files are highlighted as VBScript
- Highlighting is resilient: a parse error in one region does not break the rest of the file

## Installation (dev extension)

1. Clone this repository, and clone the grammar repository next to it:

   ```sh
   git clone https://github.com/WhiteKr/zed-classic-asp
   git clone https://github.com/WhiteKr/tree_sitter_vbscript
   ```

2. Make sure the `[grammars.vbscript]` entry in `extension.toml` points at the grammar repository (a `file://` URL works for local development; the `commit` field must match a commit that exists there).

3. In Zed, run `zed: install dev extension` from the command palette and select the `zed-classic-asp` directory.

4. Open any `.asp` file — for example `test/fixtures/sample.asp`.

## Supported

- VBScript inside `<% %>` and `<%= %>`
- HTML outside of ASP blocks
- Outline items for `Sub` / `Function` / `Class` / `Property Get|Let|Set`

## Not supported (yet)

- JScript pages and the `@Language` directive (VBScript is assumed)
- `<script runat="server">` blocks (rendered as plain HTML script content; highlighting will not break, but no VBScript highlighting inside)
- Include navigation (`<!--#include file|virtual="..."-->` ctrl-click), go-to-definition, references, hover docs — planned for phase 2 via a small language server
- Auto-indent inside `If` / `Sub` / loop blocks: the minimal grammar is intentionally flat (no block nodes) to keep highlighting resilient on broken code, so indent queries have nothing to anchor to

## Architecture

- `<% %>` boundaries are parsed with [tree-sitter-embedded-template](https://github.com/tree-sitter/tree-sitter-embedded-template); HTML and VBScript are injected into `content` / `code` nodes.
- VBScript uses a purpose-built minimal grammar ([tree_sitter_vbscript](https://github.com/WhiteKr/tree_sitter_vbscript)): a flat token-level grammar with structured nodes only for `Sub` / `Function` / `Class` / `Property` definitions, which keeps highlighting robust on partial or invalid code.
