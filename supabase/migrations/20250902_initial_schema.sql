-- Supabase Setup Script for KotaDB
-- Run this in your Supabase SQL editor to set up the required tables and policies

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE IF NOT EXISTS api_keys (
    id UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    user_id UUID REFERENCES auth.users(id) ON DELETE CASCADE,
    key_hash TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    permissions JSONB DEFAULT '{"read": true, "write": false}'::jsonb,
    rate_limit INTEGER DEFAULT 60,
    monthly_quota INTEGER DEFAULT 1000000,
    usage_count INTEGER DEFAULT 0,
    last_used_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS documents (
    id UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    user_id UUID REFERENCES auth.users(id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    title TEXT,
    content TEXT NOT NULL,
    content_hash TEXT,
    tags TEXT[],
    metadata JSONB DEFAULT '{}'::jsonb,
    embedding vector(1536), -- For OpenAI embeddings
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_id, path)
);

CREATE TABLE IF NOT EXISTS usage_metrics (
    id UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    api_key_id UUID REFERENCES api_keys(id) ON DELETE CASCADE,
    endpoint TEXT NOT NULL,
    method TEXT NOT NULL,
    status_code INTEGER,
    response_time_ms INTEGER,
    tokens_used INTEGER DEFAULT 0,
    request_size_bytes INTEGER,
    response_size_bytes INTEGER,
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_key_hash ON api_keys(key_hash);
CREATE INDEX IF NOT EXISTS idx_api_keys_expires_at ON api_keys(expires_at) WHERE expires_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_documents_user_path ON documents(user_id, path);
CREATE INDEX IF NOT EXISTS idx_documents_tags ON documents USING GIN(tags);
CREATE INDEX IF NOT EXISTS idx_documents_content_hash ON documents(content_hash);
CREATE INDEX IF NOT EXISTS idx_usage_metrics_api_key ON usage_metrics(api_key_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_usage_metrics_created_at ON usage_metrics(created_at DESC);

CREATE INDEX IF NOT EXISTS idx_documents_embedding ON documents USING ivfflat (embedding vector_cosine_ops);

-- Enable Row Level Security
ALTER TABLE api_keys ENABLE ROW LEVEL SECURITY;
ALTER TABLE documents ENABLE ROW LEVEL SECURITY;
ALTER TABLE usage_metrics ENABLE ROW LEVEL SECURITY;

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

-- Create helper functions

-- Function to hash API keys (you might want to implement this in your application instead)
CREATE OR REPLACE FUNCTION hash_api_key(key TEXT)
RETURNS TEXT AS $$
BEGIN
    -- Simple SHA-256 hash - in production, use a proper hashing library in your app
    RETURN encode(digest(key, 'sha256'), 'hex');
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- Function to check rate limits
CREATE OR REPLACE FUNCTION check_rate_limit(p_api_key_id UUID)
RETURNS BOOLEAN AS $$
DECLARE
    v_count INTEGER;
    v_rate_limit INTEGER;
BEGIN
    -- Get the rate limit for this API key
    SELECT rate_limit INTO v_rate_limit
    FROM api_keys
    WHERE id = p_api_key_id;
    
    -- Count requests in the last minute
    SELECT COUNT(*) INTO v_count
    FROM usage_metrics
    WHERE api_key_id = p_api_key_id
      AND created_at > NOW() - INTERVAL '1 minute';
    
    RETURN v_count < v_rate_limit;
END;
$$ LANGUAGE plpgsql;

-- Function to check monthly quota
CREATE OR REPLACE FUNCTION check_monthly_quota(p_api_key_id UUID)
RETURNS BOOLEAN AS $$
DECLARE
    v_usage_count INTEGER;
    v_monthly_quota INTEGER;
BEGIN
    SELECT usage_count, monthly_quota 
    INTO v_usage_count, v_monthly_quota
    FROM api_keys
    WHERE id = p_api_key_id;
    
    RETURN v_usage_count < v_monthly_quota;
END;
$$ LANGUAGE plpgsql;

-- Create updated_at trigger
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

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

-- Create a view for API key statistics
CREATE OR REPLACE VIEW api_key_stats AS
SELECT 
    ak.id,
    ak.name,
    ak.user_id,
    ak.usage_count,
    ak.monthly_quota,
    ak.rate_limit,
    COUNT(um.id) AS requests_last_hour,
    AVG(um.response_time_ms) AS avg_response_time_ms,
    MAX(um.created_at) AS last_request_at
FROM api_keys ak
LEFT JOIN usage_metrics um ON ak.id = um.api_key_id 
    AND um.created_at > NOW() - INTERVAL '1 hour'
GROUP BY ak.id, ak.name, ak.user_id, ak.usage_count, ak.monthly_quota, ak.rate_limit;

-- Grant permissions on the view
GRANT SELECT ON api_key_stats TO authenticated;

-- Sample data for testing (optional - remove in production)
-- This creates a test user and API key
-- DO $$
-- DECLARE
--     test_user_id UUID;
-- BEGIN
--     -- Create a test user (if using Supabase Auth)
--     -- Note: This is just an example, usually users are created through Supabase Auth
--     test_user_id := gen_random_uuid();
--     
--     -- Insert a test API key
--     INSERT INTO api_keys (user_id, key_hash, name, permissions, rate_limit, monthly_quota)
--     VALUES (
--         test_user_id,
--         hash_api_key('test_key_12345'),
--         'Test API Key',
--         '{"read": true, "write": true}'::jsonb,
--         100,
--         10000
--     );
-- END $$;

-- Success message
DO $$
BEGIN
    RAISE NOTICE 'Supabase setup completed successfully!';
    RAISE NOTICE 'Tables created: api_keys, documents, usage_metrics';
    RAISE NOTICE 'RLS policies applied';
    RAISE NOTICE 'Helper functions created';
    RAISE NOTICE 'Indexes created for optimal performance';
END $$;
