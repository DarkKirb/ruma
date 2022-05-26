//! `GET /_matrix/client/*/rooms/{roomId}/relations/{eventId}/{relType}/{eventType}`

pub mod v1 {
    //! `/v1/` ([MSC2675])
    //!
    //! This endpoint uses the `/unstable/` version path.
    //!
    //! [MSC2675]: https://github.com/matrix-org/matrix-spec-proposals/pull/2675

    use js_int::UInt;
    use ruma_common::{
        api::ruma_api,
        events::{relation::RelationType, AnyMessageLikeEvent, MessageLikeEventType},
        serde::{Incoming, Raw},
        EventId, RoomId,
    };

    ruma_api! {
        metadata: {
            description: "Get the related events for a given event, with optional filters.",
            method: GET,
            name: "get_events_relating_to_event",
            unstable_path: "/_matrix/client/unstable/rooms/:room_id/relations/:event_id/:rel_type/:event_type",
            rate_limited: false,
            authentication: AccessToken,
        }

        response: {
            /// The paginated events which relate to the parent event, after applicable filters.
            ///
            /// If no events are related to the parent, an empty `chunk` is returned.
            pub chunk: Vec<Raw<AnyMessageLikeEvent>>,

            /// An opaque string representing a pagination token.
            ///
            /// If this is `None`, there are no more results to fetch and the client should stop
            /// paginating.
            #[serde(skip_serializing_if = "Option::is_none")]
            pub next_batch: Option<String>,

            /// An opaque string representing a pagination token.
            ///
            /// If this is `None`, there are no prior results to fetch, i.e. this is the first
            /// batch.
            #[serde(skip_serializing_if = "Option::is_none")]
            pub prev_batch: Option<String>,
        }

        error: crate::Error
    }

    /// Data for a request to the `get_events_related_to_event` API endpoint.
    ///
    /// Retrieve all of the related events for a given event, optionally filtering
    /// down by relationship type and related event type.
    #[derive(Clone, Debug, Incoming)]
    #[cfg_attr(not(feature = "unstable-exhaustive-types"), non_exhaustive)]
    #[incoming_derive(!Deserialize)]
    pub struct Request<'a> {
        /// The ID of the room the parent event is in.
        pub room_id: &'a RoomId,

        /// The parent event ID.
        pub event_id: &'a EventId,

        /// The relationship type to search for, if any.
        ///
        /// Set to `None` to find all events which relate to the parent event with any `rel_type`.
        pub rel_type: Option<RelationType>,

        /// The event type of related events to search for, if any.
        ///
        /// Set to `None` to find all events of any type which relate to the parent event.
        ///
        /// Note that in encrypted rooms this will typically always be `m.room.encrypted`
        /// regardless of the event type contained within the encrypted payload.
        ///
        /// As this is a path parameter, this will be ignored if `rel_type` is `None`.
        pub event_type: Option<MessageLikeEventType>,

        /// The pagination token to start returning results from.
        ///
        /// If `None`, results start at the earliest topological event known to the server.
        ///
        /// Can be a `next_batch` or `prev_batch` token from a previous call, or an equivalent
        /// token from `/messages` or `/sync` to limit results to the events returned by that
        /// section of timeline.
        pub from: Option<&'a str>,

        /// The pagination token to stop returning results at.
        ///
        /// If `None`, results continue up to `limit` or until there are no more events.
        ///
        /// Like `from`, this can be a previous token from a prior call to this endpoint
        /// or from `/messages` or `/sync` to limit to a section of timeline.
        pub to: Option<&'a str>,

