use dioxus::prelude::*;

use crate::{components::organisms::chat::ActiveRoom, hooks::use_messages::use_messages};


#[inline_props]
pub fn ChatRoom(cx: Scope, name: String) -> Element {
    let _ = &name;
    let messages = use_messages(cx);

    use_coroutine(cx, |_: UnboundedReceiver<bool>| {
        to_owned![messages];

        async move {
            messages.reset();
        }
    });

    render! {
        ActiveRoom {}
    }
}
