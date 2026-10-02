import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { api, ApiError } from '../lib/api';
import type { KeyUsage } from '../lib/types';
import { Badge, ErrorText, Muted } from './Card';
import { HoverCard } from './HoverCard';
import { Table, Td, Tr } from './Table';
import { Button } from './Button';
import { Icon } from './icons';

const PAGE_SIZE = 10;

/** Org scope rolls up every team (admin console); team scope is one team. */
export type UsageScope = { org: string } | { team: string };

const num = (n: number) => n.toLocaleString();

function UsageDetail({ row }: { row: KeyUsage }) {
  const total = row.tokens_in + row.tokens_out;
  const errorRate = row.request_count ? (row.error_count / row.request_count) * 100 : 0;
  const avgTokens = row.request_count ? Math.round(total / row.request_count) : 0;
  return (
    <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-xs">
      <dt className="font-semibold text-fg">Model</dt>
      <dd className="font-mono text-muted">{row.model}</dd>
      <dt className="font-semibold text-fg">Provider</dt>
      <dd className="text-muted">{row.provider}</dd>
      <dt className="font-semibold text-fg">Tokens in</dt>
      <dd className="text-muted">{num(row.tokens_in)}</dd>
      <dt className="font-semibold text-fg">Tokens out</dt>
      <dd className="text-muted">{num(row.tokens_out)}</dd>
      <dt className="font-semibold text-fg">Total tokens</dt>
      <dd className="text-blue-400">{num(total)}</dd>
      <dt className="font-semibold text-fg">Avg / request</dt>
      <dd className="text-muted">{num(avgTokens)}</dd>
      <dt className="font-semibold text-fg">Error rate</dt>
      <dd className={errorRate > 0 ? 'text-red-400' : 'text-muted'}>{errorRate.toFixed(1)}%</dd>
      <dt className="font-semibold text-fg">Last used</dt>
      <dd className="text-muted">{row.last_used_at ? new Date(row.last_used_at).toLocaleString() : 'never'}</dd>
    </dl>
  );
}

/**
 * Usage rolled up by (team, provider, model). Fetches its own data so both
 * callers just hand it a scope, and paginates client-side — the analytics
 * endpoints return the full rollup in one response, so there's nothing to
 * page over on the server yet.
 */
export function UsageTable({ scope }: { scope: UsageScope }) {
  const orgId = 'org' in scope ? scope.org : null;
  const teamId = 'team' in scope ? scope.team : null;
  const [page, setPage] = useState(0);

  const usageQuery = useQuery({
    queryKey: ['analytics', orgId ? 'org' : 'team', orgId ?? teamId],
    queryFn: () => (orgId ? api.orgAnalytics(orgId) : api.teamAnalytics(teamId as string)),
    enabled: !!(orgId ?? teamId),
  });

  // Org scope shows a Team column; the names aren't on the analytics rows.
  const teamsQuery = useQuery({
    queryKey: ['teams', orgId],
    queryFn: () => api.listTeamsForOrg(orgId as string),
    enabled: !!orgId,
  });
  const teamName = (id: string) => teamsQuery.data?.items.find((t) => t.id === id)?.name ?? id;

  if (usageQuery.isPending) return <Muted>Loading usage…</Muted>;
  if (usageQuery.isError) {
    return (
      <ErrorText
        message={usageQuery.error instanceof ApiError ? usageQuery.error.message : 'Could not load usage.'}
      />
    );
  }

  const items = usageQuery.data.items;
  if (items.length === 0) return <Muted>No usage recorded yet.</Muted>;

  const pageCount = Math.ceil(items.length / PAGE_SIZE);
  const current = Math.min(page, pageCount - 1); // clamped, so a shrinking result set needs no effect
  const rows = items.slice(current * PAGE_SIZE, current * PAGE_SIZE + PAGE_SIZE);

  const columns = [
    ...(orgId ? ['Team'] : []),
    'Provider',
    'Model',
    'Requests',
    'Errors',
    'Tokens in / out',
    'Last used',
  ];

  return (
    <div className="flex flex-col gap-4">
      <Table columns={columns}>
        {rows.map((row) => (
          <Tr key={`${row.team_id}-${row.provider}-${row.model}`}>
            {orgId && <Td className="text-muted">{teamName(row.team_id)}</Td>}
            <Td>{row.provider}</Td>
            <Td>
              <HoverCard content={<UsageDetail row={row} />}>
                <span className="inline-flex items-center gap-1.5 font-mono text-blue-400 underline decoration-blue-500/30 decoration-dotted underline-offset-4">
                  {row.model}
                  <Icon name="info" className="h-3.5 w-3.5 opacity-60" />
                </span>
              </HoverCard>
            </Td>
            <Td>{num(row.request_count)}</Td>
            <Td>
              {row.error_count > 0 ? <Badge tone="danger">{num(row.error_count)}</Badge> : <span className="text-muted">0</span>}
            </Td>
            <Td className="text-muted">
              {num(row.tokens_in)} / {num(row.tokens_out)}
            </Td>
            <Td className="text-muted">{row.last_used_at ? new Date(row.last_used_at).toLocaleString() : '—'}</Td>
          </Tr>
        ))}
      </Table>

      {pageCount > 1 && (
        <div className="flex items-center justify-between">
          <p className="text-xs text-muted">
            {current * PAGE_SIZE + 1}–{current * PAGE_SIZE + rows.length} of {items.length}
          </p>
          <div className="flex items-center gap-2">
            <Button
              variant="secondary"
              size="sm"
              disabled={current === 0}
              onClick={() => setPage(current - 1)}
              aria-label="Previous page"
            >
              <Icon name="chevronLeft" className="h-3.5 w-3.5" />
              Prev
            </Button>
            <span className="text-xs text-muted">
              {current + 1} / {pageCount}
            </span>
            <Button
              variant="secondary"
              size="sm"
              disabled={current >= pageCount - 1}
              onClick={() => setPage(current + 1)}
              aria-label="Next page"
            >
              Next
              <Icon name="chevronRight" className="h-3.5 w-3.5" />
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
