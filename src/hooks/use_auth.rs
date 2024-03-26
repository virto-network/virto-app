use dioxus::prelude::*;
use gloo::storage::{errors::StorageError, LocalStorage};
use matrix_sdk::Client;
use ruma::api::client::discovery::discover_homeserver::Response as WellKnownResponse;
use ruma::api::IncomingResponse;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::pages::login::LoggedIn;

use super::use_client::UseClientState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthError {
    BuildError,
    InvalidHomeserver,
    ServerNotFound,
}

impl From<serde_json::Error> for AuthError {
    fn from(_: serde_json::Error) -> Self {
        AuthError::InvalidHomeserver
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheLogin {
    pub server: String,
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LoginInfo {
    pub server: Url,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone)]
pub struct LoginInfoBuilder {
    pub server: Option<Url>,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl LoginInfoBuilder {
    pub fn new() -> Self {
        LoginInfoBuilder {
            server: None,
            username: None,
            password: None,
        }
    }

    pub fn server(&mut self, server: Url) {
        self.server = Some(server);
    }

    pub fn username(&mut self, username: &str) {
        self.username = Some(username.to_owned());
    }

    pub fn password(&mut self, password: &str) {
        self.password = Some(password.to_owned());
    }

    pub fn build(self) -> Result<LoginInfo, AuthError> {
        if let Self {
            server: Some(server),
            username: Some(username),
            password: Some(password),
        } = self
        {
            Ok(LoginInfo {
                server,
                username,
                password,
            })
        } else {
            Err(AuthError::BuildError)
        }
    }
}

pub fn use_auth(cx: &ScopeState) -> &UseAuthState {
    let logged_in = use_shared_state::<LoggedIn>(cx).expect("Unable to use LoggedIn");
    let login_cache =
        use_shared_state::<Option<CacheLogin>>(cx).expect("Unable to read login cache");

    let auth_info = use_ref::<LoginInfoBuilder>(cx, || LoginInfoBuilder::new());
    let error = use_state(cx, || None);

    cx.use_hook(move || UseAuthState {
        data: auth_info.clone(),
        error: error.clone(),
        logged_in: logged_in.clone(),
        login_cache: login_cache.clone(),
    })
}

#[derive(Clone)]
pub struct UseAuthState {
    data: UseRef<LoginInfoBuilder>,
    error: UseState<Option<AuthError>>,
    logged_in: UseSharedState<LoggedIn>,
    login_cache: UseSharedState<Option<CacheLogin>>,
}

#[derive(Clone, Debug)]
pub struct UseAuth {
    pub data: LoginInfoBuilder,
    pub error: Option<AuthError>,
    pub logged_in: LoggedIn,
}

impl UseAuthState {
    pub async fn set_server(&self, homeserver: &str) -> Result<(), AuthError> {
        let server_parsed =
            if homeserver.starts_with("http://") || homeserver.starts_with("https://") {
                Url::parse(&homeserver)
            } else {
                Url::parse(&format!("https://{homeserver}"))
            };

        let server = server_parsed.map_err(|_| AuthError::InvalidHomeserver)?;

        let request_url = format!("{}.well-known/matrix/client", server.to_string());

        let res = reqwest::Client::new()
            .get(&request_url)
            .send()
            .await
            .map_err(|_| AuthError::InvalidHomeserver)?;

        let body = res.text().await.map_err(|_| AuthError::InvalidHomeserver)?;

        let well_response = WellKnownResponse::try_from_http_response(http::Response::new(body))
            .map_err(|_| AuthError::InvalidHomeserver)?;

        let url_base = Url::parse(&well_response.homeserver.base_url)
            .map_err(|_| AuthError::InvalidHomeserver)?;

        let result = Client::builder()
            .homeserver_url(&url_base)
            .build()
            .await
            .map(|_| url_base)
            .map_err(|_| AuthError::ServerNotFound);

        log::info!("client result: {:?}", result);

        match result {
            Ok(server) => {
                self.data.with_mut(|l| l.server(server));
                self.error.set(None);
            }
            Err(e) => {
                self.error.set(Some(e));
            }
        }

        Ok(())
    }

    pub fn set_username(&self, username: &str, parse: bool) {
        let mut username_parse = username.trim().to_string();

        if parse {
            if !username_parse.starts_with("@") {
                username_parse = format!("@{}", username_parse);
            }

            if !username_parse.contains(':') {
                let Some(server) = &self.data.read().server else {
                    return;
                };

                let Some(domain) = server.domain() else {
                    return;
                };

                let domain_name = extract_domain_name(domain);
                if !username_parse.ends_with(domain_name.as_str()) {
                    username_parse = format!("{}:{}", username_parse, domain_name);
                }
            }
        }

        self.data.with_mut(|l| {
            l.username(&username_parse);
        });
    }

    pub fn set_password(&self, password: &str) {
        self.data.with_mut(|l| {
            l.password(password.trim());
        });
    }

    pub fn set_login_cache(&self, data: CacheLogin) {
        *self.login_cache.write() = Some(data)
    }

    pub fn get_login_cache(&self) -> Option<CacheLogin> {
        self.login_cache.read().clone()
    }

    pub fn get(&self) -> UseAuth {
        UseAuth {
            data: self.data.read().clone(),
            error: self.error.get().clone(),
            logged_in: self.logged_in.read().clone(),
        }
    }

    pub fn reset(&self) {
        self.data.set(LoginInfoBuilder::new());
        self.error.set(None);

        <LocalStorage as gloo::storage::Storage>::delete("login_data");
    }

    pub fn build(&self) -> Result<LoginInfo, AuthError> {
        self.data.read().clone().build()
    }

    pub fn persist_data(&self, data: CacheLogin) -> anyhow::Result<(), StorageError> {
        let serialized_data = serde_json::to_string(&data)?;
        <LocalStorage as gloo::storage::Storage>::set("login_data", serialized_data)
    }

    pub fn get_storage_data(&self) -> anyhow::Result<String, StorageError> {
        <LocalStorage as gloo::storage::Storage>::get("login_data")
    }

    pub fn is_storage_data(&self) -> bool {
        let data = Self::get_storage_data(&self);

        data.is_ok()
    }

    pub fn is_logged_in(&self) -> LoggedIn {
        self.logged_in.read().clone()
    }

    pub fn set_logged_in(&self, option: bool) {
        *self.logged_in.write() = LoggedIn(option);
    }

    pub async fn logout(&self, client: &UseClientState, is_guest: bool) -> Result<(), LogoutError> {
        if !is_guest {
            client
                .get()
                .logout()
                .await
                .map_err(|_| LogoutError::Failed)?;
        }

        <LocalStorage as gloo::storage::Storage>::delete("session_file");

        client
            .default()
            .await
            .map_err(|_| LogoutError::DefaultClient)?;

        self.set_logged_in(false);

        Ok(())
    }
}

pub enum LogoutError {
    DefaultClient,
    RemoveSession,
    Failed,
}

fn extract_domain_name(host: &str) -> String {
    let segs: Vec<&str> = host
        .split('.')
        .filter(|&s| !s.is_empty())
        .rev()
        .collect::<Vec<&str>>();

    let suffix = segs[0];
    let domain = segs[1];

    format!("{}.{}", domain, suffix)
}
