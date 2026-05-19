---
title: "Motoko Visual Studio Code extension"
description: "The Motoko VS Code extension adds type checking, code formatting, autocompletion, go-to-definition, and code snippets for Motoko development."
sidebar:
  order: 4
---

[Visual Studio Code (VS Code)](https://code.visualstudio.com/) is a widely used open source IDE that supports canister development in [Motoko](../index.md).

The [Motoko VS Code extension](https://github.com/caffeinelabs/vscode-motoko) enhances the development experience by providing features such as type checking, code formatting, autocompletion, go-to-definition, code snippets, and more, making it easier to build, navigate, and manage Motoko-based canisters.

[![Showcase](https://github.com/dfinity/vscode-motoko/raw/master/guide/assets/intro.webp)](https://marketplace.visualstudio.com/items?itemName=dfinity-foundation.vscode-motoko)

### Installation

[![Visual Studio Marketplace](https://img.shields.io/visual-studio-marketplace/v/dfinity-foundation.vscode-motoko?color=brightgreen&logo=visual-studio-code)](https://marketplace.visualstudio.com/items?itemName=dfinity-foundation.vscode-motoko)

Install the extension through the [VS Marketplace](https://marketplace.visualstudio.com/items?itemName=dfinity-foundation.vscode-motoko), or alternatively the [**Extensions** panel](https://code.visualstudio.com/docs/editor/extension-marketplace) in your VS Code project.

[VSCodium](https://vscodium.com/) users can download the extension from [Open VSX](https://open-vsx.org/extension/dfinity-foundation/vscode-motoko) or the [GitHub releases](https://github.com/dfinity/vscode-motoko/releases) page.

### Keyboard shortcuts

Below are the default key bindings for commonly used features supported in the extension:

- **Code formatter** (`Shift` + `Alt` + `F`): Format a Motoko file using [prettier-plugin-motoko](https://github.com/dfinity/prettier-plugin-motoko).
- **Organize imports** (`Shift` + `Alt` + `O`): Group and sort imports at the top of your Motoko file.
- **Import code action** (`Ctrl/Cmd` + `.` while hovering over an unresolved variable): Show import quick-fix options.
- **Go to definition** (`F12`): Jump to the definition of a local or imported identifier.
- **IntelliSense** (`Ctrl` + `Space`): View all available auto-completions and code snippets.

[![Snippets](https://github.com/dfinity/vscode-motoko/raw/master/guide/assets/snippets.png)](https://marketplace.visualstudio.com/items?itemName=dfinity-foundation.vscode-motoko)

### Other features

- The [Mops](https://mops.one/) and [Vessel](https://github.com/dfinity/vessel) Motoko package managers are supported out-of-the-box in this extension.
- Quickly convert between Motoko types using code snippets such as `array-2-buffer` or `principal-2-text`.
- The Motoko VS Code extension works standalone on all major operating systems (including Windows) without any additional tooling installed.
- This extension also provides schema validation and autocompletion for `dfx.json` configuration files.
- View type information and documentation by hovering over function names, imports, and other expressions.
- Deploy temporary canisters to the ICP mainnet directly from the editor.

[![Tooltips](https://github.com/dfinity/vscode-motoko/raw/master/guide/assets/tooltips.png)](https://marketplace.visualstudio.com/items?itemName=dfinity-foundation.vscode-motoko)

#### Contributing

[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg?logo=github)](https://github.com/dfinity/prettier-plugin-motoko)

The Motoko VS Code extension is completely open source and [available on GitHub](https://github.com/caffeinelabs/vscode-motoko).
