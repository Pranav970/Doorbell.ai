import type { Transition, Variants } from 'framer-motion';

/**
 * Shared motion vocabulary for the landing page, so every section eases the
 * same way instead of each component inventing its own timing.
 *
 * Everything here is authored as "distance + fade". `useReducedMotion` in the
 * consuming components collapses the distance to 0 rather than disabling the
 * animation outright — the content still appears, it just stops moving, which
 * is what the OS-level preference actually asks for.
 */

export const EASE_OUT: Transition['ease'] = [0.16, 1, 0.3, 1];

/** Parent of a staggered group. Children inherit `initial`/`animate` from it. */
export const staggerParent = (stagger = 0.1, delayChildren = 0): Variants => ({
  hidden: {},
  visible: { transition: { staggerChildren: stagger, delayChildren } },
});

/** Fade + rise. `distance` is collapsed to 0 when reduced motion is on. */
export const fadeUp = (distance = 20, duration = 0.6): Variants => ({
  hidden: { opacity: 0, y: distance },
  visible: { opacity: 1, y: 0, transition: { duration, ease: EASE_OUT } },
});

export const fadeIn = (duration = 0.5): Variants => ({
  hidden: { opacity: 0 },
  visible: { opacity: 1, transition: { duration, ease: EASE_OUT } },
});

/** Spring used by the modal card, per spec: stiffness 200, damping 20. */
export const modalSpring: Transition = { type: 'spring', stiffness: 200, damping: 20 };

/**
 * Scroll-triggered reveal, applied with `whileInView`. `once: true` so a
 * section doesn't re-animate every time it re-enters the viewport, and a
 * negative bottom margin so it fires slightly before the element is flush
 * with the fold.
 */
export const inViewOnce = { once: true, margin: '0px 0px -80px 0px' } as const;
