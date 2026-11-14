use crate::credentials as store;

const TARGET_PREFIX: &str = "Luma/Activity";

fn target(provider: &str) -> String {
    format!("{TARGET_PREFIX}/{provider}")
}

pub fn save(provider: &str, secret: &str) -> Result<(), String> {
    store::save_text(&target(provider), secret)
}

pub fn read(provider: &str) -> Result<Option<String>, String> {
    store::read_text(&target(provider))
}

pub fn delete(provider: &str) -> Result<(), String> {
    store::delete(&target(provider))
}

#[cfg(test)]
mod tests {
    #[test]
    fn targets_are_scoped_to_luma() {
        assert_eq!(super::target("plex"), "Luma/Activity/plex");
    }
}
