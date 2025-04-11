CREATE TABLE analytics_events (
    -- EventContext fields
    client_id String,
    app_version String,
    system_os String,
    system_arch String,
    system_locale String,
    system_timezone String,
    user_id Nullable(String),
    ip Nullable(String),
    user_agent Nullable(String),
    geo_country Nullable(String),
    geo_region Nullable(String),
    geo_city Nullable(String),
    client_ts DateTime64(3),
    server_ts DateTime64(3),

    -- Event type fields
    event_type Enum8(
        'APP_START' = 8,
        'APP_EXIT' = 9,
        'USER_LOGIN' = 10,
        'USER_LOGOUT' = 11,
        'USER_REGISTER' = 12,
        'CHAT_CREATED' = 13,
        'MESSAGE_SENT' = 14,
        'CHAT_JOINED' = 15,
        'CHAT_LEFT' = 16,
        'NAVIGATION' = 17
    ),

    -- AppExitEvent specific fields
    exit_code Nullable(Enum8('EXIT_CODE_UNSPECIFIED' = 0, 'EXIT_CODE_SUCCESS' = 1, 'EXIT_CODE_FAILURE' = 2)),

    -- User auth events fields
    email Nullable(String),
    workspace_id Nullable(String),

    -- Chat events fields
    chat_id Nullable(String),

    -- MessageSentEvent specific fields
    message_type Nullable(String),
    message_size Nullable(Int32),
    total_files Nullable(Int32),

    -- NavigationEvent specific fields
    navigation_from Nullable(String),
    navigation_to Nullable(String),

    -- Timestamp for when the record was inserted
    inserted_at DateTime DEFAULT now()
) ENGINE = MergeTree()
ORDER BY (server_ts, event_type, client_id);
