-- Keep the stable `aionrs` runtime discriminator for compatibility while
-- presenting the bundled in-process engine with the Zigo product name.

UPDATE agent_metadata
SET
    name = 'Zigo CLI',
    name_i18n = CASE
        WHEN name_i18n IS NULL THEN NULL
        ELSE json_object(
            'zh-CN', 'Zigo CLI',
            'zh-TW', 'Zigo CLI',
            'en-US', 'Zigo CLI',
            'ru-RU', 'Zigo CLI'
        )
    END,
    updated_at = unixepoch('now','subsec')*1000
WHERE agent_type = 'aionrs'
  AND agent_source = 'internal';
