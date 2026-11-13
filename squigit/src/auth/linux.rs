use std::collections::HashMap;
use std::time::Duration;
use zbus::blocking::connection::Builder;
use zbus::zvariant::Value;

const ACTION_ID: &str = "org.squigit.api-keys.reveal";
const DESTINATION: &str = "org.freedesktop.PolicyKit1";
const OBJECT_PATH: &str = "/org/freedesktop/PolicyKit1/Authority";
const INTERFACE: &str = "org.freedesktop.PolicyKit1.Authority";

pub fn authenticate() -> Result<bool, String> {
    if !std::path::Path::new("/usr/share/polkit-1/actions/org.squigit.policy").is_file() {
        return Err("Squigit's native authentication policy is missing. Install org.squigit.policy in /usr/share/polkit-1/actions before revealing API keys.".to_string());
    }
    let connection = Builder::system()
        .and_then(|builder| builder.method_timeout(Duration::from_secs(120)).build())
        .map_err(|error| format!("Could not connect to system authentication: {error}"))?;
    let name = connection
        .unique_name()
        .ok_or("System authentication did not assign a D-Bus identity.")?;
    let subject = (
        "system-bus-name",
        HashMap::from([("name", Value::from(name.as_str()))]),
    );
    let details = HashMap::<&str, &str>::new();
    let cancellation_id = "squigit-api-key-reveal";
    let reply = connection.call_method(
        Some(DESTINATION),
        OBJECT_PATH,
        Some(INTERFACE),
        "CheckAuthorization",
        &(subject, ACTION_ID, details, 1u32, cancellation_id),
    );
    let message = match reply {
        Ok(message) => message,
        Err(error) => {
            let _ = connection.call_method(
                Some(DESTINATION),
                OBJECT_PATH,
                Some(INTERFACE),
                "CancelCheckAuthorization",
                &(cancellation_id,),
            );
            return Err(format!("Native authentication failed: {error}"));
        }
    };
    let ((authorized, challenge, details),): ((bool, bool, HashMap<String, String>),) = message
        .body()
        .deserialize()
        .map_err(|error| format!("Invalid system authentication response: {error}"))?;
    if authorized {
        return Ok(true);
    }
    if details.contains_key("polkit.dismissed") {
        return Ok(false);
    }
    if challenge {
        return Err("No graphical authentication agent is available. Start the GNOME or KDE Polkit agent in your desktop session and try again.".to_string());
    }
    Err("System authentication was denied. Your API key remains hidden.".to_string())
}
