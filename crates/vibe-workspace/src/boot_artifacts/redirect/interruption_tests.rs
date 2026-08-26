use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use tempfile::TempDir;
use vibe_core::manifest::SpecFormat;

use super::*;
use crate::boot_artifacts::transaction::{self, ArtifactWrite};

const CHILD_ROOT: &str = "VIBEVM_R32_REDIRECT_CHILD_ROOT";
const CHILD_DIRECTION: &str = "VIBEVM_R32_REDIRECT_CHILD_DIRECTION";
const CHILD_AFTER: &str = "VIBEVM_R32_REDIRECT_CHILD_AFTER";

#[derive(Clone, Copy)]
enum Direction {
    MarkdownToXml,
    XmlToMarkdown,
}

impl Direction {
    fn old(self) -> SpecFormat {
        match self {
            Direction::MarkdownToXml => SpecFormat::Markdown,
            Direction::XmlToMarkdown => SpecFormat::Xml,
        }
    }

    fn next(self) -> SpecFormat {
        match self {
            Direction::MarkdownToXml => SpecFormat::Xml,
            Direction::XmlToMarkdown => SpecFormat::Markdown,
        }
    }

    fn code(self) -> &'static str {
        match self {
            Direction::MarkdownToXml => "md-xml",
            Direction::XmlToMarkdown => "xml-md",
        }
    }

    fn parse(value: &str) -> Self {
        match value {
            "md-xml" => Direction::MarkdownToXml,
            "xml-md" => Direction::XmlToMarkdown,
            other => panic!("unknown direction {other}"),
        }
    }
}

struct SwitchFixture {
    root: PathBuf,
    boot: PathBuf,
    index: PathBuf,
    selected: PathBuf,
    stale: PathBuf,
    new_index: Vec<u8>,
}

impl SwitchFixture {
    fn seed(root: &Path, direction: Direction) -> Self {
        let boot = root.join(vibe_core::layout::current_boot_dir());
        fs::create_dir_all(&boot).unwrap();
        let index = boot.join("INDEX.md");
        let selected = boot.join(super::super::static_file(direction.next()));
        let stale = boot.join(super::super::static_file(direction.old()));
        let old_index = index_bytes(direction.old());
        let new_index = index_bytes(direction.next());
        fs::write(&index, old_index).unwrap();
        fs::write(&stale, b"OLD-STATIC").unwrap();
        let _ = fs::remove_file(&selected);
        let old_block = render_block(direction.old());
        for name in REDIRECT_FILES {
            fs::write(root.join(name), &old_block).unwrap();
        }
        Self {
            root: root.to_path_buf(),
            boot,
            index,
            selected,
            stale,
            new_index,
        }
    }

    fn load(root: &Path, direction: Direction) -> Self {
        let boot = root.join(vibe_core::layout::current_boot_dir());
        Self {
            root: root.to_path_buf(),
            index: boot.join("INDEX.md"),
            selected: boot.join(super::super::static_file(direction.next())),
            stale: boot.join(super::super::static_file(direction.old())),
            new_index: index_bytes(direction.next()),
            boot,
        }
    }

    fn write(&self) -> ArtifactWrite<'_> {
        ArtifactWrite {
            index_path: &self.index,
            index_bytes: &self.new_index,
            static_path: &self.selected,
            static_bytes: Some(b"NEW-STATIC"),
            stale_path: &self.stale,
        }
    }

    fn converge(&self, direction: Direction) {
        transaction::write_production_with_selectors(self.write(), |transaction| {
            write_redirect_blocks_with_transaction(&self.root, direction.next(), transaction)
        })
        .unwrap();
    }

    fn assert_partial_safe(&self) {
        assert!(entry_present(&self.selected));
        assert!(entry_present(&self.stale));
        let index = fs::read_to_string(&self.index).unwrap();
        assert!(
            index.contains(self.selected.file_name().unwrap().to_str().unwrap()),
            "INDEX does not select the published carrier"
        );
        for name in REDIRECT_FILES {
            let content = fs::read_to_string(self.root.join(name)).unwrap();
            let selected = if content.contains("STATIC.xml`") {
                self.boot.join("STATIC.xml")
            } else {
                self.boot.join("STATIC.md")
            };
            assert!(
                entry_present(&selected),
                "{} points at missing carrier",
                name
            );
        }
    }

    fn assert_final(&self, direction: Direction) {
        assert!(entry_present(&self.selected));
        assert!(!entry_present(&self.stale));
        let expected = render_block(direction.next()).into_bytes();
        let triplet = REDIRECT_FILES
            .iter()
            .map(|name| fs::read(self.root.join(name)).unwrap())
            .collect::<Vec<_>>();
        assert!(triplet.iter().all(|bytes| bytes == &expected));
        assert_eq!(triplet[0], triplet[1]);
        assert_eq!(triplet[1], triplet[2]);
    }
}

