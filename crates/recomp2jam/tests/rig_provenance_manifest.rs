//! Capture provenance guards (#281, docs/reversing.md section 6).
//!
//! Live chain: `scripts/rig-run.sh run` leaves `pids` / `env.sh` /
//! `claims` in the run out-dir -> `scripts/rig-provenance.sh emit`
//! writes `provenance.json` from them -> `scripts/run-manifest.sh verify`
//! refuses a capture that has no provenance entry. These guards drive
//! that exact operator pair on a synthetic run dir: the two scripts are
//! the feature, so the seam is the whole behavior (no game data, no
//! display, no Wine run).
//!
//! Tool probes are pinned through a temporary PATH (fake `wine`, `Xvfb`,
//! `ffmpeg`) plus `--tool-version`, the same override a rig host without
//! `pacman`/`dpkg` uses. Skips cleanly where `bash` or `python3` is absent.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

const X11_SERVER_VERSION: &str = "21.1.26-1";
const FAKE_EXE_BYTES: &[u8] = b"MZ synthetic producer, no game bytes";
const FIRST_SHOT: &[u8] = b"PNG synthetic loading frame";
const SECOND_SHOT: &[u8] = b"PNG synthetic waiting frame";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root resolves")
}

fn have(tool: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {tool} >/dev/null 2>&1"))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// A synthetic rig run dir shaped like `scripts/rig-run.sh` leaves it.
struct RunDir {
    path: PathBuf,
    fake_tools: PathBuf,
}

impl Drop for RunDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

impl RunDir {
    fn new(scenario: &str) -> RunDir {
        let path = std::env::temp_dir().join(format!("p281-{scenario}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        let fake_tools = path.join("bin");
        fs::create_dir_all(path.join("artifacts/screenshots")).expect("screenshots dir");
        fs::create_dir_all(path.join("artifacts/stats")).expect("stats dir");
        fs::create_dir_all(&fake_tools).expect("fake tool dir");
        let run = RunDir { path, fake_tools };
        run.write_fake_tool("wine", "wine-9.0");
        run.write_fake_tool("Xvfb", "Xvfb 21.1.26");
        run.write_fake_tool("ffmpeg", "ffmpeg version 6.1");
        run.write("OTwo.exe", FAKE_EXE_BYTES);
        run.write("target/release/recomp2jam", b"ELF synthetic client build");
        run.write("artifacts/screenshots/01-loading.png", FIRST_SHOT);
        run.write("input-src.log", b"frame 1 screen=loading\n");
        run.write("pids", b"xvfb 4242 Xvfb\nclient 4243 /opt/rig/OTwo.exe\n");
        run.write(
            "env.sh",
            b"export DISPLAY=':77'\nexport WINIT_UNIX_BACKEND=x11\n\
              export PULSE_SINK='wine_null'\n",
        );
        run.write("claims", b"display :77\nprefix /tmp/prefix-281\n");
        run
    }

    fn write(&self, relative: &str, bytes: &[u8]) {
        let full = self.path.join(relative);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).expect("parent dir");
        }
        fs::write(full, bytes).expect("write run-dir file");
    }

    fn write_fake_tool(&self, name: &str, version_line: &str) {
        let script = format!("#!/usr/bin/env bash\necho '{version_line}'\n");
        let full = self.fake_tools.join(name);
        fs::write(&full, script).expect("write fake tool");
        let mut perms = fs::metadata(&full)
            .expect("fake tool metadata")
            .permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            perms.set_mode(0o755);
        }
        fs::set_permissions(&full, perms).expect("chmod fake tool");
    }

    fn manifest(&self) -> Value {
        let text = fs::read_to_string(self.path.join("provenance.json")).expect("provenance.json");
        serde_json::from_str(&text).expect("provenance.json parses")
    }

    /// Simulate a hand-trimmed manifest: drop one producer field.
    fn drop_producer_key(&self, key: &str) {
        let mut manifest = self.manifest();
        if let Some(producer) = manifest["producer"].as_object_mut() {
            producer.remove(key);
        }
        self.write("provenance.json", manifest.to_string().as_bytes());
    }

    fn path_string(&self) -> String {
        self.path.to_string_lossy().into_owned()
    }

    /// The PATH a rig host would have, with the fake tools in front.
    fn path_env(&self) -> String {
        let inherited = std::env::var("PATH").unwrap_or_default();
        format!("{}:{inherited}", self.fake_tools.to_string_lossy())
    }
}

