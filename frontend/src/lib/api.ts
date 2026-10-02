import type {
  ChatCompletionResult,
  KeyUsage,
  Me,
  Org,
  OrgWithTeams,
  PendingRequest,
  ProviderKey,
  RequestLogEntry,
  RoutingRule,
  Team,
  TeamDetail,
  VirtualKey,
  VirtualKeyCreated,
} from './types';

const API_BASE = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:8080';

const ACCESS_TOKEN_KEY = 'prismaxis_access_token';
const REFRESH_TOKEN_KEY = 'prismaxis_refresh_token';

export function getAccessToken(): string | null {
  return localStorage.getItem(ACCESS_TOKEN_KEY);
}

export function getRefreshToken(): string | null {
  return localStorage.getItem(REFRESH_TOKEN_KEY);
}

export function setTokens(accessToken: string, refreshToken: string): void {
  localStorage.setItem(ACCESS_TOKEN_KEY, accessToken);
  localStorage.setItem(REFRESH_TOKEN_KEY, refreshToken);
}

export function clearTokens(): void {
  localStorage.removeItem(ACCESS_TOKEN_KEY);
  localStorage.removeItem(REFRESH_TOKEN_KEY);
}

export class ApiError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.status = status;
  }
}

/**
 * Display message for a failed request. The backend's `AppError` always
 * serialises as `{"error": "…"}`, which `ApiError` already carries as its
 * message — anything else is a network/parse failure and gets the caller's
 * fallback. Returns null for no error, so it drops straight into `ErrorText`.
 */
export function errorMessage(err: unknown, fallback: string): string | null {
  if (!err) return null;
  return err instanceof ApiError ? err.message : fallback;
}

async function parseBody(res: Response): Promise<unknown> {
  const text = await res.text();
  return text ? JSON.parse(text) : null;
}

async function raiseIfError(res: Response, body: unknown): Promise<void> {
  if (!res.ok) {
    const message =
      typeof body === 'object' && body !== null && 'error' in body
        ? String((body as { error: unknown }).error)
        : res.statusText;
    throw new ApiError(res.status, message);
  }
}

// Tries a single refresh-token rotation. Returns whether it succeeded.
async function tryRefresh(): Promise<boolean> {
  const refreshToken = getRefreshToken();
  if (!refreshToken) return false;
  const res = await fetch(`${API_BASE}/auth/refresh`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ refresh_token: refreshToken }),
  });
  if (!res.ok) {
    clearTokens();
    return false;
  }
  const body = (await parseBody(res)) as { access_token: string; refresh_token: string };
  setTokens(body.access_token, body.refresh_token);
  return true;
}

// JWT-authenticated request to the control-plane API. On a 401 it tries one
// refresh-token rotation and retries once before giving up.
async function apiFetch<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers = new Headers(init.headers);
  headers.set('Content-Type', 'application/json');
  const access = getAccessToken();
  if (access) headers.set('Authorization', `Bearer ${access}`);

  let res = await fetch(`${API_BASE}${path}`, { ...init, headers });
  if (res.status === 401 && getRefreshToken() && (await tryRefresh())) {
    headers.set('Authorization', `Bearer ${getAccessToken()}`);
    res = await fetch(`${API_BASE}${path}`, { ...init, headers });
  }

  const body = await parseBody(res);
  await raiseIfError(res, body);
  return body as T;
}

