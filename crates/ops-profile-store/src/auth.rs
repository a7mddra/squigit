// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use base64::{engine::general_purpose, Engine as _};
use ops_chat_storage::ChatStorage;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Once,
};
use std::time::{Duration, Instant};
use tiny_http::{Header, Response, Server, StatusCode};
use url::Url;

use crate::{Profile, ProfileError, ProfileStore, Result};

const EMBEDDED_SECRETS_JSON: &str = include_str!(env!("SQUIGIT_GOOGLE_CREDENTIALS_EMBEDDED_FILE"));
const SUCCESS_TEMPLATE: &str = include_str!("../assets/oauth/success.html");
const FAILURE_TEMPLATE: &str = include_str!("../assets/oauth/failure.html");
const FAVICON_BYTES: &[u8] = include_bytes!("../assets/oauth/favicon.png");
const DEFAULT_USER_INFO_URL: &str =
    "https://people.googleapis.com/v1/people/me?personFields=names,emailAddresses,photos";
const CANCELLED_CALLBACK_GRACE: Duration = Duration::from_secs(10);
static AUTH_MISSING_CREDENTIALS_LOG_ONCE: Once = Once::new();

pub type BrowserOpener = Arc<dyn Fn(&str) -> Result<()> + Send + Sync>;

#[derive(Clone, Debug)]
pub enum CredentialsSource {
    Auto,
    RawJson(String),
    File(PathBuf),
}

#[derive(Clone)]
pub struct AuthFlowSettings {
    pub app_name: String,
    pub redirect_host: String,
    pub redirect_port: u16,
    pub user_info_url: String,
    pub timeout: Duration,
    pub credentials_source: CredentialsSource,
    pub open_browser: BrowserOpener,
}

impl fmt::Debug for AuthFlowSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthFlowSettings")
            .field("app_name", &self.app_name)
            .field("redirect_host", &self.redirect_host)
            .field("redirect_port", &self.redirect_port)
            .field("user_info_url", &self.user_info_url)
            .field("timeout", &self.timeout)
            .field("credentials_source", &self.credentials_source)
            .finish()
    }
}

impl AuthFlowSettings {
    pub fn new(app_name: impl Into<String>, open_browser: BrowserOpener) -> Self {
        Self {
            app_name: app_name.into(),
            redirect_host: "127.0.0.1".to_string(),
            redirect_port: 3000,
            user_info_url: DEFAULT_USER_INFO_URL.to_string(),
            timeout: Duration::from_secs(120),
            credentials_source: CredentialsSource::Auto,
            open_browser,
        }
    }

    pub fn redirect_uri(&self) -> String {
        format!("http://{}:{}", self.redirect_host, self.redirect_port)
    }

    pub fn cancel_path(&self) -> String {
        format!("/{}-cancel", self.app_name.to_lowercase())
    }

