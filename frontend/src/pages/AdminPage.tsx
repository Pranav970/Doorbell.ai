import { useState, type FormEvent } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useAuth, primaryAdminOrgId, isMasterAdmin } from '../context/AuthContext';
import { api, errorMessage } from '../lib/api';
import { Shell, type NavItem } from '../components/Shell';
import { Badge, Card, ErrorText, Muted, StatCard } from '../components/Card';
import { Button } from '../components/Button';
import { Input, Select } from '../components/Input';
import { UsageTable } from '../components/UsageTable';

type Tab = 'home' | 'teams' | 'requests' | 'analytics';

const NAV_ITEMS: NavItem[] = [
  { key: 'home', label: 'Home', icon: 'home' },
  { key: 'teams', label: 'Teams', icon: 'users' },
  { key: 'requests', label: 'Requests', icon: 'inbox' },
  { key: 'analytics', label: 'Analytics', icon: 'chart' },
];

export function AdminPage() {
  const { me } = useAuth();
  const master = isMasterAdmin(me);
  const [tab, setTab] = useState<Tab>('home');
  const [pickedOrgId, setPickedOrgId] = useState<string | null>(null);

  // A master governs every org rather than belonging to one, so their org
  // comes from the global list (/api/orgs already returns all of them to
  // any authenticated caller) instead of from their own memberships.
  const allOrgsQuery = useQuery({
    queryKey: ['allOrgs'],
    queryFn: () => api.listOrgs(),
    enabled: master,
  });
  const allOrgs = allOrgsQuery.data?.items ?? [];
  const orgId = master ? (pickedOrgId ?? allOrgs[0]?.id ?? null) : primaryAdminOrgId(me);
  const orgName = master ? allOrgs.find((o) => o.id === orgId)?.name : undefined;

  const orgPicker = master ? (
    <div className="mb-6 flex items-center gap-3">
      <label htmlFor="master-org" className="text-xs font-medium uppercase tracking-wider text-muted">
        Organization
      </label>
      <Select id="master-org" value={orgId ?? ''} onChange={(e) => setPickedOrgId(e.target.value)} className="w-auto">
        {allOrgs.map((o) => (
          <option key={o.id} value={o.id}>
            {o.name}
          </option>
        ))}
      </Select>
      <Badge tone="accent">Master</Badge>
    </div>
  ) : null;

  const subtitle = master ? 'Master console — all organizations' : 'Admin console';

  if (!orgId) {
    return (
      <Shell navItems={NAV_ITEMS} activeKey={tab} onNavChange={(key) => setTab(key as Tab)} subtitle={subtitle}>
        {orgPicker}
        <Card title={master ? 'No organizations yet' : undefined}>
          <Muted>
            {master
              ? 'Nothing has been created on this deployment yet. Organizations appear here as soon as anyone creates one.'
              : 'Loading…'}
          </Muted>
        </Card>
      </Shell>
    );
  }

  return (
    <Shell navItems={NAV_ITEMS} activeKey={tab} onNavChange={(key) => setTab(key as Tab)} subtitle={subtitle}>
      {orgPicker}
      {tab === 'home' && <HomeTab orgId={orgId} orgNameOverride={orgName} onNavigate={setTab} />}
      {tab === 'teams' && <TeamsTab orgId={orgId} />}
      {tab === 'requests' && <RequestsTab orgId={orgId} />}
      {tab === 'analytics' && (
        <Card title="Key usage across teams">
          <UsageTable scope={{ org: orgId }} />
        </Card>
      )}
    </Shell>
  );
}

function HomeTab({
  orgId,
  orgNameOverride,
  onNavigate,
}: {
  orgId: string;
  orgNameOverride?: string;
  onNavigate: (tab: Tab) => void;
}) {
  const { me } = useAuth();
  // A master has no membership row to read the name from — the selected
  // org's name is passed down instead.
  const orgName =
    orgNameOverride ?? me?.org_memberships.find((m) => m.org_id === orgId)?.org_name ?? 'Your organization';
  const teamsQuery = useQuery({ queryKey: ['teams', orgId], queryFn: () => api.listTeamsForOrg(orgId) });
  const requestsQuery = useQuery({ queryKey: ['pending', orgId], queryFn: () => api.pendingRequestsForOrg(orgId) });

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">{orgName}</h1>
        <p className="mt-1.5 max-w-2xl text-sm text-muted">
          Manage teams, approve join requests, and watch usage across every unified API key in your organization.
        </p>
      </div>
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
        <StatCard label="Teams" value={teamsQuery.data?.items.length ?? 0} onClick={() => onNavigate('teams')} />
        <StatCard
          label="Pending requests"
          value={requestsQuery.data?.items.length ?? 0}
          onClick={() => onNavigate('requests')}
        />
        <StatCard label="Usage & analytics" value="View" onClick={() => onNavigate('analytics')} />
      </div>
    </div>
  );
}

