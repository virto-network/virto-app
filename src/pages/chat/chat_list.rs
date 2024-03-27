use std::collections::HashMap;

use dioxus::prelude::*;
use dioxus_std::{i18n::use_i18, translate};
use futures::TryFutureExt;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

use crate::{
    components::{
        atoms::{
            input::InputType, message::Messages, room::RoomItem, MessageInput, Space, SpaceSkeleton,
        },
        molecules::{
            rooms::{CurrentRoom, FormRoomEvent},
            RoomsList,
        },
        organisms::{
            chat::{ActiveRoom, PreviewRoom, PublicRooms},
            main::TitleHeaderMain,
        },
    },
    hooks::{
        use_client::use_client,
        use_lifecycle::use_lifecycle,
        use_messages::use_messages,
        use_notification::use_notification,
        use_public::use_public,
        use_room::use_room,
        use_room_preview::{use_room_preview, PreviewRoom},
        use_rooms::{use_rooms, RoomsList},
        use_session::use_session,
    },
    services::matrix::matrix::{
        invited_rooms, list_rooms_and_spaces, public_rooms_and_spaces, Conversations,
    },
};

pub enum ChatListError {
    SessionNotFound,
    InvitedRooms,
    PublicRooms,
}

#[inline_props]
pub fn ChatList(cx: Scope) -> Element {
    let i18 = use_i18(cx);
    let client = use_client(cx).get();
    let session = use_session(cx);
    let notification = use_notification(cx);
    let room = use_room(cx);
    let public = use_public(cx);
    let rooms_list = use_rooms(cx);
    let preview = use_room_preview(cx);
    let messages = use_messages(cx);

    let room_tabs = use_ref::<HashMap<CurrentRoom, Messages>>(cx, || HashMap::new());

    let key_chat_list_home = translate!(i18, "chat.list.home");
    let key_chat_list_search = translate!(i18, "chat.list.search");
    let key_chat_list_errors_public_rooms = translate!(i18, "chat.list.errors.public_rooms");
    let key_chat_list_errors_invited_rooms = translate!(i18, "chat.list.errors.invited_rooms");
    let key_session_error_not_found = translate!(i18, "chat.session.error.not_found");

    let key_chat_helper_rooms_title = translate!(i18, "chat.helpers.rooms.title");
    let key_chat_helper_rooms_description = translate!(i18, "chat.helpers.rooms.description");
    let key_chat_helper_rooms_subtitle = translate!(i18, "chat.helpers.rooms.subtitle");

    let rooms = use_state::<Vec<RoomItem>>(cx, || Vec::new());
    let all_rooms = use_state::<Vec<RoomItem>>(cx, || Vec::new());
    let spaces = use_state::<HashMap<RoomItem, Vec<RoomItem>>>(cx, || HashMap::new());
    let pattern = use_state(cx, String::new);
    let rooms_filtered = use_ref(cx, || Vec::new());
    let selected_space = use_ref::<String>(cx, || String::new());
    let title_header =
        use_shared_state::<TitleHeaderMain>(cx).expect("Unable to read title header");
    let is_loading = use_state(cx, || false);
    let chat_list_wrapper_ref = use_ref::<Option<Box<HtmlElement>>>(cx, || None);

    let r = room.clone();
    use_lifecycle(
        &cx,
        || {},
        move || {
            to_owned![r];

            r.default();
        },
    );

    use_coroutine(cx, |_: UnboundedReceiver<()>| {
        to_owned![
            client,
            rooms_list,
            rooms,
            spaces,
            rooms_filtered,
            all_rooms,
            selected_space,
            title_header,
            session,
            notification,
            key_chat_list_home,
            key_session_error_not_found,
            is_loading
        ];

        async move {
            is_loading.set(true);

            let session_data = session.get().ok_or(ChatListError::SessionNotFound)?;

            let invited = invited_rooms(&client)
                .await
                .map_err(|_| ChatListError::InvitedRooms)?;

            let Conversations {
                rooms: r,
                spaces: s,
            } = list_rooms_and_spaces(&client, session_data).await;

            let public_rooms = public_rooms_and_spaces(&client, None, None, None)
                .await
                .map_err(|_| ChatListError::PublicRooms)?;

            rooms.set(r.clone());
            spaces.set(s.clone());

            s.iter().for_each(|space| {
                all_rooms.with_mut(|all_r| {
                    all_r.extend_from_slice(&space.1);
                })
            });

            all_rooms.with_mut(|all_r| {
                all_r.extend_from_slice(&r.clone());
            });

            rooms_list.set(RoomsList {
                public: public_rooms.rooms,
                invited,
                joined: r.clone(),
            });
            rooms_filtered.set(r);

            selected_space.set(key_chat_list_home.clone());
            title_header.write().title = key_chat_list_home.clone();

            is_loading.set(false);

            Ok::<(), ChatListError>(())
        }
        .unwrap_or_else(move |e: ChatListError| {
            let message = match e {
                ChatListError::SessionNotFound => &key_session_error_not_found,
                ChatListError::PublicRooms => &key_chat_list_errors_public_rooms,
                ChatListError::InvitedRooms => &key_chat_list_errors_invited_rooms,
            };

            notification.handle_error(&message);
        })
    });

    enum ScrollToPosition {
        Top,
        Bottom,
        Right,
        Left,
        Custom(f64, f64),
    }

    let on_scroll_chat_list_wrapper = move |position: ScrollToPosition| {
        if let Some(e) = chat_list_wrapper_ref.read().as_ref() {
            let (x, y) = match position {
                ScrollToPosition::Top | ScrollToPosition::Left => (0.0, 0.0),
                ScrollToPosition::Bottom => (0.0, e.get_bounding_client_rect().height()),
                ScrollToPosition::Right => (e.get_bounding_client_rect().width(), 0.0),
                ScrollToPosition::Custom(x, y) => (x, y),
            };
            e.scroll_to_with_x_and_y(x, y);
        }
    };

    let on_click_invitation = move |evt: FormRoomEvent| {
        preview.set(PreviewRoom::Invited(evt.room.clone()));
        room.default();
    };

    let on_click_room = move |evt: FormRoomEvent| {
        room.set(evt.room.clone());
        room_tabs.with_mut(|tabs| tabs.insert(evt.room, vec![]));
        messages.reset();
        preview.default();

        on_scroll_chat_list_wrapper(ScrollToPosition::Right);
    };

    render! {
        div {
            class: "chat-list-wrapper",
            onmounted: move |event| {
                event.data.get_raw_element()
                    .ok()
                    .and_then(|raw_element| raw_element.downcast_ref::<web_sys::Element>())
                    .and_then(|element| element.clone().dyn_into::<web_sys::HtmlElement>().ok())
                    .map(|html_element| chat_list_wrapper_ref.set(Some(Box::new(html_element.clone()))));
            },
            section {
                class: "chat-list options",
                div {
                    class: "chat-list__spaces",
                    if !spaces.get().is_empty() {
                        rsx!(
                            ul {
                                class: "chat-list__wrapper",
                                Space {
                                    text: "{key_chat_list_home}",
                                    uri: None,
                                    on_click: move |_| {
                                        rooms_list.set_joined(rooms.get().clone());
                                        rooms_filtered.set(rooms.get().clone());
                                        selected_space.set(key_chat_list_home.clone());
                                        title_header.write().title = key_chat_list_home.clone();

                                        if !rooms.get().iter().any(|r| {
                                            room.get().id.eq(&r.id)
                                        }) {
                                            room.default()
                                        }
                                    }
                                }

                                spaces.get().iter().map(|(space, value)|{
                                    let name = space.name.clone();
                                    rsx!(
                                        Space {
                                            text: "{name}",
                                            uri: space.avatar_uri.clone(),
                                            on_click: move |_| {
                                                rooms_list.set_joined(value.clone());
                                                rooms_filtered.set(value.clone());
                                                selected_space.set(space.name.clone());
                                                title_header.write().title = space.name.clone();

                                                if !value.iter().any(|r| {
                                                    room.get().id.eq(&r.id)
                                                }) {
                                                    room.default()
                                                }
                                            }
                                        }
                                    )
                                })
                            }
                        )
                    } else if *is_loading.get() {
                        rsx!(
                            ul {
                                class: "chat-list__wrapper",
                                (0..5).map(|_| {
                                    rsx!(
                                        SpaceSkeleton {
                                            size: 50
                                        }
                                    )
                                })
                            }
                        )
                    } else {
                        rsx!( div {})
                    }
                }


                div {
                    class: "chat-list__rooms",
                    onclick: move |_| {
                        on_scroll_chat_list_wrapper(ScrollToPosition::Left)
                    },
                    MessageInput {
                        message: "{pattern}",
                        placeholder: "{key_chat_list_search}",
                        itype: InputType::Search,
                        error: None,
                        on_input: move |event: FormEvent| {
                            pattern.set(event.value.clone());

                            let default_rooms = all_rooms.get().iter().cloned().collect::<Vec<_>>();

                            if !event.value.is_empty() {
                                let x = default_rooms
                                    .iter()
                                    .filter(|r| r.name.to_lowercase().contains(&event.value.to_lowercase()))
                                    .cloned()
                                    .collect::<Vec<_>>();

                                rooms_filtered.set(x);
                            } else {
                                rooms_filtered.set(rooms_list.get_joined().clone())
                            }
                        },
                        on_keypress: move |_| {},
                        on_click: move |_| {
                            on_scroll_chat_list_wrapper(ScrollToPosition::Right)
                        },
                    }
                    if !rooms_list.get_invited().is_empty() {
                        rsx!{
                            h2 {
                                class: "header__title",
                                translate!(i18, "chat.list.invitate")
                            }

                            RoomsList {
                                rooms: rooms_list.get_invited().clone(),
                                is_loading: *is_loading.get(),
                                on_submit: on_click_invitation
                            }
                        }
                    }

                    h2 {
                        class: "header__title",
                        translate!(i18, "chat.list.rooms")
                    }
                    RoomsList {
                        rooms: rooms_list.get_joined().clone(),
                        is_loading: *is_loading.get(),
                        on_submit: on_click_room
                    }
                }


                div {
                    class: "chat-list__content",
                    onclick: move |_| {
                        on_scroll_chat_list_wrapper(ScrollToPosition::Right)
                    },
                    if public.get().show {
                        rsx!(
                            section {
                                class: "chat-list__active-room",
                                PublicRooms {
                                    on_back: move |_| {
                                        on_scroll_chat_list_wrapper(ScrollToPosition::Left)
                                    }
                                }
                            }
                        )
                    } else if !preview.get().is_none() {
                        rsx!(
                            section {
                                class: "chat-list__active-room",
                                PreviewRoom {
                                    on_back: move |_| {
                                        on_scroll_chat_list_wrapper(ScrollToPosition::Left)
                                    }
                                }
                            }
                        )
                    } else if !room.get().name.is_empty(){
                        rsx!(
                            section {
                                class: "chat-list__active-room",
                                ActiveRoom {
                                    on_back: move |_| {
                                        on_scroll_chat_list_wrapper(ScrollToPosition::Left)
                                    }
                                }
                            }
                        )
                    }
                }
            }
        }
    }
}
