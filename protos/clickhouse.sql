-- 创建原始事件表
CREATE TABLE analytics.analytics_events(
    -- EventContext fields
    client_id String,
    session_id String,
    duration UInt32,
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
    -- Common fields
    event_type String,
    -- AppExitEvent fields
    exit_code Nullable(String),
    -- UserLoginEvent
    login_email Nullable(String),
    -- UserLogoutEvent
    logout_email Nullable(String),
    -- UserRegisterEvent
    register_email Nullable(String),
    register_workspace_id Nullable(String),
    -- ChatCreatedEvent
    chat_created_workspace_id Nullable(String),
    -- MessageSentEvent
    message_chat_id Nullable(String),
    message_type Nullable(String),
    message_size Nullable(Int32),
    message_total_files Nullable(Int32),
    -- ChatJoinedEvent
    chat_joined_id Nullable(String),
    -- ChatLeftEvent
    chat_left_id Nullable(String),
    -- NavigationEvent
    navigation_from Nullable(String),
    navigation_to Nullable(String)) ENGINE = MergeTree()
ORDER BY
    (
        event_type,
        session_id,
        client_id,
        server_ts
);

-- 创建用于存储聚合会话的目标表
CREATE TABLE analytics.sessions (
    date Date,
    client_id String,
    session_id String,
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
    session_start SimpleAggregateFunction(min, DateTime64(3)),
    session_end SimpleAggregateFunction(max, DateTime64(3)),
    session_length SimpleAggregateFunction(sum, UInt64),
    total_events UInt32
) ENGINE = SummingMergeTree()
ORDER BY (date, client_id, session_id);

-- 创建物化视图以将数据聚合到目标表
CREATE MATERIALIZED VIEW analytics.sessions_mv TO analytics.sessions
AS SELECT
    toDate(server_ts) AS date,
    client_id,
    session_id,
    any(app_version) AS app_version,
    any(system_os) AS system_os,
    any(system_arch) AS system_arch,
    any(system_locale) AS system_locale,
    any(system_timezone) AS system_timezone,
    any(user_id) AS user_id,
    any(ip) AS ip,
    any(user_agent) AS user_agent,
    any(geo_country) AS geo_country,
    any(geo_region) AS geo_region,
    any(geo_city) AS geo_city,
    min(server_ts) AS session_start,
    max(server_ts) AS session_end,
    sum(duration) / 1000 AS session_length,
    count(1) AS total_events
FROM analytics.analytics_events
GROUP BY  date, client_id, session_id;

-- populate sessions table
-- INSERT INTO analytics.sessions...;
-- query sessions table
SELECT
    date,
    client_id,
    session_id,
    session_start,
    session_end,
    session_length,
    total_events
FROM analytics.sessions
FINAL;

CREATE TABLE analytics.daily_sessions (
    date Date,
    client_id String,
    total_session_length SimpleAggregateFunction(sum, UInt64),
    total_session_events SimpleAggregateFunction(sum, UInt64),
    unique_users AggregateFunction(uniq, Nullable(String))
) ENGINE = SummingMergeTree()
ORDER BY (date, client_id);

CREATE MATERIALIZED VIEW analytics.daily_sessions_mv TO analytics.daily_sessions
AS SELECT
    date,
    client_id,
    sum(session_length) AS total_session_length,
    sum(total_events) AS total_session_events,
    uniqState(user_id) AS unique_users
FROM analytics.sessions
GROUP BY date, client_id;

select
    date,
    client_id,
    sum(total_session_length) as total_session_length,
    sum(total_session_events) as total_session_events,
    uniqMerge(unique_users) as unique_users
from analytics.daily_sessions
group by date, client_id;
