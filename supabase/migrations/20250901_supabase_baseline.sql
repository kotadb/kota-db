-- Baseline schema for KotaDB Supabase resources
-- Ensures core tables/policies exist before downstream migrations run.

-- Required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- API keys table
CREATE TABLE IF NOT EXISTS public.api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
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

CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON public.api_keys(user_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_key_hash ON public.api_keys(key_hash);
CREATE INDEX IF NOT EXISTS idx_api_keys_expires_at ON public.api_keys(expires_at) WHERE expires_at IS NOT NULL;

-- Usage metrics table
CREATE TABLE IF NOT EXISTS public.usage_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    api_key_id UUID REFERENCES public.api_keys(id) ON DELETE CASCADE,
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

CREATE INDEX IF NOT EXISTS idx_usage_metrics_api_key ON public.usage_metrics(api_key_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_usage_metrics_created_at ON public.usage_metrics(created_at DESC);

-- Ensure RLS is enabled
ALTER TABLE public.api_keys ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.usage_metrics ENABLE ROW LEVEL SECURITY;

-- Consolidated policies mirroring production
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies
        WHERE schemaname = 'public' AND tablename = 'api_keys' AND policyname = 'api_keys_all'
    ) THEN
        CREATE POLICY api_keys_all ON public.api_keys
            FOR ALL
            USING ( (SELECT auth.uid()) = user_id OR (SELECT auth.role()) = 'service_role')
            WITH CHECK ( (SELECT auth.uid()) = user_id OR (SELECT auth.role()) = 'service_role');
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_policies
        WHERE schemaname = 'public' AND tablename = 'usage_metrics' AND policyname = 'usage_metrics_select'
    ) THEN
        CREATE POLICY usage_metrics_select ON public.usage_metrics
            FOR SELECT
            USING (
                (SELECT auth.role()) = 'service_role' OR
                api_key_id IN (
                    SELECT id FROM public.api_keys WHERE user_id = (SELECT auth.uid())
                )
            );
    END IF;
END;
$$;

-- Helper functions used by the API service
CREATE OR REPLACE FUNCTION public.check_rate_limit(p_api_key_id UUID)
RETURNS BOOLEAN
LANGUAGE plpgsql
SET search_path TO 'public'
AS $$
DECLARE
  v_count INTEGER;
  v_rate_limit INTEGER;
BEGIN
  SELECT rate_limit INTO v_rate_limit FROM public.api_keys WHERE id = p_api_key_id;
  SELECT COUNT(*) INTO v_count FROM public.usage_metrics
    WHERE api_key_id = p_api_key_id AND created_at > NOW() - INTERVAL '1 minute';
  RETURN COALESCE(v_count, 0) < COALESCE(v_rate_limit, 0);
END;
$$;

CREATE OR REPLACE FUNCTION public.check_monthly_quota(p_api_key_id UUID)
RETURNS BOOLEAN
LANGUAGE plpgsql
SET search_path TO 'public'
AS $$
DECLARE
  v_usage_count INTEGER;
  v_monthly_quota INTEGER;
BEGIN
  SELECT usage_count, monthly_quota INTO v_usage_count, v_monthly_quota
  FROM public.api_keys WHERE id = p_api_key_id;
  RETURN COALESCE(v_usage_count, 0) < COALESCE(v_monthly_quota, 0);
END;
$$;

CREATE OR REPLACE FUNCTION public.update_updated_at_column()
RETURNS TRIGGER
LANGUAGE plpgsql
SET search_path TO 'public'
AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$;

-- Trigger for api_keys updated_at
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_trigger
        WHERE tgname = 'update_api_keys_updated_at'
          AND tgrelid = 'public.api_keys'::regclass
    ) THEN
        CREATE TRIGGER update_api_keys_updated_at
        BEFORE UPDATE ON public.api_keys
        FOR EACH ROW
        EXECUTE FUNCTION public.update_updated_at_column();
    END IF;
END;
$$;
