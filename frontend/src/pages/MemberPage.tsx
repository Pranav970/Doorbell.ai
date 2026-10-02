import { useEffect, useState, type FormEvent } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useAuth, approvedTeams } from '../context/AuthContext';
import { api, chatCompletion, errorMessage } from '../lib/api';
import type { FallbackInfo, OrgWithTeams, RequestLogEntry, VirtualKeyCreated } from '../lib/types';
import { CUSTOM_PROVIDER, KNOWN_PROVIDERS, PROVIDER_LABELS } from '../lib/types';
import { Shell, type NavItem } from '../components/Shell';
import { Badge, Callout, Card, ErrorText, Muted, StatCard } from '../components/Card';
import { Button } from '../components/Button';
import { Field, Input, Select, Textarea } from '../components/Input';
import { Table, Td, Tr } from '../components/Table';
import { HoverCard } from '../components/HoverCard';
import { Icon } from '../components/icons';
import { UsageTable } from '../components/UsageTable';

type Tab = 'home' | 'analytics' | 'logs' | 'keys' | 'vkeys' | 'routing' | 'playground';

const NAV_ITEMS: NavItem[] = [
  { key: 'home', label: 'Home', icon: 'home' },
  { key: 'analytics', label: 'Analytics', icon: 'chart' },
  { key: 'logs', label: 'Logs', icon: 'logs' },
  { key: 'keys', label: 'Provider Keys', icon: 'key' },
  { key: 'vkeys', label: 'Unified API Key', icon: 'layers' },
  { key: 'routing', label: 'Routing Rules', icon: 'route' },
  { key: 'playground', label: 'Playground', icon: 'play' },
  { key: 'log-analysis', label: 'Log analysis', icon: 'chart', href: '/logs' },
];

export function MemberPage() {
  const { me, refreshMe } = useAuth();
  const teams = approvedTeams(me);
  const [selectedTeamId, setSelectedTeamId] = useState('');
  const [tab, setTab] = useState<Tab>('home');
  const [sessionKeys, setSessionKeys] = useState<VirtualKeyCreated[]>([]);

  // Derived, not defaulted in an effect: an effect only runs *after* the first
  // render, so every child below would mount once with an empty id and fire
  // `/api/teams//…` — a 400 on every dashboard load. Falling back to the first
  // team inline means the id is never empty on any render that reaches them.
  const teamId = selectedTeamId || teams[0]?.team_id || '';

  if (teams.length === 0) {
    return (
      <Shell
        navItems={[{ key: 'home', label: 'Home', icon: 'home' }]}
        activeKey="home"
        onNavChange={() => undefined}
        subtitle="Get started"
      >
        <PendingOrBrowse me={me} onRequested={refreshMe} />
      </Shell>
    );
  }

  const teamSelect = (
    <Select
      aria-label="Team"
      value={teamId}
      onChange={(e) => setSelectedTeamId(e.target.value)}
      className="w-auto py-1.5"
    >
      {teams.map((t) => (
        <option key={t.team_id} value={t.team_id}>
          {t.team_name}
        </option>
      ))}
    </Select>
  );

  return (
    <Shell
      navItems={NAV_ITEMS}
      activeKey={tab}
      onNavChange={(key) => setTab(key as Tab)}
      subtitle="Team dashboard"
      headerExtra={teamSelect}
    >
      {tab === 'home' && <HomeTab teamId={teamId} sessionKeys={sessionKeys} onNavigate={setTab} />}
      {tab === 'analytics' && (
        <Card title="Usage by provider & model">
          <UsageTable scope={{ team: teamId }} />
        </Card>
      )}
      {tab === 'logs' && <LogsTab teamId={teamId} />}
      {tab === 'keys' && <ProviderKeysTab teamId={teamId} onIssued={(vk) => setSessionKeys((s) => [vk, ...s])} />}
      {tab === 'vkeys' && <VirtualKeysTab teamId={teamId} onIssued={(vk) => setSessionKeys((s) => [vk, ...s])} />}
      {tab === 'routing' && <RoutingRulesTab teamId={teamId} />}
      {tab === 'playground' && <PlaygroundTab teamId={teamId} />}
    </Shell>
  );
}

