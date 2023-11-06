use dioxus::prelude::*;

use super::icon::IconShape;

pub struct Send;
impl IconShape for Send {
    fn view_box(&self) -> String {
        String::from("0 0 20 20")    
    }
    fn child_elements(&self) -> LazyNodes {
        rsx!(path {
            d: "m17.5 2.5-8.25 8.25M17.5 2.5l-5.25 15-3-6.75-6.75-3 15-5.25Z"
        })
    }
}
