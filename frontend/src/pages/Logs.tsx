import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';
import { approvedTeams, useAuth } from '../context/AuthContext';
import { api, errorMessage } from '../lib/api';
import { Shell, type NavItem } from '../components/Shell';
import { Select } from '../components/Input';
import { ProviderAnalysis } from '../components/logs/ProviderAnalysis';
import { ModelAnalysis } from '../components/logs/ModelAnalysis';
import { TokenAnalysis } from '../components/logs/TokenAnalysis';
import { compactNumber } from '../components/logs/chart-utils';

/**
 * Analysis Window for the Logs menu: provider mix, model mix and token
 * consumption, all derived from one `teamLogs` fetch so the three cards can
 * never disagree with each other.
 *
 * Rendered as its own route rather than a tab inside `MemberPage` so the
 * existing dashboard is untouched; it reuses `Shell` exactly the way the
 * other pages do.
 */

const NAV_ITEMS: NavItem[] = [
  { key: 'back', label: 'Dashboard', icon: 'home' },
  { key: 'analysis', label: 'Log analysis', icon: 'chart' },
];

export function LogsPage() {
  const navigate = useNavigate();
  const { me } = useAuth();
  const teams = approvedTeams(me);
  const [selectedTeamId, setSelectedTeamId] = useState('');
  // Derived rather than defaulted in an effect, so no child ever queries with
  // an empty team id.
  const teamId = selectedTeamId || teams[0]?.team_id || '';

  const logsQuery = useQuery({
    queryKey: ['team-logs', teamId],
    queryFn: () => api.teamLogs(teamId),
    enabled: !!teamId,
  });

  const logs = logsQuery.data?.items ?? [];
  const isLoading = logsQuery.isPending && !!teamId;

  const totalTokens = logs.reduce((sum, l) => sum + (l.tokens_in ?? 0) + (l.tokens_out ?? 0), 0);
  const errorCount = logs.filter((l) => l.status_code >= 400).length;

  const teamSelect =
    teams.length > 0 ? (
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
    ) : undefined;

  return (
    <Shell
      navItems={NAV_ITEMS}
      activeKey="analysis"
      onNavChange={(key) => key === 'back' && navigate('/app')}
      subtitle="Log analysis"
      headerExtra={teamSelect}
    >
      <div className="flex flex-col gap-6">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Analysis</h1>
          <p className="mt-1.5 max-w-2xl text-sm font-light text-muted">
            Where your traffic actually goes — provider mix, the models behind it, and what it costs you in tokens.
          </p>
        </div>

        {teams.length === 0 && (
          <p className="rounded-2xl border border-line bg-surface/70 p-5 text-sm font-light text-muted backdrop-blur-sm">
            You're not an approved member of any team yet, so there's nothing to analyse.
          </p>
        )}

        {logsQuery.isError && (
          <p className="rounded-2xl border border-red-500/30 bg-red-500/5 p-5 text-sm text-red-400">
            {errorMessage(logsQuery.error, 'Could not load logs.')}
          </p>
        )}

        {teams.length > 0 && !logsQuery.isError && (
          <>
            {/* Three headline numbers, so the cards below have context. */}
            <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
              <StatTile label="Requests" value={compactNumber(logs.length)} loading={isLoading} />
              <StatTile label="Tokens" value={compactNumber(totalTokens)} loading={isLoading} />
              <StatTile label="Errors" value={compactNumber(errorCount)} loading={isLoading} tone={errorCount > 0} />
            </div>

            <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
              <ProviderAnalysis logs={logs} isLoading={isLoading} />
              <ModelAnalysis logs={logs} isLoading={isLoading} />
              {/* Timeline reads better full-width — spans every column (see ChartCard.spanFull). */}
              <TokenAnalysis logs={logs} isLoading={isLoading} />
            </div>
          </>
        )}
      </div>
    </Shell>
  );
}

function StatTile({
  label,
  value,
  loading,
  tone,
}: {
  label: string;
  value: string;
  loading: boolean;
  tone?: boolean;
}) {
  return (
    <div className="rounded-2xl border border-line bg-surface/70 px-5 py-4 backdrop-blur-sm transition duration-300 hover:border-blue-500/40">
      <p className="text-[10px] font-light uppercase tracking-widest text-muted">{label}</p>
      {loading ? (
        <div className="mt-2 h-7 w-20 animate-pulse rounded-md bg-blue-500/10" aria-hidden />
      ) : (
        <p
          className={`mt-1 font-mono text-2xl font-light tracking-tight ${
            tone ? 'text-red-400' : 'text-blue-300 drop-shadow-[0_0_10px_rgba(59,130,246,0.35)]'
          }`}
        >
          {value}
        </p>
      )}
    </div>
  );
}