function TeamsTab({ orgId }: { orgId: string }) {
  const queryClient = useQueryClient();
  const [name, setName] = useState('');

  const teamsQuery = useQuery({ queryKey: ['teams', orgId], queryFn: () => api.listTeamsForOrg(orgId) });
  const createTeam = useMutation({
    mutationFn: (n: string) => api.createTeam(orgId, n),
    onSuccess: () => {
      setName('');
      queryClient.invalidateQueries({ queryKey: ['teams', orgId] });
    },
  });

  const onSubmit = (e: FormEvent) => {
    e.preventDefault();
    createTeam.mutate(name);
  };

  return (
    <div className="flex flex-col gap-6">
      <Card title="Create a team">
        <form onSubmit={onSubmit} className="flex flex-col gap-3">
          <ErrorText message={errorMessage(createTeam.error, 'Could not create team.')} />
          <div className="flex gap-3">
            <Input required placeholder="Team name" value={name} onChange={(e) => setName(e.target.value)} />
            <Button type="submit" disabled={createTeam.isPending}>
              Create
            </Button>
          </div>
        </form>
      </Card>

      <Card title="Teams">
        {teamsQuery.data?.items.length ? (
          <ul className="divide-y divide-line/70">
            {teamsQuery.data.items.map((team) => (
              <li key={team.id} className="flex items-center justify-between py-2.5 text-sm">
                <span className="font-medium">{team.name}</span>
                <span className="font-mono text-xs text-muted">{team.slug}</span>
              </li>
            ))}
          </ul>
        ) : (
          <Muted>No teams yet.</Muted>
        )}
      </Card>
    </div>
  );
}

function RequestsTab({ orgId }: { orgId: string }) {
  const queryClient = useQueryClient();

  const requestsQuery = useQuery({ queryKey: ['pending', orgId], queryFn: () => api.pendingRequestsForOrg(orgId) });

  const invalidate = () => queryClient.invalidateQueries({ queryKey: ['pending', orgId] });
  const approve = useMutation({
    mutationFn: ({ teamId, reqId }: { teamId: string; reqId: string }) => api.approveJoin(teamId, reqId),
    onSuccess: invalidate,
  });
  const reject = useMutation({
    mutationFn: ({ teamId, reqId }: { teamId: string; reqId: string }) => api.rejectJoin(teamId, reqId),
    onSuccess: invalidate,
  });

  return (
    <Card title="Pending join requests">
      <div className="flex flex-col gap-3">
        <ErrorText
          message={
            errorMessage(approve.error, 'Could not approve request.') ??
            errorMessage(reject.error, 'Could not reject request.')
          }
        />
        {requestsQuery.data?.items.length ? (
          <ul className="divide-y divide-line/70">
            {requestsQuery.data.items.map((req) => (
              <li key={req.id} className="flex items-center justify-between gap-4 py-3 text-sm">
                <div className="min-w-0">
                  <p className="truncate font-medium">{req.user_email}</p>
                  <p className="text-muted">wants to join {req.team_name}</p>
                </div>
                <div className="flex shrink-0 gap-2">
                  <Button
                    size="sm"
                    disabled={approve.isPending}
                    onClick={() => approve.mutate({ teamId: req.team_id, reqId: req.id })}
                  >
                    Approve
                  </Button>
                  <Button
                    size="sm"
                    variant="secondary"
                    disabled={reject.isPending}
                    onClick={() => reject.mutate({ teamId: req.team_id, reqId: req.id })}
                  >
                    Reject
                  </Button>
                </div>
              </li>
            ))}
          </ul>
        ) : (
          <Muted>No pending requests.</Muted>
        )}
      </div>
    </Card>
  );
}