        /// The maximum number of results to return in a single `chunk`.
        ///
        /// The server can and should apply a maximum value to this parameter to avoid large
        /// responses.
        ///
        /// Similarly, the server should apply a default value when not supplied.
        pub limit: Option<UInt>,
    }

    impl<'a> Request<'a> {
        /// Creates a new `Request` with the given room ID and parent event ID.
        pub fn new(room_id: &'a RoomId, event_id: &'a EventId) -> Self {
            Self {
                room_id,
                event_id,
                rel_type: None,
                event_type: None,
                from: None,
                to: None,
                limit: None,
            }
        }
    }

    impl Response {
        /// Creates a new `Response` with the given chunk.
        pub fn new(chunk: Vec<Raw<AnyMessageLikeEvent>>) -> Self {
            Self { chunk, next_batch: None, prev_batch: None }
        }
    }

    #[cfg(feature = "client")]
    impl<'a> ruma_common::api::OutgoingRequest for Request<'a> {
        type EndpointError = crate::Error;
        type IncomingResponse = Response;

        const METADATA: ruma_common::api::Metadata = METADATA;

        fn try_into_http_request<T: Default + bytes::BufMut>(
            self,
            base_url: &str,
            access_token: ruma_common::api::SendAccessToken<'_>,
            considering_versions: &'_ [ruma_common::api::MatrixVersion],
        ) -> Result<http::Request<T>, ruma_common::api::error::IntoHttpError> {
            use std::borrow::Cow;

            use http::header;
            use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

            let room_id_percent = utf8_percent_encode(self.room_id.as_str(), NON_ALPHANUMERIC);
            let event_id_percent = utf8_percent_encode(self.event_id.as_str(), NON_ALPHANUMERIC);

            let mut url = format!(
                "{}{}",
                base_url.strip_suffix('/').unwrap_or(base_url),
                ruma_common::api::select_path(
                    considering_versions,
                    &METADATA,
                    Some(format_args!(
                        "/_matrix/client/unstable/rooms/{}/relations/{}",
                        room_id_percent, event_id_percent
                    )),
                    None,
                    None,
                )?
            );

            if let Some(rel_type) = self.rel_type {
                url.push('/');
                url.push_str(&Cow::from(utf8_percent_encode(rel_type.as_str(), NON_ALPHANUMERIC)));

                if let Some(event_type) = self.event_type {
                    url.push('/');
                    url.push_str(&Cow::from(utf8_percent_encode(
                        &event_type.to_string(),
                        NON_ALPHANUMERIC,
                    )));
                }
            }

            #[derive(Debug, serde::Serialize)]
            struct RequestQuery<'a> {
                from: Option<&'a str>,
                to: Option<&'a str>,
                limit: Option<UInt>,
            }

            let query = RequestQuery { from: self.from, to: self.to, limit: self.limit };
            let query_string = ruma_common::serde::urlencoded::to_string(query)?;

            if !query_string.is_empty() {
                url.push('?');
                url.push_str(&query_string);
            }

            http::Request::builder()
                .method(http::Method::GET)
                .uri(url)
                .header(header::CONTENT_TYPE, "application/json")
                .header(
                    header::AUTHORIZATION,
                    format!(
                        "Bearer {}",
                        access_token
                            .get_required_for_endpoint()
                            .ok_or(ruma_common::api::error::IntoHttpError::NeedsAuthentication)?,
                    ),
                )
                .body(T::default())
                .map_err(Into::into)
        }
    }

    #[cfg(feature = "server")]
    impl ruma_common::api::IncomingRequest for IncomingRequest {
        type EndpointError = crate::Error;
        type OutgoingResponse = Response;

        const METADATA: ruma_common::api::Metadata = METADATA;

        fn try_from_http_request<B, S>(
            request: http::Request<B>,
            path_args: &[S],
        ) -> Result<Self, ruma_common::api::error::FromHttpRequestError>
        where
            B: AsRef<[u8]>,
            S: AsRef<str>,
        {
            use ruma_common::{OwnedEventId, OwnedRoomId};

            // FIXME: find a way to make this match collapse with serde recognizing trailing
            // Option
            let (room_id, event_id, rel_type, event_type): (
                OwnedRoomId,
                OwnedEventId,
                Option<RelationType>,
                Option<MessageLikeEventType>,
            ) = match path_args.len() {
                4 => serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
                    _,
                    serde::de::value::Error,
                >::new(
                    path_args.iter().map(::std::convert::AsRef::as_ref),
                ))?,
                3 => {
                    let (a, b, c) =
                        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
                            _,
                            serde::de::value::Error,
                        >::new(
                            path_args.iter().map(::std::convert::AsRef::as_ref),
                        ))?;

                    (a, b, c, None)
                }
                _ => {
                    let (a, b) =
                        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
                            _,
                            serde::de::value::Error,
                        >::new(
                            path_args.iter().map(::std::convert::AsRef::as_ref),
                        ))?;

                    (a, b, None, None)
                }
            };

            #[derive(Debug, serde::Deserialize)]
            struct IncomingRequestQuery {
                from: Option<String>,
                to: Option<String>,
                limit: Option<UInt>,
            }

            let IncomingRequestQuery { from, to, limit } =
                ruma_common::serde::urlencoded::from_str(request.uri().query().unwrap_or(""))?;

            Ok(Self { room_id, event_id, rel_type, event_type, from, to, limit })
        }
    }
}
