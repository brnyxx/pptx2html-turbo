use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use clap::Parser;
use log::info;

use pptx2html_core::model::{ConversionDiagnostic, SupportTier};
use pptx2html_core::{ConversionOptions, ExternalAsset};

/// PPTX to HTML converter — preserves original layout
#[derive(Parser)]
#[command(name = "pptx2html", version, about)]
struct Cli {
    /// Input PPTX file path
    input: PathBuf,

    /// Output HTML file path (default: input filename.html)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Slide selection (e.g. "1,3,5-8")
    #[arg(long)]
    slides: Option<String>,

    /// Output format: single HTML file or per-slide files
    #[arg(long, value_parser = ["single", "multi"], default_value = "single")]
    format: String,

    /// Do not embed images — extract to images/ directory
    #[arg(long)]
    no_embed: bool,

    /// Print presentation metadata as JSON and exit
    #[arg(long)]
    info: bool,

    /// Include hidden slides
    #[arg(long)]
    include_hidden: bool,

    /// Whole-slide zoom factor (e.g. 2.0 = 2x). Keeps coordinates and ratios unchanged.
    #[arg(long, default_value_t = 1.0)]
    scale: f64,

    /// Write the canonical conversion diagnostics JSON array to this path
    #[arg(long, visible_alias = "diagnostics-json", value_name = "PATH")]
    diagnostics: Option<PathBuf>,

    /// Exit 2 after writing outputs when fallback diagnostics are present
    #[arg(long)]
    fail_on_fallback: bool,
}

/// Parse a slide selection string like "1,3,5-8" into a sorted list of 1-based indices
fn parse_slide_selection(s: &str) -> Result<Vec<usize>, String> {
    let mut indices = Vec::new();
    for part in s.split(',') {
        let part = part.trim();
        if part.contains('-') {
            let (start_raw, end_raw) = part
                .split_once('-')
                .expect("range parsing is guarded by contains('-')");
            let start: usize = start_raw
                .trim()
                .parse()
                .map_err(|_| format!("invalid number in range: {part}"))?;
            let end: usize = end_raw
                .trim()
                .parse()
                .map_err(|_| format!("invalid number in range: {part}"))?;
            if start > end {
                return Err(format!("invalid range {start}-{end}: start > end"));
            }
            for i in start..=end {
                indices.push(i);
            }
        } else {
            let idx: usize = part
                .parse()
                .map_err(|_| format!("invalid slide number: {part}"))?;
            indices.push(idx);
        }
    }
    indices.sort_unstable();
    indices.dedup();
    Ok(indices)
}

fn write_external_assets(
    base_dir: &std::path::Path,
    assets: &[ExternalAsset],
) -> Result<(), String> {
    for asset in assets {
        let path = base_dir.join(&asset.relative_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                format!("failed to create asset directory {}: {e}", parent.display())
            })?;
        }
        std::fs::write(&path, &asset.data)
            .map_err(|e| format!("failed to write asset {}: {e}", path.display()))?;
    }
    Ok(())
}

fn report_diagnostics(diagnostics: &[ConversionDiagnostic]) -> bool {
    if diagnostics.is_empty() {
        return false;
    }
    let mut counts = BTreeMap::<&str, usize>::new();
    let mut has_fallback = false;
    for diagnostic in diagnostics {
        *counts.entry(&diagnostic.code).or_default() += 1;
        has_fallback |= matches!(
            diagnostic.support_tier,
            SupportTier::Fallback | SupportTier::Unparsed
        );
    }
    let summary = counts
        .into_iter()
        .map(|(code, count)| format!("{code}={count}"))
        .collect::<Vec<_>>()
        .join(", ");
    eprintln!("Conversion diagnostics ({}): {summary}", diagnostics.len());
    has_fallback
}

fn stable_resolved_path(path: &Path) -> Result<PathBuf, String> {
    stable_resolved_path_inner(path, 0)
}

