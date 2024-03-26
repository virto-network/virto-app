use dioxus::prelude::*;

#[derive(PartialEq, Props)]
pub struct CommunityProps<'a> {
    class: Option<&'a str>,
    title: &'a str,
    icon: &'a str,
    background: &'a str,
}

pub fn Community<'a>(cx: Scope<'a, CommunityProps<'a>>) -> Element<'a> {
    let content__background = format!("community__content--{}", cx.props.background);
    let class_content = cx.props.class.unwrap_or("");

    render!(rsx!(
        section {
            class: "community {class_content}",
            div{
                class: "community__content {content__background}",
                div {
                    class: "community__icon",
                    "{cx.props.icon}"
                }
            }
            span {
                class: "community__title",
                "{cx.props.title}"
            }
        }
    ))
}
