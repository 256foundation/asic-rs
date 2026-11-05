/// First checked-in EPIC API version exposing `/errors/history`.
pub(crate) const MIN_VERSION: semver::Version = semver::Version::new(1, 39, 21);

/// EPIC firmware with the recent error history endpoint.
pub type PowerPlayV13921 = super::shared::PowerPlayBackend<true>;
