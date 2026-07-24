-- AionRS discovers project skills from the engine-owned `.aionrs/skills`
-- directory. Keep this technical runtime path unchanged by product branding.

UPDATE agent_metadata
SET
    native_skills_dirs = '[".aionrs/skills"]',
    updated_at = unixepoch('now','subsec')*1000
WHERE agent_type = 'aionrs'
  AND agent_source = 'internal';
