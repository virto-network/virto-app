#![allow(non_snake_case)]

use matrix_sdk::Client;
pub mod components {
    pub mod atoms;
    pub mod molecules;
    pub mod organisms;
}

pub mod hooks {
    pub mod factory;
    pub mod use_attach;
    pub mod use_auth;
    pub mod use_chat;
    pub mod use_client;
    pub mod use_init_app;
    pub mod use_lifecycle;
    pub mod use_listen_message;
    pub mod use_messages;
    pub mod use_modal;
    pub mod use_notification;
    pub mod use_public;
    pub mod use_reply;
    pub mod use_room;
    pub mod use_room_preview;
    pub mod use_rooms;
    pub mod use_send_attach;
    pub mod use_send_message;
    pub mod use_session;
    pub mod use_thread;
}

pub mod services {
    pub mod matrix;
}

pub mod utils {
    pub mod get_element;
    pub mod get_homeserver;
    pub mod get_param;
    pub mod i18n_get_key_value;
    pub mod matrix;
    pub mod nice_bytes;
    pub mod sync_room;
    pub mod vec_to_url;
}

pub mod pages {
    pub mod chat;
    pub mod login;
    pub mod page_not_found;
    pub mod profile;
    pub mod route;
    pub mod signup;
}

#[derive(Clone)]
pub struct MatrixClientState {
    pub client: Option<Client>,
}