function HomeTab({
  teamId,
  sessionKeys,
  onNavigate,
}: {
  teamId: string;
  sessionKeys: VirtualKeyCreated[];
  onNavigate: (tab: Tab) => void;
}) {
  const vkeysQuery = useQuery({ queryKey: ['virtual-keys', teamId], queryFn: () => api.listVirtualKeys(teamId) });
  const providerKeysQuery = useQuery({
    queryKey: ['provider-keys', teamId],
    queryFn: () => api.listProviderKeys(teamId),
  });
  const [copied, setCopied] = useState(false);

  const activeVKey = vkeysQuery.data?.items.find((k) => k.status === 'active');
  const issuedThisSession = sessionKeys.find((k) => k.id === activeVKey?.id);

  // Keys are now retrievable on demand through the gated reveal endpoint, so
  // "copy it now or lose it" no longer applies.
  const reveal = useMutation({
    mutationFn: () => api.revealVirtualKey(teamId, activeVKey!.id),
  });
  const revealed = reveal.data?.key ?? issuedThisSession?.key ?? null;

  const copy = async (key: string) => {
    await navigator.clipboard.writeText(key);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Welcome back</h1>
        <p className="mt-1.5 max-w-2xl text-sm text-muted">
          One unified API key routes every request to whichever provider you've connected — swap providers behind it
          without touching your app.
        </p>
      </div>

      <Card title="Your unified API key">
        {!activeVKey && (
          <div>
            <Muted>Add a provider key first — your unified key gets issued automatically the moment you do.</Muted>
            <Button className="mt-3" onClick={() => onNavigate('keys')}>
              Bring your own key
            </Button>
          </div>
        )}
        {activeVKey && (
          <>
            <div className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-line bg-raised px-4 py-3">
              <code className="break-all font-mono text-sm text-blue-400">
                {revealed ?? `${activeVKey.prefix}••••••••••••••••••`}
              </code>
              {revealed ? (
                <Button size="sm" onClick={() => copy(revealed)}>
                  <Icon name="copy" className="h-3.5 w-3.5" />
                  {copied ? 'Copied!' : 'Copy'}
                </Button>
              ) : (
                <Button size="sm" variant="secondary" disabled={reveal.isPending} onClick={() => reveal.mutate()}>
                  {reveal.isPending ? 'Revealing…' : 'Reveal key'}
                </Button>
              )}
            </div>
            <ErrorText message={errorMessage(reveal.error, 'Could not reveal this key.')} />
            <p className="mt-2 text-xs text-muted">
              {revealed
                ? 'Anyone with access to this team can reveal it again — no need to store it somewhere else.'
                : 'Hidden by default. Reveal it whenever you need it; only members of this team can.'}
            </p>
          </>
        )}
      </Card>

      <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
        <StatCard
          label="Active provider keys"
          value={providerKeysQuery.data?.items.filter((k) => k.is_active).length ?? 0}
          onClick={() => onNavigate('keys')}
        />
        <StatCard
          label="Active unified keys"
          value={vkeysQuery.data?.items.filter((k) => k.status === 'active').length ?? 0}
          onClick={() => onNavigate('vkeys')}
        />
        <StatCard label="Try a request" value="Playground" onClick={() => onNavigate('playground')} />
      </div>
    </div>
  );
}

const PLACEHOLDER_FALLBACK_REASONS = [
  'Token limit reached for primary model',
  'Primary model unavailable',
  'Network/Reachability issue',
  'API error from primary provider',
] as const;

/**
 * Stand-in until the backend persists the real reason (see
 * `RequestLogEntry.fallback_reason`). Derived from the log id rather than
 * picked at random so a row shows the same reason on every render and
 * refetch instead of shuffling under the cursor.
 */
function placeholderFallbackReason(id: string): string {
  let hash = 0;
  for (let i = 0; i < id.length; i += 1) hash = (hash * 31 + id.charCodeAt(i)) >>> 0;
  return PLACEHOLDER_FALLBACK_REASONS[hash % PLACEHOLDER_FALLBACK_REASONS.length];
}

