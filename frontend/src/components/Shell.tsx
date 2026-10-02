import type { ReactNode } from 'react';
import { useNavigate } from 'react-router-dom';
import { ThemeToggle } from './ThemeToggle';
import { Icon, type IconName } from './icons';
import { useAuth } from '../context/AuthContext';

export function Logo({ className }: { className?: string }) {
  return (
    <>
      <img src="/brand/cloud-in-logo.png" alt="Cloud.in" className={`dark:hidden ${className ?? ''}`} />
      <img src="/brand/cloud-in-logo-white.png" alt="Cloud.in" className={`hidden dark:block ${className ?? ''}`} />
    </>
  );
}

export interface NavItem {
  key: string;
  label: string;
  icon: IconName;
  /** When set, the item navigates to this route instead of calling
   *  `onNavChange`. Items without it keep the existing tab behaviour. */
  href?: string;
}

export function Shell({
  navItems,
  activeKey,
  onNavChange,
  subtitle,
  headerExtra,
  children,
}: {
  navItems: NavItem[];
  activeKey: string;
  onNavChange: (key: string) => void;
  subtitle?: string;
  headerExtra?: ReactNode;
  children: ReactNode;
}) {
  const { me, logout } = useAuth();
  const navigate = useNavigate();

  return (
    <div className="flex min-h-screen bg-ink text-fg">
      {/* Labels collapse below lg so the sidebar stays usable on a narrow viewport. */}
      <aside className="flex w-16 shrink-0 flex-col border-r border-line bg-ink lg:w-60">
        <div className="flex h-16 items-center justify-center border-b border-line px-4 lg:justify-start lg:px-5">
          <Logo className="hidden h-7 w-auto lg:block" />
          <span className="text-lg font-extrabold tracking-tight text-blue-500 lg:hidden">P</span>
        </div>

        <nav className="flex-1 space-y-1 p-2 lg:p-3">
          {navItems.map((item) => {
            const active = activeKey === item.key;
            return (
              <button
                key={item.key}
                type="button"
                onClick={() => (item.href ? navigate(item.href) : onNavChange(item.key))}
                aria-current={active ? 'page' : undefined}
                title={item.label}
                className={`relative flex w-full items-center justify-center gap-3 rounded-lg px-3 py-2 text-left text-sm font-medium outline-none transition focus-visible:ring-2 focus-visible:ring-blue-500/50 lg:justify-start ${
                  active
                    ? 'bg-blue-600/10 text-blue-400'
                    : 'text-muted hover:bg-blue-500/5 hover:text-fg'
                }`}
              >
                {active && (
                  <span aria-hidden className="absolute left-0 top-1/2 h-5 w-0.5 -translate-y-1/2 rounded-r bg-blue-500" />
                )}
                <Icon name={item.icon} className="h-4 w-4 shrink-0" />
                <span className="hidden lg:inline">{item.label}</span>
              </button>
            );
          })}
        </nav>

        {me && (
          <div className="border-t border-line p-2 lg:p-3">
            <p className="mb-2 hidden truncate px-2 text-xs text-muted lg:block" title={me.email}>
              {me.email}
            </p>
            <button
              type="button"
              onClick={() => logout()}
              className="flex w-full items-center justify-center gap-2 rounded-lg px-3 py-2 text-sm text-muted outline-none transition hover:bg-blue-500/5 hover:text-blue-400 focus-visible:ring-2 focus-visible:ring-blue-500/50 lg:justify-start"
            >
              <Icon name="logout" className="h-4 w-4 shrink-0" />
              <span className="hidden lg:inline">Log out</span>
            </button>
          </div>
        )}
      </aside>

      <div className="flex min-w-0 flex-1 flex-col">
        <header className="flex h-16 items-center justify-between gap-4 border-b border-line px-6">
          <p className="truncate text-sm text-muted">{subtitle}</p>
          <div className="flex shrink-0 items-center gap-3">
            {headerExtra}
            <ThemeToggle />
          </div>
        </header>
        <main className="flex-1 px-6 py-8">
          <div className="mx-auto max-w-5xl">{children}</div>
        </main>
      </div>
    </div>
  );
}

export function AuthShell({
  title,
  subtitle,
  children,
  footer,
}: {
  title: string;
  subtitle: string;
  children: ReactNode;
  footer?: ReactNode;
}) {
  return (
    <div className="relative flex min-h-screen items-center justify-center overflow-hidden bg-ink px-4">
      {/* Two blue glows on black — the whole decorative budget of the palette. */}
      <div
        aria-hidden
        className="pointer-events-none absolute inset-0 bg-[radial-gradient(ellipse_60%_45%_at_20%_-5%,rgba(37,99,235,0.22),transparent),radial-gradient(ellipse_60%_45%_at_100%_10%,rgba(59,130,246,0.16),transparent)]"
      />
      <div className="absolute right-6 top-6">
        <ThemeToggle />
      </div>
      <div className="relative w-full max-w-sm">
        <div className="mb-6 flex flex-col items-center">
          <Logo className="mb-5 h-9 w-auto" />
          <h1 className="text-center text-2xl font-semibold tracking-tight text-fg">{title}</h1>
          <p className="mt-1.5 text-center text-sm text-muted">{subtitle}</p>
        </div>
        {children}
        {footer}
      </div>
    </div>
  );
}
