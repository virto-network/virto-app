use dioxus::prelude::*;
use dioxus_std::{i18n::use_i18, translate};
use dioxus_router::prelude::*;
use futures::TryFutureExt;

use crate::components::atoms::{ChatConversation, Icon, LogOut, MenuItem, UserCircle};
use crate::hooks::use_auth::LogoutError;
use crate::hooks::use_notification::use_notification;
use crate::hooks::use_auth::use_auth;
use crate::hooks::use_client::use_client;
use crate::hooks::use_session::use_session;
use crate::pages::route::Route;

#[derive(Props)]
pub struct MenuProps<'a> {
    on_click: EventHandler<'a, MouseEvent>,
}

pub fn Menu<'a>(cx: Scope<'a, MenuProps<'a>>) -> Element<'a> {
    let i18 = use_i18(cx);
    let nav = use_navigator(cx);
    let client = use_client(cx);
    let auth = use_auth(cx);
    let session = use_session(cx);
    let notification = use_notification(cx);

    let key_profile = translate!(i18, "menu.profile");
    let key_chats = translate!(i18, "menu.chats");
    let key_log_out = translate!(i18, "menu.log_out");
    let key_logout_error_server = translate!(i18, "logout.error.server");
    let key_chat_common_error_default_server = translate!(i18, "logout.chat.common.error.default_server");

    let log_out = move || {
        cx.spawn({
            to_owned![client, auth, session, notification, key_logout_error_server, key_chat_common_error_default_server];

            async move {
                auth.logout(&client, session.is_guest()).await
            }.unwrap_or_else(move |e: LogoutError| {
                let message = match e {
                    LogoutError::Failed |LogoutError::DefaultClient => key_logout_error_server,
                    LogoutError::RemoveSession => key_chat_common_error_default_server,
                };
                
                notification.handle_error(&message)
            })
        });
    };
    
    cx.render(rsx! {
        div {
            class: "menu fade-in-left",
            div {
                class: "menu__content",
                ul {
                    if !session.is_guest() {
                        rsx!(
                            li {
                                MenuItem {
                                    title: "{key_profile}",
                                    icon: cx.render(rsx!(Icon {height: 24, width: 24, stroke: "var(--text-1)", icon: UserCircle})),
                                    on_click: move |event| {
                                        cx.props.on_click.call(event);
                                        nav.push(Route::Profile {});
                                    }
                                }
                             }
                        )
                    }
    
                     li {
                        MenuItem {
                            title: "{key_chats}",
                            icon: cx.render(rsx!(Icon {height: 24, width: 24, stroke: "var(--text-1)", icon: ChatConversation})),
                            on_click: move |event| {
                                cx.props.on_click.call(event);
                                nav.push(Route::ChatList {});
                            }
                        }
                     }
                }
                ul {
                    li {
                        MenuItem {
                            title: "{key_log_out}",
                            icon: cx.render(rsx!(Icon {height: 24, width: 24, stroke: "var(--text-1)", icon: LogOut})),
                            on_click: move |_| {
                                log_out()
                            }
                        }
                    }
                }
            }
        }
        
    })
}
