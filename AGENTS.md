# CodeMap Agent Guide

## Product

CodeMap scans one local directory and produces a self-contained HTML repository map. The Rust binary performs the scan once; the page stores the resulting file and folder properties so its filters, layout, and display settings work without rescanning.

The product is a nested treemap, not a dependency graph. Keep the map focused on repository structure, readable properties, and direct folder navigation.

## Source of truth

- `src/main.rs` owns the CLI, path validation, report serialization, output writing, and browser launch.
- `src/scan.rs` owns filesystem traversal, line counting, file classification, and aggregate properties.
- `src/model.rs` defines the serialized report and tree nodes.
- `src/languages.json` is the embedded language metadata snapshot. `NOTICE` records its upstream attribution.
- `src/index.html` is the canonical page template. Rust embeds it with `include_str!`; keep the CSS and JavaScript in this standalone page.
- `CodeMap-map.html` is a generated local preview, not a template source. Change `src/index.html` first; the preview can be regenerated from a scan.
- `Reference/` contains local upstream projects used for comparison. Do not treat those projects as CodeMap source or modify them as part of a CodeMap change.

## Product behavior

- With no positional path, scan the current working directory.
- Scan unlimited directory depth by default. `--depth N` limits the scan and must stay separate from the page's interactive visible-depth setting.
- `--ignore PATH...` consumes one or more exact paths until the next option. Ignored directories prune their full subtree. Do not interpret values as glob patterns.
- Relative ignore paths resolve from the current directory when the resolved path is inside the scan root; otherwise they resolve from the scan root. Absolute ignore paths must be inside the scan root.
- Include regular files with recognized or unknown extensions. Skip `.git` metadata and symbolic links; do not follow symlinks.
- Count physical and nonblank lines for text-like files. Binary formats have zero line counts.
- Store bytes, lines, nonblank lines, file/folder totals, direct children, nesting, and dominant language aggregates in each node where applicable.
- Keep file and folder classification deterministic. The catalog may return alternatives for ambiguous extensions.
- The generated page must not require a server, external scripts, remote fonts, or a network connection.

## CLI contract

- `[PATH]`: repository directory; defaults to `.`.
- `--depth N`: maximum scanned directory depth; positive integer only.
- `--ignore PATH...`: one or more exact file or folder paths; place a positional repository path before this option.
- `-o, --output PATH`: destination for the standalone HTML page.
- `--no-open`: write the page without launching the system browser.
- `-V, --version` and `-h, --help`: print version or usage and exit.

Keep CLI help and README examples aligned with the parser. Do not add flags for display settings already available in the page unless the scan itself needs that option.

## Page behavior and visual direction

- The map uses nested filled rectangles. Use a consistent fill per color category and distinguish neighboring rectangles with outlines; do not use gradients.
- Keep dark mode charcoal rather than pure black. Preserve the dark and light themes, language and monochrome palettes, and the GitHub Linguist color snapshot.
- Keep size metric and sort order as separate controls. Folder/file visibility and visible detail depth affect the view, not the scan data.
- A folder opens on double-click. Folder selection shows folder totals in Properties; file selection shows file properties. Back navigation and the relative breadcrumb must reflect the current focus.
- The map label is editable. Its text and color control stay outside exported map content unless the user explicitly includes them.
- Export SVG and PNG from the map container. Keep export actions behind the compact overflow menu.
- Small rectangles need an alternate way to inspect details, such as the properties panel or a tooltip. Do not rely on text that cannot fit inside the rectangle.
- UI copy and README are in English. Chat with the user in the language they use.
- Keep keyboard focus, labels, touch behavior, and narrow-screen layout functional when changing controls.
- Do not add comments to source code unless the user changes that preference.

## Data and safety boundaries

- Treat repository names and file contents as untrusted input. Escape data before embedding it in HTML or SVG, and use text APIs rather than interpreting file names as markup.
- Keep scans local. Do not upload file names, paths, or contents.
- Do not follow symlinks or traverse outside the requested scan root.
- Preserve the output-path check that refuses to overwrite a symbolic link.
- Report filesystem errors with the affected path; do not silently omit unreadable entries.
- Do not overwrite user files other than the explicitly selected output page.

## Development and verification

Run the relevant checks before reporting a Rust change:

```sh
cargo fmt --check
cargo test
cargo build --release
```

For CLI changes, check `--help` and `--version` and exercise the affected scan behavior using a temporary repository. For page changes, use the canonical `src/index.html`, check the generated preview against the template apart from its embedded scan data, and verify interactions at desktop and mobile widths when browser tooling is available.

## Repository hygiene

- Keep `AGENTS.md` trackable. It must not be covered by an ignore rule.
- `target/`, `Reference/`, `.agents/`, and root `*-map.html` files are local/generated material and are excluded by `.gitignore`; preserve them locally unless the user explicitly asks to delete them.
- `.gitignore` is intentionally ignored by its own pattern at the user's request. If a Git repository is initialized, it must be force-added to be included in a public checkout.
- Keep `Cargo.lock` for reproducible application builds.
- The root currently has no project `LICENSE`; do not imply that the Linguist attribution in `NOTICE` licenses CodeMap itself.
