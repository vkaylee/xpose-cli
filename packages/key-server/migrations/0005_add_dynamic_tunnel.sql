-- Add dynamic flag to track tunnels created dynamically via Cloudflare API
-- cf_tunnel_id stores the Cloudflare tunnel UUID for deletion on release
ALTER TABLE tunnels ADD COLUMN dynamic INTEGER NOT NULL DEFAULT 0;
ALTER TABLE tunnels ADD COLUMN cf_tunnel_id TEXT;
