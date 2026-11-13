use block2::RcBlock;
use objc2::runtime::Bool;
use objc2_foundation::{NSError, NSString};
use objc2_local_authentication::{LAContext, LAError, LAPolicy};
use std::sync::mpsc;
use std::time::Duration;

pub fn authenticate(reason: &str) -> Result<bool, String> {
    let context = unsafe { LAContext::new() };
    let policy = LAPolicy::DeviceOwnerAuthentication;
    unsafe { context.canEvaluatePolicy_error(policy) }
        .map_err(|error| format!("Native authentication is unavailable: {error}"))?;
    let (sender, receiver) = mpsc::channel();
    let reply = RcBlock::new(move |success: Bool, error: *mut NSError| {
        let result = if success.as_bool() {
            Ok(true)
        } else if let Some(error) = unsafe { error.as_ref() } {
            match LAError(error.code()) {
                LAError::UserCancel | LAError::SystemCancel | LAError::AppCancel => Ok(false),
                _ => Err(format!("Native authentication failed: {error}")),
            }
        } else {
            Err("Native authentication failed. Your API key remains hidden.".to_string())
        };
        let _ = sender.send(result);
    });
    unsafe {
        context.evaluatePolicy_localizedReason_reply(policy, &NSString::from_str(reason), &reply);
    }
    let result = receiver
        .recv_timeout(Duration::from_secs(120))
        .map_err(|_| "Native authentication timed out. Your API key remains hidden.".to_string());
    unsafe { context.invalidate() };
    result?
}
