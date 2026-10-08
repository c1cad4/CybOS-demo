//! cybLaunch integration boundary.
//!
//! cybLaunch is kept as an external application/repository boundary rather
//! than copied into cybOS. The upstream repository currently contains no
//! source files, so cybOS exposes the registered integration point without
//! pretending an executable launcher is already bundled.

pub(crate) const REPOSITORY_URL: &str = "https://github.com/c1cad4/cybLaunch";
pub(crate) const STATUS: &str = "REGISTERED · EXTERNAL LAUNCHER";
pub(crate) const MODE: &str = "ADAPTER BOUNDARY · NO BUNDLED EXECUTABLE";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cyblaunch_registration_is_stable() {
        assert!(REPOSITORY_URL.starts_with("https://github.com/"));
        assert!(STATUS.contains("REGISTERED"));
        assert!(MODE.contains("ADAPTER"));
    }
}
