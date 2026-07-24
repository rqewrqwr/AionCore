-- Migration 023: isolate user-managed skills by authenticated user.
--
-- `skills.name` was historically globally unique. Keep that constraint as an
-- internal storage key while exposing `display_name` to application code. This
-- avoids a destructive table rebuild and allows two users to own skills with
-- the same visible name.

ALTER TABLE skills ADD COLUMN owner_user_id TEXT NOT NULL DEFAULT 'system_default_user';
ALTER TABLE skills ADD COLUMN display_name TEXT;

UPDATE skills SET display_name = name WHERE display_name IS NULL;
UPDATE skills SET owner_user_id = '__shared__' WHERE source != 'user';
UPDATE skills SET name = owner_user_id || char(31) || display_name;

CREATE INDEX IF NOT EXISTS idx_skills_owner_user_id ON skills(owner_user_id);
CREATE INDEX IF NOT EXISTS idx_skills_owner_display_name ON skills(owner_user_id, display_name);

ALTER TABLE skill_import_records ADD COLUMN owner_user_id TEXT NOT NULL DEFAULT 'system_default_user';
CREATE INDEX IF NOT EXISTS idx_skill_import_records_owner_user_id
    ON skill_import_records(owner_user_id, created_at DESC);
