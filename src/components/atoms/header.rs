use dioxus::prelude::*;

use crate::components::atoms::{header_main::HeaderCallOptions, ArrowLeft, Icon};

use super::header_main::HeaderEvent;

#[derive(Props)]
pub struct HeaderProps<'a> {
    avatar_element: Option<Element<'a>>,
    menu: Option<Element<'a>>,
    text: &'a str,
    on_event: EventHandler<'a, HeaderEvent>,
}

pub fn Header<'a>(cx: Scope<'a, HeaderProps<'a>>) -> Element<'a> {
    cx.render(rsx!(
        nav {
          class: "nav",
          div {
            class: "nav-wrapper",
            button {
              class: "nav__cta",
              onclick: move |_| {cx.props.on_event.call(HeaderEvent { value: HeaderCallOptions::CLOSE })},
              Icon {
                stroke: "var(--text-1)",
                icon: ArrowLeft,
                height: 24,
                width: 24
              }
            }
            cx.props.avatar_element.clone().map(|e| render!(e)) 
            span {
              class: "nav__title",
              "{cx.props.text}"
            }
          }
          cx.props.menu.clone().map(|e| render!(e)) 
      }
    ))
}
