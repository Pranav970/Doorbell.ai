export interface OrgMembership {
  org_id: string;
  org_name: string;
  role: 'admin' | 'member';
}

export interface TeamMembership {
  team_id: string;
  team_name: string;
  org_id: string;
  status: 'pending' | 'approved' | 'rejected';
}

export interface Me {
  user_id: string;
  email: string;
  display_name: string | null;
  org_memberships: OrgMembership[];
  team_memberships: TeamMembership[];
  /** Platform-wide master. Both membership lists above are normally empty
   *  for this account, so access must be branched on this, not on them. */
  is_master_admin: boolean;
}

export interface TeamSummary {
  id: string;
  name: string;
  slug: string;
}

export interface OrgWithTeams {
  id: string;
  name: string;
  slug: string;
  teams: TeamSummary[];
}

export interface Team {
  id: string;
  org_id: string;
  name: string;
  slug: string;
  created_at: string;
}

export interface TeamMember {
  user_id: string;
  email: string;
  display_name: string | null;
}

export interface TeamDetail extends Team {
  members: TeamMember[];
}

export interface Org {
  id: string;
  name: string;
  slug: string;
  created_at: string;
}

export interface PendingRequest {
  id: string;
  team_id: string;
  team_name: string;
  user_id: string;
  user_email: string;
  requested_at: string;
}

export interface ProviderKey {
  id: string;
  team_id: string;
  added_by: string;
  provider: string;
  label: string | null;
  masked_key: string;
  is_active: boolean;
  created_at: string;
  base_url: string | null;
}

export interface VirtualKey {
  id: string;
  name: string | null;
  prefix: string;
  status: string;
  created_at: string;
  last_used_at: string | null;
}

export interface VirtualKeyCreated extends VirtualKey {
  key: string;
}

export interface RequestLogEntry {
  id: string;
  virtual_key_id: string | null;
  provider: string;
  model: string;
  status_code: number;
  latency_ms: number;
  tokens_in: number | null;
  tokens_out: number | null;
  error_message: string | null;
  created_at: string;
  /** How many models the Fallback Engine tried before this one (0 = primary worked). */
  fallback_count: number;
  /**
   * Why the primary model failed, when `fallback_count > 0`.
   *
   * Not sent by the backend yet — `request_logs` only persists
   * `fallback_count`, while the per-attempt error strings live transiently
   * in `FallbackSuccess.failed_attempts` and are dropped after the response
   * is built. Optional so the UI falls back to a placeholder today and
   * starts showing the real thing the moment the column lands, with no
   * frontend change. Named in snake_case like every other field here
   * because this interface is a verbatim mirror of the JSON API.
   */
  fallback_reason?: string | null;
}

export interface FallbackAttemptSummary {
  model: string;
  provider: string;
  error: string;
}

/** Present on a chat-completion response only when the primary model (or an
 * earlier fallback) failed and a later candidate served the request instead. */
export interface FallbackInfo {
  attempts: FallbackAttemptSummary[];
  model_used: string;
}

export interface ChatCompletionResult {
  id: string;
  object: string;
  created: number;
  model: string;
  choices: { index: number; message: { role: string; content: string }; finish_reason: string | null }[];
  usage: { prompt_tokens: number; completion_tokens: number; total_tokens: number } | null;
  fallback?: FallbackInfo;
}

export interface KeyUsage {
  team_id: string;
  provider: string;
  model: string;
  request_count: number;
  error_count: number;
  tokens_in: number;
  tokens_out: number;
  last_used_at: string | null;
}

export const KNOWN_PROVIDERS = [
  'openai',
  'anthropic',
  'gemini',
  'mistral',
  'cohere',
  'meta-llama',
  'azure-openai',
  'deepseek',
  'xai-grok',
  'perplexity',
] as const;
export type KnownProvider = (typeof KNOWN_PROVIDERS)[number];

export const PROVIDER_LABELS: Record<KnownProvider, string> = {
  openai: 'OpenAI',
  anthropic: 'Claude (Anthropic)',
  gemini: 'Gemini (Google)',
  mistral: 'Mistral',
  cohere: 'Cohere',
  'meta-llama': 'Llama (Meta)',
  'azure-openai': 'Azure OpenAI',
  deepseek: 'DeepSeek',
  'xai-grok': 'Grok (xAI)',
  perplexity: 'Perplexity',
};

export const CUSTOM_PROVIDER = '__custom__';

export interface RoutingRule {
  id: string;
  virtual_key_id: string;
  model_pattern: string;
  provider_credential_id: string;
  priority: number;
  created_at: string;
}