export const api = {
  signup: (email: string, password: string, displayName: string) =>
    apiFetch<{ user_id: string; email: string }>('/auth/signup', {
      method: 'POST',
      body: JSON.stringify({ email, password, display_name: displayName || null }),
    }),

  login: async (email: string, password: string) => {
    const pair = await apiFetch<{ access_token: string; refresh_token: string }>('/auth/login', {
      method: 'POST',
      body: JSON.stringify({ email, password }),
    });
    setTokens(pair.access_token, pair.refresh_token);
    return pair;
  },

  logout: async () => {
    const refreshToken = getRefreshToken();
    if (refreshToken) {
      await apiFetch('/auth/logout', { method: 'POST', body: JSON.stringify({ refresh_token: refreshToken }) }).catch(
        () => undefined,
      );
    }
    clearTokens();
  },

  me: () => apiFetch<Me>('/api/me'),

  createOrg: (name: string) => apiFetch<Org>('/api/orgs', { method: 'POST', body: JSON.stringify({ name }) }),
  listOrgs: () => apiFetch<{ items: OrgWithTeams[] }>('/api/orgs'),

  createTeam: (orgId: string, name: string) =>
    apiFetch<Team>(`/api/orgs/${orgId}/teams`, { method: 'POST', body: JSON.stringify({ name }) }),
  listTeamsForOrg: (orgId: string) => apiFetch<{ items: Team[] }>(`/api/orgs/${orgId}/teams`),
  teamDetail: (teamId: string) => apiFetch<TeamDetail>(`/api/teams/${teamId}`),

  requestJoin: (teamId: string) => apiFetch(`/api/teams/${teamId}/join-requests`, { method: 'POST' }),
  pendingRequestsForOrg: (orgId: string) =>
    apiFetch<{ items: PendingRequest[] }>(`/api/orgs/${orgId}/join-requests`),
  approveJoin: (teamId: string, reqId: string) =>
    apiFetch(`/api/teams/${teamId}/join-requests/${reqId}/approve`, { method: 'POST' }),
  rejectJoin: (teamId: string, reqId: string) =>
    apiFetch(`/api/teams/${teamId}/join-requests/${reqId}/reject`, { method: 'POST' }),

  orgAnalytics: (orgId: string) => apiFetch<{ items: KeyUsage[] }>(`/api/orgs/${orgId}/analytics`),
  teamAnalytics: (teamId: string) => apiFetch<{ items: KeyUsage[] }>(`/api/teams/${teamId}/analytics`),
  teamLogs: (teamId: string) => apiFetch<{ items: RequestLogEntry[] }>(`/api/teams/${teamId}/logs`),

  listProviderKeys: (teamId: string) => apiFetch<{ items: ProviderKey[] }>(`/api/teams/${teamId}/provider-keys`),
  addProviderKey: (teamId: string, provider: string, apiKey: string, label: string, baseUrl: string) =>
    apiFetch<ProviderKey>(`/api/teams/${teamId}/provider-keys`, {
      method: 'POST',
      body: JSON.stringify({ provider, api_key: apiKey, label: label || null, base_url: baseUrl || null }),
    }),
  deactivateProviderKey: (teamId: string, id: string) =>
    apiFetch<void>(`/api/teams/${teamId}/provider-keys/${id}`, { method: 'DELETE' }),
  deleteProviderKey: (teamId: string, id: string) =>
    apiFetch<void>(`/api/teams/${teamId}/provider-keys/${id}/purge`, { method: 'DELETE' }),

  listVirtualKeys: (teamId: string) => apiFetch<{ items: VirtualKey[] }>(`/api/teams/${teamId}/virtual-keys`),
  issueVirtualKey: (teamId: string, name: string) =>
    apiFetch<VirtualKeyCreated>(`/api/teams/${teamId}/virtual-keys`, {
      method: 'POST',
      body: JSON.stringify({ name: name || null }),
    }),
  // Authorization-gated retrieval of a key's plaintext. The Playground uses
  // this so it can authenticate exactly the way an external client does,
  // rather than being handed a privileged side channel.
  revealVirtualKey: (teamId: string, id: string) =>
    apiFetch<{ id: string; key: string }>(`/api/teams/${teamId}/virtual-keys/${id}/reveal`),
  revokeVirtualKey: (teamId: string, id: string) =>
    apiFetch<void>(`/api/teams/${teamId}/virtual-keys/${id}`, { method: 'DELETE' }),
  deleteVirtualKey: (teamId: string, id: string) =>
    apiFetch<void>(`/api/teams/${teamId}/virtual-keys/${id}/purge`, { method: 'DELETE' }),

  listRoutingRules: (teamId: string, virtualKeyId: string) =>
    apiFetch<{ items: RoutingRule[] }>(`/api/teams/${teamId}/virtual-keys/${virtualKeyId}/routing-rules`),
  addRoutingRule: (teamId: string, virtualKeyId: string, modelPattern: string, providerCredentialId: string, priority: number) =>
    apiFetch<RoutingRule>(`/api/teams/${teamId}/virtual-keys/${virtualKeyId}/routing-rules`, {
      method: 'POST',
      body: JSON.stringify({ model_pattern: modelPattern, provider_credential_id: providerCredentialId, priority }),
    }),
  deleteRoutingRule: (teamId: string, virtualKeyId: string, id: string) =>
    apiFetch<void>(`/api/teams/${teamId}/virtual-keys/${virtualKeyId}/routing-rules/${id}`, { method: 'DELETE' }),
};

// Calls the OpenAI-compatible proxy directly with a virtual key — this is
// the data-plane call, authenticated by the virtual key itself rather than
// the control-plane JWT, so it bypasses `apiFetch`'s refresh logic entirely.
// `fallbackModels`, if given, is tried in order if `model` (then each prior
// entry) fails with a retryable error — see the Fallback Engine.
export async function chatCompletion(
  virtualKey: string,
  model: string,
  message: string,
  fallbackModels?: string[],
): Promise<ChatCompletionResult> {
  const res = await fetch(`${API_BASE}/v1/chat/completions`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${virtualKey}` },
    body: JSON.stringify({
      model,
      messages: [{ role: 'user', content: message }],
      ...(fallbackModels?.length ? { fallback_models: fallbackModels } : {}),
    }),
  });
  const body = await parseBody(res);
  await raiseIfError(res, body);
  return body as ChatCompletionResult;
}
