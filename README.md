<h1 align="center">CODEMAP</h1>
<p align="center">
A Rust repository scanner with a self-contained HTML treemap, cross-platform installers, and release binaries for Linux and Windows (x86-64 and ARM64).
</p>

> [!WARNING]
> CodeMap is a personal project. Features, releases, and support may change without notice.

CodeMap scans a directory once, records file and folder properties, and opens a nested treemap in your browser. The generated page is self-contained: browse the map, adjust its view, and export it without running a server or sending repository data anywhere.

## What it shows

- Nested folders and files sized by lines, file count, or bytes
- File and folder properties, including line counts, byte size, nesting, and language totals
- Folder language composition, sortable by file count, lines, or storage
- Search and filters for names, paths, categories, and languages
- Folder navigation, adjustable visible depth, and selectable detail levels
- A compact legend showing the ten most-used languages in the current folder
- Dark and light themes, with language colors or a monochrome palette
- SVG and PNG export
- Language metadata based on GitHub Linguist, plus formats for AI models and common applications

## Install

Installers download the latest release for your operating system. The current release provides these binary assets:

| Platform | Asset |
| --- | --- |
| Linux x86-64 | `codemap-linux-x86_64` |
| Linux ARM64 | `codemap-linux-aarch64` |
| Windows x86-64 | `codemap-windows-x86_64.exe` |
| Windows ARM64 | `codemap-windows-aarch64.exe` |

### Linux

```sh
curl -fsSL https://raw.githubusercontent.com/DranxX/CodeMap/main/install.sh | sh
```

The installer places `codemap` in `~/.local/bin`. Set `CODEMAP_INSTALL_DIR` to choose another directory, or `CODEMAP_VERSION` to install a specific release tag such as `v1.0.1`.

### Windows PowerShell

```powershell
irm https://raw.githubusercontent.com/DranxX/CodeMap/main/install.ps1 | iex
```

The installer places `codemap.exe` in `%LOCALAPPDATA%\Programs\CodeMap` and adds that directory to your user `PATH`. Set `CODEMAP_INSTALL_DIR` to choose another directory, or `CODEMAP_VERSION` to install a specific release tag.

## Use

Run CodeMap inside the repository you want to map, or pass its path:

```sh
cd /path/to/repository
codemap
```

```sh
codemap /path/to/repository
```

There is no `map` subcommand: `codemap` performs the repository scan and generates the map. Run `codemap --help` to see the available options.

## Build

Rust 2024 edition and Cargo are required.

```sh
cargo build --release
```

The executable is written to `target/release/codemap` on Linux, or `target/release/codemap.exe` on Windows.

When running a locally built binary, pass it the repository directory, cap scan depth, or exclude paths:

```sh
./target/release/codemap
./target/release/codemap /path/to/repository
./target/release/codemap /path/to/repository --depth 5
./target/release/codemap /path/to/repository --ignore target node_modules
```

By default, CodeMap scans unlimited directory depth and writes `<repository>-map.html` to the current working directory. It then asks the operating system to open that file in the default browser. Use `--no-open` to skip this step, or `--output PATH` to choose a different destination.

### Options

| Option | Description |
| --- | --- |
| `[PATH]` | Directory to scan. Defaults to the current directory. |
| `--depth N` | Scan at most `N` directory levels. `N` must be at least `1`. |
| `--ignore PATH...` | Exclude one or more exact file or folder paths. |
| `-o, --output PATH` | Write the standalone HTML map to this path. |
| `--no-open` | Write the map without launching a browser. |
| `-V, --version` | Print the binary version. |
| `-h, --help` | Print usage and options. |

Put the repository path before `--ignore` when supplying it explicitly, because `--ignore` consumes paths until the next option. Ignore values are paths, not wildcard patterns. A relative path is resolved from the current directory if it points inside the scanned repository; otherwise CodeMap resolves it from the repository root. Absolute paths inside the repository are also accepted. Ignoring a folder excludes its entire subtree. `.git` metadata is always skipped. Symbolic links are not followed.

The scan depth controls which data is stored in the generated page. The page's visible-depth control only changes the current view; it cannot reveal data beyond the scan limit.

## File properties

CodeMap includes regular files with known and unknown extensions. It counts physical and nonblank lines for files that look like UTF-8 text. Binary and model formats have a line count of zero. When multiple languages claim an extension, CodeMap chooses a deterministic primary classification and keeps alternatives in the file details.

The embedded language catalog is derived from [GitHub Linguist's language definitions](https://github.com/github-linguist/linguist/blob/main/lib/linguist/languages.yml). Its license and attribution are recorded in [NOTICE](NOTICE). CodeMap is licensed under the [MIT License](https://github.com/DranxX/CodeMap/blob/main/LICENSE).

## Development checks

```sh
cargo fmt --check
cargo test
cargo build --release
```

`src/index.html` is the canonical page template embedded in the Rust binary. `CodeMap-map.html` is only a generated local preview.
