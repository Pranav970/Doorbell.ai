import type { ReactNode } from 'react';

export function Card({
  title,
  actions,
  children,
  className = '',
}: {
  title?: ReactNode;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section className={`rounded-xl border border-line bg-surface p-5 ${className}`}>
      {(title || actions) && (
        <header className="mb-4 flex items-center justify-between gap-3">
          {title && <h2 className="text-sm font-semibold tracking-tight text-fg">{title}</h2>}
          {actions}
        </header>
      )}
      {children}
    </section>
  );
}

/** Inline form/mutation error. Renders nothing when there's no message. */
export function ErrorText({ message }: { message: string | null }) {
  if (!message) return null;
  return (
    <p role="alert" className="rounded-lg border border-red-500/30 bg-red-500/10 px-3 py-2 text-sm text-red-400">
      {message}
    </p>
  );
}

/** Blue-tinted informational panel — the one-time key reveal, fallback notices. */
export function Callout({ title, children }: { title?: ReactNode; children: ReactNode }) {
  return (
    <div className="rounded-lg border border-blue-500/30 bg-blue-500/10 p-3 text-sm">
      {title && <p className="mb-1 font-medium text-blue-300">{title}</p>}
      {children}
    </div>
  );
}

export function Badge({
  children,
  tone = 'neutral',
}: {
  children: ReactNode;
  tone?: 'neutral' | 'accent' | 'danger';
}) {
  const tones = {
    neutral: 'border-line text-muted',
    accent: 'border-blue-500/40 bg-blue-500/10 text-blue-400',
    danger: 'border-red-500/40 bg-red-500/10 text-red-400',
  };
  return (
    <span
      className={`inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-xs font-medium ${tones[tone]}`}
    >
      {children}
    </span>
  );
}

export function Muted({ children }: { children: ReactNode }) {
  return <p className="text-sm text-muted">{children}</p>;
}

/** Clickable metric tile — the dashboard grids on both Home tabs. */
export function StatCard({
  label,
  value,
  onClick,
}: {
  label: string;
  value: string | number;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="group rounded-xl border border-line bg-surface p-4 text-left outline-none transition hover:border-blue-600/60 focus-visible:ring-2 focus-visible:ring-blue-500/50"
    >
      <p className="text-2xl font-semibold tracking-tight text-fg transition group-hover:text-blue-400">{value}</p>
      <p className="mt-1 text-sm text-muted">{label}</p>
    </button>
  );
}
