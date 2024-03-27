use dioxus::prelude::*;

use crate::components::atoms::{room::RoomItem, RoomView, RoomViewSkeleton};

#[derive(Clone, Debug, PartialEq, Hash, Eq, Default)]
pub struct CurrentRoom {
    pub id: String,
    pub name: String,
    pub avatar_uri: Option<String>,
}

#[derive(Debug)]
pub struct FormRoomEvent {
    pub room: CurrentRoom,
}

#[derive(Props)]
pub struct RoomsListProps<'a> {
    rooms: Vec<RoomItem>,
    is_loading: bool,
    #[props(default = false)]
    wrap: bool,
    on_submit: EventHandler<'a, FormRoomEvent>,
}

pub fn RoomsList<'a>(cx: Scope<'a, RoomsListProps<'a>>) -> Element<'a> {
    let rooms_list_skeleton = if !cx.props.rooms.is_empty() {
        ""
    } else {
        "rooms-list--skeleton"
    };
    let room_list_wrap = if cx.props.wrap { "room-list--wrap" } else { "" };

    cx.render(rsx! {
        section {
            class:"rooms-list {room_list_wrap} {rooms_list_skeleton} fade-in",
            if !cx.props.rooms.is_empty() {
                rsx!(cx.props.rooms.iter().map(|room| {
                    rsx!(
                        RoomView {
                        key: "{room.id}",
                        displayname: room.name.as_str(),
                        avatar_uri: room.avatar_uri.clone(),
                        description: "",
                        wrap: cx.props.wrap,
                        on_click: move |_| {
                            cx.props.on_submit.call(FormRoomEvent {
                                room: CurrentRoom {
                                    id: room.id.clone(),
                                    name: room.name.clone(),
                                    avatar_uri: room.avatar_uri.clone(),
                                },
                            })
                        }
                    }
                )
                }))
            } else if cx.props.is_loading {
                rsx!(
                    (0..20).map(|i| {
                        rsx!(
                            RoomViewSkeleton {
                                key: "{i}"
                            }
                        )
                    })
                )
            } else {
                rsx!(div{})
            }
        }
    })
}
