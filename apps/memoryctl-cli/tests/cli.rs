//! 集成测试：每个 case 独立 HOME。

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic, clippy::pedantic)]

use std::path::Path;

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

struct Harness {
    _home: TempDir,
    home_path: std::path::PathBuf,
    project: std::path::PathBuf,
}

impl Harness {
    fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let home_path = home.path().to_path_buf();
        let project = home_path.join("project");
        std::fs::create_dir_all(&project).unwrap();
        Self { _home: home, home_path, project }
    }

    fn cmd(&self, cwd: &Path) -> Command {
        let mut c = Command::cargo_bin("memoryctl").unwrap();
        c.env("HOME", &self.home_path)
            .env_remove("RUST_LOG")
            .env("MEMORYCTL_AGENT", "test-agent")
            .current_dir(cwd);
        c
    }

    fn run_json(&self, cwd: &Path, args: &[&str]) -> Value {
        let out = self.cmd(cwd).arg("--format").arg("json").args(args).output().unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        serde_json::from_str(&stdout).unwrap_or_else(|e| {
            panic!(
                "bad json: {e}\nstdout: {stdout}\nstderr: {}",
                String::from_utf8_lossy(&out.stderr)
            )
        })
    }

    fn run_tsv(&self, cwd: &Path, args: &[&str]) -> String {
        let out = self.cmd(cwd).arg("--format").arg("tsv").args(args).output().unwrap();
        String::from_utf8(out.stdout).unwrap()
    }
}

#[test]
fn save_and_read_global() {
    let h = Harness::new();
    let r = h.run_json(
        &h.project,
        &[
            "save",
            "--type",
            "decision",
            "--topic",
            "skillctl-design",
            "--no-confirm",
            "TSV beats JSON for Agent list",
        ],
    );
    assert_eq!(r["success"], true);
    assert_eq!(r["topic"], "skillctl-design");
    assert_eq!(r["scope"], "global");

    let r = h.run_json(&h.project, &["read", "--topic", "skillctl-design"]);
    assert_eq!(r["count"], 1);
    assert_eq!(r["entries"][0]["entry_type"], "decision");
    assert!(r["entries"][0]["content"].as_str().unwrap().contains("TSV beats"));
}

#[test]
fn list_tsv_format() {
    let h = Harness::new();
    h.run_json(
        &h.project,
        &[
            "save",
            "--type",
            "lesson",
            "--topic",
            "rust-errors",
            "--no-confirm",
            "thiserror in libs",
        ],
    );
    h.run_json(
        &h.project,
        &["save", "--type", "decision", "--topic", "rust-errors", "--no-confirm", "anyhow in main"],
    );
    let tsv = h.run_tsv(&h.project, &["list"]);
    let lines: Vec<&str> = tsv.lines().collect();
    assert_eq!(lines[0], "TOPIC\tENTRIES\tLAST_UPDATED\tTYPES\tSCOPE");
    assert!(lines[1].starts_with("rust-errors\t2\t"));
    assert!(lines[1].contains("decision,lesson") || lines[1].contains("lesson,decision"));
    assert!(lines[1].ends_with("\tglobal"));
}

#[test]
fn project_scope_requires_init() {
    let h = Harness::new();
    let mut c = h.cmd(&h.project);
    let out = c
        .args([
            "--format",
            "json",
            "save",
            "--type",
            "fact",
            "--topic",
            "x",
            "--scope",
            "project",
            "--no-confirm",
            "foo",
        ])
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["success"], false);
}

#[test]
fn init_creates_project_layout() {
    let h = Harness::new();
    let r = h.run_json(&h.project, &["init"]);
    assert_eq!(r["success"], true);
    assert!(h.project.join(".memoryctl/topics").exists());
    assert!(h.project.join("AGENTS.md").exists());

    // 现在 project scope 可用
    let r = h.run_json(
        &h.project,
        &[
            "save",
            "--type",
            "fact",
            "--topic",
            "elepay-flow",
            "--scope",
            "project",
            "--no-confirm",
            "Stripe → 我们 → 商户",
        ],
    );
    assert_eq!(r["success"], true);
    assert_eq!(r["scope"], "project");
}

