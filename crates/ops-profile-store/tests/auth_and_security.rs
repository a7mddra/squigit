// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use std::env;
use std::path::Path;
use std::sync::{atomic::AtomicBool, Arc};
use std::thread;

use ops_profile_store::auth::{
    cache_avatar, start_google_auth_flow, AuthFlowSettings, CredentialsSource,
};
use ops_profile_store::{Profile, ProfileError, ProfileStore};
use serial_test::serial;
use tempfile::tempdir;
use tiny_http::{Header, Response, Server, StatusCode};
use url::Url;

fn free_port() -> u16 {
    std::net::TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn temp_store() -> ProfileStore {
    let dir = tempdir().unwrap();
    let root = dir.path().to_path_buf();
    std::mem::forget(dir);
    ProfileStore::with_base_dir(root.join("Local Storage")).unwrap()
}

struct EnvGuard {
    key: &'static str,
    old_value: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, value: impl Into<String>) -> Self {
        let old_value = env::var(key).ok();
        env::set_var(key, value.into());
        Self { key, old_value }
    }

    fn unset(key: &'static str) -> Self {
        let old_value = env::var(key).ok();
        env::remove_var(key);
        Self { key, old_value }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.old_value {
            env::set_var(self.key, value);
        } else {
            env::remove_var(self.key);
        }
    }
}

#[test]
fn cache_avatar_stores_bytes_in_profile_cas() {
    let store = temp_store();
    let profile = Profile::new("avatar@example.com", "Avatar User", None, None);
    store.upsert_profile(&profile).unwrap();

    let port = free_port();
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let handle = thread::spawn(move || {
        let server = Server::http(("127.0.0.1", port)).unwrap();
        ready_tx.send(()).unwrap();
        let request = server.recv().unwrap();
        assert_eq!(request.url(), "/avatar.png");
        let response = Response::from_data(vec![1u8, 2, 3, 4]).with_header(
            Header::from_bytes(&b"Content-Type"[..], &b"image/png"[..]).unwrap(),
        );
        request.respond(response).unwrap();
    });
    ready_rx.recv().unwrap();

    let local_path = cache_avatar(
        &store,
        &format!("http://127.0.0.1:{}/avatar.png", port),
        Some(&profile.id),
    )
    .unwrap();

    assert!(Path::new(&local_path).exists());
    assert!(local_path.starts_with(
        store
            .get_chats_dir(&profile.id)
            .to_string_lossy()
            .as_ref()
    ));

    let stored_profile = store.get_profile(&profile.id).unwrap().unwrap();
    assert_eq!(stored_profile.avatar.as_deref(), Some(local_path.as_str()));

    handle.join().unwrap();
}

#[test]
fn placeholder_credentials_are_rejected() {
    let store = temp_store();
    let mut settings = AuthFlowSettings::new("Squigit", Arc::new(|_| Ok(())));
    settings.credentials_source = CredentialsSource::RawJson(
        r#"{
            "installed": {
                "client_id": "replace-me.apps.googleusercontent.com",
                "client_secret": "replace-me",
                "auth_uri": "https://accounts.google.com/o/oauth2/auth",
                "token_uri": "https://oauth2.googleapis.com/token"
            }
        }"#
        .to_string(),
    );

    let err = start_google_auth_flow(&store, &settings, Arc::new(AtomicBool::new(false)))
        .unwrap_err();
    assert!(matches!(err, ProfileError::MissingCredentials(_)));
}

#[test]
#[serial]
fn auto_credentials_source_prefers_raw_env_over_path_env() {
    let store = temp_store();
    let dir = tempdir().unwrap();
    let path = dir.path().join("credentials.json");
    std::fs::write(&path, "not-json").unwrap();

    let _path_guard = EnvGuard::set(
        "SQUIGIT_GOOGLE_CREDENTIALS_PATH",
        path.to_string_lossy().to_string(),
    );
    let _json_guard = EnvGuard::set(
        "SQUIGIT_GOOGLE_CREDENTIALS_JSON",
        r#"{
            "installed": {
                "client_id": "test-client.apps.googleusercontent.com",
                "client_secret": "test-secret",
                "auth_uri": "https://example.com/o/oauth2/auth",
                "token_uri": "https://example.com/oauth2/token"
            }
        }"#,
    );

    let mut settings = AuthFlowSettings::new(
        "Squigit",
        Arc::new(|_| Err(ProfileError::Auth("browser-opened".to_string()))),
    );
    settings.credentials_source = CredentialsSource::Auto;
    settings.redirect_port = free_port();

    let err = start_google_auth_flow(&store, &settings, Arc::new(AtomicBool::new(false)))
        .unwrap_err();
    assert_eq!(err.to_string(), "browser-opened");
}