fn index_bytes(format: SpecFormat) -> Vec<u8> {
    format!(
        "schema = 1\nstatic = \"vibevm/vibespecs/boot/{}\"\n",
        super::super::static_file(format)
    )
    .into_bytes()
}

struct ErrorAfter(usize);

impl RedirectFaultInjector for ErrorAfter {
    fn after_replace(&self, index: usize, path: &Path) -> Result<(), WorkspaceError> {
        if index == self.0 {
            Err(WorkspaceError::Io {
                path: path.to_path_buf(),
                reason: format!("injected redirect {} error", index + 1),
            })
        } else {
            Ok(())
        }
    }
}

struct ExitAfter(usize);

impl RedirectFaultInjector for ExitAfter {
    fn after_replace(&self, index: usize, _path: &Path) -> Result<(), WorkspaceError> {
        if index == self.0 {
            std::process::exit(86);
        }
        Ok(())
    }
}

#[test]
fn each_redirect_error_keeps_selectors_live_and_retry_converges_both_directions() {
    for direction in [Direction::MarkdownToXml, Direction::XmlToMarkdown] {
        for after in 0..3 {
            let temp = TempDir::new().unwrap();
            let fixture = SwitchFixture::seed(temp.path(), direction);
            let result =
                transaction::write_production_with_selectors(fixture.write(), |transaction| {
                    write_redirect_blocks_with_faults(
                        &fixture.root,
                        direction.next(),
                        transaction,
                        &ErrorAfter(after),
                    )
                });
            assert!(result.is_err());
            fixture.assert_partial_safe();
            fixture.converge(direction);
            fixture.assert_final(direction);
        }
    }
}

#[test]
fn each_redirect_child_kill_keeps_selectors_live_and_retry_converges_both_directions() {
    for direction in [Direction::MarkdownToXml, Direction::XmlToMarkdown] {
        for after in 0..3 {
            let temp = TempDir::new().unwrap();
            let fixture = SwitchFixture::seed(temp.path(), direction);
            assert_interrupted(spawn_child(temp.path(), direction, after));
            fixture.assert_partial_safe();
            fixture.converge(direction);
            fixture.assert_final(direction);
        }
    }
}

fn spawn_child(root: &Path, direction: Direction, after: usize) -> ExitStatus {
    Command::new(env::current_exe().unwrap())
        .arg("--exact")
        .arg("boot_artifacts::redirect::interruption_tests::redirect_child_helper")
        .arg("--nocapture")
        .env(CHILD_ROOT, root)
        .env(CHILD_DIRECTION, direction.code())
        .env(CHILD_AFTER, after.to_string())
        .status()
        .unwrap()
}

fn assert_interrupted(status: ExitStatus) {
    assert_eq!(status.code(), Some(86), "child status {status}");
}

#[test]
fn redirect_child_helper() {
    let Ok(root) = env::var(CHILD_ROOT) else {
        return;
    };
    let direction = Direction::parse(&env::var(CHILD_DIRECTION).unwrap());
    let after = env::var(CHILD_AFTER).unwrap().parse::<usize>().unwrap();
    let fixture = SwitchFixture::load(Path::new(&root), direction);
    let result = transaction::write_production_with_selectors(fixture.write(), |transaction| {
        write_redirect_blocks_with_faults(
            &fixture.root,
            direction.next(),
            transaction,
            &ExitAfter(after),
        )
    });
    panic!("child did not exit at redirect: {result:?}");
}

fn entry_present(path: &Path) -> bool {
    match fs::symlink_metadata(path) {
        Ok(_) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => panic!("metadata {}: {error}", path.display()),
    }
}
