import { useEffect, useRef } from 'react';
import { AnimatePresence, motion, useReducedMotion } from 'framer-motion';
import { Link } from 'react-router-dom';
import { modalSpring } from './motion';

/**
 * The "Get started" dialog.
 *
 * Backdrop fades (0.2s), card springs in (0.95 -> 1, stiffness 200 / damping
 * 20) per spec. The accessibility work around it is the part that matters and
 * is easy to skip: focus moves in on open and returns to the trigger on
 * close, Tab is trapped inside, Escape closes, the page behind is inert to
 * scroll, and the backdrop is a sibling rather than a parent so a click on
 * the card can't bubble out and dismiss it.
 */
export function GetStartedModal({ open, onClose }: { open: boolean; onClose: () => void }) {
  const reduce = useReducedMotion();
  const panelRef = useRef<HTMLDivElement>(null);
  const restoreFocusTo = useRef<HTMLElement | null>(null);

  useEffect(() => {
    if (!open) return;

    restoreFocusTo.current = document.activeElement as HTMLElement | null;
    // Defer so the panel exists before we reach into it.
    const focusTimer = window.setTimeout(() => {
      panelRef.current?.querySelector<HTMLElement>('[data-autofocus]')?.focus();
    }, 0);

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
        return;
      }
      if (e.key !== 'Tab' || !panelRef.current) return;

      const focusable = panelRef.current.querySelectorAll<HTMLElement>(
        'a[href], button:not([disabled]), input, select, textarea, [tabindex]:not([tabindex="-1"])',
      );
      if (focusable.length === 0) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    };

    document.addEventListener('keydown', onKeyDown);
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';

    return () => {
      window.clearTimeout(focusTimer);
      document.removeEventListener('keydown', onKeyDown);
      document.body.style.overflow = previousOverflow;
      restoreFocusTo.current?.focus();
    };
  }, [open, onClose]);

  return (
    <AnimatePresence>
      {open && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
          <motion.div
            className="absolute inset-0 bg-black/70 backdrop-blur-sm"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.2 }}
            onClick={onClose}
            aria-hidden
          />
          <motion.div
            ref={panelRef}
            role="dialog"
            aria-modal="true"
            aria-labelledby="get-started-title"
            className="relative w-full max-w-md overflow-hidden rounded-2xl border border-line bg-surface p-6 shadow-2xl shadow-blue-950/40"
            initial={reduce ? { opacity: 0 } : { opacity: 0, scale: 0.95 }}
            animate={reduce ? { opacity: 1 } : { opacity: 1, scale: 1 }}
            exit={reduce ? { opacity: 0 } : { opacity: 0, scale: 0.95 }}
            transition={reduce ? { duration: 0.15 } : modalSpring}
          >
            <div
              aria-hidden
              className="pointer-events-none absolute -right-20 -top-24 h-56 w-56 rounded-full bg-blue-500/15 blur-3xl"
            />
            <div className="relative">
              <h2 id="get-started-title" className="text-xl font-semibold tracking-tight text-fg">
                Start routing in about five minutes
              </h2>
              <p className="mt-2 text-sm leading-relaxed text-muted">
                Create an organization, paste in the provider keys you already have, and get one unified key back. No
                card, no sales call — your credentials never leave your own accounts.
              </p>

              <ol className="mt-5 space-y-3">
                {[
                  'Create your org and team',
                  'Add the provider keys you already own',
                  'Call one endpoint with your unified key',
                ].map((stepText, i) => (
                  <li key={stepText} className="flex items-start gap-3 text-sm text-muted">
                    <span className="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-blue-600/15 font-mono text-[11px] text-blue-400">
                      {i + 1}
                    </span>
                    {stepText}
                  </li>
                ))}
              </ol>

              <div className="mt-6 flex flex-col gap-2 sm:flex-row">
                <Link
                  to="/signup"
                  data-autofocus
                  className="flex-1 rounded-full bg-blue-600 px-5 py-2.5 text-center text-sm font-medium text-white outline-none transition hover:bg-blue-500 focus-visible:ring-2 focus-visible:ring-blue-500/50"
                >
                  Create an account
                </Link>
                <Link
                  to="/login"
                  className="flex-1 rounded-full border border-line px-5 py-2.5 text-center text-sm font-medium text-muted outline-none transition hover:border-blue-500/40 hover:text-fg focus-visible:ring-2 focus-visible:ring-blue-500/50"
                >
                  I already have one
                </Link>
              </div>

              <button
                type="button"
                onClick={onClose}
                aria-label="Close"
                className="absolute -right-1 -top-1 rounded-lg p-1.5 text-muted outline-none transition hover:text-fg focus-visible:ring-2 focus-visible:ring-blue-500/50"
              >
                <svg viewBox="0 0 24 24" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth={2}>
                  <path d="M18 6 6 18M6 6l12 12" strokeLinecap="round" />
                </svg>
              </button>
            </div>
          </motion.div>
        </div>
      )}
    </AnimatePresence>
  );
}
