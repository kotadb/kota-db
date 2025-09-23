-- Ensure Supabase production/staging match CLI-managed schema expectations
-- This migration is safe to run against databases that already contain data.

CREATE EXTENSION IF NOT EXISTS vector;

-- Reassert indexes (no-ops when already present)
CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_key_hash ON api_keys(key_hash);
CREATE INDEX IF NOT EXISTS idx_api_keys_expires_at ON api_keys(expires_at) WHERE expires_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_documents_user_path ON documents(user_id, path);
CREATE INDEX IF NOT EXISTS idx_documents_tags ON documents USING GIN(tags);
CREATE INDEX IF NOT EXISTS idx_documents_content_hash ON documents(content_hash);
CREATE INDEX IF NOT EXISTS idx_usage_metrics_api_key ON usage_metrics(api_key_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_usage_metrics_created_at ON usage_metrics(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_documents_embedding ON documents USING ivfflat (embedding vector_cosine_ops);

-- Recreate policies so they are guaranteed to match the current definitions
DROP POLICY IF EXISTS "Users can view their own API keys" ON api_keys;
CREATE POLICY "Users can view their own API keys"
    ON api_keys FOR SELECT
    USING (auth.uid() = user_id);

DROP POLICY IF EXISTS "Users can create their own API keys" ON api_keys;
CREATE POLICY "Users can create their own API keys"
    ON api_keys FOR INSERT
    WITH CHECK (auth.uid() = user_id);

DROP POLICY IF EXISTS "Users can update their own API keys" ON api_keys;
CREATE POLICY "Users can update their own API keys"
    ON api_keys FOR UPDATE
    USING (auth.uid() = user_id)
    WITH CHECK (auth.uid() = user_id);

DROP POLICY IF EXISTS "Users can delete their own API keys" ON api_keys;
CREATE POLICY "Users can delete their own API keys"
    ON api_keys FOR DELETE
    USING (auth.uid() = user_id);

DROP POLICY IF EXISTS "Users can view their own documents" ON documents;
CREATE POLICY "Users can view their own documents"
    ON documents FOR SELECT
    USING (auth.uid() = user_id);

DROP POLICY IF EXISTS "Users can create their own documents" ON documents;
CREATE POLICY "Users can create their own documents"
    ON documents FOR INSERT
    WITH CHECK (auth.uid() = user_id);

DROP POLICY IF EXISTS "Users can update their own documents" ON documents;
CREATE POLICY "Users can update their own documents"
    ON documents FOR UPDATE
    USING (auth.uid() = user_id)
    WITH CHECK (auth.uid() = user_id);

DROP POLICY IF EXISTS "Users can delete their own documents" ON documents;
CREATE POLICY "Users can delete their own documents"
    ON documents FOR DELETE
    USING (auth.uid() = user_id);

DROP POLICY IF EXISTS "Users can view metrics for their API keys" ON usage_metrics;
CREATE POLICY "Users can view metrics for their API keys"
    ON usage_metrics FOR SELECT
    USING (
        api_key_id IN (
            SELECT id FROM api_keys WHERE user_id = auth.uid()
        )
    );

DROP POLICY IF EXISTS "Service role has full access to api_keys" ON api_keys;
CREATE POLICY "Service role has full access to api_keys"
    ON api_keys FOR ALL
    USING (auth.role() = 'service_role');

DROP POLICY IF EXISTS "Service role has full access to documents" ON documents;
CREATE POLICY "Service role has full access to documents"
    ON documents FOR ALL
    USING (auth.role() = 'service_role');

DROP POLICY IF EXISTS "Service role has full access to usage_metrics" ON usage_metrics;
CREATE POLICY "Service role has full access to usage_metrics"
    ON usage_metrics FOR ALL
    USING (auth.role() = 'service_role');

-- Ensure updated_at trigger functions are wired correctly
DROP TRIGGER IF EXISTS update_api_keys_updated_at ON api_keys;
CREATE TRIGGER update_api_keys_updated_at
    BEFORE UPDATE ON api_keys
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS update_documents_updated_at ON documents;
CREATE TRIGGER update_documents_updated_at
    BEFORE UPDATE ON documents
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
