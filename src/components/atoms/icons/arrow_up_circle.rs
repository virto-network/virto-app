use dioxus::prelude::*;

use super::icon::IconShape;

pub struct ArrowUpCircle;
impl IconShape for ArrowUpCircle {
    fn view_box(&self) -> String {
        String::from("0 0 24 24")    
    }
    fn child_elements(&self) -> LazyNodes {
        rsx!(path {
            d: "M7 11L10 8L13 11M19 10C19 5.02944 14.9706 1 10 1C5.02944 1 1 5.02944 1 10C1 14.9706 5.02944 19 10 19C14.9706 19 19 14.9706 19 10Z"
        })
    }
}
