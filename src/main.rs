mod model;
mod scan;

use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use scan::scan_repository;

const PAGE: &str = include_str!("index.html");

struct Options {
    root: PathBuf,
    output: Option<PathBuf>,
    depth: Option<usize>,
    ignored_paths: Vec<PathBuf>,
    open: bool,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("codemap: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let Some(options) = parse_args()? else {
        return Ok(());
    };
    let root = fs::canonicalize(&options.root)?;
    if !fs::metadata(&root)?.is_dir() {
        return Err(format!("{} is not a directory", root.display()).into());
    }
    let output = resolve_output(options.output.as_deref(), &root)?;
    let ignored_paths = resolve_ignored_paths(&options.ignored_paths, &root)?;
    let report = scan_repository(&root, options.depth, Some(&output), &ignored_paths)?;
    let payload = serde_json::to_string(&report)?
        .replace('&', "\\u0026")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029");
    let html = PAGE.replace("__CODEMAP_DATA__", &payload);
    write_output(&output, html.as_bytes())?;
    println!(
        "Scanned {} files and {} folders from {}",
        report.root.files,
        report.root.folders,
        root.display()
    );
    println!("Map written to {}", output.display());
    if options.open
        && let Err(error) = open_in_browser(&output)
    {
        eprintln!("Could not open the map in a browser: {error}");
        eprintln!("Open it manually: {}", output.display());
    }
    Ok(())
}

fn parse_args() -> Result<Option<Options>, Box<dyn Error>> {
    let mut args = env::args_os().skip(1).peekable();
    let mut root = None;
    let mut output = None;
    let mut depth = None;
    let mut ignored_paths = Vec::new();
    let mut open = true;
    while let Some(argument) = args.next() {
        if argument == OsStr::new("--help") || argument == OsStr::new("-h") {
            print_help();
            return Ok(None);
        }
        if argument == OsStr::new("--version") || argument == OsStr::new("-V") {
            println!("codemap {}", env!("CARGO_PKG_VERSION"));
            return Ok(None);
        }
        if argument == OsStr::new("--depth") {
            let value = args.next().ok_or("--depth requires a positive integer")?;
            let parsed = value.to_string_lossy().parse::<usize>()?;
            if parsed == 0 {
                return Err("--depth must be at least 1".into());
            }
            depth = Some(parsed);
            continue;
        }
        if argument == OsStr::new("--output") || argument == OsStr::new("-o") {
            output = Some(PathBuf::from(
                args.next().ok_or("--output requires a path")?,
            ));
            continue;
        }
        if argument == OsStr::new("--ignore") {
            let mut count = 0;
            while let Some(value) = args.next_if(|value| !value.to_string_lossy().starts_with('-'))
            {
                ignored_paths.push(PathBuf::from(value));
                count += 1;
            }
            if count == 0 {
                return Err("--ignore requires at least one path".into());
            }
            continue;
        }
        if argument == OsStr::new("--no-open") {
            open = false;
            continue;
        }
        if argument.to_string_lossy().starts_with('-') {
            return Err(format!("unknown option: {}", argument.to_string_lossy()).into());
        }
        if root.replace(PathBuf::from(argument)).is_some() {
            return Err("only one repository path can be scanned".into());
        }
    }
    Ok(Some(Options {
        root: root.unwrap_or_else(|| PathBuf::from(".")),
        output,
        depth,
        ignored_paths,
        open,
    }))
}

fn resolve_ignored_paths(paths: &[PathBuf], root: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let current_dir = env::current_dir()?.canonicalize()?;
    paths
        .iter()
        .map(|path| {
            if path.as_os_str().is_empty() {
                return Err("--ignore requires a non-empty path".into());
            }
            let absolute = if path.is_absolute() {
                normalize_absolute_path(path)?
            } else {
                let from_current_dir = normalize_absolute_path(&current_dir.join(path))?;
                if from_current_dir == root {
                    return Err("--ignore cannot target the repository root".into());
                }
                if from_current_dir.starts_with(root) {
                    from_current_dir
                } else {
                    normalize_absolute_path(&root.join(path))?
                }
            };
            let relative = absolute.strip_prefix(root).map_err(|_| {
                format!("ignore path is outside the repository: {}", path.display())
            })?;
            if relative.as_os_str().is_empty() {
                return Err("--ignore cannot target the repository root".into());
            }
            Ok(relative.to_path_buf())
        })
        .collect()
}

