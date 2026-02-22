CREATE TABLE IF NOT EXISTS auth_sessions (
    id TEXT PRIMARY KEY,           -- Session ID (UUID)
    auth_token TEXT NOT NULL,      -- Secret token for the CLI
    status TEXT NOT NULL DEFAULT 'PENDING', -- 'PENDING', 'VERIFIED', 'USED', 'EXPIRED'
    created_at INTEGER DEFAULT (cast(strftime('%s', 'now') as integer))
);

-- Index for session lookup
CREATE INDEX idx_auth_sessions_status ON auth_sessions(status);