function FallbackSummary({ log }: { log: RequestLogEntry }) {
  const reason = log.fallback_reason ?? placeholderFallbackReason(log.id);
  return (
    <div className="flex flex-col gap-1.5">
      <p className="text-sm font-semibold text-fg">Fallback summary</p>
      <p className="text-xs leading-relaxed text-muted">
        <span className="font-semibold text-fg">Reason:</span> {reason}
      </p>
      <p className="text-xs leading-relaxed text-muted">
        {/* `model` is already the model that served: the handler logs
            `FallbackSuccess.model_used`, not the originally requested one. */}
        <span className="font-semibold text-fg">Served by:</span>{' '}
        <span className="font-mono text-blue-400">{log.model}</span> ({log.provider})
      </p>
      <p className="text-xs leading-relaxed text-muted">
        <span className="font-semibold text-fg">Attempts before success:</span> {log.fallback_count}
      </p>
      {log.fallback_reason == null && (
        <p className="mt-0.5 border-t border-line pt-1.5 text-[11px] italic leading-snug text-muted">
          Sample reason — not recorded by the backend yet.
        </p>
      )}
    </div>
  );
}

function LogsTab({ teamId }: { teamId: string }) {
  const logsQuery = useQuery({ queryKey: ['logs', teamId], queryFn: () => api.teamLogs(teamId) });
  const items = logsQuery.data?.items ?? [];
  return (
    <Card title="Recent requests">
      {items.length ? (
        <Table columns={['Time', 'Provider', 'Model', 'Status', 'Latency', 'Tokens in/out', 'Fallback']}>
          {items.map((log) => (
            <Tr key={log.id}>
              <Td className="text-muted">{new Date(log.created_at).toLocaleString()}</Td>
              <Td>{log.provider}</Td>
              <Td className="font-mono text-xs">{log.model}</Td>
              <Td className={log.status_code >= 400 ? 'text-red-400' : 'text-blue-400'}>{log.status_code}</Td>
              <Td className="text-muted">{log.latency_ms}ms</Td>
              <Td className="text-muted">
                {log.tokens_in ?? 0} / {log.tokens_out ?? 0}
              </Td>
              <Td>
                {log.fallback_count > 0 ? (
                  <HoverCard content={<FallbackSummary log={log} />}>
                    <Badge tone="accent">
                      {log.fallback_count} fallback{log.fallback_count > 1 ? 's' : ''}
                      <Icon name="info" className="h-3.5 w-3.5 opacity-70" />
                    </Badge>
                  </HoverCard>
                ) : (
                  <span className="text-muted">—</span>
                )}
              </Td>
            </Tr>
          ))}
        </Table>
      ) : (
        <Muted>No requests logged yet.</Muted>
      )}
    </Card>
  );
}

function PendingOrBrowse({ me, onRequested }: { me: ReturnType<typeof useAuth>['me']; onRequested: () => void }) {
  const [selectedTeam, setSelectedTeam] = useState('');
  const orgsQuery = useQuery({ queryKey: ['allOrgs'], queryFn: () => api.listOrgs() });
  const orgs: OrgWithTeams[] = orgsQuery.data?.items ?? [];

  const nonApproved = me?.team_memberships.filter((m) => m.status !== 'approved') ?? [];

  const requestJoin = useMutation({
    mutationFn: () => api.requestJoin(selectedTeam),
    onSuccess: () => onRequested(),
  });

  const onSubmit = (e: FormEvent) => {
    e.preventDefault();
    if (!selectedTeam) return;
    requestJoin.mutate();
  };

  return (
    <div className="flex flex-col gap-6">
      {nonApproved.length > 0 && (
        <Card title="Your requests">
          <ul className="divide-y divide-line/70 text-sm">
            {nonApproved.map((m) => (
              <li key={m.team_id} className="flex justify-between py-2.5">
                <span>{m.team_name}</span>
                <span className="capitalize text-muted">{m.status}</span>
              </li>
            ))}
          </ul>
        </Card>
      )}
      <Card title="Request to join a team">
        <form onSubmit={onSubmit} className="flex flex-col gap-4">
          <ErrorText message={errorMessage(requestJoin.error, 'Could not submit request.')} />
          {orgs.map((org) => (
            <div key={org.id}>
              <p className="mb-2 text-xs font-medium uppercase tracking-wider text-muted">{org.name}</p>
              <div className="flex flex-col gap-1">
                {org.teams.map((team) => (
                  <label
                    key={team.id}
                    className="flex cursor-pointer items-center gap-2 rounded-md px-2 py-1.5 text-sm transition hover:bg-blue-500/5"
                  >
                    <input
                      type="radio"
                      name="team"
                      value={team.id}
                      checked={selectedTeam === team.id}
                      onChange={() => setSelectedTeam(team.id)}
                    />
                    {team.name}
                  </label>
                ))}
              </div>
            </div>
          ))}
          <Button type="submit" disabled={requestJoin.isPending || !selectedTeam} className="self-start">
            Request to join
          </Button>
        </form>
      </Card>
    </div>
  );
}

