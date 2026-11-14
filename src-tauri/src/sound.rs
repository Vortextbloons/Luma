#[cfg(windows)]
fn play(bytes: &[u8]) -> bool {
    use windows::{core::PCWSTR, Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT}};
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return false;
    }
    unsafe { PlaySoundW(PCWSTR(bytes.as_ptr() as *const u16), None, SND_MEMORY | SND_ASYNC | SND_NODEFAULT).as_bool() }
}

#[cfg(not(windows))]
fn play(_: &[u8]) -> bool { false }

#[cfg(windows)]
fn scaled(bytes: &[u8], gain: f32) -> Vec<u8> {
    let mut data = bytes.to_vec();
    let mut offset = 12;
    while offset + 8 <= data.len() {
        let size = u32::from_le_bytes(match data[offset + 4..offset + 8].try_into() { Ok(value) => value, Err(_) => return data }) as usize;
        if &data[offset..offset + 4] == b"data" {
            let start = offset + 8;
            let end = (start + size).min(data.len());
            let mut index = start;
            while index + 1 < end {
                let sample = i16::from_le_bytes([data[index], data[index + 1]]);
                let value = (sample as f32 * gain).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                data[index..index + 2].copy_from_slice(&value.to_le_bytes());
                index += 2;
            }
            return data;
        }
        offset = offset + 8 + size + (size & 1);
    }
    data
}

#[cfg(not(windows))]
fn scaled(bytes: &[u8], _: f32) -> Vec<u8> {
    bytes.to_vec()
}

#[tauri::command]
pub fn play_startup_sound(preset: String, level: String) -> Result<(), String> {
    let bytes: &[u8] = match preset.as_str() {
        "google" => include_bytes!("../../src/assets/sounds/google/startup.wav"),
        "roku" => include_bytes!("../../src/assets/sounds/roku/startup.wav"),
        "off" => return Ok(()),
        _ => include_bytes!("../../src/assets/sounds/real/startup.wav"),
    };
    let level = match level.as_str() {
        "soft" => 0.5,
        "loud" => 1.15,
        _ => 0.8,
    };
    if play(Box::leak(scaled(bytes, 0.35 * level).into_boxed_slice())) { Ok(()) } else { Err("Startup sound could not play.".into()) }
}
