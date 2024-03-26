use dioxus::prelude::*;
use dioxus_std::{i18n::use_i18, translate};
use gloo::storage::{errors::StorageError, LocalStorage};
use ruma::api::{
    client::discovery::discover_homeserver::Response as WellKnownResponse, IncomingResponse,
};
use url::Url;

use crate::{
    components::atoms::{Button, Community},
    hooks::{
        use_auth::{use_auth, AuthError},
        use_client::use_client,
        use_init_app::BeforeSession,
        use_notification::use_notification,
        use_session::use_session,
    },
    services::matrix::matrix::register_as_guest,
    MatrixClientState,
};

pub fn Welcome(cx: Scope) -> Element {
    let i18 = use_i18(cx);

    let auth = use_auth(cx);
    let client = use_client(cx);
    let session = use_session(cx);
    let notification = use_notification(cx);

    let key_welcome_communities_1 = translate!(i18, "welcome.communities.1");
    let key_welcome_communities_2 = translate!(i18, "welcome.communities.2");
    let key_welcome_communities_3 = translate!(i18, "welcome.communities.3");
    let key_welcome_communities_4 = translate!(i18, "welcome.communities.4");
    let key_welcome_communities_5 = translate!(i18, "welcome.communities.5");
    let key_welcome_communities_6 = translate!(i18, "welcome.communities.6");
    let key_welcome_cta_try = translate!(i18, "welcome.cta.try");

    let key_chat_common_error_sync = translate!(i18, "chat.common.error.sync");
    let key_chat_common_error_persist = translate!(i18, "chat.common.error.persist");

    let before_session =
        use_shared_state::<BeforeSession>(cx).expect("Unable to use before session");

    use_coroutine(cx, |_: UnboundedReceiver<()>| {
        to_owned![auth, session];
        async move {
            if session.is_guest() {
                let serialized_session: Result<String, StorageError> =
                    <LocalStorage as gloo::storage::Storage>::get("session_file");

                if serialized_session.is_ok() {
                    auth.set_logged_in(true)
                }
            }
        }
    });

    render!(div {
        class: "page--clamp",
        section {
            class: "login-form",
            div {
                class: "communities",
                div {
                    class: "community__blanck"
                }
                Community {
                    title: "{key_welcome_communities_1}",
                    icon: "🗳️",
                    background: "purple",
                }

                Community {
                    title: "{key_welcome_communities_2}",
                    icon: "💡",
                    background: "gray",
                }

                Community {
                    title: "{key_welcome_communities_3}",
                    icon: "🪩",
                    background: "yellow",
                }
                Community {
                    class: "community--center",
                    title: "{key_welcome_communities_4}",
                    icon: "🎡",
                    background: "green",
                }
                div {
                    class: "community__blanck"
                }
                div {
                    class: "community__blanck"
                }
                Community {
                    title: "{key_welcome_communities_5}",
                    icon: "🕹️",
                    background: "pink",
                }
                Community {
                    title: "{key_welcome_communities_6}",
                    icon: "🏡",
                    background: "blue",
                }
            }
            div {
                class: "welcome__content",
                h2 {
                    class: "login-form__title",
                    translate!(i18, "welcome.title")
                }
                p {
                    class: "login-form__description",
                    translate!(i18, "welcome.description")
                }
            }

            div {
                class: "login-form__cta--filled",
                Button {
                    text: "{key_welcome_cta_try}",
                    status: None,
                    on_click: move |_| {
                        cx.spawn({
                            to_owned![client, session, notification, auth, key_chat_common_error_persist, key_chat_common_error_sync];
                            async move {
                                let homeserver = client.get().homeserver().await;

                                let request_url = format!("{}.well-known/matrix/client", homeserver.to_string());

                                let res = reqwest::Client::new()
                                    .get(&request_url)
                                    .send()
                                    .await
                                    .map_err(|_| AuthError::InvalidHomeserver).unwrap();

                                let body = res.text().await.map_err(|_| AuthError::InvalidHomeserver).unwrap();

                                let well_response = WellKnownResponse::try_from_http_response(http::Response::new(body))
                                    .map_err(|_| AuthError::InvalidHomeserver).unwrap();

                                let url_base = Url::parse(&well_response.homeserver.base_url)
                                    .map_err(|_| AuthError::InvalidHomeserver).unwrap();

                                let Ok((c, serialized_session)) = register_as_guest(&url_base.to_string()).await else {
                                    return;
                                };

                                if let Err(_) = session.persist_session_file(&serialized_session) {
                                    notification.handle_error(&key_chat_common_error_persist);
                                };

                                if let Err(_) = session.sync(c.clone(), None).await {
                                    notification.handle_error(&key_chat_common_error_sync);
                                };

                                client.set(MatrixClientState { client: Some(c.clone()) });

                                auth.set_logged_in(true);
                            }
                        })
                    }
                }
            }

            div {
                class: "login-form__cta--action",
                small {
                    class: "login-form__form__text",
                    translate!(i18, "welcome.cta.login.message")
                    button {
                        class: "login-form__form__text login__form__text--color button button--tertiary",
                        onclick: move |_| {
                            *before_session.write() = BeforeSession::Login
                        },
                        translate!(i18, "welcome.cta.login.call")
                    }
                }
            }
        }
    })
}
