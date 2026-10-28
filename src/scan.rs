use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Component, Path, PathBuf};
use std::sync::LazyLock;

use serde::Deserialize;
use walkdir::{DirEntry, WalkDir};

use crate::model::{LanguageRef, NodeKind, ScanReport, TreeNode};

const LANGUAGE_DATA: &str = include_str!("languages.json");
const AI_MODEL_EXTENSIONS: &[&str] = &[
    ".gguf",
    ".ggml",
    ".safetensors",
    ".pt",
    ".pth",
    ".ckpt",
    ".onnx",
    ".tflite",
    ".keras",
    ".h5",
    ".pb",
    ".model",
    ".mlx",
    ".ggs",
    ".engine",
    ".plan",
    ".weights",
    ".params",
    ".ort",
    ".nemo",
    ".ptl",
    ".pt2",
    ".mlmodel",
];
const APPLICATION_EXTENSIONS: &[&str] = &[
    ".3mf",
    ".afdesign",
    ".afphoto",
    ".ai",
    ".aseprite",
    ".blend",
    ".dwg",
    ".dxf",
    ".epub",
    ".fig",
    ".godot",
    ".kra",
    ".pbix",
    ".sketch",
    ".step",
    ".stp",
    ".tscn",
    ".tres",
    ".uasset",
    ".umap",
    ".unity",
    ".prefab",
    ".asset",
    ".res",
    ".scn",
    ".obj",
    ".pdf",
    ".doc",
    ".docx",
    ".ppt",
    ".pptx",
    ".xls",
    ".xlsx",
    ".psd",
    ".xcf",
    ".psb",
    ".afpub",
    ".indd",
    ".aep",
    ".prproj",
    ".png",
    ".jpg",
    ".jpeg",
    ".gif",
    ".bmp",
    ".tif",
    ".tiff",
    ".webp",
    ".ico",
    ".gltf",
    ".glb",
    ".fbx",
    ".stl",
    ".usd",
    ".usdz",
    ".wav",
    ".mp3",
    ".flac",
    ".m4a",
    ".ogg",
    ".aac",
    ".mp4",
    ".mov",
    ".mkv",
    ".avi",
    ".webm",
    ".wmv",
    ".zip",
    ".tar",
    ".gz",
    ".7z",
    ".rar",
    ".bz2",
    ".xz",
    ".zst",
    ".dmg",
    ".apk",
    ".exe",
    ".jar",
    ".vsdx",
    ".drawio",
    ".xmind",
    ".twbx",
];
const BINARY_EXTENSIONS: &[&str] = &[
    ".7z", ".a", ".apk", ".avi", ".bin", ".bmp", ".class", ".dmg", ".doc", ".docx", ".exe", ".fbx",
    ".flac", ".gif", ".glb", ".gz", ".ico", ".jar", ".jpeg", ".jpg", ".kra", ".lib", ".m4a",
    ".m4v", ".mkv", ".mov", ".mp3", ".mp4", ".o", ".pdf", ".png", ".ppt", ".pptx", ".psd", ".rar",
    ".so", ".stl", ".tar", ".tif", ".tiff", ".uasset", ".umap", ".wav", ".webm", ".webp", ".wma",
    ".wmv", ".xls", ".xlsx", ".xcf", ".zip",
];

static CATALOG: LazyLock<Catalog> = LazyLock::new(Catalog::load);

#[derive(Debug, Deserialize)]
struct LinguistLanguage {
    name: String,
    #[serde(rename = "type")]
    language_type: Option<String>,
    color: Option<String>,
    extensions: Vec<String>,
    filenames: Vec<String>,
}

struct Catalog {
    by_extension: HashMap<String, Vec<Classification>>,
    by_filename: HashMap<String, Vec<Classification>>,
}

#[derive(Clone)]
struct Classification {
    language: String,
    category: String,
    color: String,
}

#[derive(Default)]
struct BuilderNode {
    name: String,
    path: String,
    depth: u32,
    file: bool,
    extension: String,
    language: String,
    category: String,
    color: String,
    alternatives: Vec<String>,
    bytes: u64,
    lines: u64,
    nonblank_lines: u64,
    children: BTreeMap<String, BuilderNode>,
}

#[derive(Clone, Default)]
struct LanguageTotals {
    values: BTreeMap<String, LanguageWeight>,
}

#[derive(Clone, Default)]
struct LanguageWeight {
    name: String,
    color: String,
    files: u64,
    lines: u64,
    bytes: u64,
}

