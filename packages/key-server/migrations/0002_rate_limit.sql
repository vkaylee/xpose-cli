CREATE TABLE IF NOT EXISTS rate_limits (
    ip TEXT PRIMARY KEY,
    last_request INTEGER,
    request_count INTEGER DEFAULT 1
);