fn normalize_absolute_path(path: &Path) -> Result<PathBuf, Box<dyn Error>> {
    if !path.is_absolute() {
        return Err(format!("ignore path is not absolute: {}", path.display()).into());
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err(format!(
                        "ignore path escapes its filesystem root: {}",
                        path.display()
                    )
                    .into());
                }
            }
            Component::Normal(name) => normalized.push(name),
        }
    }
    Ok(normalized)
}

fn resolve_output(output: Option<&Path>, root: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = match output {
        Some(path) => path.to_path_buf(),
        None => {
            let base = root
                .file_name()
                .and_then(OsStr::to_str)
                .filter(|name| !name.is_empty())
                .unwrap_or("repository");
            PathBuf::from(format!("{}-map.html", safe_file_name(base)))
        }
    };
    let path = if path.is_absolute() {
        path
    } else {
        env::current_dir()?.join(path)
    };
    if let Ok(metadata) = fs::symlink_metadata(&path) {
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "output path must not be a symbolic link: {}",
                path.display()
            )
            .into());
        }
        if !metadata.is_file() {
            return Err(format!("output path is not a file: {}", path.display()).into());
        }
    }
    let parent = path.parent().ok_or("output path has no parent directory")?;
    if !parent.is_dir() {
        return Err(format!("output directory does not exist: {}", parent.display()).into());
    }
    Ok(parent
        .canonicalize()?
        .join(path.file_name().ok_or("output path has no file name")?))
}

fn safe_file_name(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn write_output(path: &Path, contents: &[u8]) -> Result<(), Box<dyn Error>> {
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)?;
    file.write_all(contents)?;
    file.flush()?;
    Ok(())
}

fn open_in_browser(path: &Path) -> Result<(), Box<dyn Error>> {
    let path = fs::canonicalize(path)?;
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("rundll32");
        command.arg("url.dll,FileProtocolHandler").arg(path);
        command
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(path);
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(path);
        command
    };
    let result = command.output()?;
    if result.status.success() {
        Ok(())
    } else {
        Err(format!("the system browser opener exited with {}", result.status).into())
    }
}

fn print_help() {
    println!(
        "CodeMap: offline repository treemap\n\nUsage: codemap [PATH] [OPTIONS]\n\nOptions:\n  --depth N          Scan at most N directory levels\n  --ignore PATH...   Exclude exact file or folder paths\n  -o, --output PATH  Write the standalone HTML page to this path\n  --no-open          Do not open the generated page in a browser\n  -V, --version      Print the version\n  -h, --help         Print this help\n\nPlace PATH before --ignore when specifying a repository. Ignore paths are relative to the current directory when inside the repository; otherwise they are relative to the repository root."
    );
}

#[cfg(test)]
mod tests {
    use super::{resolve_ignored_paths, resolve_output};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn resolves_ignore_paths_from_working_directory_or_scan_root() {
        let current_dir = std::env::current_dir().unwrap().canonicalize().unwrap();
        let root = current_dir.join("codemap-ignore-root");
        let parent_relative = PathBuf::from(root.file_name().unwrap()).join("target");
        let root_relative = PathBuf::from("target");
        let resolved = resolve_ignored_paths(&[parent_relative, root_relative], &root).unwrap();
        assert_eq!(
            resolved,
            vec![PathBuf::from("target"), PathBuf::from("target")]
        );
    }

    #[test]
    fn rejects_ignore_paths_outside_or_at_the_repository_root() {
        let current_dir = std::env::current_dir().unwrap().canonicalize().unwrap();
        let root = current_dir.join("codemap-ignore-root");
        let outside = current_dir.join("outside-ignore");
        assert!(resolve_ignored_paths(&[outside], &root).is_err());
        assert!(resolve_ignored_paths(&[PathBuf::new()], &root).is_err());
        assert!(resolve_ignored_paths(&[PathBuf::from("codemap-ignore-root")], &root).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn refuses_to_overwrite_a_symbolic_link_output() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("codemap-output-{}-{stamp}", std::process::id()));
        let target = root.with_extension("target");
        fs::create_dir_all(&root).unwrap();
        fs::write(&target, "preserve").unwrap();
        let link = root.join("map.html");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert!(resolve_output(Some(&link), &root).is_err());
        assert_eq!(fs::read_to_string(&target).unwrap(), "preserve");
        fs::remove_file(&link).unwrap();
        fs::remove_file(&target).unwrap();
        fs::remove_dir_all(root).unwrap();
    }
}