impl Catalog {
    fn load() -> Self {
        let entries: Vec<LinguistLanguage> = serde_json::from_str(LANGUAGE_DATA)
            .expect("embedded Linguist metadata must be valid JSON");
        let mut by_extension: HashMap<String, Vec<Classification>> = HashMap::new();
        let mut by_filename: HashMap<String, Vec<Classification>> = HashMap::new();
        for entry in entries {
            let classification = Classification {
                language: entry.name,
                category: category_name(entry.language_type.as_deref()),
                color: entry.color.unwrap_or_else(|| "#8b949e".to_owned()),
            };
            for extension in entry.extensions {
                by_extension
                    .entry(extension.to_lowercase())
                    .or_default()
                    .push(classification.clone());
            }
            for filename in entry.filenames {
                by_filename
                    .entry(filename.to_lowercase())
                    .or_default()
                    .push(classification.clone());
            }
        }
        for options in by_extension.values_mut().chain(by_filename.values_mut()) {
            options.sort_by(|left, right| {
                category_rank(&left.category)
                    .cmp(&category_rank(&right.category))
                    .then_with(|| left.language.cmp(&right.language))
            });
            options.dedup_by(|left, right| left.language == right.language);
        }
        Self {
            by_extension,
            by_filename,
        }
    }

    fn classify(&self, filename: &str) -> (Classification, String, Vec<String>) {
        let lower = filename.to_lowercase();
        if let Some(result) = special_format(&lower) {
            let extension = extension_suffix(&lower).unwrap_or_default();
            return (result, extension, Vec::new());
        }
        if let Some(options) = self.by_filename.get(&lower) {
            let primary = primary(
                options,
                extension_suffix(&lower).as_deref().unwrap_or_default(),
            );
            return (
                primary.clone(),
                extension_suffix(&lower).unwrap_or_default(),
                alternatives(options, &primary.language),
            );
        }
        for (index, character) in lower.char_indices() {
            if character == '.' {
                let suffix = &lower[index..];
                if let Some(options) = self.by_extension.get(suffix) {
                    let primary = primary(options, suffix);
                    return (
                        primary.clone(),
                        suffix.to_owned(),
                        alternatives(options, &primary.language),
                    );
                }
            }
        }
        let extension = extension_suffix(&lower).unwrap_or_default();
        let name = if extension.is_empty() {
            "Unknown".to_owned()
        } else {
            format!("{} file", extension.trim_start_matches('.').to_uppercase())
        };
        (
            Classification {
                language: name,
                category: "Unknown".to_owned(),
                color: "#8b949e".to_owned(),
            },
            extension,
            Vec::new(),
        )
    }
}

pub fn scan_repository(
    root: &Path,
    max_depth: Option<usize>,
    output_path: Option<&Path>,
    ignored_paths: &[PathBuf],
) -> Result<ScanReport, Box<dyn Error>> {
    let root = root.canonicalize()?;
    let root_path = root.display().to_string();
    let mut root_node = BuilderNode {
        name: root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| root.display().to_string()),
        ..BuilderNode::default()
    };
    let output_path = output_path.map(Path::to_path_buf);
    let mut skipped_symlinks = 0;
    let mut walker = WalkDir::new(&root).follow_links(false).sort_by_file_name();
    if let Some(depth) = max_depth {
        walker = walker.max_depth(depth);
    }
    let iterator = walker.into_iter().filter_entry(|entry| {
        entry.depth() == 0
            || (entry.file_name() != ".git" && !is_ignored(entry.path(), &root, ignored_paths))
    });
    for result in iterator {
        let entry = result.map_err(|error| {
            let location = error
                .path()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| root.display().to_string());
            format!("could not scan {location}: {error}")
        })?;
        if entry.depth() == 0 {
            continue;
        }
        if output_path
            .as_deref()
            .is_some_and(|path| same_path(entry.path(), path))
        {
            continue;
        }
        if entry.file_type().is_symlink() {
            skipped_symlinks += 1;
            continue;
        }
        if !entry.file_type().is_dir() && !entry.file_type().is_file() {
            continue;
        }
        insert_entry(&mut root_node, &root, &entry)?;
    }
    let (root, _) = materialize(root_node, max_depth);
    Ok(ScanReport {
        root_path,
        scan_depth: max_depth,
        skipped_symlinks,
        root,
    })
}

fn is_ignored(path: &Path, root: &Path, ignored_paths: &[PathBuf]) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    ignored_paths
        .iter()
        .any(|ignored| relative == ignored || relative.starts_with(ignored))
}

