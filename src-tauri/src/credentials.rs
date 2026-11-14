#[cfg(windows)]
pub fn save(target: &str, secret: &[u8]) -> Result<(), String> {
    use windows::{
        core::PWSTR,
        Win32::Security::Credentials::{
            CredWriteW, CREDENTIALW, CRED_FLAGS, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
        },
    };
    let mut target = wide(target);
    let mut secret = secret.to_vec();
    let credential = CREDENTIALW {
        Flags: CRED_FLAGS(0),
        Type: CRED_TYPE_GENERIC,
        TargetName: PWSTR(target.as_mut_ptr()),
        Comment: PWSTR::null(),
        LastWritten: Default::default(),
        CredentialBlobSize: secret.len() as u32,
        CredentialBlob: secret.as_mut_ptr(),
        Persist: CRED_PERSIST_LOCAL_MACHINE,
        AttributeCount: 0,
        Attributes: std::ptr::null_mut(),
        TargetAlias: PWSTR::null(),
        UserName: PWSTR::null(),
    };
    unsafe { CredWriteW(&credential, 0) }
        .map_err(|error| format!("Windows Credential Manager rejected the credential: {error}"))
}

#[cfg(windows)]
pub fn read(target: &str) -> Result<Option<Vec<u8>>, String> {
    use windows::{
        core::PCWSTR,
        Win32::Security::Credentials::{CredFree, CredReadW, CRED_TYPE_GENERIC},
    };
    let target = wide(target);
    let mut credential = std::ptr::null_mut();
    let result = unsafe {
        CredReadW(
            PCWSTR(target.as_ptr()),
            CRED_TYPE_GENERIC,
            None,
            &mut credential,
        )
    };
    if result.is_err() {
        return Ok(None);
    }
    let value = unsafe {
        let credential = &*credential;
        std::slice::from_raw_parts(
            credential.CredentialBlob,
            credential.CredentialBlobSize as usize,
        )
        .to_vec()
    };
    unsafe { CredFree(credential as *const _) };
    Ok(Some(value))
}

#[cfg(windows)]
pub fn delete(target: &str) -> Result<(), String> {
    use windows::{
        core::PCWSTR,
        Win32::Security::Credentials::{CredDeleteW, CRED_TYPE_GENERIC},
    };
    let target = wide(target);
    match unsafe { CredDeleteW(PCWSTR(target.as_ptr()), CRED_TYPE_GENERIC, None) } {
        Ok(()) => Ok(()),
        Err(error) if error.code().0 as u32 == 0x8007_0490 => Ok(()),
        Err(error) => Err(format!(
            "Windows Credential Manager could not remove the credential: {error}"
        )),
    }
}

#[cfg(windows)]
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

#[cfg(not(windows))]
pub fn save(_: &str, _: &[u8]) -> Result<(), String> {
    Err("Secure credential storage requires Windows Credential Manager.".into())
}

#[cfg(not(windows))]
pub fn read(_: &str) -> Result<Option<Vec<u8>>, String> {
    Ok(None)
}

#[cfg(not(windows))]
pub fn delete(_: &str) -> Result<(), String> {
    Ok(())
}

pub fn save_text(target: &str, secret: &str) -> Result<(), String> {
    save(target, secret.as_bytes())
}

pub fn read_text(target: &str) -> Result<Option<String>, String> {
    Ok(read(target)?.map(|bytes| String::from_utf8_lossy(&bytes).into_owned()))
}
