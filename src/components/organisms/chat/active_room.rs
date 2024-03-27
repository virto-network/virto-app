use dioxus::prelude::*;
use dioxus_router::prelude::use_navigator;
use dioxus_std::{i18n::use_i18, translate};
use futures::TryFutureExt;

use crate::{
    components::{
        atoms::{
            header_main::{HeaderCallOptions, HeaderEvent},
            ArrowDownCircle, ArrowUpCircle, Avatar, Close, Exit, Header, Icon,
        },
        molecules::{input_message::FormMessageEvent, rooms::CurrentRoom, InputMessage, List},
    },
    hooks::{
        use_chat::{use_chat, UseChat},
        use_client::use_client,
        use_lifecycle::use_lifecycle,
        use_messages::use_messages,
        use_notification::use_notification,
        use_reply::use_reply,
        use_room::use_room,
        use_rooms::use_rooms,
        use_send_attach::use_send_attach,
        use_send_message::use_send_message,
        use_thread::use_thread,
    },
    pages::{chat::chat::MessageItem, route::Route},
    services::matrix::matrix::{leave_room, Attachment, AttachmentStream, LeaveRoomError},
};

#[derive(Props)]
pub struct ActiveRoomProps<'a> {
    on_back: EventHandler<'a, ()>,
}
pub fn ActiveRoom<'a>(cx: Scope<'a, ActiveRoomProps<'a>>) -> Element<'a> {
    let i18 = use_i18(cx);
    let nav = use_navigator(cx);
    let room = use_room(cx);
    let rooms = use_rooms(cx);
    let messages = use_messages(cx);
    let client = use_client(cx);
    let notification = use_notification(cx);
    let send_message = use_send_message(cx);
    let send_attach = use_send_attach(cx);

    let replying_to = use_reply(cx);
    let threading_to = use_thread(cx);

    let use_m = use_chat(cx);
    let UseChat {
        messages: _,
        isLoading: is_loading,
        limit: _,
        task: _,
    } = use_m.get();

    let messages_lifecycle = messages.clone();
    let replying_to_lifecycle = replying_to.clone();
    let threading_to_lifecycle = threading_to.clone();
    let messages = messages.get();

    let key_chat_common_error_room_id = translate!(i18, "chat.common.error.room_id");
    let key_chat_common_error_room_not_found = translate!(i18, "chat.common.error.room_not_found");
    let key_chat_actions_leave = translate!(i18, "chat.actions.leave");

    let input_placeholder = use_state::<String>(cx, || {
        translate!(i18, "chat.inputs.plain_message.placeholder")
    });

    use_lifecycle(
        &cx,
        || {},
        move || {
            to_owned![
                messages_lifecycle,
                replying_to_lifecycle,
                threading_to_lifecycle
            ];

            messages_lifecycle.set(vec![]);
            replying_to_lifecycle.set(None);
            threading_to_lifecycle.set(None);
        },
    );

    let header_event = move |evt: HeaderEvent| {
        to_owned![room];

        match evt.value {
            HeaderCallOptions::CLOSE => {
                nav.push(Route::ChatList {});
                room.set(CurrentRoom::default());
                cx.props.on_back.call(())
            }
            _ => {}
        }
    };

    let input_message_event = move |evt: HeaderEvent| {
        to_owned![replying_to];

        match evt.value {
            HeaderCallOptions::CLOSE => {
                replying_to.set(None);
            }
            _ => {}
        }
    };

    let on_push_message = move |evt: FormMessageEvent, send_to_thread: bool| {
        let reply_to = replying_to.get().map(|r| r.event_id);

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

    let on_handle_leave = move |_| {
        cx.spawn({
            to_owned![
                client,
                room,
                rooms,
                notification,
                key_chat_common_error_room_id,
                key_chat_common_error_room_not_found,
                key_chat_actions_leave
            ];
            async move {
                let id = room.get().id;
                leave_room(&client.get(), &id).await?;
                rooms
                    .remove_joined(&id)
                    .map_err(|_| LeaveRoomError::RoomNotFound)?;
                room.default();

                Ok::<(), LeaveRoomError>(())
            }
            .unwrap_or_else(move |e: LeaveRoomError| {
                let message = match e {
                    LeaveRoomError::InvalidRoomId => &key_chat_common_error_room_id,
                    LeaveRoomError::RoomNotFound => &key_chat_common_error_room_not_found,
                    LeaveRoomError::Failed => &key_chat_actions_leave,
                };

                notification.handle_error(&message);
            })
        })
    };

    let show_room_menu = use_state(cx, || false);
    let on_handle_menu = move |_| {
        let show_value = *show_room_menu.get();

        show_room_menu.set(!show_value);
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
                    menu: render!(rsx!(
                        section {
                            button {
                                class: "nav__cta",
                                onclick: on_handle_menu,
                                if *show_room_menu.get() {
                                    rsx!(
                                        Icon {
                                            stroke: "var(--text-1)",
                                            icon: ArrowUpCircle,
                                            height: 24,
                                            width: 24
                                        }
                                    )
                                } else {
                                    rsx!(
                                        Icon {
                                            stroke: "var(--text-1)",
                                            icon: ArrowDownCircle,
                                            height: 24,
                                            width: 24
                                        }
                                    )
                                },
                            }
                            if *show_room_menu.get() {
                                rsx!(
                                    div {
                                        class: "room-menu",
                                        ul {
                                            li {
                                                class: "room-menu__item",
                                                button {
                                                    class: "room-menu__cta",
                                                    onclick: on_handle_leave,
                                                    Icon {
                                                        stroke: "var(--text-1)",
                                                        icon: Exit
                                                    }
                                                    span {
                                                        translate!(i18, "chat.room-menu.leave")
                                                    }
                                                }
                                            }
                                        }
                                    }
                                )
                            }
                        }
                    )),
                    on_event: header_event
                }
                List {
                    messages: messages.clone(),
                    thread: None,
                    is_loading: is_loading,
                    show_load_button: true,
                    on_scroll: move |_| {
                        use_m.loadmore("{room.get().id}");
                    }
                },
                InputMessage {
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

            if let Some(t) = threading_to.get() {
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
                                    threading_to.set(None)
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