function slugifyProvider(name: string): string {
  return name
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 40);
}

function ProviderKeysTab({ teamId, onIssued }: { teamId: string; onIssued: (vk: VirtualKeyCreated) => void }) {
  const queryClient = useQueryClient();
  const [provider, setProvider] = useState<string>(KNOWN_PROVIDERS[0]);
  const [customName, setCustomName] = useState('');
  const [baseUrl, setBaseUrl] = useState('');
  const [apiKey, setApiKey] = useState('');
  const [label, setLabel] = useState('');
  const [formError, setFormError] = useState<string | null>(null);
  const [revealedUnifiedKey, setRevealedUnifiedKey] = useState<VirtualKeyCreated | null>(null);

  const keysQuery = useQuery({ queryKey: ['provider-keys', teamId], queryFn: () => api.listProviderKeys(teamId) });
  const isCustom = provider === CUSTOM_PROVIDER;

  const addKey = useMutation({
    mutationFn: () =>
      api.addProviderKey(teamId, isCustom ? slugifyProvider(customName) : provider, apiKey, label, baseUrl),
    onSuccess: async () => {
      setApiKey('');
      setLabel('');
      setCustomName('');
      setBaseUrl('');
      queryClient.invalidateQueries({ queryKey: ['provider-keys', teamId] });

      // First key for this team: mint its one unified API key automatically
      // so adding a key and having something to call are the same step.
      // Best-effort — the credential was already saved either way, and the
      // Unified API Key tab still covers issuing one manually.
      try {
        const existing = await api.listVirtualKeys(teamId);
        if (existing.items.length === 0) {
          const vk = await api.issueVirtualKey(teamId, 'default');
          setRevealedUnifiedKey(vk);
          onIssued(vk);
          queryClient.invalidateQueries({ queryKey: ['virtual-keys', teamId] });
        }
      } catch {
        // ignored — see comment above
      }
    },
  });

  const deactivateKey = useMutation({
    mutationFn: (id: string) => api.deactivateProviderKey(teamId, id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['provider-keys', teamId] }),
  });

  const deleteKey = useMutation({
    mutationFn: (id: string) => api.deleteProviderKey(teamId, id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['provider-keys', teamId] }),
  });

  const onSubmit = (e: FormEvent) => {
    e.preventDefault();
    setFormError(null);
    if (isCustom && !slugifyProvider(customName)) {
      setFormError('Name your custom provider.');
      return;
    }
    if (baseUrl.trim() && !/^https?:\/\//.test(baseUrl.trim())) {
      setFormError('Base URL must start with http:// or https://.');
      return;
    }
    addKey.mutate();
  };

  return (
    <div className="flex flex-col gap-6">
      <Card title="Bring your own key">
        <form onSubmit={onSubmit} className="flex flex-col gap-4">
          <ErrorText message={formError ?? errorMessage(addKey.error, 'Could not add key.')} />
          <div className="flex flex-col gap-3 sm:flex-row">
            <Select value={provider} onChange={(e) => setProvider(e.target.value)} className="sm:w-56">
              {KNOWN_PROVIDERS.map((p) => (
                <option key={p} value={p}>
                  {PROVIDER_LABELS[p]}
                </option>
              ))}
              <option value={CUSTOM_PROVIDER}>Custom…</option>
            </Select>
            <Input placeholder="Label (optional)" value={label} onChange={(e) => setLabel(e.target.value)} />
          </div>
          {isCustom && (
            <Input
              required
              placeholder="Provider name (e.g. nvidia, groq, mistral)"
              value={customName}
              onChange={(e) => setCustomName(e.target.value)}
            />
          )}
          <div>
            <Input
              placeholder="Base URL (e.g. https://integrate.api.nvidia.com)"
              value={baseUrl}
              onChange={(e) => setBaseUrl(e.target.value)}
            />
            <p className="mt-1.5 text-xs leading-relaxed text-muted">
              The endpoint this key is called at. Any vendor speaking the OpenAI wire format works here — NVIDIA NIM,
              Groq, Mistral, Cohere, Together, your own deployment — so the gateway isn't limited to the providers it
              ships adapters for. Leave blank only for OpenAI, Anthropic and Gemini, which speak their own protocols
              and carry a default endpoint. The gateway appends{' '}
              <code className="text-blue-400">/v1/chat/completions</code> itself, so enter just the host — no trailing{' '}
              <code className="text-blue-400">/v1</code> or <code className="text-blue-400">/chat/completions</code>.
            </p>
          </div>
          <Input
            type="password"
            required
            placeholder="API key"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
          />
          <Button type="submit" disabled={addKey.isPending} className="self-start">
            Add key
          </Button>
        </form>
        {revealedUnifiedKey && (
          <div className="mt-4">
            <Callout title="This is your team's unified API key — copy it now, it won't be shown again.">
              <p className="mb-2 text-xs text-muted">Use it (not your provider keys) to call the gateway.</p>
              <code className="block break-all rounded-md border border-line bg-raised px-2 py-1.5 font-mono text-xs text-blue-400">
                {revealedUnifiedKey.key}
              </code>
            </Callout>
          </div>
        )}
      </Card>

      <Card title="Team keys">
        {keysQuery.data?.items.length ? (
          <ul className="divide-y divide-line/70">
            {keysQuery.data.items.map((key) => (
              <li key={key.id} className="flex items-center justify-between gap-4 py-2.5 text-sm">
                <div className="min-w-0">
                  <span className="font-medium">{key.provider}</span>{' '}
                  <span className="font-mono text-xs text-muted">{key.masked_key}</span>
                  {key.label && <span className="ml-2 text-muted">({key.label})</span>}
                  {key.base_url && <span className="ml-2 text-xs text-muted">→ {key.base_url}</span>}
                  {!key.is_active && (
                    <span className="ml-2">
                      <Badge tone="danger">deactivated</Badge>
                    </span>
                  )}
                </div>
                <div className="flex shrink-0 gap-2">
                  {key.is_active ? (
                    <Button size="sm" variant="secondary" onClick={() => deactivateKey.mutate(key.id)}>
                      Deactivate
                    </Button>
                  ) : (
                    <Button size="sm" variant="danger" onClick={() => deleteKey.mutate(key.id)}>
                      Delete
                    </Button>
                  )}
                </div>
              </li>
            ))}
          </ul>
        ) : (
          <Muted>No keys yet.</Muted>
        )}
      </Card>
    </div>
  );
}

