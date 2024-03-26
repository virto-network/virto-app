use dioxus::prelude::*;
use dioxus_router::prelude::*;

use crate::components::molecules::modal::{ModalForm, RoomType};
use crate::components::molecules::Modal;
use crate::hooks::use_listen_invitation::use_listen_invitation;
use crate::hooks::use_listen_message::use_listen_message;
use crate::hooks::use_modal::use_modal;
use crate::pages::route::Route;

use crate::services::matrix::matrix::TimelineRelation;

use matrix_sdk::room::Room;

pub struct MessageItem {
    pub room_id: String,
    pub msg: String,
    pub reply_to: Option<String>,
    pub send_to_thread: bool,
}

pub struct MessageEvent {
    pub room: Room,
    pub mgs: Option<TimelineRelation>,
}

#[inline_props]
pub fn Chat(cx: Scope) -> Element {
    let modal = use_modal(cx);
    let navigator = use_navigator(cx);

    use_listen_message(cx);
    use_listen_invitation(cx);

    render! {
        if modal.get().show {
            rsx!(
                Modal {
                    on_click: move |event: ModalForm| {
                        match event.value {
                            RoomType::CHAT => {
                                modal.hide();
                                navigator.push(Route::RoomNew {});
                            },
                            RoomType::GROUP => {
                                modal.hide();
                                navigator.push(Route::RoomGroup {});
                            },
                            RoomType::CHANNEL => {
                                modal.hide()
                            },
                        }
                    },
                    on_close:move |_|{
                        modal.hide()
                    }
                }
            )
        }
        Outlet::<Route> {}
    }
}