fn insert_entry(
    root_node: &mut BuilderNode,
    root: &Path,
    entry: &DirEntry,
) -> Result<(), Box<dyn Error>> {
    let relative = entry.path().strip_prefix(root)?;
    let parts: Vec<String> = relative
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    if parts.is_empty() {
        return Ok(());
    }
    let is_file = entry.file_type().is_file();
    let mut path = String::new();
    let mut current = root_node;
    for (index, name) in parts.iter().enumerate() {
        if !path.is_empty() {
            path.push('/');
        }
        path.push_str(name);
        current = current
            .children
            .entry(name.clone())
            .or_insert_with(|| BuilderNode {
                name: name.clone(),
                path: path.clone(),
                depth: u32::try_from(index + 1).unwrap_or(u32::MAX),
                ..BuilderNode::default()
            });
    }
    if is_file {
        let metadata = entry
            .metadata()
            .map_err(|error| format!("could not inspect {}: {error}", entry.path().display()))?;
        let (classification, extension, alternatives) =
            CATALOG.classify(parts.last().map(String::as_str).unwrap_or_default());
        let file = File::open(entry.path())
            .map_err(|error| format!("could not read {}: {error}", entry.path().display()))?;
        let mut reader = BufReader::with_capacity(65_536, file);
        let sample = reader
            .fill_buf()
            .map_err(|error| format!("could not read {}: {error}", entry.path().display()))?
            .iter()
            .take(8192)
            .copied()
            .collect::<Vec<_>>();
        let text_file = !is_binary_extension(&extension) && looks_like_text(&sample);
        let (lines, nonblank_lines) = if text_file {
            count_lines(&mut reader)
                .map_err(|error| format!("could not read {}: {error}", entry.path().display()))?
        } else {
            (0, 0)
        };
        current.file = true;
        current.extension = extension;
        current.language = classification.language;
        current.category = classification.category;
        current.color = classification.color;
        current.alternatives = alternatives;
        current.bytes = metadata.len();
        current.lines = lines;
        current.nonblank_lines = nonblank_lines;
    }
    Ok(())
}

fn materialize(node: BuilderNode, scan_depth: Option<usize>) -> (TreeNode, LanguageTotals) {
    if node.file {
        let mut totals = LanguageTotals::default();
        totals.values.insert(
            node.language.clone(),
            LanguageWeight {
                name: node.language.clone(),
                color: node.color.clone(),
                files: 1,
                lines: node.lines,
                bytes: node.bytes,
            },
        );
        let reference = Some(LanguageRef {
            name: node.language.clone(),
            color: node.color.clone(),
        });
        return (
            TreeNode {
                name: node.name,
                path: node.path,
                kind: NodeKind::File,
                extension: node.extension,
                language: node.language,
                category: node.category,
                color: node.color,
                alternatives: node.alternatives,
                bytes: node.bytes,
                lines: node.lines,
                nonblank_lines: node.nonblank_lines,
                files: 1,
                folders: 0,
                direct_files: 0,
                direct_folders: 0,
                depth: node.depth,
                nested_levels: 0,
                truncated: false,
                dominant_by_files: reference.clone(),
                dominant_by_lines: reference.clone(),
                dominant_by_bytes: reference,
                children: Vec::new(),
            },
            totals,
        );
    }
    let mut children = Vec::with_capacity(node.children.len());
    let mut bytes = 0;
    let mut lines = 0;
    let mut nonblank_lines = 0;
    let mut files = 0;
    let mut folders = 0;
    let mut direct_files = 0;
    let mut direct_folders = 0;
    let mut deepest_level = node.depth;
    let mut totals = LanguageTotals::default();
    for child in node.children.into_values() {
        let (child, child_totals) = materialize(child, scan_depth);
        bytes += child.bytes;
        lines += child.lines;
        nonblank_lines += child.nonblank_lines;
        files += child.files;
        folders += child.folders;
        deepest_level = deepest_level.max(child.depth.saturating_add(child.nested_levels));
        if child.kind == NodeKind::File {
            direct_files += 1;
        } else {
            direct_folders += 1;
            folders += 1;
        }
        merge_totals(&mut totals, child_totals);
        children.push(child);
    }
    let nested_levels = deepest_level.saturating_sub(node.depth);
    let truncated = scan_depth.is_some_and(|limit| node.depth as usize >= limit);
    (
        TreeNode {
            name: node.name,
            path: node.path,
            kind: NodeKind::Directory,
            extension: String::new(),
            language: "Folder".to_owned(),
            category: "Folder".to_owned(),
            color: "#8b949e".to_owned(),
            alternatives: Vec::new(),
            bytes,
            lines,
            nonblank_lines,
            files,
            folders,
            direct_files,
            direct_folders,
            depth: node.depth,
            nested_levels,
            truncated,
            dominant_by_files: dominant(&totals, |value| value.files),
            dominant_by_lines: dominant(&totals, |value| value.lines),
            dominant_by_bytes: dominant(&totals, |value| value.bytes),
            children,
        },
        totals,
    )
}