function VirtualKeysTab({ teamId, onIssued }: { teamId: string; onIssued: (vk: VirtualKeyCreated) => void }) {
  const queryClient = useQueryClient();
  const [name, setName] = useState('');
  const [revealed, setRevealed] = useState<VirtualKeyCreated | null>(null);

  const vkeysQuery = useQuery({ queryKey: ['virtual-keys', teamId], queryFn: () => api.listVirtualKeys(teamId) });

  const issue = useMutation({
    mutationFn: () => api.issueVirtualKey(teamId, name),
    onSuccess: (vk) => {
      setName('');
      setRevealed(vk);
      onIssued(vk);
      queryClient.invalidateQueries({ queryKey: ['virtual-keys', teamId] });
    },
  });

  const revoke = useMutation({
    mutationFn: (id: string) => api.revokeVirtualKey(teamId, id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['virtual-keys', teamId] }),
  });

  const deleteVk = useMutation({
    mutationFn: (id: string) => api.deleteVirtualKey(teamId, id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['virtual-keys', teamId] }),
  });

  const onSubmit = (e: FormEvent) => {
    e.preventDefault();
    issue.mutate();
  };

  return (
    <div className="flex flex-col gap-6">
      <Card title="Your unified API key">
        <Muted>
          This is the one key/endpoint you use to call the gateway, no matter which provider keys are behind it — one
          gets issued automatically the first time you add a provider key on the Provider Keys tab. Issue another here
          only if you need a separate key for a different app or environment.
        </Muted>
        <form onSubmit={onSubmit} className="mt-4 flex flex-col gap-3">
          <ErrorText message={errorMessage(issue.error, 'Could not issue key. Add a provider key first.')} />
          <div className="flex gap-3">
            <Input placeholder="Name (optional)" value={name} onChange={(e) => setName(e.target.value)} />
            <Button type="submit" disabled={issue.isPending}>
              Issue key
            </Button>
          </div>
        </form>
        {revealed && (
          <div className="mt-4">
            <Callout title="Copy this key now — it won't be shown again.">
              <code className="block break-all rounded-md border border-line bg-raised px-2 py-1.5 font-mono text-xs text-blue-400">
                {revealed.key}
              </code>
            </Callout>
          </div>
        )}
      </Card>

      <Card title="Team unified keys">
        {vkeysQuery.data?.items.length ? (
          <ul className="divide-y divide-line/70">
            {vkeysQuery.data.items.map((vk) => (
              <li key={vk.id} className="flex items-center justify-between gap-4 py-2.5 text-sm">
                <div className="min-w-0">
                  <span className="font-mono text-blue-400">{vk.prefix}…</span>
                  {vk.name && <span className="ml-2 text-muted">({vk.name})</span>}
                  <span className="ml-2 text-xs capitalize text-muted">{vk.status}</span>
                </div>
                <div className="flex shrink-0 gap-2">
                  {vk.status === 'active' ? (
                    <Button size="sm" variant="secondary" onClick={() => revoke.mutate(vk.id)}>
                      Revoke
                    </Button>
                  ) : (
                    <Button size="sm" variant="danger" onClick={() => deleteVk.mutate(vk.id)}>
                      Delete
                    </Button>
                  )}
                </div>
              </li>
            ))}
          </ul>
        ) : (
          <Muted>No unified keys yet.</Muted>
        )}
      </Card>
    </div>
  );
}

