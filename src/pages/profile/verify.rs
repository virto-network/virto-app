use dioxus::prelude::*;
use dioxus_std::{i18n::use_i18, translate};
use log::info;
use matrix_sdk::encryption::verification::{SasVerification, Verification};

use crate::{components::atoms::Button, hooks::{use_client::use_client, use_notification::use_notification}};

use futures_util::StreamExt;

use matrix_sdk::{
    self,
    config::SyncSettings,
    ruma::events::{
        key::verification::{
            done::{OriginalSyncKeyVerificationDoneEvent, ToDeviceKeyVerificationDoneEvent},
            key::{OriginalSyncKeyVerificationKeyEvent, ToDeviceKeyVerificationKeyEvent},
            request::ToDeviceKeyVerificationRequestEvent,
            start::{OriginalSyncKeyVerificationStartEvent, ToDeviceKeyVerificationStartEvent},
        },
        room::message::{MessageType, OriginalSyncRoomMessageEvent},
    },
    Client,
};

pub enum VerificationError {
    FlowNotFound,
    SasAcceptFailed,
    SasConfirmFailed,
    SasCancelFailed,
    SyncFailed
}

#[inline_props]
pub fn Verify(cx: Scope, id: String) -> Element {
    let _ = &id;
    let i18 = use_i18(cx);
    let notification = use_notification(cx);

    let key_verify_error_flow_not_found = translate!(i18, "verify.errors.flow_not_found");
    let key_verify_error_sas_accept = translate!(i18, "verify.errors.sas_accept");
    let key_verify_error_sas_confirm = translate!(i18, "verify.errors.sas_confirm");
    let key_verify_error_sas_cancel = translate!(i18, "verify.errors.sas_cancel");
    let key_chat_common_error_sync = translate!(i18, "verify.errors.sas_sync");

    let key_verify_unverified_cta_match = translate!(i18, "verify.unverified.cta_match");
    let key_verify_unverified_cta_disagree = translate!(i18, "verify.unverified.cta_disagree");

    let is_verified = use_ref::<bool>(cx, || false);

    let emoji = use_state::<Option<SasVerification>>(cx, || None);
    let client = use_client(cx).get();

    let task_wait_confirmation = use_coroutine(cx, |mut rx: UnboundedReceiver<SasVerification>| {
        to_owned![emoji];

        async move {
            while let Some(sas) = rx.next().await {
                emoji.set(Some(sas));
                info!("Confirm with `yes` or cancel with `no`: ");
            }
        }
    })
    .clone();

    let task_verify = use_coroutine(cx, |mut rx: UnboundedReceiver<bool>| {
        to_owned![is_verified];

        async move {
            while let Some(verify) = rx.next().await {
                is_verified.set(verify);
            }
        }
    });

    let task_handle_error = use_coroutine(cx, |mut rx: UnboundedReceiver<VerificationError>| {
        to_owned![notification, key_verify_error_flow_not_found, key_verify_error_sas_accept, key_verify_error_sas_confirm, key_verify_error_sas_cancel, key_chat_common_error_sync];

        async move {
            while let Some(e) = rx.next().await {
                let message = match e {
                    VerificationError::FlowNotFound => &key_verify_error_flow_not_found,
                    VerificationError::SasAcceptFailed => &key_verify_error_sas_accept,
                    VerificationError::SasConfirmFailed => &key_verify_error_sas_confirm,
                    VerificationError::SasCancelFailed => &key_verify_error_sas_cancel,
                    VerificationError::SyncFailed => &key_chat_common_error_sync
                };
                notification.handle_error(&message);
            }
        }
    });

    let task_handle_error_a = task_handle_error.clone();
    let task_handle_error_b = task_handle_error.clone();
    let task_handle_error_c = task_handle_error.clone();
    let task_handle_error_d = task_handle_error.clone();
    let task_verify_to_device = task_verify.clone();

    use_coroutine(cx, |mut _rx: UnboundedReceiver<()>| {
        to_owned![task_wait_confirmation, client, task_verify, task_handle_error, task_handle_error_a, task_handle_error_b, task_handle_error_c, task_handle_error_d];

        async move {
            client.add_event_handler(
                move |ev: ToDeviceKeyVerificationRequestEvent, client: Client| {
                    let task_handle_error_a = task_handle_error_a.clone();
                    async move {
                    info!("here ToDeviceKeyVerificationRequestEvent");
                    let request = client
                        .encryption()
                        .get_verification_request(&ev.sender, &ev.content.transaction_id)
                        .await
                        .expect("Request object wasn't created");

                    if let Err(_) = request.accept().await {
                        task_handle_error_a.send(VerificationError::SasAcceptFailed)
                    };
                }},
            );

            client.add_event_handler(
                move |ev: ToDeviceKeyVerificationStartEvent, client: Client| {
                    let task_handle_error_b = task_handle_error_b.clone();
                    async move {
                        if let Some(Verification::SasV1(sas)) = client
                            .encryption()
                            .get_verification(&ev.sender, ev.content.transaction_id.as_str())
                            .await
                        {
                            info!(
                                "ToDeviceKeyVerificationStartEvent Starting verification with {} {}",
                                &sas.other_device().user_id(),
                                &sas.other_device().device_id()
                            );
                            if let Err(_) = sas.accept().await {
                                task_handle_error_b.send(VerificationError::SasAcceptFailed);
                            };
                        }    
                    }
                }
            );

            client.add_event_handler(move |ev: ToDeviceKeyVerificationKeyEvent, client: Client| {
                to_owned![task_wait_confirmation];

                async move {
                    if let Some(Verification::SasV1(sas)) = client
                        .encryption()
                        .get_verification(&ev.sender, ev.content.transaction_id.as_str())
                        .await
                    {
                        task_wait_confirmation.send(sas);
                    }
                }
            });

            client.add_event_handler(
                move |ev: ToDeviceKeyVerificationDoneEvent, client: Client| {
                    let task_verify = task_verify_to_device.clone();
                    async move {
                        if let Some(Verification::SasV1(sas)) = client
                            .encryption()
                            .get_verification(&ev.sender, ev.content.transaction_id.as_str())
                            .await
                        {
                            if sas.is_done() {
                                task_verify.send(true);
                            }
                        }
                    }
                },
            );

            client.add_event_handler(
                move |ev: OriginalSyncKeyVerificationStartEvent, client: Client| {
                    let task_handle_error_c = task_handle_error_c.clone();
                    async move {
                    if let Some(Verification::SasV1(sas)) = client
                        .encryption()
                        .get_verification(&ev.sender, ev.content.relates_to.event_id.as_str())
                        .await
                    {
                        info!(
                            "OriginalSyncKeyVerificationStartEvent Starting verification with {} {}",
                            &sas.other_device().user_id(),
                            &sas.other_device().device_id()
                        );
                        if let Err(_) = sas.accept().await {
                            task_handle_error_c.send(VerificationError::SasAcceptFailed)
                        };
                    }
                }},
            );

            client.add_event_handler(
                move |ev: OriginalSyncRoomMessageEvent, client: Client| {
                    let task_handle_error_d = task_handle_error_d.clone();
                    async move {
                        info!("here OriginalSyncRoomMessageEvent");

                        if let MessageType::VerificationRequest(_) = &ev.content.msgtype {
                            let request = client
                                .encryption()
                                .get_verification_request(&ev.sender, &ev.event_id)
                                .await
                                .expect("Request object wasn't created");

                                if let Err(_) = request.accept().await {
                                    task_handle_error_d.send(VerificationError::SasAcceptFailed);
                                };
                        }
                    }
                },
            );

            client.add_event_handler(
                    |ev: OriginalSyncKeyVerificationKeyEvent, client: Client| async move {
                        if let Some(Verification::SasV1(_)) = client
                            .encryption()
                            .get_verification(&ev.sender, ev.content.relates_to.event_id.as_str())
                            .await
                        {
                            info!("here OriginalSyncKeyVerificationKeyEvent this function need task_wait_confirmation");
                        }
                    },
                );

            client.add_event_handler(
                move |ev: OriginalSyncKeyVerificationDoneEvent, client: Client| {
                    let task_verify = task_verify.clone();
                    async move {
                        if let Some(Verification::SasV1(sas)) = client
                            .encryption()
                            .get_verification(&ev.sender, ev.content.relates_to.event_id.as_str())
                            .await
                        {
                            if sas.is_done() {
                                task_verify.send(true);
                            }
                        }
                    }
                },
            );

            if let Err(_) = client.sync(SyncSettings::new()).await {
                task_handle_error.send(VerificationError::SyncFailed)
            };
        }
    });

    let on_handle_confirm = move |sas: SasVerification| {
        to_owned![is_verified, emoji, task_handle_error];

        cx.spawn({
            let sas = sas.clone();
            let is_verified = is_verified.clone();

            async move {
                if let Err(_) = sas.confirm().await {
                    task_handle_error.send(VerificationError::SasConfirmFailed)
                };

                if sas.is_done() {
                    is_verified.set(true);
                } else {
                    emoji.set(None);
                }
            }
        })
    };

    let on_handle_cancel = move |sas: SasVerification| {
        to_owned![emoji, is_verified, task_handle_error];

        cx.spawn({
            let sas = sas.clone();

            async move {
                if let Err(_) = sas.cancel().await {
                    task_handle_error.send(VerificationError::SasCancelFailed)
                };

                if sas.is_cancelled() {
                    is_verified.set(false);
                    emoji.set(None);
                }
            }
        })
    };

    render! {
        if !*is_verified.read() {
            rsx!(
                h2 {
                    class: "verify__title",
                    translate!(i18, "verify.unverified.title")
                }

                div {
                    class: "verify__spacer",
                    match emoji.get(){
                        Some(sas) => {
                            let emojis = sas.emoji().expect("emoji shoudl be available now");

                                rsx!(
                                    p {
                                        class: "verify__description",
                                        translate!(i18, "verify.unverified.question")
                                    }
                                    div {
                                        class: "verify__wrapper",
                                        emojis.into_iter().map(|emoji| {
                                            rsx!(
                                                div {
                                                    class: "verify__emojis",
                                                    span {
                                                        class: "verify__method__title",
                                                        "{emoji.symbol}"
                                                    }
                                                    p {
                                                        class: "verify__method__description",
                                                        "{emoji.description}"
                                                    }
                                                }
                                            )
                                        })
                                    }
                                    div {
                                        class: "verify__spacer row",
                                        Button {
                                            text: "{key_verify_unverified_cta_disagree}",
                                            status: None,
                                            on_click: move |_| {
                                                on_handle_cancel(sas.clone());
                                            }
                                        }
                                        Button {
                                            text: "{key_verify_unverified_cta_match}",
                                            status: None,
                                            on_click: move |_| {
                                                on_handle_confirm(sas.clone());
                                            }
                                        }
                                    }
                                )

                        }
                        None => {
                            rsx!(
                                div {
                                    class: "verify__info",
                                    translate!(i18, "verify.unverified.description")
                                }
                            )
                        }

                    }
                }

            )
        } else {
            rsx!(
                h2 {
                    class: "verify__title--verified",
                    translate!(i18, "verify.verified.title")
                }

                p {
                    class: "verify__description--verified",
                    translate!(i18, "verify.verified.description")
                }
            )
        }
    }
}
