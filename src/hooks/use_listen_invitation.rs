use dioxus::prelude::*;
use futures::StreamExt;
use matrix_sdk::{room::Room, Client};
use ruma::events::room::member::StrippedRoomMemberEvent;

use crate::services::matrix::matrix::format_invited_room;

use super::{use_client::use_client, use_rooms::use_rooms};

pub fn use_listen_invitation(cx: &ScopeState) -> &UseListenInvitationState {
    let client = use_client(cx).get();
    let rooms = use_rooms(cx);

    let handler_added = use_ref(cx, || false);

    let task_push_invited = use_coroutine(cx, |mut rx: UnboundedReceiver<Room>| {
        to_owned![client, rooms];

        async move {
            while let Some(room) = rx.next().await {
                if let Room::Invited(room) = room {
                    let Ok(item) = format_invited_room(&client, room).await else {
                        continue;
                    };

                    rooms.push_invited(item);
                }
            }
        }
    });

    use_coroutine(cx, |_: UnboundedReceiver<String>| {
        to_owned![client, handler_added, task_push_invited];

        async move {
            if !*handler_added.read() {
                client.add_event_handler(
                    move |_: StrippedRoomMemberEvent, _: Client, room: Room| {
                        let task_push_invited = task_push_invited.clone();
                        async move {
                            task_push_invited.send(room)
                        }
                    },
                );

                handler_added.set(true);
            }
        }
    });
    cx.use_hook(move || UseListenInvitationState {})
}

#[derive(Clone)]
pub struct UseListenInvitationState {}
