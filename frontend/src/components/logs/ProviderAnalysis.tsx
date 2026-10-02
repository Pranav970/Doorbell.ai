import { useMemo } from 'react';
import { Cell, Pie, PieChart, ResponsiveContainer, Tooltip } from 'recharts';
import type { RequestLogEntry } from '../../lib/types';
import { ChartCard, ChartEmpty, ChartSkeleton, TooltipShell } from './chart-kit';
import { blueAt, compactNumber } from './chart-utils';

interface Slice {
  provider: string;
  requests: number;
  errors: number;
}

function toSlices(logs: RequestLogEntry[]): Slice[] {
  const byProvider = new Map<string, Slice>();
  for (const log of logs) {
    const slice = byProvider.get(log.provider) ?? { provider: log.provider, requests: 0, errors: 0 };
    slice.requests += 1;
    if (log.status_code >= 400) slice.errors += 1;
    byProvider.set(log.provider, slice);
  }
  return [...byProvider.values()].sort((a, b) => b.requests - a.requests);
}

export function ProviderAnalysis({ logs, isLoading }: { logs: RequestLogEntry[]; isLoading: boolean }) {
  const slices = useMemo(() => toSlices(logs), [logs]);
  const total = slices.reduce((sum, s) => sum + s.requests, 0);

  return (
    <ChartCard title="Provider usage" hint="Share of requests by provider">
      {isLoading ? (
        <ChartSkeleton />
      ) : slices.length === 0 ? (
        <ChartEmpty message="No requests yet. Send one from the Playground and it will show up here." />
      ) : (
        <div className="flex flex-col items-center gap-4">
          <div className="relative w-full" style={{ height: 200 }}>
            <ResponsiveContainer width="100%" height="100%">
              <PieChart>
                <Pie
                  data={slices}
                  dataKey="requests"
                  nameKey="provider"
                  innerRadius="62%"
                  outerRadius="92%"
                  paddingAngle={3}
                  stroke="none"
                  animationDuration={700}
                >
                  {slices.map((slice, i) => (
                    <Cell
                      key={slice.provider}
                      fill={blueAt(i)}
                      className="origin-center transition-opacity duration-200 hover:opacity-80"
                    />
                  ))}
                </Pie>
                <Tooltip
                  cursor={false}
                  content={({ active, payload }) => {
                    if (!active || !payload?.length) return null;
                    const slice = payload[0].payload as Slice;
                    const pct = total ? ((slice.requests / total) * 100).toFixed(1) : '0';
                    return (
                      <TooltipShell
                        title={slice.provider}
                        rows={[
                          { label: 'Requests', value: compactNumber(slice.requests), color: payload[0].color },
                          { label: 'Share', value: `${pct}%` },
                          ...(slice.errors ? [{ label: 'Errors', value: String(slice.errors) }] : []),
                        ]}
                      />
                    );
                  }}
                />
              </PieChart>
            </ResponsiveContainer>
            {/* Total sits in the donut hole rather than in a legend. */}
            <div className="pointer-events-none absolute inset-0 flex flex-col items-center justify-center">
              <span className="font-mono text-2xl font-light tracking-tight text-blue-300 drop-shadow-[0_0_10px_rgba(59,130,246,0.45)]">
                {compactNumber(total)}
              </span>
              <span className="text-[10px] font-light uppercase tracking-widest text-muted">requests</span>
            </div>
          </div>

          <ul className="w-full space-y-1.5">
            {slices.map((slice, i) => (
              <li key={slice.provider} className="flex items-center gap-2 text-xs">
                <span aria-hidden className="h-2 w-2 shrink-0 rounded-full" style={{ backgroundColor: blueAt(i) }} />
                <span className="truncate font-light text-muted">{slice.provider}</span>
                <span className="ml-auto shrink-0 font-mono text-fg">
                  {total ? Math.round((slice.requests / total) * 100) : 0}%
                </span>
              </li>
            ))}
          </ul>
        </div>
      )}
    </ChartCard>
  );
}