fn bash_script(script: &str, args: &[String], run: &RunDir) -> Output {
    Command::new("bash")
        .arg(script)
        .args(args)
        .current_dir(repo_root())
        .env("PATH", run.path_env())
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .output()
        .expect("script runs")
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn write_traditional_manifest(run: &RunDir, pair: &str) {
    let args = vec![
        "write".to_string(),
        run.path_string(),
        "--pair".to_string(),
        pair.to_string(),
        "--input-log".to_string(),
        run.path
            .join("input-src.log")
            .to_string_lossy()
            .into_owned(),
    ];
    let output = bash_script("scripts/run-manifest.sh", &args, run);
    assert!(
        output.status.success(),
        "run-manifest write failed: {}",
        stderr_of(&output)
    );
}

fn emit_arguments(run: &RunDir, producer_kind: &str, extra: &[&str]) -> Vec<String> {
    let producer = if producer_kind == "exe" {
        run.path.join("OTwo.exe")
    } else {
        run.path.join("target/release/recomp2jam")
    };
    let mut args = vec![
        "emit".to_string(),
        run.path_string(),
        "--producer".to_string(),
        producer.to_string_lossy().into_owned(),
        "--producer-kind".to_string(),
        producer_kind.to_string(),
        "--capture-tool".to_string(),
        "ffmpeg".to_string(),
        "--tool-version".to_string(),
        "xvfb".to_string(),
        X11_SERVER_VERSION.to_string(),
    ];
    if producer_kind == "exe" {
        args.push("--producer-version".to_string());
        args.push("7.55".to_string());
    }
    args.extend(extra.iter().map(|value| value.to_string()));
    args
}

fn emit(run: &RunDir, producer_kind: &str) -> Output {
    emit_with(run, producer_kind, &[])
}

fn emit_with(run: &RunDir, producer_kind: &str, extra: &[&str]) -> Output {
    bash_script(
        "scripts/rig-provenance.sh",
        &emit_arguments(run, producer_kind, extra),
        run,
    )
}

fn verify(run: &RunDir) -> Output {
    bash_script(
        "scripts/run-manifest.sh",
        &["verify".to_string(), run.path_string()],
        run,
    )
}

fn cite(run: &RunDir, claim: &str) -> Output {
    bash_script(
        "scripts/rig-provenance.sh",
        &[
            "cite".to_string(),
            run.path_string(),
            "--claim".to_string(),
            claim.to_string(),
        ],
        run,
    )
}

fn skip_without_script_tools(test_name: &str) -> bool {
    if have("bash") && have("python3") {
        return false;
    }
    eprintln!("SKIP {test_name}: bash or python3 is missing; rig provenance guards need both");
    true
}

#[test]
fn emitting_from_a_rig_run_dir_fills_every_manifest_field_without_hand_editing() {
    // Chain: scripts/rig-run.sh run -> pids/env.sh/claims -> rig-provenance.sh emit.
    if skip_without_script_tools(stringify!(
        emitting_from_a_rig_run_dir_fills_every_manifest_field_without_hand_editing
    )) {
        return;
    }
    let run = RunDir::new("emit-fields");
    write_traditional_manifest(&run, "wine-x11-sendinput");
    let output = emit(&run, "exe");
    assert!(
        output.status.success(),
        "emit failed: {}",
        stderr_of(&output)
    );

    let manifest = run.manifest();
    assert_eq!(manifest["schema"], "rig-provenance-v1");
    assert_eq!(manifest["run"]["pair"], "wine-x11-sendinput");
    assert!(
        manifest["run"]["build"]
            .as_str()
            .is_some_and(|b| b.len() >= 7),
        "run.build must name the repo commit: {}",
        manifest["run"]["build"]
    );
    assert_eq!(manifest["producer"]["name"], "OTwo.exe");
    assert_eq!(manifest["producer"]["version"], "7.55");
    assert_eq!(
        manifest["producer"]["size_bytes"],
        FAKE_EXE_BYTES.len() as u64
    );
    assert_eq!(manifest["session"]["display"], ":77");
    assert_eq!(manifest["session"]["pulse_sink"], "wine_null");
    assert_eq!(manifest["session"]["command"][0], "/opt/rig/OTwo.exe");
    assert_eq!(manifest["tools"]["wine"]["version"], "9.0");
    assert_eq!(manifest["tools"]["xvfb"]["version"], X11_SERVER_VERSION);
    assert_eq!(manifest["tools"]["capture"]["name"], "ffmpeg");
    assert_eq!(
        manifest["artifacts"][0]["path"],
        "artifacts/screenshots/01-loading.png"
    );
    assert!(
        manifest["manifest_sha256"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64),
        "manifest hash must be recorded last"
    );
}

#[test]
fn verify_rejects_a_capture_without_a_provenance_entry() {
    // Chain: scripts/rig-run.sh run -> scripts/run-manifest.sh verify.
    if skip_without_script_tools(stringify!(
        verify_rejects_a_capture_without_a_provenance_entry
    )) {
        return;
    }
    let run = RunDir::new("no-provenance");
    write_traditional_manifest(&run, "wine-x11-sendinput");

    let output = verify(&run);
    assert!(
        !output.status.success(),
        "verify accepted a capture with no provenance: {}",
        stdout_of(&output)
    );
    assert!(
        stderr_of(&output).contains("no provenance.json"),
        "failure must name the missing manifest: {}",
        stderr_of(&output)
    );
}

#[test]
fn verify_rejects_provenance_that_omits_a_captured_artifact() {
    if skip_without_script_tools(stringify!(
        verify_rejects_provenance_that_omits_a_captured_artifact
    )) {
        return;
    }
    let run = RunDir::new("omitted-artifact");
    write_traditional_manifest(&run, "wine-x11-sendinput");
    assert!(emit(&run, "exe").status.success());
    run.write("artifacts/screenshots/02-waiting.png", SECOND_SHOT);

    let output = verify(&run);
    assert!(
        !output.status.success(),
        "verify accepted an unlisted capture"
    );
    assert!(
        stderr_of(&output).contains("02-waiting.png has no provenance entry"),
        "failure must name the unlisted capture: {}",
        stderr_of(&output)
    );
}

#[test]
fn verify_rejects_a_capture_whose_bytes_changed_after_emitting() {
    if skip_without_script_tools(stringify!(
        verify_rejects_a_capture_whose_bytes_changed_after_emitting
    )) {
        return;
    }
    let run = RunDir::new("tampered-capture");
    write_traditional_manifest(&run, "wine-x11-sendinput");
    assert!(emit(&run, "exe").status.success());
    run.write(
        "artifacts/screenshots/01-loading.png",
        b"PNG re-rendered frame",
    );

    let output = verify(&run);
    assert!(!output.status.success(), "verify ignored a changed capture");
    assert!(
        stderr_of(&output).contains("bytes changed since the manifest"),
        "failure must name the changed capture: {}",
        stderr_of(&output)
    );
}

#[test]
fn verify_rejects_a_provenance_manifest_missing_the_exe_version() {
    if skip_without_script_tools(stringify!(
        verify_rejects_a_provenance_manifest_missing_the_exe_version
    )) {
        return;
    }
    let run = RunDir::new("no-exe-version");
    write_traditional_manifest(&run, "wine-x11-sendinput");
    assert!(emit(&run, "exe").status.success());
    run.drop_producer_key("version");

    let output = verify(&run);
    assert!(!output.status.success(), "verify accepted an unpinned exe");
    assert!(
        stderr_of(&output).contains("producer.version"),
        "failure must name the exe version pin: {}",
        stderr_of(&output)
    );
}

#[test]
fn verify_accepts_a_complete_provenance_manifest() {
    if skip_without_script_tools(stringify!(verify_accepts_a_complete_provenance_manifest)) {
        return;
    }
    let run = RunDir::new("complete");
    write_traditional_manifest(&run, "native-xvfb-xtest");
    let emitted = emit(&run, "client-build");
    assert!(
        emitted.status.success(),
        "emit failed: {}",
        stderr_of(&emitted)
    );

    let output = verify(&run);
    assert!(
        output.status.success(),
        "verify rejected a complete manifest: {}",
        stderr_of(&output)
    );
    assert!(
        stdout_of(&output).contains("manifest complete"),
        "verify must report the manifest: {}",
        stdout_of(&output)
    );
}

#[test]
fn citing_an_exe_display_claim_needs_exe_provenance() {
    // The #278 check: "the exe shows X" cannot be printed without a
    // manifest-backed exe capture.
    if skip_without_script_tools(stringify!(citing_an_exe_display_claim_needs_exe_provenance)) {
        return;
    }
    let run = RunDir::new("cite-exe");
    write_traditional_manifest(&run, "native-xvfb-xtest");
    assert!(emit(&run, "client-build").status.success());

    let unmanifested_claim = cite(&run, "exe");
    assert!(
        !unmanifested_claim.status.success(),
        "a client-build run produced an exe citation: {}",
        stdout_of(&unmanifested_claim)
    );
    assert!(
        stderr_of(&unmanifested_claim).contains("the producer kind"),
        "failure must name the producer kind: {}",
        stderr_of(&unmanifested_claim)
    );

    let exe_run = RunDir::new("cite-exe-producer");
    write_traditional_manifest(&exe_run, "wine-x11-sendinput");
    assert!(emit(&exe_run, "exe").status.success());
    let exe_claim = cite(&exe_run, "exe");
    assert!(
        exe_claim.status.success(),
        "an exe-backed run was refused a citation: {}",
        stderr_of(&exe_claim)
    );
    let paragraph = stdout_of(&exe_claim);
    assert!(
        paragraph.contains("producer exe OTwo.exe 7.55"),
        "{paragraph}"
    );
    assert!(paragraph.contains("manifest sha256"), "{paragraph}");
    assert!(
        paragraph.contains("exe evidence only through this manifest"),
        "{paragraph}"
    );
}

#[test]
fn citing_a_run_without_any_manifest_fails() {
    if skip_without_script_tools(stringify!(citing_a_run_without_any_manifest_fails)) {
        return;
    }
    let run = RunDir::new("cite-unmanifested");
    write_traditional_manifest(&run, "wine-x11-sendinput");

    let output = cite(&run, "exe");
    assert!(!output.status.success(), "cite printed an uncitable run");
    assert!(
        stderr_of(&output).contains("no provenance.json"),
        "failure must name the missing manifest: {}",
        stderr_of(&output)
    );
}

#[test]
fn verify_still_accepts_a_run_without_captures() {
    if skip_without_script_tools(stringify!(verify_still_accepts_a_run_without_captures)) {
        return;
    }
    let run = RunDir::new("no-captures");
    write_traditional_manifest(&run, "native-xvfb-xtest");
    fs::remove_file(run.path.join("artifacts/screenshots/01-loading.png")).expect("drop capture");

    let output = verify(&run);
    assert!(
        output.status.success(),
        "a stats-only run needs no capture manifest: {}",
        stderr_of(&output)
    );
    assert!(
        stdout_of(&output).contains("provenance not required"),
        "the skip must be loud: {}",
        stdout_of(&output)
    );
}

#[test]
fn emitting_refuses_a_producer_that_does_not_match_the_pinned_hash() {
    if skip_without_script_tools(stringify!(
        emitting_refuses_a_producer_that_does_not_match_the_pinned_hash
    )) {
        return;
    }
    let run = RunDir::new("pinned-hash");
    write_traditional_manifest(&run, "wine-x11-sendinput");
    let output = emit_with(&run, "exe", &["--expect-producer-sha256", &"ab".repeat(32)]);

    assert!(!output.status.success(), "emit accepted the wrong binary");
    assert!(
        stderr_of(&output).contains("does not match the pinned expectation"),
        "failure must name the pin: {}",
        stderr_of(&output)
    );
}

#[test]
fn duplicate_capture_check_still_fails_with_provenance_present() {
    if skip_without_script_tools(stringify!(
        duplicate_capture_check_still_fails_with_provenance_present
    )) {
        return;
    }
    let run = RunDir::new("duplicate");
    write_traditional_manifest(&run, "wine-x11-sendinput");
    run.write("artifacts/screenshots/01-loading.png", FIRST_SHOT);
    run.write("artifacts/screenshots/02-waiting.png", FIRST_SHOT);
    assert!(emit(&run, "exe").status.success());

    let output = verify(&run);
    assert!(
        !output.status.success(),
        "a copied screenshot passed verify once provenance existed"
    );
    assert!(
        stderr_of(&output).contains("DUPLICATE screenshots"),
        "the duplicate check must still speak: {}",
        stderr_of(&output)
    );
}
