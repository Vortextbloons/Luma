use super::{safe_artwork, ActivityItem, ActivityType, PlaybackReport, ProgressSource, ResumeKind};

pub fn youtube(report: &PlaybackReport, now: &str) -> ActivityItem {
    let thumbnail = format!(
        "https://i.ytimg.com/vi/{}/hqdefault.jpg",
        report.video_id
    );
    let artwork = safe_artwork(report.artwork.as_deref(), Some(thumbnail));
    let duration = report.duration_seconds.max(0.0);
    let position = report.position_seconds.clamp(0.0, duration);
    ActivityItem {
        id: format!("youtube:video:{}", report.video_id),
        provider: "youtube".into(),
        provider_item_id: Some(report.video_id.clone()),
        profile_id: None,
        item_type: ActivityType::Video,
        title: report.title.trim().to_string(),
        subtitle: report
            .channel
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        artwork,
        last_activity_at: now.into(),
        progress_seconds: Some(position),
        duration_seconds: Some(duration),
        progress_source: ProgressSource::Luma,
        completed: Some(report.completed),
        can_resume: false,
        launch_target: Some(format!(
            "https://www.youtube.com/tv#/watch?v={}",
            report.video_id
        )),
        resume_kind: ResumeKind::Content,
        launch_count: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn youtube_progress_is_clamped_to_duration() {
        let item = youtube(
            &PlaybackReport {
                video_id: "abcdefghijk".into(),
                title: "A video".into(),
                channel: Some("A channel".into()),
                artwork: None,
                position_seconds: 500.0,
                duration_seconds: 120.0,
                completed: false,
            },
            "2026-01-01T00:00:00.000Z",
        );
        assert_eq!(item.progress_seconds, Some(120.0));
        assert_eq!(item.progress_source, ProgressSource::Luma);
        assert_eq!(item.resume_kind, ResumeKind::Content);
        assert!(!item.can_resume);
    }
}
