/**
 * Section transitions. Two flavours so consecutive boundaries don't repeat:
 * a hairline gradient rule, and a soft SVG wave that carries a faint blue
 * wash from one section into the next.
 *
 * Both are decorative, so both are `aria-hidden`.
 */

export function GradientDivider() {
  return (
    <div aria-hidden className="mx-auto max-w-6xl px-5 sm:px-8">
      <div className="h-px w-full bg-gradient-to-r from-transparent via-blue-500/30 to-transparent" />
    </div>
  );
}

export function WaveDivider({ flip = false }: { flip?: boolean }) {
  return (
    <div aria-hidden className={`pointer-events-none relative -my-px ${flip ? 'rotate-180' : ''}`}>
      <svg viewBox="0 0 1440 120" preserveAspectRatio="none" className="block h-16 w-full sm:h-24">
        <defs>
          <linearGradient id={`wave-fill-${flip ? 'b' : 'a'}`} x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="#3b82f6" stopOpacity="0.10" />
            <stop offset="100%" stopColor="#3b82f6" stopOpacity="0" />
          </linearGradient>
        </defs>
        <path
          d="M0,64 C240,120 480,8 720,40 C960,72 1200,120 1440,72 L1440,120 L0,120 Z"
          fill={`url(#wave-fill-${flip ? 'b' : 'a'})`}
        />
        <path
          d="M0,64 C240,120 480,8 720,40 C960,72 1200,120 1440,72"
          fill="none"
          stroke="#3b82f6"
          strokeOpacity="0.22"
          strokeWidth="1"
          vectorEffect="non-scaling-stroke"
        />
      </svg>
    </div>
  );
}