function RoutingRulesTab({ teamId }: { teamId: string }) {
  const queryClient = useQueryClient();
  const [virtualKeyId, setVirtualKeyId] = useState('');
  const [modelPattern, setModelPattern] = useState('');
  const [providerCredentialId, setProviderCredentialId] = useState('');
  const [priority, setPriority] = useState('0');
  const [formError, setFormError] = useState<string | null>(null);

  const vkeysQuery = useQuery({ queryKey: ['virtual-keys', teamId], queryFn: () => api.listVirtualKeys(teamId) });
  const keysQuery = useQuery({ queryKey: ['provider-keys', teamId], queryFn: () => api.listProviderKeys(teamId) });
  const rulesQuery = useQuery({
    queryKey: ['routing-rules', teamId, virtualKeyId],
    queryFn: () => api.listRoutingRules(teamId, virtualKeyId),
    enabled: !!virtualKeyId,
  });

  useEffect(() => {
    if (!virtualKeyId && vkeysQuery.data?.items[0]) setVirtualKeyId(vkeysQuery.data.items[0].id);
  }, [vkeysQuery.data, virtualKeyId]);

  useEffect(() => {
    if (!providerCredentialId && keysQuery.data?.items[0]) setProviderCredentialId(keysQuery.data.items[0].id);
  }, [keysQuery.data, providerCredentialId]);

  const addRule = useMutation({
    mutationFn: () =>
      api.addRoutingRule(teamId, virtualKeyId, modelPattern.trim(), providerCredentialId, Number(priority) || 0),
    onSuccess: () => {
      setModelPattern('');
      setPriority('0');
      queryClient.invalidateQueries({ queryKey: ['routing-rules', teamId, virtualKeyId] });
    },
  });

  const deleteRule = useMutation({
    mutationFn: (id: string) => api.deleteRoutingRule(teamId, virtualKeyId, id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['routing-rules', teamId, virtualKeyId] }),
  });

  const onSubmit = (e: FormEvent) => {
    e.preventDefault();
    setFormError(null);
    if (!virtualKeyId) {
      setFormError('Issue a unified API key first.');
      return;
    }
    if (!modelPattern.trim()) {
      setFormError('Model pattern must not be empty.');
      return;
    }
    if (!providerCredentialId) {
      setFormError('Add a provider key first.');
      return;
    }
    addRule.mutate();
  };

  const credentialLabel = (id: string) => {
    const cred = keysQuery.data?.items.find((k) => k.id === id);
    if (!cred) return id;
    return `${cred.provider}${cred.label ? ` (${cred.label})` : ''} ${cred.masked_key}`;
  };

  return (
    <div className="flex flex-col gap-6">
      <Card title="Model routing">
        <Muted>
          Point a model-name pattern at any of your provider keys. You only need this if you've connected more than one
          provider key — with just one, every model you send automatically routes there, whatever vendor it is (OpenAI,
          Anthropic, Gemini, or a custom/BYOK vendor like NVIDIA NIM, Mistral or Groq reached via a Base URL). Add a
          rule once you have multiple keys and need to say which model goes where. A pattern ending in{' '}
          <code className="text-blue-400">*</code> matches by prefix; otherwise it must match the{' '}
          <code className="text-blue-400">model</code> field exactly. Lower priority number wins when more than one
          rule matches.
        </Muted>

        <div className="mt-4 max-w-sm">
          <Field label="Unified API key">
            <Select value={virtualKeyId} onChange={(e) => setVirtualKeyId(e.target.value)}>
              {vkeysQuery.data?.items.map((vk) => (
                <option key={vk.id} value={vk.id}>
                  {vk.name ?? vk.prefix} ({vk.status})
                </option>
              ))}
            </Select>
          </Field>
        </div>

        <form onSubmit={onSubmit} className="mt-4 flex flex-col gap-3">
          <ErrorText message={formError ?? errorMessage(addRule.error, 'Could not add routing rule.')} />
          <div className="flex flex-col gap-3 sm:flex-row sm:items-end">
            <Field label="Model pattern" className="flex-1">
              <Input placeholder="nvidia/*" value={modelPattern} onChange={(e) => setModelPattern(e.target.value)} />
            </Field>
            <Field label="Provider key" className="flex-1">
              <Select value={providerCredentialId} onChange={(e) => setProviderCredentialId(e.target.value)}>
                {keysQuery.data?.items.map((k) => (
                  <option key={k.id} value={k.id} disabled={!k.is_active}>
                    {credentialLabel(k.id)}
                    {!k.is_active ? ' — deactivated' : ''}
                  </option>
                ))}
              </Select>
            </Field>
            <Field label="Priority" className="w-24">
              <Input type="number" value={priority} onChange={(e) => setPriority(e.target.value)} />
            </Field>
            <Button type="submit" disabled={addRule.isPending}>
              Add rule
            </Button>
          </div>
        </form>
      </Card>

      <Card title="Rules for this key">
        {rulesQuery.data?.items.length ? (
          <Table columns={['Model pattern', 'Provider key', 'Priority', 'Created', '']}>
            {rulesQuery.data.items.map((rule) => (
              <Tr key={rule.id}>
                <Td className="font-mono text-blue-400">{rule.model_pattern}</Td>
                <Td>{credentialLabel(rule.provider_credential_id)}</Td>
                <Td className="text-muted">{rule.priority}</Td>
                <Td className="text-muted">{new Date(rule.created_at).toLocaleString()}</Td>
                <Td className="text-right">
                  <Button size="sm" variant="danger" onClick={() => deleteRule.mutate(rule.id)}>
                    Delete
                  </Button>
                </Td>
              </Tr>
            ))}
          </Table>
        ) : (
          <Muted>
            No rules yet for this key — traffic falls back to the gpt-/claude-/gemini- prefix map, or automatically to
            your only active provider key if you've connected just one. Add a rule once you connect a second key.
          </Muted>
        )}
      </Card>
    </div>
  );
}

