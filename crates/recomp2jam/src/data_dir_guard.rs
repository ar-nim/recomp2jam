//! Guardrails for the git-ignored data directories (retro 2026-11-12).
//!
//! A tracked `extracted` symlink once merged over the git-ignored
//! `extracted/` directory; git silently replaced the directory's
//! contents and the census was lost. These tests pin the two
//! guardrails that prevent recurrence: no tracked path may shadow the
//! data dirs, and `.gitignore` must ignore their bare names (a
//! trailing-slash rule matches directories only, never symlinks).

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::process::Command;

    fn repo_root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn no_tracked_path_may_shadow_the_git_ignored_data_dirs() {
        let out = Command::new("git")
            .args(["ls-files"])
            .current_dir(repo_root())
            .output();
        let Ok(out) = out else {
            return; // no git available here: nothing to check
        };
        if !out.status.success() {
            return; // not a git worktree (e.g. a source tarball)
        }
        let tracked = String::from_utf8_lossy(&out.stdout);
        for path in tracked.lines() {
            let top = path.split('/').next().unwrap_or(path);
            assert!(
                !matches!(top, "install-root" | "extracted"),
                "tracked path {path:?} shadows a git-ignored data directory — \
                 a merge can silently delete local data (2026-11-12 incident)"
            );
        }
    }

    #[test]
    fn gitignore_bare_names_cover_symlinks_too() {
        let text =
            std::fs::read_to_string(repo_root().join(".gitignore")).expect(".gitignore readable");
        for need in ["install-root", "extracted"] {
            assert!(
                text.lines().any(|l| l.trim() == need),
                ".gitignore must carry the bare name {need:?}: a trailing-slash \
                 rule matches directories only, not symlinks"
            );
        }
    }
}
