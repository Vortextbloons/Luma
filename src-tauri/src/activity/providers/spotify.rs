use super::{ActivityItem, ActivityType, ProgressSource, ResumeKind};

fn fingerprint(title: &str, artist: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in title.to_lowercase().bytes().chain(artist.to_lowercase().bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

pub fn activity(title: &str, artist: &str, track_id: Option<&str>, now: &str) -> ActivityItem {
    ActivityItem {
        id: format!("spotify:track:{:016x}", fingerprint(title, artist)),
        provider: "spotify".into(),
        provider_item_id: track_id.map(str::to_string),
        profile_id: None,
        item_type: ActivityType::Track,
        title: title.trim().into(),
        subtitle: {
            let artist = artist.trim();
            (!artist.is_empty()).then(|| artist.to_string())
        },
        artwork: None,
        last_activity_at: now.into(),
        progress_seconds: None,
        duration_seconds: None,
        progress_source: ProgressSource::Unknown,
        completed: None,
        can_resume: false,
        launch_target: Some("Spotify".into()),
        resume_kind: ResumeKind::Service,
        launch_count: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observed_tracks_are_stable_and_never_claim_progress() {
        let first = activity("Night Drive", "The Example", Some("track-1"), "2026-01-01T00:00:00.000Z");
        let second = activity("night drive", "the example", Some("track-1"), "2026-01-02T00:00:00.000Z");
        assert_eq!(first.id, second.id);
        assert_eq!(first.progress_seconds, None);
        assert_eq!(first.progress_source, ProgressSource::Unknown);
        assert_eq!(first.resume_kind, ResumeKind::Service);
    }
}