#[test]
#[serial]
fn auto_credentials_source_can_use_path_env() {
    let store = temp_store();
    let dir = tempdir().unwrap();
    let path = dir.path().join("credentials.json");
    std::fs::write(
        &path,
        r#"{
            "installed": {
                "client_id": "test-client.apps.googleusercontent.com",
                "client_secret": "test-secret",
                "auth_uri": "https://example.com/o/oauth2/auth",
                "token_uri": "https://example.com/oauth2/token"
            }
        }"#,
    )
    .unwrap();

    let _json_guard = EnvGuard::unset("SQUIGIT_GOOGLE_CREDENTIALS_JSON");
    let _path_guard = EnvGuard::set(
        "SQUIGIT_GOOGLE_CREDENTIALS_PATH",
        path.to_string_lossy().to_string(),
    );

    let mut settings = AuthFlowSettings::new(
        "Squigit",
        Arc::new(|_| Err(ProfileError::Auth("browser-opened".to_string()))),
    );
    settings.credentials_source = CredentialsSource::Auto;
    settings.redirect_port = free_port();

    let err = start_google_auth_flow(&store, &settings, Arc::new(AtomicBool::new(false)))
        .unwrap_err();
    assert_eq!(err.to_string(), "browser-opened");
}

#[test]
#[serial]
fn start_google_auth_flow_round_trips_against_stub_endpoints() {
    let store = temp_store();
    let oauth_port = free_port();
    let callback_port = free_port();
    let original_avatar = format!("http://127.0.0.1:{}/avatar", oauth_port);
    let _no_proxy_guard = EnvGuard::set("NO_PROXY", "127.0.0.1,localhost");
    let _no_proxy_lower_guard = EnvGuard::set("no_proxy", "127.0.0.1,localhost");

    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let server_handle = thread::spawn({
        let original_avatar = original_avatar.clone();
        move || {
            let server = Server::http(("127.0.0.1", oauth_port)).unwrap();
            ready_tx.send(()).unwrap();
            for _ in 0..4 {
                let request = server.recv().unwrap();
                let parsed = Url::parse(&format!("http://127.0.0.1:{}{}", oauth_port, request.url()))
                    .unwrap();

                match parsed.path() {
                    "/auth" => {
                        let redirect_uri = parsed
                            .query_pairs()
                            .find(|(key, _)| key == "redirect_uri")
                            .map(|(_, value)| value.into_owned())
                            .unwrap();
                        let state = parsed
                            .query_pairs()
                            .find(|(key, _)| key == "state")
                            .map(|(_, value)| value.into_owned())
                            .unwrap();
                        let location = format!("{}/?code=test-code&state={}", redirect_uri, state);
                        let response = Response::from_string("")
                            .with_status_code(StatusCode(302))
                            .with_header(
                                Header::from_bytes(&b"Location"[..], location.as_bytes()).unwrap(),
                            );
                        request.respond(response).unwrap();
                    }
                    "/token" => {
                        let response = Response::from_string(r#"{"access_token":"test-access"}"#)
                            .with_header(
                                Header::from_bytes(
                                    &b"Content-Type"[..],
                                    &b"application/json"[..],
                                )
                                .unwrap(),
                            );
                        request.respond(response).unwrap();
                    }
                    "/userinfo" => {
                        let response = Response::from_string(format!(
                            r#"{{
                                "names": [{{ "displayName": "Integration User" }}],
                                "emailAddresses": [{{ "value": "integration@example.com" }}],
                                "photos": [{{ "url": "{}" }}]
                            }}"#,
                            original_avatar
                        ))
                        .with_header(
                            Header::from_bytes(
                                &b"Content-Type"[..],
                                &b"application/json"[..],
                            )
                            .unwrap(),
                        );
                        request.respond(response).unwrap();
                    }
                    "/avatar" => {
                        let response = Response::from_data(vec![9u8, 8, 7, 6]).with_header(
                            Header::from_bytes(&b"Content-Type"[..], &b"image/png"[..]).unwrap(),
                        );
                        request.respond(response).unwrap();
                    }
                    path => panic!("unexpected stub path: {}", path),
                }
            }
        }
    });
    ready_rx.recv().unwrap();

    let raw_credentials = format!(
        r#"{{
            "installed": {{
                "client_id": "test-client.apps.googleusercontent.com",
                "client_secret": "test-secret",
                "auth_uri": "http://127.0.0.1:{}/auth",
                "token_uri": "http://127.0.0.1:{}/token"
            }}
        }}"#,
        oauth_port, oauth_port
    );

    let mut settings = AuthFlowSettings::new("Squigit", Arc::new(|url| {
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(ProfileError::Network)?;
        let response = client.get(url).send()?;
        if response.status() != reqwest::StatusCode::FOUND {
            return Err(ProfileError::Auth(format!(
                "Unexpected browser status: {}",
                response.status()
            )));
        }

        let location = response
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| ProfileError::Auth("OAuth stub redirect did not include Location".to_string()))?
            .to_string();

        thread::spawn(move || {
            let callback_client = reqwest::blocking::Client::builder()
                .no_proxy()
                .build()
                .unwrap();
            let _ = callback_client.get(location).send();
        });

        Ok(())
    }));
    settings.redirect_port = callback_port;
    settings.user_info_url = format!("http://127.0.0.1:{}/userinfo", oauth_port);
    settings.credentials_source = CredentialsSource::RawJson(raw_credentials);

    let result = start_google_auth_flow(&store, &settings, Arc::new(AtomicBool::new(false)))
        .unwrap();
    assert_eq!(result.name, "Integration User");
    assert_eq!(result.email, "integration@example.com");
    assert_eq!(result.original_picture.as_deref(), Some(original_avatar.as_str()));
    assert!(!result.avatar.is_empty());
    assert!(Path::new(&result.avatar).exists());

    let active_id = store.get_active_profile_id().unwrap().unwrap();
    assert_eq!(active_id, result.id);

    let stored_profile = store.get_profile(&result.id).unwrap().unwrap();
    assert_eq!(stored_profile.email, "integration@example.com");
    assert_eq!(
        stored_profile.original_avatar.as_deref(),
        Some(original_avatar.as_str())
    );
    assert_eq!(stored_profile.avatar.as_deref(), Some(result.avatar.as_str()));

    server_handle.join().unwrap();
}

