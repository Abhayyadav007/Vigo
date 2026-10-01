-- Account deletion (App Store / Play Store requirement). Deleted users keep
-- their row (orders reference it) but lose all personal data.
ALTER TABLE users ALTER COLUMN phone DROP NOT NULL;
ALTER TABLE users ADD COLUMN deleted_at TIMESTAMPTZ;
