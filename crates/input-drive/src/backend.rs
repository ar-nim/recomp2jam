//! Backend selection: explicit override wins, otherwise
//! auto-detect reads `DISPLAY` (the Xvfb/XTEST arm) only. The Wayland
//! fixture arm is never selected from an ambient `WAYLAND_DISPLAY` --
//! it must be pinned explicitly (issue #267 hermeticity fence).

/// The two Linux input arms behind the trait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputBackend {
    /// Headless fixture compositor with server-side pointer injection.
    WaylandFixture,
    /// Self-spawned Xvfb with XTEST injection.
    X11Xvfb,
}

impl InputBackend {
    /// CLI spelling for `--backend`.
    #[must_use]
    pub fn as_flag(&self) -> &'static str {
        match self {
            Self::WaylandFixture => "wayland",
            Self::X11Xvfb => "x11",
        }
    }

    /// Parse a `--backend` value.
    #[must_use]
    pub fn from_flag(flag: &str) -> Option<Self> {
        match flag {
            "wayland" => Some(Self::WaylandFixture),
            "x11" => Some(Self::X11Xvfb),
            _ => None,
        }
    }
}

/// Pick the arm: the explicit override wins; without one,
/// an explicitly provided `WAYLAND_DISPLAY` selects the fixture arm,
/// else `DISPLAY` selects X11, else there is nothing to drive. Pure
/// over the env snapshot so tests pin the priority without mutating
/// the process environment. `auto_backend` never feeds an ambient
/// `WAYLAND_DISPLAY` here (issue #267).
#[must_use]
pub fn select_backend(
    explicit: Option<InputBackend>,
    wayland_display: Option<&str>,
    display: Option<&str>,
) -> Option<InputBackend> {
    if let Some(backend) = explicit {
        return Some(backend);
    }
    if wayland_display.is_some_and(|socket| !socket.is_empty()) {
        return Some(InputBackend::WaylandFixture);
    }
    if display.is_some_and(|socket| !socket.is_empty()) {
        return Some(InputBackend::X11Xvfb);
    }
    None
}

/// Read the live process environment and select.
#[must_use]
pub fn auto_backend() -> Option<InputBackend> {
    // Hermeticity fence (#267): never auto-detect an ambient
    // `WAYLAND_DISPLAY` -- it may point at the owner's live session.
    // The Wayland fixture arm is opt-in only, pinned by an explicit
    // `--backend wayland` or by a caller that feeds its own socket to
    // `select_backend`. Only `DISPLAY` is read from the environment,
    // and the X11 arm overrides it with a self-spawned Xvfb.
    select_backend(None, None, std::env::var("DISPLAY").ok().as_deref())
}

#[cfg(test)]
mod backend_tests {
    use super::*;

    #[test]
    fn explicit_override_beats_both_env_vars() {
        let picked = select_backend(Some(InputBackend::X11Xvfb), Some("wl-smoke-1"), Some(":99"));
        assert_eq!(picked, Some(InputBackend::X11Xvfb));
    }

    #[test]
    fn wayland_display_selects_the_fixture_arm() {
        let picked = select_backend(None, Some("wl-smoke-1"), Some(":99"));
        assert_eq!(picked, Some(InputBackend::WaylandFixture));
    }

    #[test]
    fn display_alone_selects_the_x11_arm() {
        let picked = select_backend(None, None, Some(":99"));
        assert_eq!(picked, Some(InputBackend::X11Xvfb));
    }

    #[test]
    fn empty_env_selects_no_arm() {
        assert_eq!(select_backend(None, None, None), None);
        assert_eq!(select_backend(None, Some(""), Some("")), None);
    }

    /// Hermeticity guard (#267): an ambient `WAYLAND_DISPLAY` must never
    /// route the auto-detect to the fixture arm, even when it is set in
    /// the process environment (as it is on the owner's live KDE
    /// session). Seam-suffices: `auto_backend` is the single seam between
    /// the process environment and backend selection; the smoke example's
    /// `select_auto` is the only live caller, and it may reach the fixture
    /// arm only through an explicitly pinned socket or `--backend wayland`.
    #[test]
    fn auto_backend_ignores_ambient_wayland_display() {
        // Whatever the ambient environment holds, the fixture arm must not
        // be selected by ambient state alone.
        assert_ne!(auto_backend(), Some(InputBackend::WaylandFixture));
    }

    #[test]
    fn backend_flags_round_trip() {
        assert_eq!(
            InputBackend::from_flag(InputBackend::WaylandFixture.as_flag()),
            Some(InputBackend::WaylandFixture)
        );
        assert_eq!(
            InputBackend::from_flag(InputBackend::X11Xvfb.as_flag()),
            Some(InputBackend::X11Xvfb)
        );
        assert_eq!(InputBackend::from_flag("kms"), None);
    }
}