function PlaygroundTab({ teamId }: { teamId: string }) {
  const [selectedKeyId, setSelectedKeyId] = useState('');
  const [model, setModel] = useState('');
  const [fallbackModels, setFallbackModels] = useState('');
  const [message, setMessage] = useState('Say hello in one short sentence.');

  const vkeysQuery = useQuery({ queryKey: ['virtual-keys', teamId], queryFn: () => api.listVirtualKeys(teamId) });
  const activeKeys = vkeysQuery.data?.items.filter((k) => k.status === 'active') ?? [];
  // Derived rather than defaulted in an effect — `activeKeys` is a fresh array
  // every render, so an effect keyed on it would re-run every render.
  const virtualKeyId = selectedKeyId || activeKeys[0]?.id || '';

  const send = useMutation({
    mutationFn: async () => {
      const fallbacks = fallbackModels
        .split(',')
        .map((m) => m.trim())
        .filter(Boolean);
      // Fetch the plaintext through the authorization-gated reveal endpoint,
      // then call the gateway exactly as any external client would:
      // `Authorization: Bearer <virtual_key>` against /v1/chat/completions.
      // No privileged shortcut — this exercises the same virtual-key
      // authentication a customer's own code hits.
      const { key } = await api.revealVirtualKey(teamId, virtualKeyId);
      return chatCompletion(key, model, message, fallbacks);
    },
  });

  // Hidden while a new request is in flight so the previous response can't be
  // mistaken for the pending one.
  const result = send.isPending ? null : (send.data ?? null);
  const fallbackInfo: FallbackInfo | null = result?.fallback ?? null;

  const onSubmit = (e: FormEvent) => {
    e.preventDefault();
    send.mutate();
  };

  return (
    <Card title="Call your unified API">
      <form onSubmit={onSubmit} className="flex flex-col gap-4">
        <ErrorText message={errorMessage(send.error, 'Request failed.')} />
        <Field
          label="Unified API key"
          hint="Sent as Authorization: Bearer — the same way your own code would call the gateway."
        >
          {activeKeys.length > 0 ? (
            <Select value={virtualKeyId} onChange={(e) => setSelectedKeyId(e.target.value)}>
              {activeKeys.map((vk) => (
                <option key={vk.id} value={vk.id}>
                  {vk.name ?? vk.prefix} ({vk.prefix}…)
                </option>
              ))}
            </Select>
          ) : (
            <Muted>No active unified keys — issue one on the Unified API Key tab.</Muted>
          )}
        </Field>
        <Field label="Model" hint="Whatever your routing rules point at a provider key.">
          <Input value={model} onChange={(e) => setModel(e.target.value)} className="font-mono" />
        </Field>
        <Field
          label="Fallback models"
          hint="If the model above fails with a rate limit, server error, timeout, context-length overflow, or isn't reachable at all (404 — wrong name, deprecated, not enabled for this key), the gateway retries each of these in order — even across providers."
        >
          <Input
            placeholder="e.g. claude-3-opus-20240229, gpt-4o"
            value={fallbackModels}
            onChange={(e) => setFallbackModels(e.target.value)}
            className="font-mono"
          />
        </Field>
        <Field label="Message">
          <Textarea value={message} onChange={(e) => setMessage(e.target.value)} rows={3} />
        </Field>
        <Button type="submit" disabled={send.isPending || !virtualKeyId || !model.trim()} className="self-start">
          {send.isPending ? 'Sending…' : 'Send'}
        </Button>
      </form>
      {fallbackInfo && (
        <div className="mt-4">
          <Callout title={`Primary model failed — served by ${fallbackInfo.model_used} instead.`}>
            <ul className="list-disc space-y-0.5 pl-4 text-xs text-muted">
              {fallbackInfo.attempts.map((a, i) => (
                <li key={i}>
                  <code className="font-mono text-blue-400">{a.model}</code> via {a.provider}: {a.error}
                </li>
              ))}
            </ul>
          </Callout>
        </div>
      )}
      {result && (
        <pre className="mt-4 max-h-96 overflow-auto rounded-lg border border-line bg-raised p-3 font-mono text-xs text-muted">
          {JSON.stringify(result, null, 2)}
        </pre>
      )}
    </Card>
  );
}
