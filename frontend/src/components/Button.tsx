import type { ButtonHTMLAttributes } from 'react';

export type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger';
export type ButtonSize = 'sm' | 'md';

const BASE =
  'inline-flex items-center justify-center gap-1.5 rounded-lg font-medium whitespace-nowrap transition ' +
  'outline-none focus-visible:ring-2 focus-visible:ring-blue-500/50 disabled:cursor-not-allowed disabled:opacity-50';

const VARIANTS: Record<ButtonVariant, string> = {
  primary: 'bg-blue-600 text-white shadow-sm shadow-blue-950/40 hover:bg-blue-500 active:bg-blue-600',
  secondary: 'border border-line bg-surface text-fg hover:border-blue-600/60 hover:text-blue-400',
  ghost: 'text-muted hover:bg-blue-600/10 hover:text-blue-400',
  danger: 'border border-red-500/40 text-red-500 hover:bg-red-500/10 dark:text-red-400',
};

const SIZES: Record<ButtonSize, string> = {
  sm: 'px-3 py-1.5 text-xs',
  md: 'px-4 py-2 text-sm',
};

export function Button({
  variant = 'primary',
  size = 'md',
  className = '',
  type = 'button',
  ...rest
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: ButtonVariant; size?: ButtonSize }) {
  return <button type={type} className={`${BASE} ${VARIANTS[variant]} ${SIZES[size]} ${className}`} {...rest} />;
}