    pub fn cancel_url(&self) -> String {
        format!("{}{}", self.redirect_uri(), self.cancel_path())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthSuccessData {
    pub id: String,
    pub name: String,
    pub email: String,
    pub avatar: String,
    pub original_picture: Option<String>,
}

#[derive(Deserialize, Debug)]
struct GoogleCredentials {
    installed: Option<OAuthConfig>,
    web: Option<OAuthConfig>,
}

#[derive(Deserialize, Debug, Clone)]
struct OAuthConfig {
    client_id: String,
    client_secret: String,
    auth_uri: String,
    token_uri: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct UserProfile {
    names: Option<Vec<Name>>,
    #[serde(rename = "emailAddresses")]
    email_addresses: Option<Vec<Email>>,
    photos: Option<Vec<Photo>>,
}

#[derive(Deserialize)]
struct Name {
    #[serde(rename = "displayName")]
    display_name: Option<String>,
}

#[derive(Deserialize)]
struct Email {
    value: Option<String>,
}

#[derive(Deserialize)]
struct Photo {
    url: Option<String>,
}

fn missing_credentials_message() -> String {
    "Google authentication is not configured in this build. The app can run normally, but sign-in is disabled.\n\nTo enable Google auth, provide credentials using one of:\n- copy crates/ops-profile-store/assets/oauth/credentials.example.json to crates/ops-profile-store/assets/oauth/credentials.json (gitignored)\n- SQUIGIT_GOOGLE_CREDENTIALS_PATH=<absolute path to credentials.json>\n- SQUIGIT_GOOGLE_CREDENTIALS_JSON=<raw credentials json>".to_string()
}

fn load_google_credentials_raw(source: &CredentialsSource) -> Result<String> {
    match source {
        CredentialsSource::RawJson(raw) => Ok(raw.clone()),
        CredentialsSource::File(path) => Ok(fs::read_to_string(path)?),
        CredentialsSource::Auto => {
            if let Ok(raw) = std::env::var("SQUIGIT_GOOGLE_CREDENTIALS_JSON") {
                if !raw.trim().is_empty() {
                    return Ok(raw);
                }
            }

            if let Ok(path) = std::env::var("SQUIGIT_GOOGLE_CREDENTIALS_PATH") {
                let trimmed = path.trim();
                if !trimmed.is_empty() {
                    return fs::read_to_string(trimmed).map_err(|err| {
                        ProfileError::Auth(format!(
                            "Failed reading SQUIGIT_GOOGLE_CREDENTIALS_PATH: {}",
                            err
                        ))
                    });
                }
            }

            Ok(EMBEDDED_SECRETS_JSON.to_string())
        }
    }
}

fn is_placeholder_config(config: &OAuthConfig) -> bool {
    config.client_id.contains("replace-me")
        || config.client_secret.contains("replace-me")
        || config.client_id.trim().is_empty()
        || config.client_secret.trim().is_empty()
}

fn load_google_oauth_config(settings: &AuthFlowSettings) -> Result<OAuthConfig> {
    let raw = load_google_credentials_raw(&settings.credentials_source)?;
    let raw = raw.trim();
    if raw.is_empty() {
        let message = missing_credentials_message();
        AUTH_MISSING_CREDENTIALS_LOG_ONCE.call_once(|| {
            eprintln!("[auth] {}", message.replace('\n', "\n[auth] "));
        });
        return Err(ProfileError::MissingCredentials(message));
    }

    let wrapper: GoogleCredentials = serde_json::from_str(raw).map_err(|err| {
        ProfileError::Auth(format!("Failed to parse Google OAuth credentials: {}", err))
    })?;

    let config = wrapper.installed.or(wrapper.web).ok_or_else(|| {
        ProfileError::Auth(
            "Invalid credentials.json: missing 'installed' or 'web' object".to_string(),
        )
    })?;

    if is_placeholder_config(&config) {
        let message = missing_credentials_message();
        AUTH_MISSING_CREDENTIALS_LOG_ONCE.call_once(|| {
            eprintln!("[auth] {}", message.replace('\n', "\n[auth] "));
        });
        return Err(ProfileError::MissingCredentials(message));
    }

    Ok(config)
}

pub fn validate_google_credentials(settings: &AuthFlowSettings) -> Result<()> {
    load_google_oauth_config(settings).map(|_| ())
}

fn generate_state_token() -> String {
    use rand::{rngs::OsRng, RngCore};

    let mut bytes = [0u8; 16];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

pub fn cache_avatar(
    store: &ProfileStore,
    url: &str,
    profile_id: Option<&str>,
) -> Result<String> {
    let target_id = match profile_id {
        Some(id) => id.to_string(),
        None => store
            .get_active_profile_id()?
            .ok_or_else(|| ProfileError::Auth("No active profile and no profile ID provided.".to_string()))?,
    };

    let client = Client::new();
    let response = client.get(url).send()?;
    if !response.status().is_success() {
        return Err(ProfileError::Auth(format!(
            "Failed to download avatar: HTTP {}",
            response.status()
        )));
    }

    let bytes = response.bytes()?;
    let chats_dir = store.get_chats_dir(&target_id);
    let storage = ChatStorage::with_base_dir(chats_dir)
        .map_err(|err| ProfileError::Auth(format!("Failed to initialize storage: {}", err)))?;
    let stored_image = storage
        .store_image(&bytes, None)
        .map_err(|err| ProfileError::Auth(format!("Failed to store avatar: {}", err)))?;

    let local_path = stored_image.path.clone();
    if let Some(mut profile) = store.get_profile(&target_id)? {
        profile.avatar = Some(local_path.clone());
        store.upsert_profile(&profile)?;
    }

    Ok(local_path)
}

pub fn start_google_auth_flow(
    store: &ProfileStore,
    settings: &AuthFlowSettings,
    auth_cancelled: Arc<AtomicBool>,
) -> Result<AuthSuccessData> {
    let secrets = load_google_oauth_config(settings)?;
    let bind_addr = format!("{}:{}", settings.redirect_host, settings.redirect_port);
    let server = Server::http(&bind_addr).map_err(|err| {
        ProfileError::Auth(format!(
            "Failed to start auth server on {}: {}",
            bind_addr, err
        ))
    })?;

    let expected_state = generate_state_token();
    let redirect_uri = settings.redirect_uri();

    let mut auth_url = Url::parse(&secrets.auth_uri)?;
    auth_url
        .query_pairs_mut()
        .append_pair("client_id", &secrets.client_id)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", "profile email")
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent")
        .append_pair("state", &expected_state);

    (settings.open_browser)(auth_url.as_str())?;

    let started_at = Instant::now();
    let mut cancelled_callback_deadline: Option<Instant> = None;
    loop {
        if auth_cancelled.load(Ordering::SeqCst) && cancelled_callback_deadline.is_none() {
            cancelled_callback_deadline = Some(Instant::now() + CANCELLED_CALLBACK_GRACE);
        }

        if let Some(deadline) = cancelled_callback_deadline {
            if Instant::now() >= deadline {
                return Err(ProfileError::Auth("Authentication cancelled".to_string()));
            }
        }

        if started_at.elapsed() > settings.timeout {
            return Err(ProfileError::Auth("Authentication timed out".to_string()));
        }

        let request = match server.recv_timeout(Duration::from_millis(250)) {
            Ok(Some(request)) => request,
            Ok(None) => continue,
            Err(err) => {
                return Err(ProfileError::Auth(format!(
                    "Failed waiting for OAuth callback: {}",
                    err
                )))
            }
        };

        let request_url = request.url().to_string();
        if request_url == "/favicon.ico" || request_url == "/favicon.png" {
            let response = Response::from_data(FAVICON_BYTES.to_vec()).with_header(
                Header::from_bytes(&b"Content-Type"[..], &b"image/png"[..]).unwrap(),
            );
            let _ = request.respond(response);
            continue;
        }

        if request_url == settings.cancel_path() {
            cancelled_callback_deadline = Some(Instant::now() + CANCELLED_CALLBACK_GRACE);
            let _ = request.respond(Response::empty(StatusCode(200)));
            continue;
        }

        if auth_cancelled.load(Ordering::SeqCst) || started_at.elapsed() > settings.timeout {
            let _ = respond_failure(
                request,
                "Authentication Expired",
                "<p>The authentication request has expired or was cancelled.</p><p>Please close this tab and try again from Squigit.</p>",
            );
            return Err(ProfileError::Auth("Authentication expired".to_string()));
        }

        let callback_url = format!("{}{}", redirect_uri, request_url);
        let url = match Url::parse(&callback_url) {
            Ok(url) => url,
            Err(err) => {
                let _ = respond_failure(
                    request,
                    "Authentication Failed",
                    "<p>The OAuth callback URL was invalid.</p><p>Please close this tab and try again.</p>",
                );
                return Err(ProfileError::Auth(format!(
                    "Failed to parse OAuth callback URL: {}",
                    err
                )));
            }
        };

        if let Some((_, error_code)) = url.query_pairs().find(|(key, _)| key == "error") {
            let message = format!("<p>Google returned an error while signing in: <strong>{}</strong>.</p><p>Please close this tab and try again.</p>", error_code);
            let _ = respond_failure(request, "Authentication Failed", &message);
            return Err(ProfileError::Auth(format!(
                "Google sign-in returned an error: {}",
                error_code
            )));
        }

        let returned_state = url
            .query_pairs()
            .find(|(key, _)| key == "state")
            .map(|(_, value)| value.into_owned());
        if returned_state.as_deref() != Some(expected_state.as_str()) {
            let _ = respond_failure(
                request,
                "Authentication Failed",
                "<p>The OAuth callback state was invalid. This request may be stale or tampered with.</p><p>Please close this tab and try again.</p>",
            );
            return Err(ProfileError::Auth(
                "OAuth callback state mismatch".to_string(),
            ));
        }

        let Some((_, code)) = url.query_pairs().find(|(key, _)| key == "code") else {
            let _ = respond_failure(
                request,
                "Authentication Failed",
                "<p>No authorization code was returned by Google.</p><p>Please close this tab and try again.</p>",
            );
            return Err(ProfileError::Auth(
                "No authorization code found in callback".to_string(),
            ));
        };

        let client = Client::new();
        let token_res = match client
            .post(&secrets.token_uri)
            .form(&[
                ("client_id", secrets.client_id.clone()),
                ("client_secret", secrets.client_secret.clone()),
                ("code", code.to_string()),
                ("grant_type", "authorization_code".to_string()),
                ("redirect_uri", redirect_uri.clone()),
            ])
            .send()
        {
            Ok(response) => response,
            Err(err) => {
                let _ = respond_failure(
                    request,
                    "Authentication Failed",
                    "<p>Token exchange with Google failed.</p><p>Please close this tab and try again.</p>",
                );
                return Err(ProfileError::Auth(format!("Token exchange failed: {}", err)));
            }
        };

        if !token_res.status().is_success() {
            let _ = respond_failure(
                request,
                "Authentication Failed",
                "<p>Google refused the authorization code exchange.</p><p>Please close this tab and try again.</p>",
            );
            return Err(ProfileError::Auth(format!(
                "Google refused token exchange: HTTP {}",
                token_res.status()
            )));
        }

        let token_data: TokenResponse = match token_res.json() {
            Ok(token_data) => token_data,
            Err(err) => {
                let _ = respond_failure(
                    request,
                    "Authentication Failed",
                    "<p>Google returned an unexpected token response.</p><p>Please close this tab and try again.</p>",
                );
                return Err(ProfileError::Auth(format!(
                    "Failed to decode Google token response: {}",
                    err
                )));
            }
        };

        let profile_res = match client
            .get(&settings.user_info_url)
            .bearer_auth(&token_data.access_token)
            .send()
        {
            Ok(profile_res) => profile_res,
            Err(err) => {
                let _ = respond_failure(
                    request,
                    "Authentication Failed",
                    "<p>Google profile lookup failed.</p><p>Please close this tab and try again.</p>",
                );
                return Err(ProfileError::Auth(format!("Profile fetch failed: {}", err)));
            }
        };

        if !profile_res.status().is_success() {
            let _ = respond_failure(
                request,
                "Authentication Failed",
                "<p>Google profile lookup failed.</p><p>Please close this tab and try again.</p>",
            );
            return Err(ProfileError::Auth(format!(
                "Google profile lookup failed: HTTP {}",
                profile_res.status()
            )));
        }

        let profile: UserProfile = match profile_res.json() {
            Ok(profile) => profile,
            Err(err) => {
                let _ = respond_failure(
                    request,
                    "Authentication Failed",
                    "<p>Google returned an unexpected profile response.</p><p>Please close this tab and try again.</p>",
                );
                return Err(ProfileError::Auth(format!(
                    "Failed to decode Google profile response: {}",
                    err
                )));
            }
        };

        let name = profile
            .names
            .and_then(|items| items.first().and_then(|item| item.display_name.clone()))
            .unwrap_or_else(|| format!("{} User", settings.app_name));
        let email = profile
            .email_addresses
            .and_then(|items| items.first().and_then(|item| item.value.clone()))
            .unwrap_or_default();

        if email.trim().is_empty() {
            let _ = respond_failure(
                request,
                "Authentication Failed",
                "<p>Google did not return an email address for this account.</p><p>Please close this tab and try again.</p>",
            );
            return Err(ProfileError::Auth(
                "Google profile response did not include an email address".to_string(),
            ));
        }

        let mut avatar_url = profile
            .photos
            .and_then(|items| items.first().and_then(|item| item.url.clone()))
            .unwrap_or_default();

        let original_picture = if avatar_url.trim().is_empty() {
            None
        } else {
            if avatar_url.starts_with("http://")
                && !avatar_url.starts_with("http://127.0.0.1")
                && !avatar_url.starts_with("http://localhost")
            {
                avatar_url = avatar_url.replacen("http://", "https://", 1);
            }
            Some(avatar_url.clone())
        };

        let profile_id = Profile::id_from_email(&email);
        let local_avatar = if let Some(url) = original_picture.as_deref() {
            cache_avatar(store, url, Some(&profile_id)).unwrap_or_default()
        } else {
            String::new()
        };

        let mut profile = Profile::new(
            &email,
            &name,
            if local_avatar.is_empty() {
                None
            } else {
                Some(local_avatar.clone())
            },
            original_picture.clone(),
        );
        if let Some(existing_profile) = store.get_profile(&profile.id)? {
            profile.created_at = existing_profile.created_at;
        }
        profile.touch();

        if let Err(err) = store.upsert_profile(&profile) {
            let _ = respond_failure(
                request,
                "Authentication Failed",
                "<p>Squigit could not save your profile locally.</p><p>Please close this tab and try again.</p>",
            );
            return Err(err);
        }
        if let Err(err) = store.set_active_profile_id(&profile.id) {
            let _ = respond_failure(
                request,
                "Authentication Failed",
                "<p>Squigit could not activate your profile locally.</p><p>Please close this tab and try again.</p>",
            );
            return Err(err);
        }

        let user_data = AuthSuccessData {
            id: profile.id.clone(),
            name: profile.name.clone(),
            email: profile.email.clone(),
            avatar: local_avatar,
            original_picture,
        };

        respond_success(
            request,
            "Authentication Successful",
            &format!(
                "<p>{} is now connected to your Google Account.</p><p>You can close this tab.</p>",
                settings.app_name
            ),
        )?;

        return Ok(user_data);
    }
}

fn respond_success(request: tiny_http::Request, title: &str, content: &str) -> Result<()> {
    respond_html(
        request,
        SUCCESS_TEMPLATE,
        title,
        content,
        "Confirmation",
        false,
    )
}

fn respond_failure(request: tiny_http::Request, title: &str, content: &str) -> Result<()> {
    respond_html(request, FAILURE_TEMPLATE, title, content, "Error", true)
}

fn respond_html(
    request: tiny_http::Request,
    template: &str,
    title: &str,
    content: &str,
    breadcrumb: &str,
    is_error: bool,
) -> Result<()> {
    let title_color = if is_error { "#d93025" } else { "#202124" };
    let dynamic_style = format!("<style>:root {{ --title-color: {}; }}</style>", title_color);
    let favicon_href = format!(
        "data:image/png;base64,{}",
        general_purpose::STANDARD.encode(FAVICON_BYTES)
    );
    let html = template
        .replace("${title}", title)
        .replace("${dynamicStyle}", &dynamic_style)
        .replace("${faviconHref}", &favicon_href)
        .replace("${breadcrumb}", breadcrumb)
        .replace("${bodyContent}", content);

    let response = Response::from_string(html).with_header(
        Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap(),
    );
    request.respond(response)?;
    Ok(())
}
