import { useMemo } from 'react';
import { Bar, BarChart, Cell, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts';
import type { RequestLogEntry } from '../../lib/types';
import { ChartCard, ChartEmpty, ChartSkeleton, TooltipShell } from './chart-kit';
import { AXIS_PROPS, compactNumber } from './chart-utils';

const TOP_N = 6;

interface ModelRow {
  model: string;
  provider: string;
  requests: number;
  tokens: number;
}

function toRows(logs: RequestLogEntry[]): ModelRow[] {
  const byModel = new Map<string, ModelRow>();
  for (const log of logs) {
    const row = byModel.get(log.model) ?? { model: log.model, provider: log.provider, requests: 0, tokens: 0 };
    row.requests += 1;
    row.tokens += (log.tokens_in ?? 0) + (log.tokens_out ?? 0);
    byModel.set(log.model, row);
  }
  return [...byModel.values()].sort((a, b) => b.requests - a.requests).slice(0, TOP_N);
}

export function ModelAnalysis({ logs, isLoading }: { logs: RequestLogEntry[]; isLoading: boolean }) {
  const rows = useMemo(() => toRows(logs), [logs]);
  const max = rows[0]?.requests ?? 0;

  return (
    <ChartCard title="Model usage" hint={`Top ${TOP_N} models by request count`}>
      {isLoading ? (
        <ChartSkeleton />
      ) : rows.length === 0 ? (
        <ChartEmpty message="No models called yet." />
      ) : (
        <div className="w-full" style={{ height: 280 }}>
          <ResponsiveContainer width="100%" height="100%">
            <BarChart data={rows} layout="vertical" margin={{ top: 0, right: 12, bottom: 0, left: 0 }} barSize={14}>
              <defs>
                {/* Left-to-right blue gradient so each bar reads as a beam. */}
                <linearGradient id="model-bar" x1="0" y1="0" x2="1" y2="0">
                  <stop offset="0%" stopColor="#1d4ed8" />
                  <stop offset="100%" stopColor="#60a5fa" />
                </linearGradient>
                <linearGradient id="model-bar-dim" x1="0" y1="0" x2="1" y2="0">
                  <stop offset="0%" stopColor="#1e40af" stopOpacity={0.7} />
                  <stop offset="100%" stopColor="#3b82f6" stopOpacity={0.7} />
                </linearGradient>
              </defs>
              <XAxis type="number" hide />
              <YAxis
                type="category"
                dataKey="model"
                width={138}
                {...AXIS_PROPS}
                tickFormatter={(m: string) => (m.length > 16 ? `${m.slice(0, 15)}…` : m)}
              />
              <Tooltip
                cursor={{ fill: 'rgba(59,130,246,0.06)' }}
                content={({ active, payload }) => {
                  if (!active || !payload?.length) return null;
                  const row = payload[0].payload as ModelRow;
                  return (
                    <TooltipShell
                      title={row.model}
                      rows={[
                        { label: 'Provider', value: row.provider },
                        { label: 'Requests', value: compactNumber(row.requests), color: '#60a5fa' },
                        { label: 'Tokens', value: compactNumber(row.tokens) },
                      ]}
                    />
                  );
                }}
              />
              <Bar dataKey="requests" radius={[0, 7, 7, 0]} animationDuration={700}>
                {rows.map((row) => (
                  <Cell
                    key={row.model}
                    // The leader glows; the rest stay dimmer so rank reads at a glance.
                    fill={row.requests === max ? 'url(#model-bar)' : 'url(#model-bar-dim)'}
                    className={
                      row.requests === max
                        ? 'drop-shadow-[0_0_8px_rgba(59,130,246,0.5)] transition-opacity duration-200 hover:opacity-90'
                        : 'transition-opacity duration-200 hover:opacity-90'
                    }
                  />
                ))}
              </Bar>
            </BarChart>
          </ResponsiveContainer>
        </div>
      )}
    </ChartCard>
  );
}