#[test]
fn agents_md_byte_stable() {
    let h = Harness::new();
    h.run_json(&h.project, &["init"]);
    let snap1 = std::fs::read(h.project.join("AGENTS.md")).unwrap();

    // save 多条
    h.run_json(&h.project, &["save", "--type", "lesson", "--topic", "t1", "--no-confirm", "one"]);
    h.run_json(&h.project, &["save", "--type", "decision", "--topic", "t2", "--no-confirm", "two"]);
    h.run_json(&h.project, &["read", "--topic", "t1"]);
    h.run_json(&h.project, &["search", "one"]);

    let snap2 = std::fs::read(h.project.join("AGENTS.md")).unwrap();
    assert_eq!(snap1, snap2, "AGENTS.md must not change after save/read/search");
}

#[test]
fn coexists_with_skillctl_block() {
    let h = Harness::new();
    let agents = h.project.join("AGENTS.md");
    std::fs::write(&agents, "<!-- skillctl:start version=1 -->\nskill\n<!-- skillctl:end -->\n")
        .unwrap();

    h.run_json(&h.project, &["enable"]);
    let s = std::fs::read_to_string(&agents).unwrap();
    assert!(s.contains("skillctl:start"));
    assert!(s.contains("memoryctl:start"));

    h.run_json(&h.project, &["disable"]);
    let s = std::fs::read_to_string(&agents).unwrap();
    assert!(s.contains("skillctl:start"), "skillctl block must remain");
    assert!(!s.contains("memoryctl:"));
}

#[test]
fn search_finds_match() {
    let h = Harness::new();
    h.run_json(
        &h.project,
        &["save", "--type", "lesson", "--topic", "t", "--no-confirm", "the quick brown fox"],
    );
    h.run_json(
        &h.project,
        &["save", "--type", "decision", "--topic", "u", "--no-confirm", "lazy dog"],
    );
    let r = h.run_json(&h.project, &["search", "fox"]);
    assert_eq!(r["count"], 1);
    let m = &r["matches"][0];
    assert_eq!(m["topic"], "t");
    assert!(m["snippet"].as_str().unwrap().contains("fox"));
}

#[test]
fn forget_specific_entry() {
    let h = Harness::new();
    h.run_json(&h.project, &["save", "--type", "lesson", "--topic", "t", "--no-confirm", "first"]);
    h.run_json(&h.project, &["save", "--type", "lesson", "--topic", "t", "--no-confirm", "second"]);
    h.run_json(&h.project, &["save", "--type", "lesson", "--topic", "t", "--no-confirm", "third"]);

    let r = h.run_json(&h.project, &["forget", "--topic", "t", "--entry", "2"]);
    assert_eq!(r["removed"], 1);

    let r = h.run_json(&h.project, &["read", "--topic", "t"]);
    assert_eq!(r["count"], 2);
    let contents: Vec<&str> =
        r["entries"].as_array().unwrap().iter().map(|e| e["content"].as_str().unwrap()).collect();
    assert_eq!(contents, vec!["first", "third"]);
}

#[test]
fn type_filter_in_list_and_read() {
    let h = Harness::new();
    h.run_json(&h.project, &["save", "--type", "lesson", "--topic", "t", "--no-confirm", "L1"]);
    h.run_json(&h.project, &["save", "--type", "decision", "--topic", "t", "--no-confirm", "D1"]);

    let r = h.run_json(&h.project, &["read", "--topic", "t", "--type", "decision"]);
    assert_eq!(r["count"], 1);
    assert_eq!(r["entries"][0]["entry_type"], "decision");
}

#[test]
fn invalid_topic_name_error_envelope() {
    let h = Harness::new();
    let mut c = h.cmd(&h.project);
    let out = c
        .args([
            "--format",
            "json",
            "save",
            "--type",
            "lesson",
            "--topic",
            "Bad-Name",
            "--no-confirm",
            "x",
        ])
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["success"], false);
    assert_eq!(v["error"], "invalid_topic_name");
}

#[test]
fn search_cjk_content() {
    // 回归：snippet 切片必须按字符边界，不能在多字节字符中间断开
    let h = Harness::new();
    h.run_json(
        &h.project,
        &[
            "save",
            "--type",
            "decision",
            "--topic",
            "design",
            "--no-confirm",
            "memoryctl 与 skillctl 分开做。理由：skill 是策划过的稳定能力，memory 是涌现的活观察",
        ],
    );
    let r = h.run_json(&h.project, &["search", "策划"]);
    assert_eq!(r["count"], 1);
    assert!(r["matches"][0]["snippet"].as_str().unwrap().contains("策划"));
}

#[test]
fn read_topic_not_found() {
    let h = Harness::new();
    let mut c = h.cmd(&h.project);
    let out = c.args(["--format", "json", "read", "--topic", "nonexistent"]).output().unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["success"], false);
    assert_eq!(v["error"], "topic_not_found");
}
