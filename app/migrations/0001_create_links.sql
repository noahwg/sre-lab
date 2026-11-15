-- Short links. One row per code; `code` is the primary key, so redirect lookups are
-- a single index hit and a duplicate code surfaces as a unique violation.
CREATE TABLE links (
    code       text        PRIMARY KEY,
    url        text        NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz,
    clicks     bigint      NOT NULL DEFAULT 0,

    -- Mirror the app's validation so bad data is rejected even if something else writes here.
    CONSTRAINT links_code_format         CHECK (code ~ '^[A-Za-z0-9_-]{3,32}$'),
    CONSTRAINT links_url_scheme          CHECK (url ~ '^https?://'),
    CONSTRAINT links_url_length          CHECK (length(url) <= 2048),
    CONSTRAINT links_clicks_nonneg       CHECK (clicks >= 0),
    CONSTRAINT links_expiry_after_create CHECK (expires_at IS NULL OR expires_at > created_at)
);

-- Only rows that can expire are indexed (for a future cleanup job).
CREATE INDEX links_expires_at_idx ON links (expires_at) WHERE expires_at IS NOT NULL;
