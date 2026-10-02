import { useCallback, useEffect, useId, useLayoutEffect, useRef, useState, type ReactNode } from 'react';
import { createPortal } from 'react-dom';

const CARD_WIDTH = 288; // matches w-72 below
const GAP = 8;
const VIEWPORT_MARGIN = 12;

/**
 * A hover/focus popover. Rendered through a portal with `position: fixed`
 * rather than absolutely inside its trigger, because the callers that need
 * it sit in `overflow-x-auto` containers (the logs table) which would
 * otherwise clip it.
 *
 * Opens on hover *and* keyboard focus, and closes on Escape, so it isn't
 * mouse-only. Flips above the trigger when there isn't room below and
 * clamps to the viewport horizontally.
 */
export function HoverCard({ children, content }: { children: ReactNode; content: ReactNode }) {
  const triggerRef = useRef<HTMLSpanElement>(null);
  const cardRef = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [pos, setPos] = useState<{ top: number; left: number } | null>(null);
  const cardId = useId();

  const place = useCallback(() => {
    const trigger = triggerRef.current?.getBoundingClientRect();
    if (!trigger) return;
    // Measured once rendered; the first pass paints off-screen (see style
    // below) so this never flickers at the wrong spot.
    const height = cardRef.current?.offsetHeight ?? 0;
    const below = trigger.bottom + GAP;
    const fitsBelow = below + height <= window.innerHeight - VIEWPORT_MARGIN;
    setPos({
      top: fitsBelow ? below : Math.max(VIEWPORT_MARGIN, trigger.top - GAP - height),
      left: Math.min(
        Math.max(VIEWPORT_MARGIN, trigger.left),
        Math.max(VIEWPORT_MARGIN, window.innerWidth - CARD_WIDTH - VIEWPORT_MARGIN),
      ),
    });
  }, []);

  useLayoutEffect(() => {
    if (!open) {
      setPos(null);
      return;
    }
    place();
  }, [open, place]);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setOpen(false);
    };
    const reposition = () => place();
    window.addEventListener('keydown', onKey);
    window.addEventListener('scroll', reposition, true);
    window.addEventListener('resize', reposition);
    return () => {
      window.removeEventListener('keydown', onKey);
      window.removeEventListener('scroll', reposition, true);
      window.removeEventListener('resize', reposition);
    };
  }, [open, place]);

  return (
    <>
      <span
        ref={triggerRef}
        tabIndex={0}
        aria-describedby={open ? cardId : undefined}
        onMouseEnter={() => setOpen(true)}
        onMouseLeave={() => setOpen(false)}
        onFocus={() => setOpen(true)}
        onBlur={() => setOpen(false)}
        className="inline-flex cursor-help rounded-full outline-none focus-visible:ring-2 focus-visible:ring-blue-500/50"
      >
        {children}
      </span>
      {open &&
        createPortal(
          <div
            ref={cardRef}
            id={cardId}
            role="tooltip"
            style={{
              top: pos?.top ?? -9999,
              left: pos?.left ?? -9999,
              width: CARD_WIDTH,
              opacity: pos ? 1 : 0,
            }}
            className="pointer-events-none fixed z-50 rounded-xl border border-line bg-surface p-3 text-left text-fg shadow-xl shadow-black/40 ring-1 ring-blue-500/10 transition-opacity"
          >
            {content}
          </div>,
          document.body,
        )}
    </>
  );
}
