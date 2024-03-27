use dioxus::prelude::*;
use dioxus_router::prelude::use_navigator;
use dioxus_std::{i18n::use_i18, translate};

use crate::{
    components::{
        atoms::{
            header_main::{HeaderCallOptions, HeaderEvent},
            Header,
        },
        molecules::{rooms::FormRoomEvent, RoomsList},
    },
    hooks::{
        use_messages::use_messages,
        use_public::use_public,
        use_room_preview::{use_room_preview, PreviewRoom},
        use_rooms::use_rooms,
    },
    pages::route::Route,
};

pub enum PreviewRoomError {
    InvalidRoomId,
    InvitationNotFound,
    AcceptFailed,
}

#[derive(Props)]
pub struct PublicRoomProps<'a> {
    on_back: EventHandler<'a, ()>,
}
pub fn PublicRooms<'a>(cx: Scope<'a, PublicRoomProps<'a>>) -> Element<'a> {
    let i18 = use_i18(cx);
    let nav = use_navigator(cx);
    let preview = use_room_preview(cx);
    let rooms = use_rooms(cx);
    let messages = use_messages(cx);
    let public = use_public(cx);

    let key_public_title = translate!(i18, "chat.public.title");

    let header_event = move |evt: HeaderEvent| {
        to_owned![public];

        match evt.value {
            HeaderCallOptions::CLOSE => {
                nav.push(Route::ChatList {});
                public.default();
                cx.props.on_back.call(())
            }
            _ => {}
        }
    };

    let on_click_room = move |evt: FormRoomEvent| {
        messages.reset();
        preview.set(PreviewRoom::Joining(evt.room.clone()));
        public.default();
    };

    render!(rsx! {
        div {
            class: "active-room",
            Header {
                text: "{key_public_title}",
                on_event: header_event
            }

            RoomsList {
                rooms: rooms.get_public().clone(),
                is_loading: false,
                on_submit: on_click_room,
                wrap: true,
            }
        }
    })
}