#[test]
#[serial]
fn cancelled_auth_serves_failure_page_for_late_callback() {
    let store = temp_store();
    let oauth_port = free_port();
    let callback_port = free_port();
    let _no_proxy_guard = EnvGuard::set("NO_PROXY", "127.0.0.1,localhost");
    let _no_proxy_lower_guard = EnvGuard::set("no_proxy", "127.0.0.1,localhost");

    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let server_handle = thread::spawn(move || {
        let server = Server::http(("127.0.0.1", oauth_port)).unwrap();
        ready_tx.send(()).unwrap();
        let request = server.recv().unwrap();
        let parsed = Url::parse(&format!("http://127.0.0.1:{}{}", oauth_port, request.url()))
            .unwrap();
        assert_eq!(parsed.path(), "/auth");

        let redirect_uri = parsed
            .query_pairs()
            .find(|(key, _)| key == "redirect_uri")
            .map(|(_, value)| value.into_owned())
            .unwrap();
        let state = parsed
            .query_pairs()
            .find(|(key, _)| key == "state")
            .map(|(_, value)| value.into_owned())
            .unwrap();
        let location = format!("{}/?code=late-code&state={}", redirect_uri, state);
        let response = Response::from_string("")
            .with_status_code(StatusCode(302))
            .with_header(Header::from_bytes(&b"Location"[..], location.as_bytes()).unwrap());
        request.respond(response).unwrap();
    });
    ready_rx.recv().unwrap();

    let raw_credentials = format!(
        r#"{{
            "installed": {{
                "client_id": "test-client.apps.googleusercontent.com",
                "client_secret": "test-secret",
                "auth_uri": "http://127.0.0.1:{}/auth",
                "token_uri": "http://127.0.0.1:{}/token"
            }}
        }}"#,
        oauth_port, oauth_port
    );

    let (location_tx, location_rx) = std::sync::mpsc::channel();
    let mut settings = AuthFlowSettings::new("Squigit", Arc::new(move |url| {
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(ProfileError::Network)?;
        let response = client.get(url).send()?;
        let location = response
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| ProfileError::Auth("OAuth stub redirect did not include Location".to_string()))?
            .to_string();
        location_tx.send(location).unwrap();
        Ok(())
    }));
    settings.redirect_port = callback_port;
    settings.credentials_source = CredentialsSource::RawJson(raw_credentials);

    let auth_cancelled = Arc::new(AtomicBool::new(false));
    let cancel_flag = auth_cancelled.clone();
    let cancel_url = settings.cancel_url();
    let store_base_dir = store.base_dir().clone();

    let auth_thread = thread::spawn(move || {
        start_google_auth_flow(&store, &settings, cancel_flag)
    });

    let callback_location = location_rx.recv().unwrap();
    auth_cancelled.store(true, std::sync::atomic::Ordering::SeqCst);

    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .build()
        .unwrap();
    let cancel_response = client.get(&cancel_url).send().unwrap();
    assert!(cancel_response.status().is_success());

    let callback_response = client.get(&callback_location).send().unwrap();
    let callback_body = callback_response.text().unwrap();
    assert!(callback_body.contains("Authentication Expired"));
    assert!(callback_body.contains("Please close this tab and try again from Squigit."));

    let result = auth_thread.join().unwrap();
    assert!(matches!(result, Err(ProfileError::Auth(message)) if message == "Authentication expired"));

    let check_store = ProfileStore::with_base_dir(store_base_dir).unwrap();
    assert!(check_store.get_active_profile_id().unwrap().is_none());
    assert!(check_store.list_profiles().unwrap().is_empty());

    server_handle.join().unwrap();
}
