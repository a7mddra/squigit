use windows::core::{factory, HSTRING};
use windows::Security::Credentials::UI::{
    UserConsentVerificationResult, UserConsentVerifier, UserConsentVerifierAvailability,
};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::WinRT::{
    IUserConsentVerifierInterop, RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED,
};
use windows_future::IAsyncOperation;

struct WinRtApartment;

impl Drop for WinRtApartment {
    fn drop(&mut self) {
        unsafe { RoUninitialize() };
    }
}

pub fn authenticate(window_handle: &[u8], reason: &str) -> Result<bool, String> {
    let bytes: [u8; std::mem::size_of::<usize>()] = window_handle
        .try_into()
        .map_err(|_| "Native authentication needs a valid Squigit window handle.".to_string())?;
    let handle = usize::from_le_bytes(bytes);
    if handle == 0 {
        return Err("Native authentication needs an open Squigit window.".to_string());
    }
    unsafe { RoInitialize(RO_INIT_MULTITHREADED) }
        .map_err(|error| format!("Could not initialize Windows Hello: {error}"))?;
    let _apartment = WinRtApartment;
    let availability = UserConsentVerifier::CheckAvailabilityAsync()
        .and_then(|operation| operation.join())
        .map_err(|error| format!("Could not check Windows Hello availability: {error}"))?;
    if availability != UserConsentVerifierAvailability::Available {
        return Err("Windows Hello is unavailable. Set up a Windows Hello PIN, fingerprint, or face recognition in Windows Settings.".to_string());
    }
    let interop: IUserConsentVerifierInterop = factory::<UserConsentVerifier, _>()
        .map_err(|error| format!("Could not access Windows Hello: {error}"))?;
    let operation: IAsyncOperation<UserConsentVerificationResult> = unsafe {
        interop.RequestVerificationForWindowAsync(
            HWND(handle as *mut std::ffi::c_void),
            &HSTRING::from(reason),
        )
    }
    .map_err(|error| format!("Could not open Windows Hello: {error}"))?;
    match operation
        .join()
        .map_err(|error| format!("Windows Hello authentication failed: {error}"))?
    {
        UserConsentVerificationResult::Verified => Ok(true),
        UserConsentVerificationResult::Canceled => Ok(false),
        _ => Err("Windows Hello authentication failed. Your API key remains hidden.".to_string()),
    }
}
