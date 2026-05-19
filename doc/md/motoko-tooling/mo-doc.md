---
title: "Generating Motoko documentation"
description: "mo-doc is a command-line tool for generating documentation for Motoko source code in HTML, Markdown, or AsciiDoc format."
sidebar:
  order: 3
---

`mo-doc` is a command-line tool for generating documentation for Motoko source code. It processes source files and generates documentation in various formats.

Download `mo-doc` from Motoko's [GitHub releases page](https://github.com/caffeinelabs/motoko/releases).

### Generate in HTML format (default)

```bash
mo-doc
```

### Generate in Markdown format

```bash
mo-doc --format plain
```

### Generate in AsciiDoc format

```bash
mo-doc --format adoc
```

### Use a custom source code directory (defaults to `src`)

```bash
mo-doc --src path/to/motoko/files
```

### Use a custom output directory (defaults to `docs`)

```bash
mo-doc --output path/to/custom/output
```

## Options

- `--source <path>`: Specifies the directory to search for Motoko source files. Defaults to `src`.

- `--output <path>`: Specifies the directory where the documentation will be generated. Defaults to `docs`.

- `--format <format>`: Specifies the generated format. Should be one of the following:
  - `html`: Generates HTML format documentation.
  - `adoc`: Generates AsciiDoc format documentation.
  - `plain`: Generates Markdown documentation.

  Defaults to `html`.

- `--help`: Shows usage information.

## Examples

1. Generate HTML documentation from the default source directory (`src`) and place it in the default output directory (`docs`):

   ```bash
   mo-doc
   ```

2. Generate Markdown documentation from a specific source directory:

   ```bash
   mo-doc --format plain --source ./motoko-code
   ```

3. Generate AsciiDoc documentation in a custom output directory:

   ```bash
   mo-doc --format adoc --output ./public
   ```

## Writing doc comments

`mo-doc` supports documenting your Motoko code using special block comments (`/** */`) and line comments (`///`).

Doc comments can be used to provide explanations for functions, classes, types, modules, variables, and more. They can span multiple lines and may contain rich Markdown formatting:

```motoko no-repl
/// Calculate the factorial of a given positive integer.
///
/// Example:
/// ```motoko
/// factorial(0); // => null
/// factorial(3); // => ?6
/// ```
func factorial(n : Nat) : ?Nat {
    // ...
}
```

## Resources

Check out Motoko's [core package source code](https://github.com/caffeinelabs/motoko-core/tree/main/src) for additional examples and best practices.

The source code for `mo-doc` is available in the [caffeinelabs/motoko](https://github.com/caffeinelabs/motoko/tree/master/src/docs) GitHub repository. Contributions are welcome!