fn stable_resolved_path_inner(path: &Path, symlink_depth: usize) -> Result<PathBuf, String> {
    const MAX_SYMLINK_DEPTH: usize = 40;
    if symlink_depth > MAX_SYMLINK_DEPTH {
        return Err(format!(
            "failed to resolve path {}: too many symbolic links",
            path.display()
        ));
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| format!("failed to resolve path {}: {error}", path.display()))?
            .join(path)
    };

    let mut ancestor = absolute.as_path();
    loop {
        match std::fs::canonicalize(ancestor) {
            Ok(resolved_ancestor) => {
                let suffix = absolute.strip_prefix(ancestor).map_err(|error| {
                    format!("failed to resolve path {}: {error}", path.display())
                })?;
                return Ok(normalize_path(&resolved_ancestor.join(suffix)));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if std::fs::symlink_metadata(ancestor).is_ok_and(|metadata| metadata.is_symlink()) {
                    let target = std::fs::read_link(ancestor).map_err(|error| {
                        format!("failed to resolve symlink {}: {error}", ancestor.display())
                    })?;
                    let target = if target.is_absolute() {
                        target
                    } else {
                        ancestor.parent().unwrap_or(Path::new("/")).join(target)
                    };
                    let resolved_target = stable_resolved_path_inner(&target, symlink_depth + 1)?;
                    let suffix = absolute.strip_prefix(ancestor).map_err(|error| {
                        format!("failed to resolve path {}: {error}", path.display())
                    })?;
                    return Ok(normalize_path(&resolved_target.join(suffix)));
                }
                ancestor = ancestor.parent().ok_or_else(|| {
                    format!(
                        "failed to resolve path {}: no existing ancestor",
                        path.display()
                    )
                })?;
            }
            Err(error) => {
                return Err(format!(
                    "failed to resolve path {}: {error}",
                    path.display()
                ));
            }
        }
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    use std::path::Component;

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }
    normalized
}

fn same_file_identity(left: &Path, right: &Path) -> Result<bool, String> {
    match same_file::is_same_file(left, right) {
        Ok(is_same) => Ok(is_same),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!(
            "failed to compare file identity for {} and {}: {error}",
            left.display(),
            right.display()
        )),
    }
}

fn reject_sidecar_collision(
    sidecar: Option<&Path>,
    protected_paths: &[(PathBuf, &'static str)],
) -> Result<(), String> {
    let Some(sidecar) = sidecar else {
        return Ok(());
    };
    let resolved_sidecar = stable_resolved_path(sidecar)?;
    for (protected_path, kind) in protected_paths {
        let resolved_protected = stable_resolved_path(protected_path)?;
        if resolved_sidecar == resolved_protected || same_file_identity(sidecar, protected_path)? {
            return Err(format!(
                "diagnostics path {} has the same resolved path as {kind} {}",
                sidecar.display(),
                protected_path.display()
            ));
        }
    }
    Ok(())
}

fn write_diagnostics_sidecar(path: Option<&Path>, json: &str) -> Result<(), String> {
    let Some(path) = path else {
        return Ok(());
    };
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create diagnostics directory {}: {error}",
                parent.display()
            )
        })?;
    }
    std::fs::write(path, json)
        .map_err(|error| format!("failed to write diagnostics {}: {error}", path.display()))
}

