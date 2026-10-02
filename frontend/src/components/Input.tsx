import type {
  InputHTMLAttributes,
  ReactNode,
  SelectHTMLAttributes,
  TextareaHTMLAttributes,
} from 'react';

// Shared control skin — one string so an input, a select and a textarea can
// never drift apart visually.
const CONTROL =
  'w-full rounded-lg border border-line bg-raised px-3 py-2 text-sm text-fg placeholder:text-muted/70 ' +
  'outline-none transition focus:border-blue-500 focus:ring-2 focus:ring-blue-500/30 disabled:opacity-50';

export function Input({ className = '', ...rest }: InputHTMLAttributes<HTMLInputElement>) {
  return <input className={`${CONTROL} ${className}`} {...rest} />;
}

export function Textarea({ className = '', ...rest }: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return <textarea className={`${CONTROL} resize-y ${className}`} {...rest} />;
}

export function Select({ className = '', ...rest }: SelectHTMLAttributes<HTMLSelectElement>) {
  return <select className={`${CONTROL} cursor-pointer ${className}`} {...rest} />;
}

/** Label + control + optional hint. Wraps in a `<label>`, so no id plumbing. */
export function Field({
  label,
  hint,
  children,
  className = '',
}: {
  label: ReactNode;
  hint?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <label className={`flex flex-col gap-1.5 ${className}`}>
      <span className="text-xs font-medium uppercase tracking-wider text-muted">{label}</span>
      {children}
      {hint && <span className="text-xs leading-relaxed text-muted">{hint}</span>}
    </label>
  );
}
