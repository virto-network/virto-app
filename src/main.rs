#![allow(non_snake_case)]
use chat::pages::welcome::Welcome;
use dioxus::prelude::*;
use dioxus_router::prelude::Router;
use dioxus_std::{i18n::*, translate};
use futures_util::TryFutureExt;
use gloo::storage::errors::StorageError;
use gloo::storage::LocalStorage;

use std::str::FromStr;
use unic_langid::LanguageIdentifier;
use web_sys::window;

use chat::components::atoms::{LoadingStatus, Notification, Spinner};
use chat::hooks::use_auth::use_auth;
use chat::hooks::use_client::use_client;
use chat::hooks::use_init_app::{use_init_app, BeforeSession};
use chat::hooks::use_notification::{use_notification, NotificationType};
use chat::hooks::use_session::use_session;
use chat::pages::login::Login;
use chat::pages::route::Route;
use chat::pages::signup::Signup;
use chat::services::matrix::matrix::*;
use chat::MatrixClientState;

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    dioxus_web::launch(App);
}

static EN_US: &str = include_str!("./locales/en-US.json");
static ES_ES: &str = include_str!("./locales/es-ES.json");

pub enum MainError {
    DefaultServer,
    RestoreFailed,
    SyncFailed,
}

fn App(cx: Scope) -> Element {
    if let Some(static_login_form) = window()?.document()?.get_element_by_id("static-login-form") {
        if let Some(parent) = static_login_form.parent_node() {
            let _ = parent.remove_child(&static_login_form);
        };
    };

    let navigator_language = window()
        .expect("window")
        .navigator()
        .language()
        .unwrap_or("en-US".to_string());

    let default_language = if navigator_language.starts_with("es") {
        "es-ES"
    } else {
        "en-US"
    };

    let selected_language: LanguageIdentifier = default_language
        .parse()
        .expect("can't parse es-ES language");

    let fallback_language: LanguageIdentifier = selected_language.clone();

    use_init_i18n(cx, selected_language, fallback_language, || {
        let en_us = Language::from_str(EN_US).expect("can't get EN_US language");
        let es_es = Language::from_str(ES_ES).expect("can't get ES_ES language");
        vec![en_us, es_es]
    });

    use_init_app(cx);

    let client = use_client(cx);
    let auth = use_auth(cx);
    let session = use_session(cx);
    let notification = use_notification(cx);
    let i18 = use_i18(cx);

    let matrix_client =
        use_shared_state::<MatrixClientState>(cx).expect("Unable to use matrix client");
    let before_session =
        use_shared_state::<BeforeSession>(cx).expect("Unable to use before session");

    let key_chat_common_error_sync = translate!(i18, "chat.common.error.sync");
    let key_chat_common_error_default_server = translate!(i18, "chat.common.error.default_server");
    let key_main_error_restore = translate!(i18, "main.errors.restore");

    let restoring_session = use_ref::<bool>(cx, || true);

    use_coroutine(cx, |_: UnboundedReceiver<MatrixClientState>| {
        to_owned![client, auth, restoring_session, session, notification];

        async move {
            let serialized_session: Result<String, StorageError> =
                <LocalStorage as gloo::storage::Storage>::get("session_file");

            match serialized_session {
                Ok(s) => {
                    let (c, sync_token) = restore_session(&s)
                        .await
                        .map_err(|_| MainError::RestoreFailed)?;

                    client.set(MatrixClientState {
                        client: Some(c.clone()),
                    });

                    session
                        .sync(c.clone(), sync_token)
                        .await
                        .map_err(|_| MainError::SyncFailed)?;

                    auth.set_logged_in(true);
                }
                Err(_) => {
                    client
                        .default()
                        .await
                        .map_err(|_| MainError::DefaultServer)?;
                }
            }
            restoring_session.set(false);

            Ok::<(), MainError>(())
        }
        .unwrap_or_else(move |e: MainError| {
            let message = match e {
                MainError::DefaultServer => &key_chat_common_error_default_server,
                MainError::RestoreFailed => &key_main_error_restore,
                MainError::SyncFailed => &key_chat_common_error_sync,
            };
            notification.handle_error(&message);
        })
    });

    render! {
        if notification.get().show {
            rsx!(
                Notification {
                    title: "{notification.get().title}",
                    body: "{notification.get().body}",
                    on_click: move |_| {
                       match notification.get().handle.value {
                            NotificationType::Click => {

                            },
                            NotificationType::AcceptSas(_, _) => {
                                cx.spawn({
                                    async move {
                                        todo!()
                                    }
                                });
                            }
                            NotificationType::None => {

                            }
                       }
                    }
                }
            )
        }
        rsx!(
            match &matrix_client.read().client {
                Some(_) => {
                    rsx!(div {
                        class: "page",
                        if auth.is_logged_in().0 {
                            rsx!(
                                section {
                                    class: "chat",
                                    Router::<Route> {}
                                }
                            )
                        } else if *restoring_session.read() {
                            let key_main_loading_title = translate!(
                                i18,
                                "main.loading.title"
                            );
                            rsx!(
                                LoadingStatus {
                                    text: "{key_main_loading_title}",
                                }
                            )
                        } else {
                            match *before_session.read() {
                                BeforeSession::Login => rsx!(
                                    section {
                                        class: "login",
                                        Login {}
                                    }
                                ),
                                BeforeSession::Signup => rsx!(
                                    section {
                                        class: "login",
                                        Signup {}
                                    }
                                ),
                                BeforeSession::Guest => rsx!(
                                    section {
                                        class: "login",
                                        Welcome {}
                                    }
                                )
                            }
                        }
                    })
                }
                None => rsx!(
                    div {
                        class: "spinner-dual-ring--center",
                        Spinner {}
                    }
                ),
            }
        )
    }
}
