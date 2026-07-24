-- Keep the internal agent metadata aligned with the Zigo-branded workspace
-- directory used by runtime skill provisioning.

UPDATE agent_metadata
SET
    native_skills_dirs = '[".zigo/skills"]',
    updated_at = unixepoch('now','subsec')*1000
WHERE agent_type = 'aionrs'
  AND agent_source = 'internal';
