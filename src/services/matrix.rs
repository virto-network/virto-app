pub mod matrix {
    use std::{
        collections::HashMap,
        ops::Deref,
        time::{Duration, UNIX_EPOCH},
    };

    use chrono::{DateTime, Local, Utc};
    use log::info;

    use matrix_sdk::{
        attachment::AttachmentConfig,
        config::RequestConfig,
        deserialized_responses::{SyncTimelineEvent, TimelineSlice},
        media::{MediaFormat, MediaRequest, MediaThumbnailSize},
        room::{Invited, MessagesOptions, Room},
        ruma::{
            api::{
                self,
                client::{
                    filter::{LazyLoadOptions, RoomEventFilter},
                    media::get_content_thumbnail::v3::Method,
                    room::{create_room::v3::RoomPreset, Visibility},
                    uiaa,
                },
            },
            assign,
            events::{
                room::{
                    avatar::RoomAvatarEventContent,
                    message::{
                        FileMessageEventContent, ImageMessageEventContent, InReplyTo,
                        MessageFormat, MessageType, OriginalSyncRoomMessageEvent, Relation,
                        RoomMessageEventContent, VideoMessageEventContent,
                    },
                    MediaSource,
                },
                AnyInitialStateEvent, AnyMessageLikeEvent, AnySyncMessageLikeEvent,
                AnySyncTimelineEvent, AnyTimelineEvent, EmptyStateKey, InitialStateEvent,
                MessageLikeEvent, SyncMessageLikeEvent,
            },
            serde::Raw,
            MilliSecondsSinceUnixEpoch, OwnedEventId, OwnedUserId, RoomId, TransactionId, UInt,
        },
        Client, Error,
    };
    use mime::Mime;
    use ruma::{
        api::client::{
            account::register::RegistrationKind, directory::get_public_rooms,
            message::send_message_event::v3::Response,
        },
        events::room::message::Thread,
        OwnedMxcUri, OwnedRoomId, UserId,
    };
    use url::Url;

    use crate::{
        components::atoms::room::RoomItem,
        hooks::{use_send_message::SendMessageError, use_session::UserSession},
        pages::chat::room::group::Profile,
        utils::matrix::{mxc_to_download_uri, mxc_to_thumbnail_uri, ImageMethod, ImageSize},
    };

    use matrix_sdk::Session;

    use serde::{Deserialize, Serialize};

    // #[derive(Sized)]
    pub struct Attachment {
        pub body: String,
        pub(crate) data: Vec<u8>,
        pub content_type: Mime,
    }

    pub struct AttachmentStream {
        pub attachment: Attachment,
        pub send_to_thread: bool,
    }

    pub enum ClientError {
        InvalidHomeserver,
        RequestFailed,
    }
    pub async fn create_client(homeserver_url_str: &str) -> Result<Client, ClientError> {
        let homeserver_url =
            Url::parse(&homeserver_url_str).map_err(|_| ClientError::InvalidHomeserver)?;
        Client::new(homeserver_url)
            .await
            .map_err(|_| ClientError::RequestFailed)
    }

    pub enum JoinRoomError {
        RequestFailed,
        InvalidRoomId,
    }

    pub async fn join_room(
        client: &Client,
        room_id: &RoomId,
    ) -> Result<OwnedRoomId, JoinRoomError> {
        let response = client
            .join_room_by_id(&room_id)
            .await
            .map_err(|_| JoinRoomError::RequestFailed)?;

        Ok(response.room_id)
    }

    pub async fn send_message(
        client: &Client,
        room_id: &RoomId,
        msg: MessageType,
        reply_to: Option<OwnedEventId>,
        thread_to: Option<OwnedEventId>,
        latest_event: Option<OwnedEventId>,
    ) -> Result<Response, SendMessageError> {
        let room = client
            .get_joined_room(&room_id)
            .ok_or(SendMessageError::RoomNotFound)?;
        let tx_id = TransactionId::new();

        let mut event_content = RoomMessageEventContent::new(msg);

        event_content.relates_to = if let Some(l) = latest_event {
            thread_to
                .as_ref()
                .map(|t| Relation::Thread(Thread::plain(t.clone(), l)))
        } else if let Some(r) = reply_to {
            if let Some(t) = &thread_to {
                Some(Relation::Thread(Thread::reply(t.clone(), r)))
            } else {
                Some(Relation::Reply {
                    in_reply_to: InReplyTo::new(r.clone()),
                })
            }
        } else {
            None
        };

        room.send(event_content, Some(&tx_id))
            .await
            .map_err(|_| SendMessageError::DispatchMessage)
    }

    pub async fn upload_attachment(
        client: &Client,
        attach: &Attachment,
    ) -> Result<ruma::api::client::media::create_content::v3::Response, Error> {
        client
            .media()
            .upload(&attach.content_type, &attach.data)
            .await
    }

    pub async fn send_attachment(
        client: &Client,
        room_id: &RoomId,
        uri: &OwnedMxcUri,
        attach: &Attachment,
        reply_to: Option<OwnedEventId>,
        thread_to: Option<OwnedEventId>,
        latest_event: Option<OwnedEventId>,
    ) -> Result<Response, SendMessageError> {
        let room = client
            .get_joined_room(&room_id)
            .ok_or(SendMessageError::RoomNotFound)?;

        let message_type = match attach.content_type.type_() {
            mime::IMAGE => {
                let event_content =
                    ImageMessageEventContent::plain(attach.body.clone(), uri.clone(), None);

                MessageType::Image(event_content)
            }
            mime::VIDEO => {
                let event_content =
                    VideoMessageEventContent::plain(attach.body.clone(), uri.clone(), None);

                MessageType::Video(event_content)
            }
            mime::APPLICATION => {
                let event_content =
                    FileMessageEventContent::plain(attach.body.clone(), uri.clone(), None);

                MessageType::File(event_content)
            }
            _ => return Err(SendMessageError::InvalidFile),
        };

        if reply_to.is_some() || latest_event.is_some() {
            send_message(
                client,
                room_id,
                message_type,
                reply_to,
                thread_to,
                latest_event,
            )
            .await
        } else {
            room.send_attachment(
                &attach.body,
                &attach.content_type,
                &attach.data,
                AttachmentConfig::new(),
            )
            .await
            .map_err(|_| SendMessageError::DispatchMessage)
        }
    }

    pub fn listen_messages(client: &Client) {
        client.add_event_handler(|ev: OriginalSyncRoomMessageEvent| async move {
            info!("Received event {}: {:?}", ev.sender, ev.content.body());
        });
    }

    pub struct Conversations {
        pub rooms: Vec<RoomItem>,
        pub spaces: HashMap<RoomItem, Vec<RoomItem>>,
    }

    pub async fn invited_rooms(client: &Client) -> Result<Vec<RoomItem>, String> {
        let mut rooms = Vec::new();

        for room in client.invited_rooms() {
            let Ok(item) = format_invited_room(&client, room).await else {
                continue;
            };

            rooms.push(item);
        }

        Ok(rooms)
    }

    pub async fn format_invited_room(client: &Client, room: Invited) -> Result<RoomItem, String> {
        let avatar_uri: Option<String> = room
            .avatar_url()
            .and_then(|uri| mxc_to_thumbnail_uri(&uri, ImageSize::default(), ImageMethod::CROP));

        let Some(content) = room.create_content() else {
            return Err(String::from("Content not found"));
        };
        log::info!("{:?}", content.creator);

        let Ok(room_creator) = find_user_by_id(content.creator.as_str(), &client).await else {
            return Err(String::from("User not found"));
        };

        let room = RoomItem {
            avatar_uri: avatar_uri,
            id: room.room_id().to_string(),
            name: room.name().unwrap_or(room_creator.displayname),
            is_public: true,
            is_direct: false,
        };

        Ok(room)
    }

    pub async fn public_rooms_and_spaces(
        client: &Client,
        limit: Option<u32>,
        since: Option<&str>,
        server: Option<&str>,
    ) -> Result<Conversations, String> {
        let mut rooms = Vec::new();
        let mut spaces: HashMap<RoomItem, Vec<RoomItem>> = HashMap::new();

        let limit = limit.map(UInt::from);
        let server = server.map(|s| s.try_into().ok()).flatten();

        let request = assign!(get_public_rooms::v3::Request::new(), {
            limit,
            since,
            server,
        });

        let response = client
            .send(request, Some(RequestConfig::default().force_auth()))
            .await
            .map_err(|_| String::from("ServerError"))?;

        for room in response.chunk {
            let avatar_uri: Option<String> = room.avatar_url.and_then(|uri| {
                mxc_to_thumbnail_uri(&uri, ImageSize::default(), ImageMethod::CROP)
            });

            let room = RoomItem {
                avatar_uri: avatar_uri,
                id: room.room_id.to_string(),
                name: room.name.unwrap_or(String::from("Unnamed")),
                is_public: true,
                is_direct: false,
            };

            rooms.push(room);
        }

        let mut to_list_rooms = vec![];

        for (key, value) in spaces.iter_mut() {
            rooms.iter().for_each(|room| {
                let room_homeserver = room.id.split(":").collect::<Vec<&str>>()[1];
                let space_homeserver = key.id.split(":").collect::<Vec<&str>>()[1];

                if room_homeserver.eq(space_homeserver) && !room.is_direct && !room.id.eq(&key.id) {
                    value.push(room.clone());
                } else {
                    to_list_rooms.push(room.clone());
                }
            });
        }

        if spaces.len() == 0 {
            to_list_rooms = rooms;
        }

        Ok(Conversations {
            rooms: to_list_rooms,
            spaces: spaces,
        })
    }

    pub async fn list_rooms_and_spaces(
        client: &Client,
        session_data: UserSession,
    ) -> Conversations {
        let mut rooms = Vec::new();
        let mut spaces = HashMap::new();
        let rooms_response = client.rooms();

        for room in rooms_response {
            if let Room::Left(r) = &room {
                if !r.is_space() {
                    continue;
                }
            }

            let is_direct = room.is_direct();
            let is_space = room.is_space();

            let avatar_url = room.avatar_url();

            let avatar_uri: Option<String> = if is_direct {
                if let Some(member) = room.direct_targets().into_iter().next() {
                    room.get_member(&member)
                        .await
                        .ok()
                        .flatten()
                        .map(|member| {
                            let avatar_url = member.avatar_url();

                            avatar_url.and_then(|uri| {
                                mxc_to_thumbnail_uri(&uri, ImageSize::default(), ImageMethod::CROP)
                            })
                        })
                        .flatten()
                } else {
                    None
                }
            } else {
                avatar_url.and_then(|uri| {
                    mxc_to_thumbnail_uri(&uri, ImageSize::default(), ImageMethod::CROP)
                })
            };

            if let Some(name) = room.name() {
                let room = RoomItem {
                    avatar_uri: avatar_uri,
                    id: room.room_id().to_string(),
                    name: name,
                    is_public: room.is_public(),
                    is_direct,
                };

                if is_space {
                    spaces.insert(room, vec![]);
                } else {
                    rooms.push(room);
                }
            } else {
                let users = room.members().await;

                if let Ok(members) = users {
                    let member = members
                        .into_iter()
                        .find(|member| !member.user_id().to_string().eq(&session_data.user_id));

                    if let Some(m) = member {
                        let name = m.name();

                        rooms.push(RoomItem {
                            avatar_uri: avatar_uri,
                            id: room.room_id().to_string(),
                            name: String::from(name),
                            is_public: room.is_public(),
                            is_direct,
                        })
                    }
                }
            }
        }

        let mut to_list_rooms = vec![];

        for (key, value) in spaces.iter_mut() {
            rooms.iter().for_each(|room| {
                let room_homeserver = room.id.split(":").collect::<Vec<&str>>()[1];
                let space_homeserver = key.id.split(":").collect::<Vec<&str>>()[1];

                if room_homeserver.eq(space_homeserver) && !room.is_direct && !room.id.eq(&key.id) {
                    value.push(room.clone());
                } else {
                    to_list_rooms.push(room.clone());
                }
            });
        }

        if spaces.len() == 0 {
            to_list_rooms = rooms;
        }

        Conversations {
            rooms: to_list_rooms,
            spaces: spaces,
        }
    }

    pub enum LeaveRoomError {
        InvalidRoomId,
        RoomNotFound,
        Failed,
    }

    pub async fn leave_room(client: &Client, id: &str) -> Result<(), LeaveRoomError> {
        let room_id = RoomId::parse(id).map_err(|_| LeaveRoomError::InvalidRoomId)?;
        let room = client
            .get_joined_room(&room_id)
            .ok_or(LeaveRoomError::RoomNotFound)?;

        room.leave().await.map_err(|_| LeaveRoomError::Failed)
    }

    #[derive(PartialEq, Debug, Clone)]
    pub struct RoomMember {
        pub id: String,
        pub name: String,
        pub avatar_uri: Option<String>,
    }

    pub enum RoomMemberError {
        NotFound,
    }

    #[derive(Clone, Debug)]
    pub enum FindUserError {
        InvalidUserId,
        UserNotFound,
        InvalidUsername,
    }

    pub async fn find_user_by_id(id: &str, client: &Client) -> Result<Profile, FindUserError> {
        let u = UserId::parse(&id).map_err(|_| FindUserError::InvalidUserId)?;

        let u = u.deref();

        let request = matrix_sdk::ruma::api::client::profile::get_profile::v3::Request::new(u);

        let response = client
            .send(request, None)
            .await
            .map_err(|_| FindUserError::UserNotFound)?;

        let displayname = response.displayname.ok_or(FindUserError::InvalidUsername)?;

        let avatar_uri = response
            .avatar_url
            .map(|uri| {
                mxc_to_thumbnail_uri(
                    &uri,
                    ImageSize {
                        width: 48,
                        height: 48,
                    },
                    ImageMethod::CROP,
                )
            })
            .flatten();

        let profile = Profile {
            displayname,
            avatar_uri,
            id: id.to_string(),
        };

        Ok(profile)
    }

    pub async fn room_member(
        member_id: OwnedUserId,
        room: &Room,
    ) -> Result<RoomMember, RoomMemberError> {
        let member = room
            .get_member(&member_id)
            .await
            .map_err(|_| RoomMemberError::NotFound)?
            .ok_or(RoomMemberError::NotFound)?;

        let avatar_uri = member
            .avatar_url()
            .and_then(|uri| mxc_to_thumbnail_uri(&uri, ImageSize::default(), ImageMethod::SCALE));

        let name = member.display_name().ok_or(RoomMemberError::NotFound)?;

        Ok(RoomMember {
            id: member.user_id().to_string(),
            name: name.to_string(),
            avatar_uri,
        })
    }

    #[derive(Clone)]
    pub struct AccountInfo {
        pub name: String,
        pub avatar_uri: Option<String>,
    }

    pub async fn account(client: &Client) -> AccountInfo {
        let avatar = client.account().get_avatar_url().await;
        let display_name = client.account().get_display_name().await;

        let avatar_uri = avatar
            .ok()
            .flatten()
            .map(|uri| mxc_to_thumbnail_uri(&uri, ImageSize::default(), ImageMethod::CROP))
            .flatten();

        let name = display_name.ok().flatten().unwrap_or(String::from(""));

        AccountInfo { name, avatar_uri }
    }

    #[derive(Debug)]
    pub enum CreateRoomError {
        RequestFailed,
        InvalidMedia,
        InvalidInfo,
    }

    pub async fn create_room(
        client: &Client,
        is_dm: bool,
        users: &[OwnedUserId],
        name: Option<String>,
        avatar: Option<Vec<u8>>,
    ) -> Result<api::client::room::create_room::v3::Response, CreateRoomError> {
        let mut request = api::client::room::create_room::v3::Request::new();

        let mut init_state_ev_vec: Vec<Raw<AnyInitialStateEvent>> = vec![];

        if let Some(data) = avatar {
            let response = client
                .media()
                .upload(&mime::IMAGE_JPEG, &data)
                .await
                .map_err(|_| CreateRoomError::InvalidMedia)?;

            let mut avatar_content = RoomAvatarEventContent::new();
            avatar_content.url = Some(response.content_uri);

            let init_state_ev: InitialStateEvent<RoomAvatarEventContent> = InitialStateEvent {
                content: avatar_content,
                state_key: EmptyStateKey,
            };

            let raw_init_state_ev =
                Raw::new(&init_state_ev).map_err(|_| CreateRoomError::InvalidInfo)?;

            let raw_any_init_state_ev: Raw<AnyInitialStateEvent> = raw_init_state_ev.cast();
            init_state_ev_vec.push(raw_any_init_state_ev);

            request.initial_state = &init_state_ev_vec;
        }

        request.name = name.as_deref();
        request.is_direct = is_dm;

        let visibility = Visibility::Private;
        if is_dm {
            request.invite = users;
            request.visibility = visibility.clone();
            request.preset = Some(RoomPreset::PrivateChat);
        }

        client.create_room(request).await.map_err(|e| {
            log::error!("{:?}", e);
            CreateRoomError::RequestFailed
        })
    }

    #[derive(PartialEq, Debug, Clone)]
    pub enum ImageType {
        URL(String),
        Media(Vec<u8>),
    }

    #[derive(PartialEq, Debug, Clone)]
    pub struct FileContent {
        pub size: Option<u64>,
        pub body: String,
        pub source: Option<ImageType>,
    }

    #[derive(PartialEq, Debug, Clone)]
    pub enum TimelineMessageType {
        Image(FileContent),
        Text(String),
        Html(String),
        File(FileContent),
        Video(FileContent),
    }

    #[derive(PartialEq, Debug, Clone)]
    pub enum EventOrigin {
        OTHER,
        ME,
    }

    #[derive(PartialEq, Debug, Clone)]
    pub struct TimelineMessage {
        pub event_id: String,
        pub sender: RoomMember,
        pub body: TimelineMessageType,
        pub origin: EventOrigin,
        pub time: String,
    }

    #[derive(PartialEq, Debug, Clone)]
    pub struct TimelineMessageReply {
        pub event: TimelineMessage,
        pub reply: Option<TimelineMessage>,
    }

    #[derive(PartialEq, Debug, Clone)]
    pub struct TimelineMessageThread {
        pub event_id: String,
        pub thread: Vec<TimelineMessage>,
    }

    #[derive(PartialEq, Debug, Clone)]
    pub struct TimelineThread {
        pub event_id: String,
        pub thread: Vec<TimelineMessage>,
        pub latest_event: String,
        pub count: usize,
    }

    #[derive(PartialEq, Debug, Clone)]
    pub enum TimelineRelation {
        None(TimelineMessage),
        Reply(TimelineMessageReply),
        CustomThread(TimelineThread),
        Thread(TimelineMessageThread),
    }

    #[derive(PartialEq, Debug, Clone)]
    pub enum TimelineError {
        RoomNotFound,
        InvalidLimit,
        MessagesNotFound,
    }

    pub async fn timeline(
        client: &Client,
        room_id: &RoomId,
        limit: u64,
        from: Option<String>,
        old_messages: Vec<TimelineRelation>,
        session_data: UserSession,
    ) -> Result<(Option<String>, Vec<TimelineRelation>), TimelineError> {
        let mut messages: Vec<TimelineRelation> = old_messages;

        let room = client
            .get_room(&room_id)
            .ok_or(TimelineError::RoomNotFound)?;

        let filter = assign!(RoomEventFilter::default(), {
            lazy_load_options: LazyLoadOptions::Enabled { include_redundant_members: false },
        });
        let options = assign!(MessagesOptions::backward(), {
            limit: UInt::new(limit).ok_or(TimelineError::InvalidLimit)?,
            filter,
            from: from.as_deref()
        });

        let m = room
            .messages(options)
            .await
            .map_err(|_| TimelineError::MessagesNotFound)?;

        let t = TimelineSlice::new(
            m.chunk.into_iter().map(SyncTimelineEvent::from).collect(),
            m.start,
            m.end.clone(),
            false,
            false,
        );

        for sync_timeline_event in t.events.iter() {
            let deserialized = deserialize_any_timeline_event(
                sync_timeline_event
                    .event
                    .deserialize()
                    .expect("can't deserialize iter events: timeline"),
                &room,
                &session_data.user_id,
                &client,
            )
            .await;

            if let Some(timeline_relation) = deserialized {
                match &timeline_relation {
                    TimelineRelation::Thread(thread) => {
                        // Position of an existing thread timeline

                        let position = messages.iter().position(|m| {
                            if let TimelineRelation::CustomThread(thread) = m {
                                thread.event_id.eq(&thread.event_id)
                            } else {
                                false
                            }
                        });

                        if let Some(p) = position {
                            if let TimelineRelation::CustomThread(ref mut timeline_thread) =
                                messages[p]
                            {
                                timeline_thread.thread.push(thread.thread[0].clone());
                                timeline_thread.thread.rotate_right(1);
                            };
                        } else {
                            let n = TimelineRelation::CustomThread(TimelineThread {
                                event_id: thread.event_id.clone(),
                                thread: thread.thread.clone(),
                                latest_event: thread.thread[thread.thread.len() - 1]
                                    .clone()
                                    .event_id,
                                count: thread.thread.len(),
                            });

                            messages.push(n);
                            messages.rotate_right(1);
                        }
                    }
                    TimelineRelation::None(message) => {
                        // Position of a head thread timeline
                        let position = messages.iter().position(|m| {
                            let TimelineRelation::CustomThread(thread) = m else {
                                return false;
                            };

                            thread.event_id.eq(&message.event_id)
                        });

                        let Some(p) = position else {
                            messages.push(timeline_relation);
                            messages.rotate_right(1);
                            continue;
                        };

                        if let TimelineRelation::CustomThread(ref mut timeline_thread) = messages[p]
                        {
                            let formatted_thread = format_head_thread(
                                sync_timeline_event
                                    .event
                                    .deserialize()
                                    .expect("can't deserialize event custom thread: timeline"),
                            );

                            if let Some((_, latest_event)) = formatted_thread {
                                timeline_thread.latest_event = latest_event;
                            }
                            timeline_thread.thread.push(message.clone());
                            timeline_thread.thread.rotate_right(1);
                        };
                    }
                    _ => {
                        messages.push(timeline_relation);
                        messages.rotate_right(1);
                    }
                }
            }
        }

        Ok((m.end, messages))
    }

    pub fn format_head_thread(ev: AnySyncTimelineEvent) -> Option<(usize, String)> {
        let AnySyncTimelineEvent::MessageLike(AnySyncMessageLikeEvent::RoomMessage(
            SyncMessageLikeEvent::Original(original),
        )) = ev
        else {
            return None;
        };

        original
            .unsigned
            .relations
            .map(|relations| {
                relations.thread.map(|thread| {
                    (
                        2,
                        thread
                            .latest_event
                            .deserialize()
                            .expect("can't deserialize latest event: format_head_thread")
                            .event_id()
                            .to_string(),
                    )
                })
            })
            .flatten()
    }

    pub async fn deserialize_any_timeline_event(
        event: AnySyncTimelineEvent,
        room: &Room,
        logged_user_id: &str,
        client: &Client,
    ) -> Option<TimelineRelation> {
        log::info!("{:?}", event);
        let AnySyncTimelineEvent::MessageLike(AnySyncMessageLikeEvent::RoomMessage(
            SyncMessageLikeEvent::Original(original),
        )) = event
        else {
            return None;
        };

        let message_type = &original.content.msgtype;
        let event_id = original.event_id;

        let Ok(member) = room_member(original.sender, &room).await else {
            return None;
        };

        let relates = &original.content.relates_to;
        let time = original.origin_server_ts;

        let formatted_message = format_original_any_room_message_event(
            &message_type,
            event_id,
            &member,
            &logged_user_id,
            time,
            &client,
        )
        .await;

        let mut message_result = None;

        let Some(relation) = relates else {
            if let Some(message) = formatted_message {
                message_result = Some(TimelineRelation::None(message));
            }

            return message_result;
        };

        match &relation {
            Relation::_Custom => {
                if let Some(message) = formatted_message {
                    message_result = Some(TimelineRelation::None(message));
                }
            }
            _ => {
                if let Some(message) = formatted_message {
                    message_result = format_relation_from_event(
                        &message_type,
                        relates,
                        &room,
                        message,
                        &member,
                        &logged_user_id,
                        time,
                        &client,
                    )
                    .await;
                }
            }
        }
        message_result
    }

    pub async fn deserialize_timeline_event(
        event: AnyTimelineEvent,
        room: &Room,
        logged_user_id: &str,
        client: &Client,
    ) -> Option<TimelineMessage> {
        let AnyTimelineEvent::MessageLike(AnyMessageLikeEvent::RoomMessage(
            MessageLikeEvent::Original(original),
        )) = event
        else {
            return None;
        };

        let Ok(member) = room_member(original.sender, &room).await else {
            return None;
        };

        let message_type = &original.content.msgtype;
        let event_id = original.event_id;
        let time = original.origin_server_ts;

        let message_result = format_original_any_room_message_event(
            &message_type,
            event_id,
            &member,
            &logged_user_id,
            time,
            &client,
        )
        .await;

        message_result
    }

    pub async fn format_original_any_room_message_event(
        n: &MessageType,
        event: OwnedEventId,
        member: &RoomMember,
        logged_user_id: &str,
        time: MilliSecondsSinceUnixEpoch,
        client: &Client,
    ) -> Option<TimelineMessage> {
        let mut message_result = None;

        let timestamp = {
            let d = UNIX_EPOCH + Duration::from_millis(time.0.into());

            let datetime = DateTime::<Local>::from(d);
            datetime.format("%H:%M").to_string()
        };

        match &n {
            MessageType::Image(message_event_content) => match &message_event_content.source {
                MediaSource::Plain(mx_uri) => {
                    let https_uri = mxc_to_download_uri(&mx_uri);

                    let size = message_event_content
                        .info
                        .as_ref()
                        .and_then(|file_info| {
                            file_info
                                .size
                                .map(|size| size.to_string().parse::<u64>().ok())
                        })
                        .flatten();

                    if let Some(uri) = https_uri {
                        message_result = Some(TimelineMessage {
                            event_id: event.to_string(),
                            sender: member.clone(),
                            body: TimelineMessageType::Image(FileContent {
                                size,
                                body: message_event_content.body.clone(),
                                source: Some(ImageType::URL(uri)),
                            }),
                            origin: if member.id.eq(logged_user_id) {
                                EventOrigin::ME
                            } else {
                                EventOrigin::OTHER
                            },
                            time: timestamp,
                        });
                    }
                }
                MediaSource::Encrypted(_) => {
                    let media_content = client
                        .media()
                        .get_media_content(
                            &MediaRequest {
                                source: message_event_content.source.clone(),
                                format: MediaFormat::Thumbnail(MediaThumbnailSize {
                                    method: Method::Crop,
                                    width: UInt::new(16).unwrap(),
                                    height: UInt::new(16).unwrap(),
                                }),
                            },
                            true,
                        )
                        .await;

                    let size = message_event_content
                        .info
                        .as_ref()
                        .and_then(|file_info| {
                            file_info
                                .size
                                .map(|size| size.to_string().parse::<u64>().ok())
                        })
                        .flatten();

                    if let Ok(content) = media_content {
                        message_result = Some(TimelineMessage {
                            event_id: event.to_string(),
                            sender: member.clone(),
                            body: TimelineMessageType::Image(FileContent {
                                size,
                                body: message_event_content.body.clone(),
                                source: Some(ImageType::Media(content)),
                            }),
                            origin: if member.id.eq(logged_user_id) {
                                EventOrigin::ME
                            } else {
                                EventOrigin::OTHER
                            },
                            time: timestamp,
                        });
                    }
                }
            },
            MessageType::Text(content) => {
                message_result = Some(TimelineMessage {
                    event_id: event.to_string(),
                    sender: member.clone(),
                    body: TimelineMessageType::Text(content.body.clone()),
                    origin: if member.id.eq(logged_user_id) {
                        EventOrigin::ME
                    } else {
                        EventOrigin::OTHER
                    },
                    time: timestamp,
                });

                if let Some(formatted) = &content.formatted {
                    match formatted.format {
                        MessageFormat::Html => {
                            if let Some(ref mut message) = message_result {
                                message.body = TimelineMessageType::Html(formatted.body.clone());
                            }
                        }
                        _ => {}
                    }
                };
            }
            MessageType::File(message) => match &message.source {
                MediaSource::Plain(mx_uri) => {
                    let uri = mxc_to_download_uri(&mx_uri);
                    let source = uri.and_then(|uri| Some(ImageType::URL(uri)));

                    let size = message
                        .info
                        .as_ref()
                        .and_then(|file_info| {
                            file_info
                                .size
                                .map(|size| size.to_string().parse::<u64>().ok())
                        })
                        .flatten();

                    message_result = Some(TimelineMessage {
                        event_id: event.to_string(),
                        sender: member.clone(),
                        body: TimelineMessageType::File(FileContent {
                            size,
                            body: message.body.clone(),
                            source,
                        }),
                        origin: if member.id.eq(logged_user_id) {
                            EventOrigin::ME
                        } else {
                            EventOrigin::OTHER
                        },
                        time: timestamp,
                    });
                }
                MediaSource::Encrypted(file) => {
                    let uri = mxc_to_download_uri(&file.url);
                    let source = uri.and_then(|uri| Some(ImageType::URL(uri)));

                    let size = message
                        .info
                        .as_ref()
                        .and_then(|file_info| {
                            file_info
                                .size
                                .map(|size| size.to_string().parse::<u64>().ok())
                        })
                        .flatten();

                    message_result = Some(TimelineMessage {
                        event_id: event.to_string(),
                        sender: member.clone(),
                        body: TimelineMessageType::File(FileContent {
                            size,
                            body: message.body.clone(),
                            source,
                        }),
                        origin: if member.id.eq(logged_user_id) {
                            EventOrigin::ME
                        } else {
                            EventOrigin::OTHER
                        },
                        time: timestamp,
                    });
                }
            },
            MessageType::Video(video) => match &video.source {
                MediaSource::Plain(mx_uri) => {
                    let uri = mxc_to_download_uri(&mx_uri);
                    let source = uri.and_then(|uri| Some(ImageType::URL(uri)));

                    let size = video
                        .info
                        .as_ref()
                        .and_then(|file_info| {
                            file_info
                                .size
                                .map(|size| size.to_string().parse::<u64>().ok())
                        })
                        .flatten();

                    message_result = Some(TimelineMessage {
                        event_id: event.to_string(),
                        sender: member.clone(),
                        body: TimelineMessageType::Video(FileContent {
                            size,
                            body: video.body.clone(),
                            source,
                        }),
                        origin: if member.id.eq(logged_user_id) {
                            EventOrigin::ME
                        } else {
                            EventOrigin::OTHER
                        },
                        time: timestamp,
                    });
                }
                MediaSource::Encrypted(_) => {
                    let message_content = client
                        .media()
                        .get_media_content(
                            &MediaRequest {
                                source: video.source.clone(),
                                format: MediaFormat::File,
                            },
                            true,
                        )
                        .await;

                    let size = video
                        .info
                        .as_ref()
                        .and_then(|file_info| {
                            file_info
                                .size
                                .map(|size| size.to_string().parse::<u64>().ok())
                        })
                        .flatten();

                    if let Ok(content) = message_content {
                        message_result = Some(TimelineMessage {
                            event_id: event.to_string(),
                            sender: member.clone(),
                            body: TimelineMessageType::Video(FileContent {
                                size,
                                body: video.body.clone(),
                                source: Some(ImageType::Media(content)),
                            }),
                            origin: if member.id.eq(logged_user_id) {
                                EventOrigin::ME
                            } else {
                                EventOrigin::OTHER
                            },
                            time: timestamp,
                        });
                    }
                }
            },
            _ => {
                info!("unsuported message_type matrix");
            }
        }

        return message_result;
    }

    pub async fn format_relation_from_event(
        n: &MessageType,
        relates: &Option<Relation>,
        room: &Room,
        message_result: TimelineMessage,
        member: &RoomMember,
        logged_user_id: &str,
        time: MilliSecondsSinceUnixEpoch,
        client: &Client,
    ) -> Option<TimelineRelation> {
        let Some(r) = relates else {
            return None;
        };
        match r {
            Relation::Reply { in_reply_to } => {
                let event = room.event(&in_reply_to.event_id).await.ok()?;
                let timestamp = {
                    let d = UNIX_EPOCH + Duration::from_millis(time.0.into());
                    let datetime = DateTime::<Utc>::from(d);
                    datetime.format("%H:%M").to_string()
                };

                let desc_event = event
                    .event
                    .deserialize()
                    .expect("can't deserialize event: format_relation_from_event");

                let reply =
                    deserialize_timeline_event(desc_event, room, &logged_user_id, &client).await;

                reply.map(|r| {
                    let mut final_message = TimelineMessageReply {
                        event: message_result,
                        reply: Some(r.clone()),
                    };

                    match &r.body {
                        TimelineMessageType::Image(_) => {
                            if n.body().contains("sent an image.") {
                                let to_remove = format!("> <{}> {}", r.sender.id, "sent an image.");

                                let uncleared_content = n.body();
                                let n = uncleared_content.replace(&to_remove, "").clone();

                                let content_body = TimelineMessageType::Text(n);
                                let event = event.event.deserialize().ok()?.event_id().to_string();
                                final_message.event = TimelineMessage {
                                    event_id: event,
                                    sender: member.clone(),
                                    body: content_body,
                                    origin: if member.id.eq(logged_user_id) {
                                        EventOrigin::ME
                                    } else {
                                        EventOrigin::OTHER
                                    },
                                    time: timestamp,
                                };
                            }
                        }
                        TimelineMessageType::Text(body) => {
                            if body.starts_with(">") {
                                let to_remove =
                                    format!("> <{}> {}", r.clone().sender.id, body.trim());

                                let uncleared_content = n.body();
                                let n = uncleared_content.replace(&to_remove, "").clone();

                                let content_body = TimelineMessageType::Text(n);
                                let event = event.event.deserialize().ok()?.event_id().to_string();
                                final_message.event = TimelineMessage {
                                    event_id: event,
                                    sender: member.clone(),
                                    body: content_body,
                                    origin: if member.id.eq(logged_user_id) {
                                        EventOrigin::ME
                                    } else {
                                        EventOrigin::OTHER
                                    },
                                    time: timestamp,
                                };
                            } else {
                                final_message.reply = Some(r);
                            }
                        }
                        TimelineMessageType::Html(_) => {
                            final_message.reply = Some(r);
                        }
                        TimelineMessageType::File(_) => {
                            final_message.reply = Some(r);
                        }
                        TimelineMessageType::Video(_) => {
                            final_message.reply = Some(r);
                        }
                    }

                    Some(TimelineRelation::Reply(final_message))
                })?
            }
            Relation::Thread(in_reply_to) => {
                let final_message = TimelineMessageThread {
                    event_id: in_reply_to.event_id.to_string(),
                    thread: vec![message_result.clone()],
                };

                Some(TimelineRelation::Thread(final_message))
            }
            _ => None,
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct ClientSession {
        pub homeserver: String,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct FullSession {
        pub client_session: ClientSession,
        pub user_session: Session,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub sync_token: Option<String>,
    }

    pub struct LoginResult {
        // client
    }

    use matrix_sdk::ruma::api::client::account::register::v3::Request as RegistrationRequest;

    pub async fn prepare_register(
        homeserver: &str,
        username: &str,
        password: &str,
    ) -> anyhow::Result<(Client, String), Error> {
        let mut request = RegistrationRequest::new();
        request.username = Some(&username);
        request.password = Some(&password);

        let uiaa_dummy = uiaa::Dummy::new();
        request.auth = Some(uiaa::AuthData::Dummy(uiaa_dummy));

        // Temporal use of UnknownError
        let (client, _) = build_client(homeserver, username)
            .await
            .map_err(|e| matrix_sdk::Error::UnknownError(e.into()))?;

        match client.register(request.clone()).await {
            Ok(info) => {
                info!("{:?}", info);
                Ok((client, "registered".to_string()))
            }
            Err(error) => Err(Error::Http(error)),
        }
    }
    pub async fn register(
        homeserver: &str,
        username: &str,
        password: &str,
        recaptcha_token: Option<String>,
        session: Option<String>,
    ) -> anyhow::Result<(Client, String), Error> {
        let mut request = RegistrationRequest::new();
        request.username = Some(&username);
        request.password = Some(&password);

        if let Some(token) = &recaptcha_token {
            let mut uiaa_recaptcha = uiaa::ReCaptcha::new(&token);
            uiaa_recaptcha.session = session.as_deref();
            request.auth = Some(uiaa::AuthData::ReCaptcha(uiaa_recaptcha));
        }

        // Temporal use of UnknownError
        let (client, _) = build_client(homeserver, username)
            .await
            .map_err(|e| matrix_sdk::Error::UnknownError(e.into()))?;

        match client.register(request.clone()).await {
            Ok(info) => {
                info!("signup result {:?}", info);

                client.logout().await?;

                Ok((client, "registered".to_string()))
            }
            Err(error) => Err(Error::Http(error)),
        }
    }

    pub async fn register_as_guest(homeserver: &str) -> Result<(Client, String), String> {
        let mut request = RegistrationRequest::new();
        request.kind = RegistrationKind::Guest;

        let uiaa_dummy = uiaa::Dummy::new();
        request.auth = Some(uiaa::AuthData::Dummy(uiaa_dummy));
        log::info!("{:?}", request);

        let client = Client::builder()
            .homeserver_url(&homeserver)
            .build()
            .await
            .map_err(|_| String::from("Error"))?;

        let Ok(info) = client.register(request.clone()).await else {
            return Err(String::from("Signup Failed"));
        };

        log::info!("signup guest result {:?}", info);

        let (Some(access_token), Some(device_id)) = (info.access_token, info.device_id) else {
            return Err(String::from("Access token or device_id not found"));
        };

        let (client, client_session) = build_client(homeserver, &info.user_id.to_string())
            .await
            .map_err(|_| String::from("Error"))?;

        let user_session = Session {
            access_token: access_token,
            refresh_token: info.refresh_token,
            user_id: info.user_id,
            device_id: device_id,
        };

        let serialized_session = serde_json::to_string(&FullSession {
            client_session,
            user_session: user_session.clone(),
            sync_token: None,
        })
        .map_err(|_| String::from("Serialization failed"))?;

        client
            .restore_login(user_session.clone())
            .await
            .map_err(|_| String::from("Restore failed"))?;

        Ok((client, serialized_session))
    }

    pub async fn login(
        homeserver: &str,
        username: &str,
        password: &str,
    ) -> anyhow::Result<(Client, String)> {
        info!("No previous session found, logging in…");

        let (client, client_session) = build_client(homeserver, username).await?;

        match client
            .login_username(&username, &password)
            .initial_device_display_name("Fido")
            .send()
            .await
        {
            Ok(info) => {
                info!("Logged in as {username}");

                info!("{:?}", info.user_id);
            }
            Err(error) => {
                info!("Error logging in: {error}");
                match error {
                    _ => return Err(error.into()),
                }
            }
        }

        let user_session = client
            .session()
            .expect("A logged-in client should have a session");

        let serialized_session = serde_json::to_string(&FullSession {
            client_session,
            user_session,
            sync_token: None,
        })?;

        info!("Syncing");
        // client.sync_once(SyncSettings::default()).await.unwrap();

        Ok((client, serialized_session))
    }

    pub async fn restore_session(
        serialized_session: &str,
    ) -> anyhow::Result<(Client, Option<String>)> {
        info!("Previous session found in session_file",);

        let FullSession {
            client_session,
            user_session,
            sync_token,
        } = serde_json::from_str(&serialized_session)?;

        let client = Client::builder()
            .homeserver_url(client_session.homeserver.clone())
            .indexeddb_store(&user_session.user_id.to_string(), None)
            .await?;

        let client = client.build().await?;

        info!("Restoring session for {}…", user_session.user_id);

        client.restore_login(user_session.clone()).await?;

        Ok((client, sync_token))
    }

    pub async fn build_client(
        homeserver: &str,
        username: &str,
    ) -> anyhow::Result<(Client, ClientSession)> {
        loop {
            match Client::builder()
                .homeserver_url(&homeserver)
                .indexeddb_store(username, None)
                .await
            {
                Ok(builder) => match builder.build().await {
                    Ok(client) => {
                        return Ok((
                            client,
                            ClientSession {
                                homeserver: homeserver.into(),
                            },
                        ))
                    }
                    Err(error) => match &error {
                        matrix_sdk::ClientBuildError::AutoDiscovery(_)
                        | matrix_sdk::ClientBuildError::Url(_)
                        | matrix_sdk::ClientBuildError::Http(_) => {
                            info!("{error}");
                            return Err(error.into());
                        }
                        _ => {
                            return Err(error.into());
                        }
                    },
                },
                Err(err) => {
                    info!("err {}", err)
                }
            }
        }
    }
}
