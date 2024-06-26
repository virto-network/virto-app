use dioxus::prelude::*;
use dioxus_std::i18n::use_i18;
use dioxus_std::translate;
use gloo::events::EventListener;
use log::info;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

use crate::components::atoms::message::Sender;
use crate::components::atoms::message::ThreadPreview;
use crate::components::atoms::messages::message::MessageView;
use crate::components::atoms::messages::MessageSkeleton;
use crate::components::{
    atoms::{
        message::Message,
        messages::hover_menu::{MenuEvent, MenuOption},
        *,
    },
    molecules::input_message::ReplyingTo,
};
use crate::hooks::use_reply::use_reply;
use crate::hooks::use_thread::use_thread;
use crate::services::matrix::matrix::EventOrigin;
use crate::services::matrix::matrix::TimelineMessage;
use crate::services::matrix::matrix::TimelineRelation;
use crate::services::matrix::matrix::TimelineThread;

pub struct ListEvent {}

#[derive(PartialEq, Props, Clone)]
pub struct ListProps {
    messages: Vec<TimelineRelation>,
    #[props(!optional)]
    thread: Option<Vec<TimelineMessage>>,
    is_loading: bool,
    on_scroll: EventHandler<ListEvent>,
    #[props(default = false)]
    show_load_button: bool,
}

