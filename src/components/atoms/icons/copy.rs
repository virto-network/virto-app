use dioxus::prelude::*;

use super::icon::IconShape;

pub struct CopyIcon;
impl IconShape for CopyIcon {
    fn view_box(&self) -> String {
        String::from("0 0 19 18")
    }
    fn child_elements(&self) -> LazyNodes {
        rsx!(path {
            d: "M4.542 13.303h1.142v.998c0 1.551.854 2.4 2.413 2.4h7.027c1.559 0 2.413-.849 2.413-2.4V7.246c0-1.552-.854-2.4-2.413-2.4h-1.142V3.843c0-1.545-.854-2.4-2.413-2.4H4.542c-1.559 0-2.413.848-2.413 2.4v7.061c0 1.552.854 2.4 2.413 2.4Zm.191-1.764c-.546 0-.84-.273-.84-.854V4.06c0-.582.294-.848.84-.848h6.645c.547 0 .84.266.84.848v.786h-4.12c-1.559 0-2.413.847-2.413 2.4v4.292h-.95Zm3.555 3.398c-.547 0-.84-.267-.84-.848V7.458c0-.581.293-.848.84-.848h6.645c.546 0 .84.267.84.848v6.63c0 .582-.293.849-.84.849H8.288Z"
        })
    }
}