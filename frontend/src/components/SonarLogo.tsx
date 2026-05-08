import type { CSSProperties } from 'react';

type SonarProps = {
  size?: number;
  ink?: string;
  accent?: string;
  dot?: string;
  /** When true, arcs draw in on mount (700ms). Off by default — arcs draw
   *  at most once per route to avoid distraction on repeat renders. */
  animate?: boolean;
  className?: string;
  style?: CSSProperties;
  title?: string;
};

export function SonarLogo({
  size = 36,
  ink = 'var(--ink, #1A1A1A)',
  accent = 'var(--cobalt, #3B5BDB)',
  dot = 'var(--rust, #C44536)',
  animate = false,
  className,
  style,
  title = 'Aegis',
}: SonarProps) {
  const w = size;
  const h = size * 0.75;
  const drawClass = animate ? 'sonar-draw' : undefined;

  return (
    <svg
      width={w}
      height={h}
      viewBox="0 0 120 90"
      fill="none"
      role="img"
      aria-label={title}
      className={className}
      style={style}
    >
      <line x1="14" y1="74" x2="106" y2="74" stroke={ink} strokeWidth="2" />
      <path
        className={drawClass}
        d="M22 74 A 38 38 0 0 1 98 74"
        stroke={ink}
        strokeWidth="2"
        fill="none"
        strokeLinecap="round"
      />
      <path
        className={drawClass}
        d="M32 74 A 28 28 0 0 1 88 74"
        stroke={ink}
        strokeWidth="2"
        fill="none"
        strokeLinecap="round"
        style={animate ? { animationDelay: '120ms' } : undefined}
      />
      <path
        className={drawClass}
        d="M42 74 A 18 18 0 0 1 78 74"
        stroke={accent}
        strokeWidth="3"
        fill="none"
        strokeLinecap="round"
        style={animate ? { animationDelay: '240ms' } : undefined}
      />
      <circle cx="60" cy="74" r="4" fill={dot} />
      <line x1="14" y1="74" x2="14" y2="80" stroke={ink} strokeWidth="1.5" />
      <line x1="106" y1="74" x2="106" y2="80" stroke={ink} strokeWidth="1.5" />
    </svg>
  );
}

type WordProps = {
  size?: number;
  color?: string;
  accent?: string;
  italic?: boolean;
  className?: string;
};

/** "Aeg𝑖s" — Fraunces with the `i` swapped to Instrument Serif italic.
 *  Use alongside <SonarLogo /> for the full lockup. */
export function AegisWord({
  size = 28,
  color = 'var(--ink, #1A1A1A)',
  accent = 'var(--rust, #C44536)',
  italic = true,
  className,
}: WordProps) {
  return (
    <span
      className={className}
      style={{
        fontFamily: 'Fraunces, serif',
        fontWeight: 500,
        fontSize: size,
        lineHeight: 1,
        letterSpacing: '-0.025em',
        color,
      }}
    >
      Aeg
      <span
        style={{
          fontFamily: italic ? 'Instrument Serif, serif' : 'inherit',
          fontStyle: italic ? 'italic' : 'normal',
          color: accent,
        }}
      >
        i
      </span>
      s
    </span>
  );
}

type LockupProps = {
  size?: number;
  /** Inverted = white on ink for use on dark surfaces / pill buttons. */
  inverted?: boolean;
  className?: string;
};

export function AegisLockup({ size = 28, inverted = false, className }: LockupProps) {
  const ink = inverted ? 'var(--paper, #F6F4EE)' : 'var(--ink, #1A1A1A)';
  const accent = inverted ? 'var(--paper, #F6F4EE)' : 'var(--cobalt, #3B5BDB)';
  const wordAccent = inverted ? 'var(--cobalt, #3B5BDB)' : 'var(--rust, #C44536)';

  return (
    <span
      className={className}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: size * 0.35,
        background: inverted ? 'var(--ink, #1A1A1A)' : 'transparent',
        color: ink,
        padding: inverted ? `${size * 0.45}px ${size * 0.6}px` : 0,
        borderRadius: inverted ? 2 : 0,
      }}
    >
      <SonarLogo size={size * 1.1} ink={ink} accent={accent} dot={wordAccent} />
      <AegisWord size={size} color={ink} accent={wordAccent} />
    </span>
  );
}
