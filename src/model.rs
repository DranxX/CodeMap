use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct LanguageRef {
    pub name: String,
    pub color: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct TreeNode {
    pub name: String,
    pub path: String,
    pub kind: NodeKind,
    pub extension: String,
    pub language: String,
    pub category: String,
    pub color: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub alternatives: Vec<String>,
    pub bytes: u64,
    pub lines: u64,
    pub nonblank_lines: u64,
    pub files: u64,
    pub folders: u64,
    pub direct_files: u64,
    pub direct_folders: u64,
    pub depth: u32,
    pub nested_levels: u32,
    pub truncated: bool,
    pub dominant_by_files: Option<LanguageRef>,
    pub dominant_by_lines: Option<LanguageRef>,
    pub dominant_by_bytes: Option<LanguageRef>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<TreeNode>,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    Directory,
    File,
}

#[derive(Debug, Serialize)]
pub struct ScanReport {
    pub root_path: String,
    pub scan_depth: Option<usize>,
    pub skipped_symlinks: u64,
    pub root: TreeNode,
}
