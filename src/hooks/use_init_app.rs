use std::collections::HashMap;

use crate::components::atoms::message::Messages;
use crate::services::matrix::matrix::TimelineThread;
use crate::{
    components::molecules::{input_message::ReplyingTo, rooms::CurrentRoom},
    pages::login::LoggedIn,
    MatrixClientState,
};
use dioxus::prelude::*;
use matrix_sdk::encryption::verification::SasVerification;
use ruma::api::client::uiaa::AuthType;

use super::use_auth::CacheLogin;
use super::use_notification::NotificationItem;
use super::use_public::PublicState;
use super::use_room_preview::PreviewRoom;
use super::use_rooms::RoomsList;
use super::use_send_attach::SendAttachStatus;
use super::use_session::UserSession;
use super::{use_attach::AttachFile, use_modal::ModalState};

pub enum BeforeSession {
    Login,
    Signup,
}

#[derive(Clone, Debug)]
pub struct MessageDispatchId {
    pub value: HashMap<String, Option<String>>,
}

pub fn use_init_app(cx: &ScopeState) {
    use_shared_state_provider::<LoggedIn>(cx, || LoggedIn(false));
    use_shared_state_provider::<MatrixClientState>(cx, || MatrixClientState { client: None });
    use_shared_state_provider::<ModalState>(cx, || ModalState {
        show: false,
        account: None,
    });

    // Temporarily moved here because Route has an unexpected
    // change when we push a ChatRoom from a different nest route

    use_shared_state_provider::<CurrentRoom>(cx, || CurrentRoom::default());
    use_shared_state_provider::<PreviewRoom>(cx, || PreviewRoom::default());
    use_shared_state_provider::<RoomsList>(cx, || RoomsList::default());
    use_shared_state_provider::<Messages>(cx, || Vec::new());
    use_shared_state_provider::<Option<AttachFile>>(cx, || None);
    use_shared_state_provider::<Option<ReplyingTo>>(cx, || None);
    use_shared_state_provider::<NotificationItem>(cx, || NotificationItem::default());

    use_shared_state_provider::<Option<SasVerification>>(cx, || None);
    use_shared_state_provider::<Option<TimelineThread>>(cx, || None);

    use_shared_state_provider::<BeforeSession>(cx, || BeforeSession::Login);
    use_shared_state_provider::<Option<CacheLogin>>(cx, || None);
    use_shared_state_provider::<Vec<AuthType>>(cx, || vec![]);

    use_shared_state_provider::<Option<UserSession>>(cx, || None);

    use_shared_state_provider::<MessageDispatchId>(cx, || MessageDispatchId {
        value: HashMap::new(),
    });
    use_shared_state_provider(cx, || SendAttachStatus::Loading(0));
    use_shared_state_provider::<PublicState>(cx, || PublicState::default());
}
