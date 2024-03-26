use dioxus::prelude::*;
use gloo::storage::{errors::StorageError, LocalStorage};
use log::info;
use matrix_sdk::HttpError;
use matrix_sdk::{config::SyncSettings, Client};

use ruma::api::client::filter::{FilterDefinition, RoomEventFilter};
use ruma::api::client::sync::sync_events;
use ruma::events::RoomEventType;
use std::time::Duration;

use crate::services::matrix::matrix::FullSession;

pub fn use_session(cx: &ScopeState) -> &UseSessionState {
    let user = use_shared_state::<Option<UserSession>>(cx).expect("Unable to use UserSession");

    cx.use_hook(move || UseSessionState { data: user.clone() })
}

#[derive(Clone)]
pub struct UseSessionState {
    data: UseSharedState<Option<UserSession>>,
}

#[derive(Clone, Debug)]
pub struct UserSession {
    pub user_id: String,
    pub device_id: Option<String>,
    pub is_guest: bool,
}

pub enum SessionError {
    SaveFailed,
    GetFailed,
    WhoamiFailed,
    SyncFailed,
}

impl UseSessionState {
    fn set(&self, data: UserSession) {
        *self.data.write() = Some(data);
    }

    pub fn get(&self) -> Option<UserSession> {
        self.data.read().clone()
    }

    pub fn is_guest(&self) -> bool {
        self.get().is_some_and(|s|s.is_guest)
    }

    pub async fn whoami(&self, client: Client) -> Result<UserSession, HttpError> {
        let user_id = client.user_id();
        let device_id = client.device_id();

        let data = if let Some(user_id) = user_id {
            UserSession {
                user_id: user_id.to_string(),
                device_id: device_id.map(|id| id.to_string()),
                is_guest: device_id
                    .map(|id| {
                        if id.to_string().contains("guest") {
                            Some(())
                        } else {
                            None
                        }
                    })
                    .flatten()
                    .is_some(),
            }
        } else {
            let user = client.whoami().await?;
            UserSession {
                user_id: user.user_id.to_string(),
                device_id: user.device_id.map(|id| id.to_string()),
                is_guest: user.is_guest,
            }
        };

        Self::set(&self, data.clone());
        Ok(data)
    }

    pub async fn sync(
        &self,
        client: Client,
        initial_sync_token: Option<String>,
    ) -> Result<(), SessionError> {
        let mut room_event_filter = RoomEventFilter::empty();
        room_event_filter.rooms = Some(&[]);

        let filter_event_type = vec![RoomEventType::RoomMessage.to_string()];
        room_event_filter.types = Some(&filter_event_type);

        let mut filter = FilterDefinition::empty();
        filter.room.include_leave = false;
        filter.room.account_data = room_event_filter.clone();
        filter.room.timeline = room_event_filter.clone();
        filter.room.ephemeral = room_event_filter.clone();
        filter.room.state = room_event_filter.clone();

        let mut sync_settings = SyncSettings::default()
            .filter(sync_events::v3::Filter::FilterDefinition(filter))
            .timeout(Duration::from_millis(1000))
            .full_state(true);

        if let Some(sync_token) = initial_sync_token {
            sync_settings = sync_settings.token(sync_token);
        }

        match client.sync_once(sync_settings.clone()).await {
            Ok(response) => {
                Self::whoami(&self, client)
                    .await
                    .map_err(|_| SessionError::WhoamiFailed)?;
                Self::persist_sync_token(&response.next_batch).await?;

                Ok(())
            }
            Err(err) => {
                info!("An error occurred during initial sync: {err}");
                info!("Trying again from login…");

                Err(SessionError::SyncFailed)
            }
        }
    }

    pub fn persist_session_file(&self, session_file: &str) -> Result<(), SessionError> {
        <LocalStorage as gloo::storage::Storage>::set("session_file", session_file)
            .map_err(|_| SessionError::SaveFailed)
    }

    async fn persist_sync_token(sync_token: &str) -> anyhow::Result<(), SessionError> {
        let serialized_session: Result<String, StorageError> =
            <LocalStorage as gloo::storage::Storage>::get("session_file");

        let serialized_session = match serialized_session {
            Ok(s) => s,
            Err(e) => e.to_string(),
        };

        let mut full_session: FullSession =
            serde_json::from_str(&serialized_session).map_err(|_| SessionError::GetFailed)?;

        full_session.sync_token = Some(sync_token.to_owned());
        let serialized_session =
            serde_json::to_string(&full_session).map_err(|_| SessionError::GetFailed)?;
        <LocalStorage as gloo::storage::Storage>::set("session_file", serialized_session)
            .map_err(|_| SessionError::SaveFailed)?;

        Ok(())
    }
}
