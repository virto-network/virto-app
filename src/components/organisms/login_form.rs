use dioxus::prelude::*;
use dioxus_std::{i18n::use_i18, translate};

use crate::{
    components::atoms::{Button, Icon, Warning},
    hooks::{use_auth::use_auth, use_init_app::BeforeSession},
};

pub enum FormLoginEvent {
    CreateAccount,
    Login,
    FilledForm,
    ClearData,
    Guest,
}

#[derive(Props)]
pub struct LoginFormProps<'a> {
    title: &'a str,
    description: &'a str,
    button_text: &'a str,
    emoji: &'a str,
    #[props(!optional)]
    error: Option<&'a String>,
    body: Element<'a>,
    #[props(default = false)]
    clear_data: bool,
    on_handle: EventHandler<'a, FormLoginEvent>,
    #[props(!optional)]
    status: Option<String>,
}

pub fn LoginForm<'a>(cx: Scope<'a, LoginFormProps<'a>>) -> Element<'a> {
    let i18 = use_i18(cx);
    let auth = use_auth(cx);

    let before_session =
        use_shared_state::<BeforeSession>(cx).expect("Unable to use before session");

    render! {
        section {
            class: "login-form",
            div{
                class: "login-form__avatar",
                div {
                    class: "login-form__avatar__content",
                    "{cx.props.emoji}"
                }
            }
            h2 {
                class: "login-form__title",
                "{cx.props.title}"
            }
            p {
                class: "login-form__description",
                "{cx.props.description}"
            }

            div {
                class: "login-form__form__head",
                &cx.props.body

                if let Some(error) = cx.props.error {
                    rsx!(
                        div {
                            class: "login-form__form--error",
                            Icon {
                                stroke: "var(--secondary-red-100)",
                                height: 16,
                                width: 16,
                                icon: Warning
                            }
                            "{error}"
                        }
                    )
                }
            }

            div {
                class: "login-form__cta--filled",
                Button {
                    text: "{cx.props.button_text}",
                    status: cx.props.status.clone(),
                    on_click: move |_| {
                        cx.props.on_handle.call(FormLoginEvent::FilledForm)
                    }
                }
            }

            div {
                class: "login-form__cta--action",
                small {
                    class: "login-form__form__text",
                    if cx.props.clear_data {
                        auth.get_login_cache().map(|data| {
                            render!(
                                rsx!(
                                    p {
                                        class: "login-form__cta--another",
                                        translate!(i18, "onboard.login.user") " {data.username}?"
                                        button {
                                            class: "login-form__form__text login__form__text--color button button--tertiary",
                                            onclick: move |_| {
                                                cx.props.on_handle.call(FormLoginEvent::ClearData)
                                            },
                                            translate!(i18, "onboard.login.cta.another")
                                        }
                                    }
                                )
                            )
                        })
                    }
                    match *before_session.read() {
                        BeforeSession::Login => rsx!(
                            translate!(i18, "onboard.signup.description")
                            button {
                                class: "login-form__form__text login__form__text--color button button--tertiary",
                                onclick: move |_| {
                                        cx.props.on_handle.call(FormLoginEvent::CreateAccount)
                                },
                                translate!(i18, "onboard.signup.cta"),
                            }
                            p {
                                class: "login-form__cta--another",
                                translate!(i18, "onboard.guest.description")
                                button {
                                    class: "login-form__form__text login__form__text--color button button--tertiary",
                                    onclick: move |_| {
                                        cx.props.on_handle.call(FormLoginEvent::Guest)
                                    },
                                    translate!(i18, "onboard.guest.cta")
                                }
                            }
                        ),
                        BeforeSession::Signup => rsx!(
                            translate!(i18, "onboard.login.description")
                            button {
                                class: "login-form__form__text login__form__text--color button button--tertiary",
                                onclick: move |_| {
                                        cx.props.on_handle.call(FormLoginEvent::Login)
                                },
                                translate!(i18, "onboard.login.cta"),
                            }
                            p {
                                class: "login-form__cta--another",
                                translate!(i18, "onboard.guest.description")
                                button {
                                    class: "login-form__form__text login__form__text--color button button--tertiary",
                                    onclick: move |_| {
                                        cx.props.on_handle.call(FormLoginEvent::Guest)
                                    },
                                    translate!(i18, "onboard.guest.cta")
                                }
                            }
                        ),
                        BeforeSession::Guest => rsx!(
                            translate!(i18, "onboard.login.description")
                            button {
                                class: "login-form__form__text login__form__text--color button button--tertiary",
                                onclick: move |_| {
                                        cx.props.on_handle.call(FormLoginEvent::Login)
                                },
                                translate!(i18, "onboard.login.cta"),
                            }
                            p {
                                class: "login-form__cta--another",
                                translate!(i18, "onboard.signup.description")
                            button {
                                class: "login-form__form__text login__form__text--color button button--tertiary",
                                onclick: move |_| {
                                        cx.props.on_handle.call(FormLoginEvent::CreateAccount)
                                },
                                translate!(i18, "onboard.signup.cta"),
                            }
                            }
                        )
                    }
                }
            }
        }
    }
}
