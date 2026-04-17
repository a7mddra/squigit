// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CitationSource {
    pub title: String,
    pub url: String,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favicon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebSearchResult {
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_url: Option<String>,
    pub context_markdown: String,
    pub sources: Vec<CitationSource>,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchFailureClass {
    ProxyTransport,
    ConnectTimeout,
    ReadTimeout,
    Challenge,
    HttpStatus,
    NoResults,
    Dns,
    InvalidUrl,
    BlockedTarget,
    Parse,
    Other,
}

impl SearchFailureClass {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            SearchFailureClass::ProxyTransport => "proxy_transport",
            SearchFailureClass::ConnectTimeout => "connect_timeout",
            SearchFailureClass::ReadTimeout => "read_timeout",
            SearchFailureClass::Challenge => "challenge",
            SearchFailureClass::HttpStatus => "http_status",
            SearchFailureClass::NoResults => "no_results",
            SearchFailureClass::Dns => "dns",
            SearchFailureClass::InvalidUrl => "invalid_url",
            SearchFailureClass::BlockedTarget => "blocked_target",
            SearchFailureClass::Parse => "parse",
            SearchFailureClass::Other => "other",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SearchError {
    pub(crate) kind: SearchFailureClass,
    pub(crate) message: String,
    pub(crate) retriable: bool,
}

impl SearchError {
    pub(crate) fn fatal(kind: SearchFailureClass, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            retriable: false,
        }
    }

    pub(crate) fn retriable(kind: SearchFailureClass, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            retriable: true,
        }
    }

    pub(crate) fn public_message(&self) -> String {
        format!("[{}] {}", self.kind.as_str(), self.message)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransportRoute {
    Direct,
    Proxy,
}

impl TransportRoute {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            TransportRoute::Direct => "direct",
            TransportRoute::Proxy => "proxy",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchBackend {
    DuckDuckGo,
    Mojeek,
}

impl SearchBackend {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            SearchBackend::DuckDuckGo => "ddg",
            SearchBackend::Mojeek => "mojeek",
        }
    }
}
