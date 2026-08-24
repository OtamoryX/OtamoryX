-- Normalize the persisted OCR settings before the new API contract is used.
INSERT INTO settings (key, value, updated_at)
VALUES (
    'ocr_settings',
    '{"enabled":false,"activeModelId":"ppocrv5-mobile-zh","image":{"targetLongEdge":2048,"preferredDecodeBytes":100663296,"jpegQuality":86,"maxOutputBytes":2097152,"largeImageLongEdge":2560,"largeImageDecodeBytes":268435456,"largeImageJpegQuality":88,"largeImageMaxOutputBytes":4194304},"failurePolicy":{"skipUnreadablePages":true,"maxPageRetries":1}}',
    NOW()
)
ON CONFLICT (key) DO NOTHING;

UPDATE settings
SET value = (
        '{"image":{"targetLongEdge":2048,"preferredDecodeBytes":100663296,"jpegQuality":86,"maxOutputBytes":2097152,"largeImageLongEdge":2560,"largeImageDecodeBytes":268435456,"largeImageJpegQuality":88,"largeImageMaxOutputBytes":4194304},"failurePolicy":{"skipUnreadablePages":true,"maxPageRetries":1}}'::jsonb
        || value::jsonb
    )::text,
    updated_at = NOW()
WHERE key = 'ocr_settings';
