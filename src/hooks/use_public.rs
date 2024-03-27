use dioxus::prelude::*;

#[derive(Default, Debug, Clone)]
pub struct PublicState {
    pub show: bool,
}

pub fn use_public(cx: &ScopeState) -> &UsePublicState {
    let public_state = use_shared_state::<PublicState>(cx).expect("Unable to use PublicState");

    cx.use_hook(move || UsePublicState {
        inner: public_state.clone(),
    })
}

#[derive(Clone)]
pub struct UsePublicState {
    inner: UseSharedState<PublicState>,
}

impl UsePublicState {
    pub fn get(&self) -> PublicState {
        self.inner.read().clone()
    }

    pub fn set(&self, room: PublicState) {
        let mut inner = self.inner.write();
        *inner = room;
    }

    pub fn default(&self) {
        self.set(PublicState::default())
    }
}