pub fn List(props: ListProps) -> Element {
    let i18 = use_i18();
    let mut replying_to = use_reply();
    let mut threading_to = use_thread();

    let mut container_to_scroll = use_signal::<Option<Box<HtmlElement>>>(|| None);
    let mut list_to_scroll = use_signal::<Option<Box<HtmlElement>>>(|| None);
    let on_scroll = use_signal::<bool>(|| false);
    let mut is_loading = use_signal::<bool>(|| props.is_loading);

    let messages_list_thread = match threading_to.get() {
        Some(_) => "messages-list--is-thread",
        None => "messages-list--not-thread",
    };

    use_effect(use_reactive((&on_scroll(),), move |(_,)| {
        if on_scroll() && !props.is_loading {
            props.on_scroll.call(ListEvent {});
        }
    }));

    let messages_list_skeleton = if !props.messages.is_empty() {
        ""
    } else {
        "messages-list--skeleton"
    };

    rsx! {
        div {
            class:"messages-list {messages_list_thread} {messages_list_skeleton}",
            onmounted: move |event| {
                event.data.downcast::<web_sys::Element>()
                    .and_then(|element| element.clone().dyn_into::<web_sys::HtmlElement>().ok())
                    .map(|html_element| container_to_scroll.set(Some(Box::new(html_element.clone()))));
            },
            div {
                class: "messages-list__wrapper",
                onmounted: move |event| {
                    event.data.downcast::<web_sys::Element>()
                        .and_then(|element| element.clone().dyn_into::<web_sys::HtmlElement>().ok())
                        .map(|html_element| list_to_scroll.set(Some(Box::new(html_element.clone()))));

                    if let Some(container) = &*container_to_scroll.read() {
                        if let Some(list) = list_to_scroll.read().clone() {
                            to_owned!(container, on_scroll);

                            let mut old_value = 0;

                            EventListener::new(&container.clone(), "scroll", move |_| {
                                let container_height = container.client_height();
                                let scroll_top = container.scroll_top() * -1;
                                let list_height = list.client_height();

                                let scrolled_top = list_height * 80 / 100;

                                if container_height + scroll_top >= scrolled_top && scroll_top > old_value && !is_loading() {
                                    on_scroll.set(true);
                                }

                                old_value = scroll_top;
                            }).forget();
                        }
                    }
                },

                if !props.messages.is_empty() {
                    {props.messages.iter().enumerate().map(|(i, m)| {
                        match m {
                            TimelineRelation::None(message) => {
                                let message = message.clone();
                                let event_id = message.event_id.clone();
                                rsx!(
                                    MessageView {
                                        key: "{event_id}",
                                        message: Message {
                                            id: i as i64,
                                            event_id: message.event_id,
                                            display_name: message.sender.name.clone(),
                                            avatar_uri: message.sender.avatar_uri.clone(),
                                            content: message.body.clone(),
                                            reply: None,
                                            origin: message.origin.clone(),
                                            time: message.time.clone(),
                                            thread: None
                                        },
                                        is_replying: false,
                                        on_event: move |event: MenuEvent| {
                                            match event.option {
                                                MenuOption::Download => {
                                                    info!("TODO: handle download")
                                                }
                                                MenuOption::Reply => {
                                                    let replying = ReplyingTo {
                                                        event_id: event_id.clone(),
                                                        content: message.body.clone(),
                                                        display_name: message.sender.name.clone(),
                                                        avatar_uri: message.sender.avatar_uri.clone(),
                                                        origin: message.origin.clone()
                                                    };

                                                    replying_to.set(Some(replying));
                                                }
                                                MenuOption::Close => {
                                                    info!("close");
                                                }
                                                MenuOption::ShowThread => {
                                                    info!("thread");
                                                }
                                                MenuOption::CreateThread => {
                                                    info!("create thread");
                                                    let thread = vec![TimelineMessage { event_id: event_id.clone(), sender: message.sender.clone(), body: message.body.clone(), origin: message.origin.clone(), time: message.time.clone() }];
                                                    threading_to.set(Some(TimelineThread { event_id: event_id.clone(), thread, count: 0, latest_event: event_id.clone() }));
                                                }

                                            }

                                        }
                                    }
                                )
                            }
                            TimelineRelation::Reply(message) => {
                                let r = message.reply.clone();
                                let message = message.event.clone();
                                let event_id = message.event_id.clone();

                                let reply = r.map(|r| MessageReply {
                                    content: r.body,
                                    display_name: r.sender.name,
                                    avatar_uri: r.sender.avatar_uri
                                });

                                rsx!(
                                    MessageView {
                                        key: "{event_id}",
                                        message: Message {
                                            id: i as i64,
                                            event_id: message.event_id,
                                            display_name: message.sender.name.clone(),
                                            avatar_uri: message.sender.avatar_uri.clone(),
                                            content: message.body.clone(),
                                            reply,
                                            origin: message.origin.clone(),
                                            time: message.time.clone(),
                                            thread: None
                                        },
                                        is_replying: false,
                                        on_event: move |event: MenuEvent| {
                                            info!("menu option list: {:?}", event.option);

                                            match event.option {
                                                MenuOption::Download => {
                                                    info!("TODO: handle download")
                                                }
                                                MenuOption::Reply => {
                                                    let replying = ReplyingTo {
                                                        event_id: event_id.clone(),
                                                        content: message.body.clone(),
                                                        display_name: message.sender.name.clone(),
                                                        avatar_uri: message.sender.avatar_uri.clone(),
                                                        origin: message.origin.clone()
                                                    };

                                                    replying_to.set(Some(replying));
                                                }
                                                MenuOption::Close => {
                                                    info!("close");
                                                }
                                                MenuOption::ShowThread => {
                                                    info!("thread");
                                                }
                                                MenuOption::CreateThread => {

                                                }
                                            }

                                        }
                                    }
                                )
                            }
                            TimelineRelation::CustomThread(message) => {
                                let event_id = message.event_id.clone();
                                let thread = message.thread.clone();
                                let latest_event = message.latest_event.clone();
                                let count = message.count.clone();
                                let head = thread.get(0).cloned();

                                let Some(head_message) = head else {
                                    return
                                        rsx!(
                                            div {
                                                class: "message__content",
                                                {translate!(i18, "chat.message_list.errors.thread_not_found")}
                                            }
                                        )
                                    ;
                                };

                                let mut thread_avatars: Vec<Sender> = vec![];

                                for (i, t) in thread.iter().enumerate() {
                                    if i == 2 {
                                        break;
                                    }

                                    thread_avatars.push(Sender{avatar_uri: t.sender.avatar_uri.clone(), display_name: t.sender.name.clone()})
                                }

                                rsx!(
                                    MessageView {
                                        key: "{event_id}",
                                        message: Message {
                                            id: i as i64,
                                            event_id: head_message.event_id.clone(),
                                            display_name: head_message.sender.name.clone(),
                                            avatar_uri: head_message.sender.avatar_uri.clone(),
                                            content: head_message.body.clone(),
                                            reply: None,
                                            origin: head_message.origin.clone(),
                                            time: head_message.time.clone(),
                                            thread: Some(ThreadPreview{meta_senders: thread_avatars, count: (thread.len() - 1) as i8 })
                                        },
                                        is_replying: false,
                                        on_event: move |event: MenuEvent| {
                                            match event.option {
                                                MenuOption::Download => {
                                                    info!("TODO: handle download")
                                                }
                                                MenuOption::Reply => {
                                                    let replying = ReplyingTo {
                                                        event_id: event_id.clone(),
                                                        content: head_message.body.clone(),
                                                        display_name: head_message.sender.name.clone(),
                                                        avatar_uri: head_message.sender.avatar_uri.clone(),
                                                        origin: head_message.origin.clone()
                                                    };

                                                    replying_to.set(Some(replying));
                                                }
                                                MenuOption::Close => {
                                                    info!("close");
                                                }
                                                MenuOption::ShowThread => {
                                                    info!("thread");
                                                    threading_to.set(Some(TimelineThread { event_id: event_id.clone(), thread: thread.clone(), count: count.clone(), latest_event: latest_event.clone() }))
                                                }
                                                MenuOption::CreateThread => {

                                                }
                                            }

                                        }
                                    }
                                )
                            }
                            TimelineRelation::Thread(_) => {
                                None
                            }

                        }})}
                } else if props.is_loading {
                    {
                        (0..15).map(|i| {
                            let origin = if i % 3 == 0 {
                                EventOrigin::OTHER
                            } else {
                                EventOrigin::ME
                            };

                            rsx!(
                                MessageSkeleton {
                                    origin: origin
                                }
                            )
                        })
                    }
                } else {
                    div {}
                }

                if let Some(messages) = &props.thread {
                    {
                        messages.iter().enumerate().map(|(i, m)| {
                            let message = m.clone();
                            let event_id = message.event_id.clone();
                            rsx!(
                                MessageView {
                                    key: "{event_id}",
                                    message: Message {
                                        id: i as i64,
                                        event_id: message.event_id,
                                        display_name: message.sender.name.clone(),
                                        avatar_uri: message.sender.avatar_uri.clone(),
                                        content: message.body.clone(),
                                        reply: None,
                                        origin: message.origin.clone(),
                                        time: message.time.clone(),
                                        thread: None
                                    },
                                    is_replying: false,
                                    on_event: move |event: MenuEvent| {
                                        info!("menu option list: {:?}", event.option);

                                        match event.option {
                                            MenuOption::Download => {
                                                info!("TODO: handle download")
                                            }
                                            MenuOption::Reply => {
                                                let replying = ReplyingTo {
                                                    event_id: event_id.clone(),
                                                    content: message.body.clone(),
                                                    display_name: message.sender.name.clone(),
                                                    avatar_uri: message.sender.avatar_uri.clone(),
                                                    origin: message.origin.clone()
                                                };

                                                replying_to.set(Some(replying));
                                            }
                                            MenuOption::Close => {
                                                info!("close");
                                            }
                                            MenuOption::ShowThread => {
                                                info!("thread");
                                            }
                                            MenuOption::CreateThread => {
                                                info!("thread");
                                            }
                                        }

                                    }
                                }
                            )
                        })
                    }
                }
            }

            if props.show_load_button && !props.is_loading {
                div {
                    class: "list__cta",
                    button {
                        class: "button button--secondary button--small",
                        onclick: move |_| {
                            props.on_scroll.call(ListEvent {  });
                        },
                        {translate!(i18, "chat.message_list.see_more")}
                    }
                }
            }
        }
    }
}
