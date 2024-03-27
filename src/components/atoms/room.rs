use dioxus::prelude::*;

use crate::components::atoms::Avatar;
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RoomItem {
    pub avatar_uri: Option<String>,
    pub id: String,
    pub name: String,
    pub is_public: bool,
    pub is_direct: bool,
}

#[derive(Props)]
pub struct RoomViewProps<'a> {
    displayname: &'a str,
    #[props(!optional)]
    avatar_uri: Option<String>,
    description: Option<&'a str>,
    #[props(default = false)]
    wrap: bool,
    on_click: EventHandler<'a, MouseEvent>,
}

pub fn RoomView<'a>(cx: Scope<'a, RoomViewProps<'a>>) -> Element<'a> {
    let description = cx.props.description.unwrap_or("");
    let room_view_wrap = if cx.props.wrap { "room-view--wrap" } else { "" };

    cx.render(rsx! {
      div {
        class: "room-view {room_view_wrap} fade-in",
        onclick: move |event| cx.props.on_click.call(event),

        Avatar {
          name: String::from(cx.props.displayname),
          size: 60,
          uri: cx.props.avatar_uri.clone()
        }
        article {
          p {
            class: "room-view__title",
            "{cx.props.displayname}"
          }
          p {
            class: "room-view__message",
            span {
              "{description}"
            }
          }
        }
      }
    })
}
