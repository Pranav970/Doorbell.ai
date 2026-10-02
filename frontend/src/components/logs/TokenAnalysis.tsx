import { useMemo } from 'react';
import { Area, AreaChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts';
import type { RequestLogEntry } from '../../lib/types';
import { ChartCard, ChartEmpty, ChartSkeleton, TooltipShell } from './chart-kit';
import { AXIS_PROPS, compactNumber } from './chart-utils';

interface Bucket {
  /** Bucket start, ms since epoch — numeric so the axis stays ordered. */
  t: number;
  label: string;
  prompt: number;
  completion: number;
}

/**
 * Buckets by day, or by hour when every log falls inside a single day — a
 * dev/demo dataset otherwise collapses into one meaningless column.
 */
function toBuckets(logs: RequestLogEntry[]): Bucket[] {
  if (logs.length === 0) return [];

  const times = logs.map((l) => new Date(l.created_at).getTime()).filter((t) => Number.isFinite(t));
  if (times.length === 0) return [];

  const spanMs = Math.max(...times) - Math.min(...times);
  const byHour = spanMs < 36 * 60 * 60 * 1000;
  const sizeMs = byHour ? 60 * 60 * 1000 : 24 * 60 * 60 * 1000;

  const buckets = new Map<number, Bucket>();
  for (const log of logs) {
    const ms = new Date(log.created_at).getTime();
    if (!Number.isFinite(ms)) continue;
    const t = Math.floor(ms / sizeMs) * sizeMs;
    const bucket =
      buckets.get(t) ??
      {
        t,
        label: byHour
          ? new Date(t).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
          : new Date(t).toLocaleDateString([], { month: 'short', day: 'numeric' }),
        prompt: 0,
        completion: 0,
      };
    bucket.prompt += log.tokens_in ?? 0;
    bucket.completion += log.tokens_out ?? 0;
    buckets.set(t, bucket);
  }
  return [...buckets.values()].sort((a, b) => a.t - b.t);
}

export function TokenAnalysis({ logs, isLoading }: { logs: RequestLogEntry[]; isLoading: boolean }) {
  const buckets = useMemo(() => toBuckets(logs), [logs]);
  const totals = useMemo(
    () =>
      buckets.reduce(
        (acc, b) => ({ prompt: acc.prompt + b.prompt, completion: acc.completion + b.completion }),
        { prompt: 0, completion: 0 },
      ),
    [buckets],
  );

  const legend = (
    <div className="flex items-center gap-4 text-[11px] font-light text-muted">
      <span className="flex items-center gap-1.5">
        <span aria-hidden className="h-1.5 w-1.5 rounded-full bg-blue-300" />
        Prompt {compactNumber(totals.prompt)}
      </span>
      <span className="flex items-center gap-1.5">
        <span aria-hidden className="h-1.5 w-1.5 rounded-full bg-blue-600" />
        Completion {compactNumber(totals.completion)}
      </span>
    </div>
  );

  return (
    <ChartCard
      title="Token consumption"
      hint="Prompt vs completion tokens over time"
      action={buckets.length > 0 ? legend : undefined}
      spanFull
    >
      {isLoading ? (
        <ChartSkeleton height={260} />
      ) : buckets.length === 0 ? (
        <ChartEmpty message="No token usage recorded yet." height={260} />
      ) : (
        <div className="w-full" style={{ height: 260 }}>
          <ResponsiveContainer width="100%" height="100%">
            <AreaChart data={buckets} margin={{ top: 8, right: 8, bottom: 0, left: 0 }}>
              <defs>
                <linearGradient id="tok-prompt" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor="#93c5fd" stopOpacity={0.45} />
                  <stop offset="100%" stopColor="#93c5fd" stopOpacity={0} />
                </linearGradient>
                <linearGradient id="tok-completion" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor="#2563eb" stopOpacity={0.5} />
                  <stop offset="100%" stopColor="#2563eb" stopOpacity={0} />
                </linearGradient>
              </defs>
              <XAxis dataKey="label" {...AXIS_PROPS} minTickGap={24} />
              <YAxis {...AXIS_PROPS} width={44} tickFormatter={compactNumber} />
              <Tooltip
                cursor={{ stroke: '#3b82f6', strokeOpacity: 0.35, strokeWidth: 1 }}
                content={({ active, payload }) => {
                  if (!active || !payload?.length) return null;
                  const bucket = payload[0].payload as Bucket;
                  return (
                    <TooltipShell
                      title={bucket.label}
                      rows={[
                        { label: 'Prompt', value: compactNumber(bucket.prompt), color: '#93c5fd' },
                        { label: 'Completion', value: compactNumber(bucket.completion), color: '#2563eb' },
                        { label: 'Total', value: compactNumber(bucket.prompt + bucket.completion) },
                      ]}
                    />
                  );
                }}
              />
              <Area
                type="monotone"
                dataKey="prompt"
                stackId="tokens"
                stroke="#93c5fd"
                strokeWidth={1.5}
                fill="url(#tok-prompt)"
                animationDuration={800}
                activeDot={{
                  r: 4,
                  fill: '#93c5fd',
                  stroke: 'none',
                  className: 'drop-shadow-[0_0_8px_rgba(147,197,253,0.8)]',
                }}
              />
              <Area
                type="monotone"
                dataKey="completion"
                stackId="tokens"
                stroke="#3b82f6"
                strokeWidth={1.5}
                fill="url(#tok-completion)"
                animationDuration={800}
                activeDot={{
                  r: 4,
                  fill: '#3b82f6',
                  stroke: 'none',
                  className: 'drop-shadow-[0_0_8px_rgba(59,130,246,0.8)]',
                }}
              />
            </AreaChart>
          </ResponsiveContainer>
        </div>
      )}
    </ChartCard>
  );
}
