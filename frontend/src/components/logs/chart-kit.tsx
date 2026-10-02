import type { ReactNode } from 'react';

/**
 * Shared chrome for the analysis cards — card shell, skeleton, empty state and
 * tooltip. Kept in one file so the three charts stay purely about their data.
 *
 * Colours come from the existing semantic tokens (`surface`, `line`, `muted`),
 * which in the default dark theme resolve to exactly the requested palette:
 * `ink` = #000, `surface` = zinc-900, `line` = zinc-800. Using the tokens
 * rather than literal `bg-zinc-900` keeps the page correct in light mode too
 * and leaves the one-file repalette in `index.css` intact.
 */

export function ChartCard({
  title,
  hint,
  action,
  spanFull,
  className,
  children,
}: {
  title: string;
  hint?: string;
  action?: ReactNode;
  /**
   * Stretch across every column of the parent grid, at any breakpoint.
   *
   * `gridColumn: '1 / -1'` rather than `lg:col-span-3` because it tracks the
   * actual column count instead of hardcoding it — so the card stays
   * full-width in the 2-column `md` layout too, and keeps working if the grid
   * ever gains or loses a column.
   */
  spanFull?: boolean;
  className?: string;
  children: ReactNode;
}) {
  return (
    <section
      style={spanFull ? { gridColumn: '1 / -1' } : undefined}
      className={`group relative overflow-hidden rounded-2xl border border-line bg-surface/70 p-5 backdrop-blur-sm transition duration-300 hover:border-blue-500/40 ${className ?? ''}`}
    >
      {/* A single soft blue bloom, brightening on hover — the whole decorative
          budget of the card. */}
      <div
        aria-hidden
        className="pointer-events-none absolute -right-16 -top-20 h-48 w-48 rounded-full bg-blue-500/10 blur-3xl transition duration-500 group-hover:bg-blue-500/20"
      />
      <div className="relative mb-4 flex flex-wrap items-start justify-between gap-x-3 gap-y-1">
        <div className="min-w-0">
          <h2 className="text-sm font-medium tracking-tight text-fg">{title}</h2>
          {hint && <p className="mt-0.5 text-xs font-light text-muted">{hint}</p>}
        </div>
        {action && <div className="shrink-0">{action}</div>}
      </div>
      <div className="relative">{children}</div>
    </section>
  );
}

/** Pulsing blue skeleton, sized to the chart it stands in for. */
export function ChartSkeleton({ height = 240 }: { height?: number }) {
  return (
    <div className="animate-pulse space-y-3" style={{ height }} aria-hidden>
      <div className="h-full w-full rounded-xl bg-gradient-to-br from-blue-500/10 via-blue-500/5 to-transparent" />
    </div>
  );
}

export function ChartEmpty({ message, height = 240 }: { message: string; height?: number }) {
  return (
    <div className="flex items-center justify-center text-center" style={{ height }}>
      <p className="max-w-xs text-xs font-light leading-relaxed text-muted">{message}</p>
    </div>
  );
}

export interface TooltipRow {
  label: string;
  value: string;
  color?: string;
}

/** Sleek dark tooltip shared by all three charts. */
export function TooltipShell({ title, rows }: { title: string; rows: TooltipRow[] }) {
  return (
    <div className="rounded-xl border border-line bg-raised/95 px-3 py-2 shadow-xl shadow-black/40 backdrop-blur-sm">
      <p className="mb-1 text-[11px] font-medium tracking-tight text-fg">{title}</p>
      {rows.map((row) => (
        <p key={row.label} className="flex items-center gap-2 text-[11px] font-light text-muted">
          {row.color && (
            <span aria-hidden className="h-1.5 w-1.5 rounded-full" style={{ backgroundColor: row.color }} />
          )}
          <span>{row.label}</span>
          <span className="ml-auto font-mono text-blue-300">{row.value}</span>
        </p>
      ))}
    </div>
  );
}
