use std::ops::Deref;

use dioxus::prelude::*;
use gloo::events::EventListener;
use log::info;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

use crate::components::atoms::message::Sender;
use crate::components::atoms::message::ThreadPreview;
use crate::components::atoms::messages::message::MessageView;
use crate::services::matrix::matrix::TimelineMessage;
use crate::services::matrix::matrix::TimelineRelation;
use crate::services::matrix::matrix::TimelineThread;
use crate::components::{atoms::{
    message::Message,
    *, messages::hover_menu::{MenuEvent, MenuOption},
}, molecules::input_message::ReplyingTo};

pub struct ListEvent {}

#[derive(Props)]
pub struct ListProps<'a> {
    messages: Vec<TimelineRelation>,
    #[props(!optional)]
    thread: Option<Vec<TimelineMessage>>,
    is_loading: bool,
    on_scroll: EventHandler<'a, ListEvent>,
}

pub fn List<'a>(cx: Scope<'a, ListProps<'a>>) -> Element<'a> {
    let replying_to = use_shared_state::<Option<ReplyingTo>>(cx).expect("Unable to load Replying to");
    let timeline_thread = use_shared_state::<Option<TimelineThread>>(cx).expect("Unable to load timeline thread");

    let container_to_scroll = use_ref::<Option<Box<HtmlElement>>>(cx, || None);
    let list_to_scroll = use_ref::<Option<Box<HtmlElement>>>(cx, || None);
    let on_scroll = use_state(cx, || false);
    let is_loading = use_state(cx, || cx.props.is_loading);
    
    let messages_list_thread = match timeline_thread.read().deref() {
        Some(_)=> "messages-list--is-thread",
        None => "messages-list--not-thread"
    };

    use_effect(cx, (on_scroll,), |(_,)| {
        let props = cx.props.to_owned();

        if *on_scroll.get() && !cx.props.is_loading {
            props.on_scroll.call(ListEvent {  });
        }
        async move {}
    });

    cx.render(rsx! {  
        div {
            class:"messages-list messages_list_thread",
            onmounted: move |event| {
                event.data.get_raw_element()
                    .ok()
                    .and_then(|raw_element| raw_element.downcast_ref::<web_sys::Element>())
                    .and_then(|element| element.clone().dyn_into::<web_sys::HtmlElement>().ok())
                    .map(|html_element| container_to_scroll.set(Some(Box::new(html_element.clone()))));
            },
            rsx!(
                div {
                    class: "messages-list__wrapper",
                    onmounted: move |event| {
                        event.data.get_raw_element()
                            .ok()
                            .and_then(|raw_element| raw_element.downcast_ref::<web_sys::Element>())
                            .and_then(|element| element.clone().dyn_into::<web_sys::HtmlElement>().ok())
                            .map(|html_element| list_to_scroll.set(Some(Box::new(html_element.clone()))));
                        
                        if let Some(container) = &*container_to_scroll.read() {
                            if let Some(list) = list_to_scroll.read().clone() {
                                to_owned!(container, is_loading, on_scroll);
                                
                                let mut old_value = 0;
                                
                                let on_down = EventListener::new(&container.clone(), "scroll", move |_| {
                                    let container_height = container.client_height();
                                    let scroll_top = container.scroll_top() * -1; 
                                    let list_height = list.client_height();
                                    
                                    let scrolled_top = list_height * 80 / 100;

                                    if container_height + scroll_top >= scrolled_top && scroll_top > old_value && !is_loading.get() {
                                        on_scroll.set(true);
                                    }

                                    old_value = scroll_top;
                                }).forget();
                            }
                        }
                    },
                    rsx!(
                        cx.props.messages.iter().enumerate().map(|(i, m)| {
                            match m {
                                TimelineRelation::None(message) => {
                                    let message = message.clone();
                                    let event_id = message.event_id.clone();

                                    cx.render(rsx!(
                                        MessageView {
                                            key: "{i}",
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
                                                        if let Some(eid) = &event_id {
                                                            let replying = ReplyingTo { 
                                                                event_id: eid.clone(), 
                                                                content: message.body.clone(), 
                                                                display_name: message.sender.name.clone(), 
                                                                avatar_uri: message.sender.avatar_uri.clone(),
                                                                origin: message.origin.clone()
                                                            };
                                                            
                                                            *replying_to.write() = Some(replying);
                                                        }
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
                                                        if let Some(e) = &event_id {
                                                            *timeline_thread.write() = Some(TimelineThread { event_id: e.clone(), thread, count: 0, latest_event: e.clone() })
                                                        }
                                                    }

                                                }
                                                
                                            }
                                        }
                                    ))
                                }
                                TimelineRelation::Reply(message) => {
                                    let r = message.reply.clone();
                                    let message = message.event.clone();
                                    let event_id = message.event_id.clone();

                                    let reply = match r {
                                        Some(r) => {
                                            Some(MessageReply {
                                                content: r.body,
                                                display_name: r.sender.name,
                                                avatar_uri: r.sender.avatar_uri
                                            })
                                        }
                                        None => {
                                            None
                                        }
                                    };

                                    cx.render(rsx!(
                                        MessageView {
                                            key: "{i}",
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
                                                        if let Some(eid) = &event_id {
                                                            let replying = ReplyingTo { 
                                                                event_id: eid.clone(), 
                                                                content: message.body.clone(), 
                                                                display_name: message.sender.name.clone(), 
                                                                avatar_uri: message.sender.avatar_uri.clone(),
                                                                origin: message.origin.clone()
                                                            };
                                                            
                                                            *replying_to.write() = Some(replying);
                                                        }
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
                                    ))
                                }
                                TimelineRelation::CustomThread(message) => {
                                    let event_id = message.event_id.clone();
                                    let thread = message.thread.clone();
                                    let latest_event = message.latest_event.clone();
                                    let count = message.count.clone();
                                    let head_message = thread[0].clone();
                                    
                                    let mut thread_avatars: Vec<Sender> = vec![];

                                    for (i, t) in thread.iter().enumerate() {
                                        if i == 2 {
                                            break;
                                        }

                                        thread_avatars.push(Sender{avatar_uri: t.sender.avatar_uri.clone(), display_name: t.sender.name.clone()})
                                    }

                                    cx.render(rsx!(
                                        MessageView {
                                            key: "{i}",
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
                                                        
                                                        *replying_to.write() = Some(replying);
                                                    }
                                                    MenuOption::Close => {
                                                        info!("close");
                                                    }
                                                    MenuOption::ShowThread => {
                                                        info!("thread");
                                                        *timeline_thread.write() = Some(TimelineThread { event_id: event_id.clone(), thread: thread.clone(), count: count.clone(), latest_event: latest_event.clone() })
                                                    }
                                                    MenuOption::CreateThread => {

                                                    }
                                                }
                                                
                                            }
                                        }
                                    ))
                                }
                                TimelineRelation::Thread(_) => {
                                    None
                                }
                            }
                        })

                        if let Some(messages) = &cx.props.thread {
                            rsx!(
                                messages.iter().enumerate().map(|(i, m)| {
                                    let message = m.clone();
                                    let event_id = message.event_id.clone();

                                    cx.render(rsx!(
                                        MessageView {
                                            key: "{i}",
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
                                                        if let Some(eid) = &event_id {
                                                            let replying = ReplyingTo { 
                                                                event_id: eid.clone(), 
                                                                content: message.body.clone(), 
                                                                display_name: message.sender.name.clone(), 
                                                                avatar_uri: message.sender.avatar_uri.clone(),
                                                                origin: message.origin.clone()
                                                            };
                                                            
                                                            *replying_to.write() = Some(replying);
                                                        }
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
                                    ))
                                })
                            )
                        }
                    )
                }
            )
        }
    })
}
