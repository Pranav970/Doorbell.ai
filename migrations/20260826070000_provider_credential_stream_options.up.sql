-- Whether this credential's endpoint accepts OpenAI's `stream_options`
-- parameter on a streaming chat-completions request.
--
-- The gateway sends `stream_options: {"include_usage": true}` so a stream
-- reports real output-token counts instead of falling back to the
-- character-ratio estimate. OpenAI and Groq accept it; some stricter
-- OpenAI-compatible vendors reject any unrecognised field outright and 400
-- the whole request. Setting this false omits the parameter for that
-- credential, trading exact counts for compatibility.
--
-- Defaults to true so every existing credential keeps today's behaviour.
ALTER TABLE provider_credentials
    ADD COLUMN supports_stream_options BOOLEAN NOT NULL DEFAULT true;
