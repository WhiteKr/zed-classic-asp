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
   git clone https://github.com/WhiteKr/tree-sitter-vbscript
   ```

2. Make sure the `[grammars.vbscript]` entry in `extension.toml` points at the grammar repository (a `file://` URL works for local development; the `commit` field must match a commit that exists there).

3. In Zed, run `zed: install dev extension` from the command palette and select the `zed-classic-asp` directory.

4. Open any `.asp` file — for example `test/fixtures/sample.asp`.

## Language server (asp-ls)

The extension ships with a minimal language server providing:

- **Go to definition** on `<!--#include file|virtual="..."-->` directives (opens the included file). A leading `/` in `file=` paths resolves against the web root, same as `virtual=`
- **Go to definition** on function/sub names — searched through the current file and its include chain, falling back to the whole workspace
- **Find references** — lists every file that includes the current file (or, on an include directive, the directive's target)
- **Workspace symbol search** for `Sub` / `Function` / `Class` / `Property` definitions
- **Diagnostics** for include directives whose target file cannot be found
- **Hover docs** for the intrinsic objects (`Request`, `Response`, `Server`, `Session`, `Application`), common ADO members (`MoveNext`, `EOF`, `BeginTrans`, ...), and user-defined functions (shows the definition line)

### Setup

Build and install the server binary so Zed can find it:

```sh
cargo install --path server
```

(or build it anywhere and point Zed at it via settings):

```jsonc
{
  "lsp": {
    "asp-ls": {
      "binary": { "path": "/path/to/asp-ls" }
    }
  }
}
```

### Web root for `virtual=` includes

`<!--#include virtual="/lib/db.asp"-->` paths resolve against the web root, which defaults to the workspace root. If your web root is a subdirectory, set it in Zed settings:

```jsonc
{
  "lsp": {
    "asp-ls": {
      "initialization_options": { "webRoot": "wwwroot" }
    }
  }
}
```

## Supported

- VBScript inside `<% %>` and `<%= %>`
- HTML outside of ASP blocks
- Outline items for `Sub` / `Function` / `Class` / `Property Get|Let|Set`

## Not supported (yet)

- JScript pages and the `@Language` directive (VBScript is assumed)
- `<script runat="server">` blocks (rendered as plain HTML script content; highlighting will not break, but no VBScript highlighting inside)
- Static analysis (undefined-function warnings and the like) — VBScript's case-insensitivity and `Execute`/`Eval` make false positives too likely
- Auto-indent inside `If` / `Sub` / loop blocks: the minimal grammar is intentionally flat (no block nodes) to keep highlighting resilient on broken code, so indent queries have nothing to anchor to

## Architecture

- `<% %>` boundaries are parsed with [tree-sitter-embedded-template](https://github.com/tree-sitter/tree-sitter-embedded-template); HTML and VBScript are injected into `content` / `code` nodes.
- VBScript uses a purpose-built minimal grammar ([tree-sitter-vbscript](https://github.com/WhiteKr/tree-sitter-vbscript)): a flat token-level grammar with structured nodes only for `Sub` / `Function` / `Class` / `Property` definitions, which keeps highlighting robust on partial or invalid code.
