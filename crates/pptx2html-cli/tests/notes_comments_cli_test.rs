use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_DIRECTORY_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn create() -> Self {
        let sequence = TEST_DIRECTORY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "pptx2html-notes-cli-{}-{sequence}",
            std::process::id(),
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create test directory");
        Self(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn generate_completion_decks(repo_root: &Path, output: &Path) {
    let result = Command::new("python3")
        .arg(repo_root.join("evaluate/create_completion_decks.py"))
        .arg("--output-dir")
        .arg(output)
        .current_dir(repo_root)
        .output()
        .expect("run completion deck generator");
    assert!(
        result.status.success(),
        "completion deck generation failed: {}",
        String::from_utf8_lossy(&result.stderr),
    );
}

fn convert(input: &Path, output: &Path) {
    convert_with_args(input, output, &[]);
}

fn convert_with_args(input: &Path, output: &Path, args: &[&str]) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_pptx2html"));
    command.arg(input).arg("--output").arg(output);
    for argument in args {
        command.arg(argument);
    }
    let result = command.output().expect("run pptx2html CLI");
    assert!(
        result.status.success(),
        "CLI conversion failed: {}",
        String::from_utf8_lossy(&result.stderr),
    );
}

fn diagnostics(html: &str) -> &str {
    html.split_once(r#"<script type="application/json" id="pptx2html-diagnostics">"#)
        .and_then(|(_, tail)| tail.split_once("</script>"))
        .map(|(payload, _)| payload)
        .expect("diagnostics manifest")
}

#[test]
fn notes_and_comments_cli_output_is_deterministic_and_off_canvas() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root");
    let test_directory = TestDirectory::create();
    let decks = test_directory.0.join("decks");
    generate_completion_decks(repo_root, &decks);

    let input = decks.join("notes-comments.pptx");
    let first = test_directory.0.join("first.html");
    let second = test_directory.0.join("second.html");
    convert(&input, &first);
    convert(&input, &second);

    let first_bytes = fs::read(&first).expect("read first HTML");
    let second_bytes = fs::read(&second).expect("read second HTML");
    assert_eq!(first_bytes, second_bytes);

    let html = String::from_utf8(first_bytes).expect("HTML is UTF-8");
    let diagnostics_start = html
        .find(r#"<script type="application/json" id="pptx2html-diagnostics">"#)
        .expect("diagnostics manifest");
    let visible_html = &html[..diagnostics_start];
    for sentinel in [
        "NOTES_SENTINEL",
        "CLASSIC_COMMENT_SENTINEL",
        "MODERN_COMMENT_SENTINEL",
    ] {
        assert!(!visible_html.contains(sentinel));
    }
    for code in [
        "NOTES_SLIDE_METADATA",
        "LEGACY_COMMENT_METADATA",
        "MODERN_COMMENT_METADATA",
    ] {
        assert!(html.contains(code));
    }
}

#[test]
fn cli_slide_selection_filters_annotation_metadata() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root");
    let test_directory = TestDirectory::create();
    let decks = test_directory.0.join("decks");
    generate_completion_decks(repo_root, &decks);

    let input = decks.join("notes-comments.pptx");
    let selected = test_directory.0.join("selected.html");
    convert_with_args(&input, &selected, &["--slides", "2"]);

    let html = fs::read_to_string(selected).expect("read selected HTML");
    let diagnostics = diagnostics(&html);
    assert!(!diagnostics.contains("NOTES_SENTINEL"));
    assert!(!diagnostics.contains("CLASSIC_COMMENT_SENTINEL"));
    assert!(!diagnostics.contains("MODERN_COMMENT_SENTINEL"));
    assert!(!diagnostics.contains("Fixture"));
}
