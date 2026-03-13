-- Add access_token for app-level client authentication
-- Server shares this token with authorized clients
ALTER TABLE tunnels ADD COLUMN access_token TEXT;
