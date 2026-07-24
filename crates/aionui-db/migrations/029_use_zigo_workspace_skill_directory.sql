-- Keep Zigo's workspace contract branded while the AionRS integration
-- adapts it to the upstream engine's discovery convention at runtime.

UPDATE agent_metadata
SET
    native_skills_dirs = '[".zigo/skills"]',
    updated_at = unixepoch('now','subsec')*1000
WHERE agent_type = 'aionrs'
  AND agent_source = 'internal';
