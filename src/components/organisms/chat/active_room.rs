use std::ops::Deref;

use dioxus::prelude::*;
use dioxus_router::prelude::use_navigator;
use dioxus_std::{i18n::use_i18, translate};

use crate::{
    components::{
        atoms::{
            header_main::{HeaderCallOptions, HeaderEvent},
            input::InputType,
            Avatar, Close, Header, Icon,
        },
        molecules::{
            input_message::{FormMessageEvent, ReplyingTo},
            rooms::CurrentRoom,
            InputMessage, List,
        },
    },
    hooks::{
        use_messages::{use_messages, UseMessages},
        use_room::use_room,
        use_send_attach::use_send_attach,
        use_send_message::use_send_message,
    },
    pages::{chat::chat::MessageItem, route::Route},
    services::matrix::matrix::{Attachment, AttachmentStream, TimelineThread},
};

pub fn ActiveRoom(cx: Scope) -> Element {
    let i18 = use_i18(cx);
    let nav = use_navigator(cx);
    let room = use_room(cx);
    let send_message = use_send_message(cx);
    let send_attach = use_send_attach(cx);

    let replying_to = use_shared_state::<Option<ReplyingTo>>(cx).expect("Unable to use ReplyingTo");
    let timeline_thread =
        use_shared_state::<Option<TimelineThread>>(cx).expect("Unable to use TimelineThread");

    let use_m = use_messages(cx);
    let UseMessages {
        messages,
        isLoading: is_loading,
        limit: _,
        task: _,
    } = use_m.get();

    let input_placeholder = use_state::<String>(cx, || {
        translate!(i18, "chat.inputs.plain_message.placeholder")
    });

    let header_event = move |evt: HeaderEvent| {
        to_owned![room];

        match evt.value {
            HeaderCallOptions::CLOSE => {
                nav.push(Route::ChatList {});
                room.set(CurrentRoom {
                    id: String::new(),
                    name: String::new(),
                    avatar_uri: None,
                });
            }
            _ => {}
        }
    };

    let input_message_event = move |evt: HeaderEvent| {
        to_owned![replying_to];

        match evt.value {
            HeaderCallOptions::CLOSE => {
                *replying_to.write() = None;
            }
            _ => {}
        }
    };

    let on_push_message = move |evt: FormMessageEvent, send_to_thread: bool| {
        let mut reply_to = None;

        if let Some(r) = replying_to.read().deref() {
            reply_to = Some(r.event_id.clone());
        }

        send_message.send(MessageItem {
            room_id: room.get().id.clone(),
            msg: evt.value,
            reply_to,
            send_to_thread,
        });
    };

    let on_handle_attach = move |attachment: Attachment, send_to_thread: bool| {
        send_attach.send(AttachmentStream {
            attachment,
            send_to_thread,
        });
    };

    cx.render(rsx! {
            div {
                class: "active-room",
                Header {
                    text: "{room.get().name.clone()}",
                    avatar_element: render!(rsx!(
                        Avatar {
                            name: (room.get()).name.to_string(),
                            size: 32,
                            uri: room.get().avatar_uri.clone()
                        }
                    )),
                    on_event: header_event
                }
                List {
                    messages: messages.clone(),
                    thread: None,
                    is_loading: is_loading,
                    on_scroll: move |_| {
                        use_m.loadmore("{room.get().id}");
                    }
                },
                InputMessage {
                    message_type: InputType::Message,
                    placeholder: input_placeholder.get().as_str(),
                    on_submit: move |event| {
                        on_push_message(event, false)
                    },
                    on_event: input_message_event,
                    on_attach: move |event|{
                        on_handle_attach(event, false)
                    }
                }
            }

            if let Some(t) = timeline_thread.read().deref() {
                rsx!(
                    div {
                        class: "active-room__thread",
                        // thread title
                        div {
                            class: "active-room__thread__head",
                            p {
                                class: "active-room__thread__title",
                                translate!(i18, "chat.thread.title")
                            }
                            button {
                                class: "active-room__close",
                                onclick: move |_| {
                                    *timeline_thread.write() = None
                                },
                                Icon {
                                    stroke: "var(--icon-subdued)",
                                    icon: Close,
                                    height: 24,
                                    width: 24
                                }
                            }
                        }

                        // thread messages
                        List {
                            messages: vec![],
                            thread: Some(t.thread.clone()),
                            is_loading: is_loading,
                            on_scroll: move |_| {
                                use_m.loadmore("{room.get().id}");
                            }
                        },
                        InputMessage {
                            message_type: InputType::Message,
                            placeholder: input_placeholder.get().as_str(),
                            on_submit: move |event| {
                                on_push_message(event, true)
                            },
                            on_event: input_message_event,
                            on_attach: move |event|{
                                on_handle_attach(event, true)
                            }
                        }

                    }
                )
            }
    })
}