fn merge_totals(destination: &mut LanguageTotals, source: LanguageTotals) {
    for (language, value) in source.values {
        let total = destination.values.entry(language).or_default();
        if total.name.is_empty() {
            total.name = value.name;
            total.color = value.color;
        }
        total.files += value.files;
        total.lines += value.lines;
        total.bytes += value.bytes;
    }
}

fn dominant(
    totals: &LanguageTotals,
    metric: impl Fn(&LanguageWeight) -> u64,
) -> Option<LanguageRef> {
    totals
        .values
        .values()
        .max_by(|left, right| {
            metric(left)
                .cmp(&metric(right))
                .then_with(|| left.name.cmp(&right.name))
        })
        .map(|value| LanguageRef {
            name: value.name.clone(),
            color: value.color.clone(),
        })
}

fn primary(options: &[Classification], extension: &str) -> Classification {
    let preferred = match extension {
        ".rs" => "Rust",
        ".h" => "C",
        ".m" => "Objective-C",
        ".pl" => "Perl",
        ".ts" => "TypeScript",
        ".js" => "JavaScript",
        ".md" => "Markdown",
        ".sh" => "Shell",
        ".yml" | ".yaml" => "YAML",
        ".json" => "JSON",
        _ => "",
    };
    options
        .iter()
        .find(|value| value.language == preferred)
        .unwrap_or(&options[0])
        .clone()
}

fn alternatives(options: &[Classification], primary: &str) -> Vec<String> {
    options
        .iter()
        .filter(|value| value.language != primary)
        .map(|value| value.language.clone())
        .collect()
}

fn category_name(language_type: Option<&str>) -> String {
    match language_type {
        Some("programming") => "Programming".to_owned(),
        Some("data") => "Data".to_owned(),
        Some("markup") => "Markup".to_owned(),
        Some("prose") => "Prose".to_owned(),
        Some(value) => value
            .chars()
            .enumerate()
            .map(|(index, character)| {
                if index == 0 {
                    character.to_ascii_uppercase()
                } else {
                    character
                }
            })
            .collect(),
        None => "Other".to_owned(),
    }
}

fn category_rank(category: &str) -> u8 {
    match category {
        "Programming" => 0,
        "Markup" => 1,
        "Data" => 2,
        "Prose" => 3,
        "Other" => 4,
        _ => 5,
    }
}

fn extension_suffix(filename: &str) -> Option<String> {
    filename
        .char_indices()
        .rev()
        .find(|(index, character)| *character == '.' && *index > 0)
        .map(|(index, _)| filename[index..].to_owned())
        .or_else(|| (filename.starts_with('.') && filename.len() > 1).then(|| filename.to_owned()))
}

fn special_format(filename: &str) -> Option<Classification> {
    let extension = extension_suffix(filename)?;
    let model_binary = extension == ".bin"
        && ["pytorch_model", "model", "weights", "checkpoint"]
            .iter()
            .any(|marker| filename.contains(marker));
    let entry = if AI_MODEL_EXTENSIONS.contains(&extension.as_str()) || model_binary {
        ("AI model", "AI Model", "#d2a8ff")
    } else if APPLICATION_EXTENSIONS.contains(&extension.as_str()) {
        ("Application file", "Application", "#79c0ff")
    } else {
        return None;
    };
    Some(Classification {
        language: format!(
            "{} ({})",
            entry.0,
            extension.trim_start_matches('.').to_uppercase()
        ),
        category: entry.1.to_owned(),
        color: entry.2.to_owned(),
    })
}

fn is_binary_extension(extension: &str) -> bool {
    BINARY_EXTENSIONS.contains(&extension)
        || AI_MODEL_EXTENSIONS.contains(&extension)
        || matches!(
            extension,
            ".3mf"
                | ".afdesign"
                | ".afphoto"
                | ".ai"
                | ".aseprite"
                | ".blend"
                | ".dwg"
                | ".dxf"
                | ".fig"
                | ".sketch"
                | ".step"
                | ".stp"
                | ".aep"
                | ".afpub"
                | ".indd"
                | ".prproj"
                | ".psb"
                | ".usdz"
                | ".twbx"
        )
}

