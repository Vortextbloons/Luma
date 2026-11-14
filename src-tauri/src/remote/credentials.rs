use crate::credentials as store;

const TARGET_PREFIX: &str = "Luma/Remote";

fn target(kind: &str) -> String {
    format!("{TARGET_PREFIX}/{kind}")
}

pub fn save(kind: &str, secret: &str) -> Result<(), String> {
    store::save_text(&target(kind), secret)
}

pub fn read(kind: &str) -> Result<Option<String>, String> {
    store::read_text(&target(kind))
}

pub fn delete(kind: &str) -> Result<(), String> {
    store::delete(&target(kind))
}

#[cfg(test)]
mod tests {
    #[test]
    fn targets_are_scoped_to_luma_remotes() {
        assert_eq!(super::target("display"), "Luma/Remote/display");
        assert_eq!(super::target("phone"), "Luma/Remote/phone");
    }
}
