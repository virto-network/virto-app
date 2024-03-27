use dioxus::prelude::*;

use crate::components::molecules::rooms::CurrentRoom;

#[derive(Clone, Debug, PartialEq, Hash, Eq, Default)]
pub enum PreviewRoom {
    Invited(CurrentRoom),
    Creating(CurrentRoom),
    Joining(CurrentRoom),
    #[default]
    None,
}

impl PreviewRoom {
    pub fn is_none(self) -> bool {
        match self {
            PreviewRoom::None => true,
            _ => false,
        }
    }
}

pub fn use_room_preview(cx: &ScopeState) -> &UseRoomState {
    let preview_room = use_shared_state::<PreviewRoom>(cx).expect("Unable to use PreviewRoom");

    cx.use_hook(move || UseRoomState {
        inner: preview_room.clone(),
    })
}

#[derive(Clone)]
pub struct UseRoomState {
    inner: UseSharedState<PreviewRoom>,
}

impl UseRoomState {
    pub fn get(&self) -> PreviewRoom {
        self.inner.read().clone()
    }

    pub fn set(&self, room: PreviewRoom) {
        let mut inner = self.inner.write();
        *inner = room;
    }

    pub fn default(&self) {
        self.set(PreviewRoom::default())
    }
}