fn looks_like_text(sample: &[u8]) -> bool {
    if sample.iter().any(|byte| {
        *byte == 0
            || (*byte < 0x20 && !matches!(*byte, b'\t' | b'\n' | b'\r' | 0x0b | 0x0c))
            || *byte == 0x7f
    }) {
        return false;
    }
    match std::str::from_utf8(sample) {
        Ok(_) => true,
        Err(error) if error.error_len().is_none() => {
            std::str::from_utf8(&sample[..error.valid_up_to()]).is_ok()
        }
        Err(_) => false,
    }
}

fn count_lines(reader: &mut impl Read) -> Result<(u64, u64), std::io::Error> {
    let mut buffer = [0_u8; 65_536];
    let mut lines = 0;
    let mut nonblank_lines = 0;
    let mut line_has_bytes = false;
    let mut line_is_nonblank = false;
    let mut sequence = [0_u8; 4];
    let mut sequence_len = 0;
    let mut sequence_width = 0;
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        for byte in buffer.iter().take(count).copied() {
            if byte == b'\n' {
                if sequence_len > 0 {
                    line_is_nonblank = true;
                    sequence_len = 0;
                }
                lines += 1;
                nonblank_lines += u64::from(line_is_nonblank);
                line_has_bytes = false;
                line_is_nonblank = false;
            } else {
                line_has_bytes = true;
                consume_character(
                    byte,
                    &mut sequence,
                    &mut sequence_len,
                    &mut sequence_width,
                    &mut line_is_nonblank,
                );
            }
        }
    }
    if sequence_len > 0 {
        line_is_nonblank = true;
    }
    if line_has_bytes {
        lines += 1;
        nonblank_lines += u64::from(line_is_nonblank);
    }
    Ok((lines, nonblank_lines))
}

fn consume_character(
    byte: u8,
    sequence: &mut [u8; 4],
    sequence_len: &mut usize,
    sequence_width: &mut usize,
    line_is_nonblank: &mut bool,
) {
    if *sequence_len == 0 {
        if byte.is_ascii() {
            *line_is_nonblank |= !(byte as char).is_whitespace();
            return;
        }
        *sequence_width = match byte {
            0xc2..=0xdf => 2,
            0xe0..=0xef => 3,
            0xf0..=0xf4 => 4,
            _ => {
                *line_is_nonblank = true;
                return;
            }
        };
        sequence[0] = byte;
        *sequence_len = 1;
        return;
    }
    if (0x80..=0xbf).contains(&byte) {
        sequence[*sequence_len] = byte;
        *sequence_len += 1;
        if *sequence_len == *sequence_width {
            match std::str::from_utf8(&sequence[..*sequence_len]) {
                Ok(value) => {
                    if let Some(character) = value.chars().next() {
                        *line_is_nonblank |= !character.is_whitespace();
                    }
                }
                Err(_) => *line_is_nonblank = true,
            }
            *sequence_len = 0;
        }
        return;
    }
    *line_is_nonblank = true;
    *sequence_len = 0;
    consume_character(
        byte,
        sequence,
        sequence_len,
        sequence_width,
        line_is_nonblank,
    );
}

fn same_path(left: &Path, right: &Path) -> bool {
    left == right
}