fn finish_diagnostics(
    diagnostics: &[ConversionDiagnostic],
    diagnostics_json: &str,
    sidecar: Option<&Path>,
    fail_on_fallback: bool,
) -> Result<bool, String> {
    write_diagnostics_sidecar(sidecar, diagnostics_json)?;
    let has_fallback = report_diagnostics(diagnostics);
    Ok(fail_on_fallback && has_fallback)
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();

    if let Err(error) = reject_sidecar_collision(
        cli.diagnostics.as_deref(),
        &[(cli.input.clone(), "input PPTX")],
    ) {
        eprintln!("Invalid diagnostics path: {error}");
        std::process::exit(1);
    }

    // --info: print metadata and exit
    if cli.info {
        match pptx2html_core::get_info(&cli.input) {
            Ok(info) => {
                let title = match &info.title {
                    Some(t) => format!("\"{}\"", t.replace('\\', "\\\\").replace('"', "\\\"")),
                    None => "null".to_string(),
                };
                println!(
                    r#"{{"slide_count":{},"width_px":{:.1},"height_px":{:.1},"title":{}}}"#,
                    info.slide_count, info.width_px, info.height_px, title
                );
            }
            Err(e) => {
                eprintln!("Failed to read presentation: {e}");
                std::process::exit(1);
            }
        }
        return;
    }

    // Build conversion options
    let slide_indices = if let Some(ref sel) = cli.slides {
        match parse_slide_selection(sel) {
            Ok(indices) => Some(indices),
            Err(e) => {
                eprintln!("Invalid --slides value: {e}");
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    let opts = ConversionOptions {
        embed_images: !cli.no_embed,
        include_hidden: cli.include_hidden,
        slide_range: None,
        slide_indices: slide_indices.clone(),
        scale: cli.scale,
    };

    if cli.format == "multi" {
        // Multi-file output: one HTML per slide
        let output_dir = cli
            .output
            .clone()
            .unwrap_or_else(|| cli.input.with_extension(""));
        // Determine which slides to render
        let info = match pptx2html_core::get_info(&cli.input) {
            Ok(info) => info,
            Err(e) => {
                eprintln!("Failed to read presentation: {e}");
                std::process::exit(1);
            }
        };

        let indices_to_render: Vec<usize> = match &slide_indices {
            Some(indices) => indices.clone(),
            None => (1..=info.slide_count).collect(),
        };
        let html_outputs = indices_to_render
            .iter()
            .map(|idx| output_dir.join(format!("slide-{idx}.html")))
            .collect::<Vec<_>>();
        let html_protected_paths = html_outputs
            .iter()
            .cloned()
            .map(|path| (path, "HTML output"))
            .collect::<Vec<_>>();
        if let Err(error) =
            reject_sidecar_collision(cli.diagnostics.as_deref(), &html_protected_paths)
        {
            eprintln!("Invalid diagnostics path: {error}");
            std::process::exit(1);
        }
        let aggregate = match pptx2html_core::convert_file_with_options_metadata(&cli.input, &opts)
        {
            Ok(result) => result,
            Err(error) => {
                eprintln!("Conversion failed: {error}");
                std::process::exit(1);
            }
        };

        let mut protected_paths = html_protected_paths;
        protected_paths.extend(aggregate.external_assets.iter().map(|asset| {
            (
                output_dir.join(&asset.relative_path),
                "emitted external asset",
            )
        }));
        if let Err(error) = reject_sidecar_collision(cli.diagnostics.as_deref(), &protected_paths) {
            eprintln!("Invalid diagnostics path: {error}");
            std::process::exit(1);
        }
        if let Err(e) = std::fs::create_dir_all(&output_dir) {
            eprintln!("Failed to create output directory: {e}");
            std::process::exit(1);
        }

        for &idx in &indices_to_render {
            let per_slide_opts = ConversionOptions {
                embed_images: !cli.no_embed,
                include_hidden: cli.include_hidden,
                slide_range: None,
                slide_indices: Some(vec![idx]),
                scale: cli.scale,
            };
            match pptx2html_core::convert_file_with_options_metadata(&cli.input, &per_slide_opts) {
                Ok(result) => {
                    let path = output_dir.join(format!("slide-{idx}.html"));
                    if let Err(e) = std::fs::write(&path, &result.html) {
                        eprintln!("Failed to write {}: {e}", path.display());
                        std::process::exit(1);
                    }
                    if let Err(e) = write_external_assets(&output_dir, &result.external_assets) {
                        eprintln!("Failed to write external assets: {e}");
                        std::process::exit(1);
                    }
                    info!("Written: {}", path.display());
                }
                Err(e) => {
                    eprintln!("Failed to convert slide {idx}: {e}");
                    std::process::exit(1);
                }
            }
        }
        let diagnostics_json = aggregate.diagnostics_json();
        let strict_failure = match finish_diagnostics(
            &aggregate.diagnostics,
            &diagnostics_json,
            cli.diagnostics.as_deref(),
            cli.fail_on_fallback,
        ) {
            Ok(strict_failure) => strict_failure,
            Err(error) => {
                eprintln!("Failed to write diagnostics: {error}");
                std::process::exit(1);
            }
        };
        println!(
            "Conversion complete: {} slides → {}",
            indices_to_render.len(),
            output_dir.display()
        );
        if strict_failure {
            std::process::exit(2);
        }
    } else {
        // Single-file output
        let output = cli
            .output
            .clone()
            .unwrap_or_else(|| cli.input.with_extension("html"));
        if let Err(error) = reject_sidecar_collision(
            cli.diagnostics.as_deref(),
            &[(output.clone(), "HTML output")],
        ) {
            eprintln!("Invalid diagnostics path: {error}");
            std::process::exit(1);
        }

        match pptx2html_core::convert_file_with_options_metadata(&cli.input, &opts) {
            Ok(result) => {
                let asset_base = output.parent().unwrap_or(Path::new("."));
                let mut protected_paths = vec![(output.clone(), "HTML output")];
                protected_paths.extend(result.external_assets.iter().map(|asset| {
                    (
                        asset_base.join(&asset.relative_path),
                        "emitted external asset",
                    )
                }));
                if let Err(error) =
                    reject_sidecar_collision(cli.diagnostics.as_deref(), &protected_paths)
                {
                    eprintln!("Invalid diagnostics path: {error}");
                    std::process::exit(1);
                }
                if let Err(e) = std::fs::write(&output, &result.html) {
                    eprintln!("Failed to write output file: {e}");
                    std::process::exit(1);
                }
                if let Err(e) = write_external_assets(asset_base, &result.external_assets) {
                    eprintln!("Failed to write external assets: {e}");
                    std::process::exit(1);
                }
                let diagnostics_json = result.diagnostics_json();
                let strict_failure = match finish_diagnostics(
                    &result.diagnostics,
                    &diagnostics_json,
                    cli.diagnostics.as_deref(),
                    cli.fail_on_fallback,
                ) {
                    Ok(strict_failure) => strict_failure,
                    Err(error) => {
                        eprintln!("Failed to write diagnostics: {error}");
                        std::process::exit(1);
                    }
                };
                println!(
                    "Conversion complete: {} -> {}",
                    cli.input.display(),
                    output.display()
                );
                if strict_failure {
                    std::process::exit(2);
                }
            }
            Err(e) => {
                eprintln!("Conversion failed: {e}");
                std::process::exit(1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pptx2html_core::ExternalAsset;

    #[test]
    fn test_parse_single_slides() {
        assert_eq!(parse_slide_selection("1,3,5").unwrap(), vec![1, 3, 5]);
    }

    #[test]
    fn test_parse_slide_range() {
        assert_eq!(parse_slide_selection("2-5").unwrap(), vec![2, 3, 4, 5]);
    }

    #[test]
    fn test_parse_mixed_selection() {
        assert_eq!(
            parse_slide_selection("1,3-5,8").unwrap(),
            vec![1, 3, 4, 5, 8]
        );
    }

    #[test]
    fn test_parse_dedup() {
        assert_eq!(parse_slide_selection("1,1,2,2-3").unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn test_parse_invalid_range() {
        assert!(parse_slide_selection("5-2").is_err());
    }

    #[test]
    fn test_parse_invalid_number() {
        assert!(parse_slide_selection("abc").is_err());
    }

    #[test]
    fn test_parse_invalid_missing_range_bounds() {
        assert!(parse_slide_selection("-3").is_err());
        assert!(parse_slide_selection("3-").is_err());
    }

    #[test]
    fn test_write_external_assets_creates_nested_files() {
        let tmpdir =
            std::env::temp_dir().join(format!("pptx2html-cli-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmpdir);
        std::fs::create_dir_all(&tmpdir).expect("create tempdir");
        let assets = vec![ExternalAsset {
            relative_path: "images/slide-1/image-0.png".to_string(),
            content_type: "image/png".to_string(),
            data: vec![1, 2, 3, 4],
        }];

        write_external_assets(&tmpdir, &assets).expect("asset write should succeed");

        let output = tmpdir.join("images/slide-1/image-0.png");
        assert!(output.exists(), "Expected asset file to be created");
        assert_eq!(std::fs::read(output).expect("read asset"), vec![1, 2, 3, 4]);
        std::fs::remove_dir_all(&tmpdir).expect("remove tempdir");
    }

    #[test]
    fn test_write_external_assets_reports_directory_creation_failure() {
        let tmpdir = std::env::temp_dir().join(format!(
            "pptx2html-cli-test-create-dir-error-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmpdir);
        std::fs::create_dir_all(&tmpdir).expect("create tempdir");
        std::fs::write(tmpdir.join("images"), b"not a directory").expect("create blocking file");

        let assets = vec![ExternalAsset {
            relative_path: "images/slide-1/image-0.png".to_string(),
            content_type: "image/png".to_string(),
            data: vec![1, 2, 3, 4],
        }];

        let err = write_external_assets(&tmpdir, &assets).expect_err("directory creation fails");
        assert!(err.contains("failed to create asset directory"));

        std::fs::remove_file(tmpdir.join("images")).expect("remove blocking file");
        std::fs::remove_dir_all(&tmpdir).expect("remove tempdir");
    }

    #[test]
    fn test_write_external_assets_reports_file_write_failure() {
        let tmpdir = std::env::temp_dir().join(format!(
            "pptx2html-cli-test-write-error-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmpdir);
        std::fs::create_dir_all(tmpdir.join("images")).expect("create images dir");

        let assets = vec![ExternalAsset {
            relative_path: "images".to_string(),
            content_type: "image/png".to_string(),
            data: vec![1, 2, 3, 4],
        }];

        let err = write_external_assets(&tmpdir, &assets).expect_err("write should fail for dir");
        assert!(err.contains("failed to write asset"));

        std::fs::remove_dir_all(&tmpdir).expect("remove tempdir");
    }
}
