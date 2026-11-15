use std::sync::Mutex;
use windows::{
    core::{IUnknown, IUnknown_Vtbl, Interface, GUID, PCWSTR},
    Win32::{
        Foundation::PROPERTYKEY,
        Media::Audio::{
            eConsole, eMultimedia, eRender, IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator,
            DEVICE_STATE_ACTIVE,
        },
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize,
            StructuredStorage::{PropVariantClear, PropVariantToStringAlloc},
            CLSCTX_ALL, COINIT_MULTITHREADED, STGM_READ,
        },
    },
};

#[derive(serde::Serialize)]
pub struct Output {
    id: String,
    name: String,
}

static PREVIOUS: Mutex<Option<(String, String, String)>> = Mutex::new(None);

struct Com(bool);
impl Com {
    fn new() -> windows::core::Result<Self> {
        let result = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        if result.0 == 0x80010106u32 as i32 {
            return Ok(Self(false));
        }
        result.ok()?;
        Ok(Self(true))
    }
}
impl Drop for Com {
    fn drop(&mut self) {
        if self.0 {
            unsafe {
                CoUninitialize();
            }
        }
    }
}

fn id(device: &IMMDevice) -> windows::core::Result<String> {
    unsafe {
        let value = device.GetId()?;
        let result = value.to_string();
        CoTaskMemFree(Some(value.0.cast()));
        Ok(result?)
    }
}

pub fn outputs() -> Result<Vec<Output>, String> {
    let run = || -> windows::core::Result<Vec<Output>> {
        let _com = Com::new()?;
        unsafe {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let devices = enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)?;
            let mut outputs = Vec::new();
            for index in 0..devices.GetCount()? {
                let device = devices.Item(index)?;
                let store = device.OpenPropertyStore(STGM_READ)?;
                let mut value = store.GetValue(&PROPERTYKEY {
                    fmtid: GUID::from_u128(0xa45c254e_df1c_4efd_8020_67d146a850e0),
                    pid: 14,
                })?;
                let text = PropVariantToStringAlloc(&value);
                PropVariantClear(&mut value)?;
                let text = text?;
                let name = text.to_string();
                CoTaskMemFree(Some(text.0.cast()));
                outputs.push(Output {
                    id: id(&device)?,
                    name: name?,
                });
            }
            Ok(outputs)
        }
    };
    run().map_err(|e| format!("Audio devices could not be read: {e}"))
}

#[repr(transparent)]
#[derive(Clone)]
struct PolicyConfig(IUnknown);
unsafe impl Interface for PolicyConfig {
    type Vtable = PolicyConfigVtbl;
    const IID: GUID = GUID::from_u128(0xf8679f50_850a_41cf_9c72_430f290290c8);
}
#[repr(C)]
struct PolicyConfigVtbl {
    base: IUnknown_Vtbl,
    preceding: [usize; 10],
    set_default:
        unsafe extern "system" fn(*mut core::ffi::c_void, PCWSTR, i32) -> windows::core::HRESULT,
}

fn set(policy: &PolicyConfig, device: &str, role: i32) -> windows::core::Result<()> {
    let wide: Vec<u16> = device.encode_utf16().chain(Some(0)).collect();
    unsafe { (policy.vtable().set_default)(policy.as_raw(), PCWSTR(wide.as_ptr()), role).ok() }
}

pub fn select(selected: &str) -> Result<(), String> {
    if selected.is_empty() {
        return restore();
    }
    let run = || -> windows::core::Result<()> {
        let _com = Com::new()?;
        unsafe {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let wide: Vec<u16> = selected.encode_utf16().chain(Some(0)).collect();
            let device = enumerator.GetDevice(PCWSTR(wide.as_ptr()))?;
            if device.GetState()? != DEVICE_STATE_ACTIVE {
                return Err(windows::core::Error::from_hresult(windows::core::HRESULT(
                    0x80070490u32 as i32,
                )));
            }
            let mut previous = PREVIOUS.lock().unwrap_or_else(|e| e.into_inner());
            let console = id(&enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?)?;
            let multimedia = id(&enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?)?;
            let policy: PolicyConfig = CoCreateInstance(
                &GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9),
                None,
                CLSCTX_ALL,
            )?;
            set(&policy, selected, eConsole.0)?;
            if let Err(error) = set(&policy, selected, eMultimedia.0) {
                let _ = set(&policy, &console, eConsole.0);
                return Err(error);
            }
            if let Some(value) = previous.as_mut() {
                value.2 = selected.into();
            } else {
                *previous = Some((console, multimedia, selected.into()));
            }
            Ok(())
        }
    };
    run().map_err(|e| {
        format!("Could not use the TV audio output. Check that it is connected and turned on: {e}")
    })
}

pub fn restore() -> Result<(), String> {
    let mut previous = PREVIOUS.lock().map_err(|e| e.to_string())?;
    let Some((console, multimedia, selected)) = previous.as_ref() else {
        return Ok(());
    };
    let run = || -> windows::core::Result<()> {
        let _com = Com::new()?;
        unsafe {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let policy: PolicyConfig = CoCreateInstance(
                &GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9),
                None,
                CLSCTX_ALL,
            )?;
            if id(&enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?)? == *selected {
                set(&policy, console, eConsole.0)?;
            }
            if id(&enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?)? == *selected {
                set(&policy, multimedia, eMultimedia.0)?;
            }
            Ok(())
        }
    };
    run().map_err(|e| format!("Could not restore PC audio: {e}"))?;
    *previous = None;
    Ok(())
}