#[cfg(test)]
mod tests {
    use super::{CATALOG, count_lines, looks_like_text, scan_repository};
    use std::fs;
    use std::io::Cursor;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture() -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("codemap-{}-{stamp}", std::process::id()));
        fs::create_dir_all(path.join("nested/deeper")).unwrap();
        fs::create_dir_all(path.join(".git/objects")).unwrap();
        fs::write(path.join("main.rs"), "fn main() {}\n\n").unwrap();
        fs::write(
            path.join("nested/deeper/weights.safetensors"),
            [0_u8, 1, 2, 3],
        )
        .unwrap();
        fs::write(path.join("nested/deeper/notes.unknown"), "one\ntwo").unwrap();
        fs::write(path.join(".git/objects/ignored.rs"), "ignored").unwrap();
        path
    }

    fn remove_fixture(path: &PathBuf) {
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn counts_physical_and_nonblank_lines_across_unicode_whitespace() {
        let contents = "alpha\n  \n　\nβ";
        assert_eq!(
            count_lines(&mut Cursor::new(contents.as_bytes())).unwrap(),
            (4, 2)
        );
    }

    #[test]
    fn counts_final_line_without_newline_and_empty_input() {
        assert_eq!(
            count_lines(&mut Cursor::new(b"x" as &[u8])).unwrap(),
            (1, 1)
        );
        assert_eq!(count_lines(&mut Cursor::new(b"" as &[u8])).unwrap(), (0, 0));
        assert_eq!(
            count_lines(&mut Cursor::new(b"\n\n" as &[u8])).unwrap(),
            (2, 0)
        );
    }

    #[test]
    fn classifies_linguist_and_special_formats() {
        let (rust, extension, alternatives) = CATALOG.classify("main.rs");
        assert_eq!(rust.language, "Rust");
        assert_eq!(extension, ".rs");
        assert!(alternatives.contains(&"RenderScript".to_owned()));
        let (model, _, _) = CATALOG.classify("weights.safetensors");
        assert_eq!(model.category, "AI Model");
        let (unknown, _, _) = CATALOG.classify("opaque.custom");
        assert_eq!(unknown.category, "Unknown");
        let (application, _, _) = CATALOG.classify("design.psd");
        assert_eq!(application.category, "Application");
        let (model_binary, _, _) = CATALOG.classify("pytorch_model.bin");
        assert_eq!(model_binary.category, "AI Model");
    }

    #[test]
    fn distinguishes_text_from_binary_samples() {
        assert!(looks_like_text("hello\n世界".as_bytes()));
        assert!(!looks_like_text(&[0, 1, 2, 3]));
    }

    #[test]
    fn scans_to_the_requested_depth_and_marks_the_boundary() {
        let path = fixture();
        let output = path.join("map.html");
        fs::write(&output, "previous output").unwrap();
        let report = scan_repository(&path, Some(1), Some(&output), &[]).unwrap();
        assert_eq!(report.root.files, 1);
        assert_eq!(report.root.folders, 1);
        assert_eq!(report.root.lines, 2);
        assert!(
            report
                .root
                .children
                .iter()
                .any(|node| node.name == "nested" && node.truncated)
        );
        remove_fixture(&path);
    }

    #[test]
    fn scans_unknown_files_and_excludes_git_metadata_and_output() {
        let path = fixture();
        let output = path.join("map.html");
        fs::write(&output, "previous output").unwrap();
        let report = scan_repository(&path, None, Some(&output), &[]).unwrap();
        assert_eq!(report.root.files, 3);
        assert_eq!(report.root.folders, 2);
        assert_eq!(report.root.lines, 4);
        let nested = report
            .root
            .children
            .iter()
            .find(|node| node.name == "nested")
            .unwrap();
        let deeper = nested.children.first().unwrap();
        assert!(
            deeper
                .children
                .iter()
                .any(|node| node.name == "notes.unknown" && node.category == "Unknown")
        );
        assert!(
            deeper
                .children
                .iter()
                .any(|node| node.name == "weights.safetensors" && node.category == "AI Model")
        );
        remove_fixture(&path);
    }

    #[test]
    fn ignores_exact_files_and_prunes_ignored_folders() {
        let path = fixture();
        fs::create_dir_all(path.join("nested-copy")).unwrap();
        fs::write(path.join("nested-copy/keep.txt"), "keep").unwrap();
        let ignored = [PathBuf::from("nested"), PathBuf::from("main.rs")];
        let report = scan_repository(&path, None, None, &ignored).unwrap();
        assert_eq!(report.root.files, 1);
        assert_eq!(report.root.folders, 1);
        assert!(
            report
                .root
                .children
                .iter()
                .any(|node| node.name == "nested-copy")
        );
        assert!(
            !report
                .root
                .children
                .iter()
                .any(|node| node.name == "nested")
        );
        assert!(
            !report
                .root
                .children
                .iter()
                .any(|node| node.name == "main.rs")
        );
        remove_fixture(&path);
    }

    #[cfg(unix)]
    #[test]
    fn skips_symbolic_links_without_following_them() {
        let path = fixture();
        let target = path.join("external");
        fs::write(&target, "external").unwrap();
        std::os::unix::fs::symlink(&target, path.join("linked.txt")).unwrap();
        let report = scan_repository(&path, None, None, &[]).unwrap();
        assert_eq!(report.skipped_symlinks, 1);
        assert_eq!(report.root.files, 4);
        remove_fixture(&path);
    }
}
