// Sonar — brand UI primitives. Use these everywhere.
import type { CSSProperties, MouseEventHandler, ReactNode } from 'react';
import { motion, useInView } from 'framer-motion';
import { useEffect, useRef, useState } from 'react';
import { useTicker } from '../hooks';
import { MOCK_PRICES } from '../mockData';

export const tokens = {
  ink: 'var(--ink)',
  ink2: 'var(--ink-2)',
  paper: 'var(--paper)',
  paper2: 'var(--paper-2)',
  sand: 'var(--sand)',
  cobalt: 'var(--cobalt)',
  rust: 'var(--rust)',
  moss: 'var(--moss)',
  line: 'var(--line)',
  lineSoft: 'var(--line-soft)',
  serif: 'var(--serif)',
  serifI: 'var(--serif-italic)',
  sans: 'var(--sans)',
  mono: 'var(--mono)',
};

export function Eyebrow({ children, style }: { children: ReactNode; style?: CSSProperties }) {
  return <div className="eyebrow" style={style}>{children}</div>;
}

export function Card({
  children,
  pad = 20,
  style,
  raised = false,
  className,
  onClick,
}: {
  children: ReactNode;
  pad?: number;
  raised?: boolean;
  style?: CSSProperties;
  className?: string;
  onClick?: MouseEventHandler<HTMLDivElement>;
}) {
  return (
    <div
      className={className}
      onClick={onClick}
      style={{
        background: raised
          ? 'linear-gradient(180deg, var(--surface-2) 0%, var(--surface-1) 100%)'
          : 'var(--surface-1)',
        border: `1px solid ${tokens.lineSoft}`,
        borderRadius: 12,
        padding: pad,
        boxShadow: raised ? 'var(--shadow-md)' : 'var(--shadow-sm)',
        ...style,
      }}
    >
      {children}
    </div>
  );
}

export function Skeleton({
  width = '100%',
  height = 20,
  style,
}: {
  width?: number | string;
  height?: number | string;
  style?: CSSProperties;
}) {
  return (
    <div
      aria-hidden="true"
      style={{
        width,
        height,
        background: `linear-gradient(90deg, ${tokens.paper2} 0%, var(--sand) 50%, ${tokens.paper2} 100%)`,
        backgroundSize: '200% 100%',
        animation: 'shimmer 1.6s ease-in-out infinite',
        border: `1px solid ${tokens.lineSoft}`,
        ...style,
      }}
    />
  );
}

export function Stat({
  label,
  value,
  delta,
  tone,
}: {
  label: string;
  value: ReactNode;
  delta?: string;
  tone?: 'pos' | 'neg' | 'warn';
}) {
  const dColor =
    tone === 'pos' ? tokens.moss : tone === 'neg' ? tokens.rust : tokens.ink2;
  return (
    <div style={{ padding: '16px 18px' }}>
      <Eyebrow>{label}</Eyebrow>
      <div
        style={{
          fontFamily: tokens.mono,
          fontWeight: 600,
          fontSize: 28,
          letterSpacing: 0,
          marginTop: 6,
          color: tokens.ink,
          fontVariantNumeric: 'tabular-nums',
        }}
      >
        {value}
      </div>
      {delta && (
        <div
          style={{
            fontFamily: tokens.mono,
            fontSize: 11,
            marginTop: 4,
            color: dColor,
            fontVariantNumeric: 'tabular-nums',
          }}
        >
          {delta}
        </div>
      )}
    </div>
  );
}

export function Chip({
  tone = 'neutral',
  children,
  style,
}: {
  tone?: 'neutral' | 'healthy' | 'watch' | 'critical';
  children: ReactNode;
  style?: CSSProperties;
}) {
  const map = {
    neutral: { bg: 'var(--surface-2)', fg: tokens.ink2, dot: tokens.ink2 },
    healthy: { bg: 'rgba(90,107,71,0.10)', fg: tokens.moss, dot: tokens.moss },
    watch: { bg: 'rgba(228,184,96,0.16)', fg: '#8a6a1f', dot: '#c69423' },
    critical: { bg: 'rgba(196,69,54,0.10)', fg: tokens.rust, dot: tokens.rust },
  } as const;
  const c = map[tone];
  return (
    <span
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: 6,
        fontFamily: tokens.mono,
        fontSize: 10.5,
        letterSpacing: '0.06em',
        padding: '3px 9px',
        borderRadius: 999,
        border: `1px solid ${tokens.line}`,
        background: c.bg,
        color: c.fg,
        ...style,
      }}
    >
      <span
        style={{
          width: 6,
          height: 6,
          borderRadius: '50%',
          background: c.dot,
          display: 'inline-block',
        }}
      />
      {children}
    </span>
  );
}

export function ProtocolDot({ tone, size = 22 }: { tone: 'good' | 'warn' | 'bad'; size?: number }) {
  const c = tone === 'bad' ? tokens.rust : tone === 'warn' ? '#c69423' : tokens.moss;
  return (
    <svg width={size} height={size} viewBox="0 0 22 22">
      <circle cx="11" cy="11" r="9" stroke="var(--ink)" strokeWidth="1.25" fill="none" />
      <circle cx="11" cy="11" r="6" stroke="var(--ink)" strokeWidth="1" fill="none" opacity=".4" />
      <circle cx="11" cy="11" r="3" fill={c} />
    </svg>
  );
}

const PROTOCOL_ICONS: Record<string, { url: string; bg: string }> = {
  Kamino:   { url: 'https://icons.llamao.fi/icons/protocols/kamino?w=64&h=64',   bg: '#0E0E10' },
  Save:     { url: 'https://icons.llamao.fi/icons/protocols/save?w=64&h=64',     bg: '#0E0E10' },
  Marginfi: { url: 'https://icons.llamao.fi/icons/protocols/marginfi?w=64&h=64', bg: '#0E0E10' },
};

export function ProtocolBadge({
  protocol,
  size = 28,
}: {
  protocol: string;
  size?: number;
}) {
  const meta = PROTOCOL_ICONS[protocol];
  if (meta) {
    return (
      <span
        style={{
          width: size,
          height: size,
          background: meta.bg,
          display: 'inline-flex',
          alignItems: 'center',
          justifyContent: 'center',
          borderRadius: 6,
          overflow: 'hidden',
          boxShadow: '0 1px 0 rgba(255,255,255,0.06) inset, 0 1px 2px rgba(0,0,0,0.18)',
        }}
      >
        <img
          src={meta.url}
          alt={protocol}
          width={size - 6}
          height={size - 6}
          style={{ display: 'block', borderRadius: 4 }}
          loading="lazy"
        />
      </span>
    );
  }
  return (
    <span
      style={{
        width: size,
        height: size,
        background: tokens.ink,
        color: tokens.paper,
        display: 'inline-flex',
        alignItems: 'center',
        justifyContent: 'center',
        fontFamily: tokens.serif,
        fontWeight: 600,
        fontSize: size * 0.5,
        borderRadius: 6,
      }}
    >
      {protocol[0] ?? '?'}
    </span>
  );
}

/** Half-circle health gauge — sonar geometry. value: 0..1, threshold: 0..1 */
export function ArcGauge({
  value,
  threshold = 0.85,
  size = 180,
  label,
  sub,
}: {
  value: number;
  threshold?: number;
  size?: number;
  label?: string;
  sub?: string;
}) {
  const v = Math.max(0, Math.min(1, value));
  const tone = v >= threshold ? tokens.rust : v >= 0.6 ? '#c69423' : tokens.moss;
  const cx = 70;
  const cy = 72;
  const rOuter = 58;
  const rMid = 46;
  const rInner = 34;
  const ang = Math.PI * (1 - v);
  const x = cx + Math.cos(ang) * rInner;
  const y = cy - Math.sin(ang) * rInner;
  const tx = cx + Math.cos(Math.PI * (1 - threshold)) * rOuter;
  const ty = cy - Math.sin(Math.PI * (1 - threshold)) * rOuter;
  return (
    <div style={{ display: 'inline-flex', flexDirection: 'column', alignItems: 'center', gap: 6 }}>
      <svg width={size} height={size * 0.62} viewBox="0 0 140 90" fill="none">
        <line x1="6" y1="72" x2="134" y2="72" stroke="var(--ink)" strokeWidth="1.25" />
        <path d={`M ${cx - rOuter} ${cy} A ${rOuter} ${rOuter} 0 0 1 ${cx + rOuter} ${cy}`} stroke="var(--ink)" strokeWidth="1.25" fill="none" opacity=".55" />
        <path d={`M ${cx - rMid} ${cy} A ${rMid} ${rMid} 0 0 1 ${cx + rMid} ${cy}`} stroke="var(--ink)" strokeWidth="1.25" fill="none" opacity=".4" />
        <path d={`M ${cx - rInner} ${cy} A ${rInner} ${rInner} 0 0 1 ${cx + rInner} ${cy}`} stroke="var(--ink)" strokeWidth="1.25" fill="none" opacity=".25" />
        <line
          x1={cx + Math.cos(Math.PI * (1 - threshold)) * (rInner - 3)}
          y1={cy - Math.sin(Math.PI * (1 - threshold)) * (rInner - 3)}
          x2={tx}
          y2={ty}
          stroke="var(--rust)"
          strokeWidth="1.5"
          strokeDasharray="2 2"
        />
        <motion.line
          x1={cx}
          y1={cy}
          initial={{ x2: cx, y2: cy }}
          animate={{ x2: x, y2: y }}
          transition={{ duration: 0.7, ease: 'easeOut' }}
          stroke={tone}
          strokeWidth="2.5"
          strokeLinecap="round"
        />
        <circle cx={cx} cy={cy} r="3.5" fill="var(--rust)" />
        <circle cx={cx} cy={cy} r="7" stroke="var(--ink)" strokeWidth="1" fill="none" />
      </svg>
      {label && (
        <div style={{ fontFamily: tokens.mono, fontSize: 10, letterSpacing: '0.14em', textTransform: 'uppercase', color: 'color-mix(in oklab, var(--ink) 55%, transparent)' }}>
          {label}
        </div>
      )}
      {sub && (
        <div style={{ fontFamily: tokens.serifI, fontStyle: 'italic', fontSize: 14, color: tokens.ink2 }}>
          {sub}
        </div>
      )}
    </div>
  );
}

/** Fade-up reveal on scroll-into-view. */
export function Reveal({
  children,
  delay = 0,
  y = 24,
}: {
  children: ReactNode;
  delay?: number;
  y?: number;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const inView = useInView(ref, { once: true, margin: '-10% 0px' });
  return (
    <motion.div
      ref={ref}
      initial={{ opacity: 0, y }}
      animate={inView ? { opacity: 1, y: 0 } : {}}
      transition={{ duration: 0.5, ease: [0.2, 0.8, 0.2, 1], delay }}
    >
      {children}
    </motion.div>
  );
}

/** Count-up number — only animates the first time it scrolls into view. */
export function CountUp({
  to,
  duration = 700,
  prefix = '',
  suffix = '',
  decimals = 0,
}: {
  to: number;
  duration?: number;
  prefix?: string;
  suffix?: string;
  decimals?: number;
}) {
  const ref = useRef<HTMLSpanElement>(null);
  const inView = useInView(ref, { once: true });
  const [v, setV] = useState(0);
  useEffect(() => {
    if (!inView) return;
    const start = performance.now();
    let raf = 0;
    const tick = (now: number) => {
      const t = Math.min(1, (now - start) / duration);
      const eased = 1 - Math.pow(1 - t, 3);
      setV(to * eased);
      if (t < 1) raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [inView, to, duration]);
  return (
    <span ref={ref} style={{ fontVariantNumeric: 'tabular-nums' }}>
      {prefix}
      {v.toLocaleString(undefined, {
        minimumFractionDigits: decimals,
        maximumFractionDigits: decimals,
      })}
      {suffix}
    </span>
  );
}

/** Background sonar watermark — large, faint, decorative. */
export function SonarWatermark({
  size = 540,
  opacity = 0.07,
  style,
}: {
  size?: number;
  opacity?: number;
  style?: CSSProperties;
}) {
  return (
    <svg
      style={{ position: 'absolute', pointerEvents: 'none', opacity, ...style }}
      width={size}
      height={size}
      viewBox="0 0 400 400"
      fill="none"
      aria-hidden
    >
      <line x1="0" y1="320" x2="400" y2="320" stroke="var(--ink)" strokeWidth="1.5" />
      {[60, 110, 160, 210, 260, 310].map((r) => (
        <path
          key={r}
          d={`M ${200 - r} 320 A ${r} ${r} 0 0 1 ${200 + r} 320`}
          stroke="var(--ink)"
          strokeWidth="1.25"
          fill="none"
        />
      ))}
      <circle cx="200" cy="320" r="6" fill="var(--rust)" />
    </svg>
  );
}

/** ─── Temperature Bar ─── 
 *  Horizontal gradient green → yellow → red with a dot marker showing current value.
 *  value: 0..1 (0 = safe/green, 1 = danger/red)
 */
export function TemperatureBar({
  value,
  threshold,
  height = 10,
  style,
}: {
  value: number;
  threshold?: number;
  height?: number;
  style?: CSSProperties;
}) {
  const v = Math.max(0, Math.min(1, value));
  const dotColor = v >= 0.85 ? '#C44536' : v >= 0.6 ? '#c69423' : '#5A6B47';

  return (
    <div style={{ position: 'relative', width: '100%', ...style }}>
      {/* Track */}
      <div
        style={{
          height,
          borderRadius: height / 2,
          background: 'linear-gradient(90deg, #5A6B47 0%, #8a9a3e 25%, #c69423 55%, #d97757 78%, #C44536 100%)',
          opacity: 0.85,
        }}
      />
      {/* Threshold marker */}
      {threshold !== undefined && (
        <div
          style={{
            position: 'absolute',
            left: `${threshold * 100}%`,
            top: -2,
            width: 2,
            height: height + 4,
            background: 'var(--ink)',
            opacity: 0.5,
          }}
        />
      )}
      {/* Value dot */}
      <motion.div
        initial={{ left: '0%' }}
        animate={{ left: `${v * 100}%` }}
        transition={{ duration: 0.6, ease: 'easeOut' }}
        style={{
          position: 'absolute',
          top: -(12 - height) / 2,
          width: 12,
          height: 12,
          borderRadius: '50%',
          background: dotColor,
          border: '2px solid #F6F4EE',
          boxShadow: `0 0 6px 2px ${dotColor}44`,
          transform: 'translateX(-50%)',
        }}
      />
    </div>
  );
}

/** ─── Live Sonar Background ───
 *  Animated pulsing concentric arcs that radiate outward.
 *  Creates a "live radar" feeling.
 */
export function LiveSonarBackground({
  size = 640,
  style,
}: {
  size?: number;
  style?: CSSProperties;
}) {
  const arcCount = 5;
  const durations = [3.5, 4.0, 4.5, 5.0, 5.5];
  const delays = [0, 0.7, 1.4, 2.1, 2.8];

  return (
    <div
      style={{
        position: 'absolute',
        width: size,
        height: size * 0.55,
        overflow: 'hidden',
        pointerEvents: 'none',
        ...style,
      }}
      aria-hidden
    >
      <svg
        width="100%"
        height="100%"
        viewBox="0 0 640 352"
        fill="none"
        style={{ position: 'absolute', inset: 0 }}
      >
        {/* Base horizon line */}
        <line x1="0" y1="340" x2="640" y2="340" stroke="var(--ink)" strokeWidth="1" opacity="0.08" />

        {/* Pulsing arcs */}
        {Array.from({ length: arcCount }).map((_, i) => {
          const maxR = 80 + i * 50;
          return (
            <motion.path
              key={i}
              d={`M ${320 - maxR} 340 A ${maxR} ${maxR} 0 0 1 ${320 + maxR} 340`}
              stroke="var(--cobalt)"
              strokeWidth="1.5"
              fill="none"
              initial={{ opacity: 0, pathLength: 0 }}
              animate={{
                opacity: [0, 0.25, 0.12, 0],
                pathLength: [0, 1, 1, 1],
              }}
              transition={{
                duration: durations[i],
                delay: delays[i],
                repeat: Infinity,
                ease: 'easeOut',
              }}
            />
          );
        })}

        {/* Static faint arcs (skeleton) */}
        {[80, 140, 200, 260].map((r) => (
          <path
            key={`s-${r}`}
            d={`M ${320 - r} 340 A ${r} ${r} 0 0 1 ${320 + r} 340`}
            stroke="var(--ink)"
            strokeWidth="0.75"
            fill="none"
            opacity="0.06"
          />
        ))}

        {/* Center dot */}
        <circle cx="320" cy="340" r="4" fill="var(--rust)" opacity="0.7" />
        <motion.circle
          cx="320"
          cy="340"
          r="4"
          fill="var(--rust)"
          initial={{ r: 4, opacity: 0.5 }}
          animate={{ r: 12, opacity: 0 }}
          transition={{ duration: 2, repeat: Infinity, ease: 'easeOut' }}
        />
      </svg>
    </div>
  );
}

/** ─── Price Ticker ───
 *  Infinite scrolling strip of token prices, right → left.
 *  Pulls live prices from /api/ticker; falls back to MOCK_PRICES while loading
 *  or if the indexer hasn't priced a particular asset yet.
 */
const TICKER_SYMBOLS: { symbol: string; mint: string }[] = [
  { symbol: 'SOL', mint: 'So11111111111111111111111111111111111111112' },
  { symbol: 'USDC', mint: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v' },
  { symbol: 'USDT', mint: 'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB' },
  { symbol: 'mSOL', mint: 'mSoLzYCxHdYgdzU16g5QSh3i5K3z1Zbk7Jo47ZvZEuQ' },
  { symbol: 'JitoSOL', mint: 'J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn' },
  { symbol: 'BONK', mint: 'DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263' },
  { symbol: 'WIF', mint: 'EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm' },
  { symbol: 'JUP', mint: 'JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN' },
  { symbol: 'RAY', mint: '4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R' },
];

export function PriceTicker({ style }: { style?: CSSProperties }) {
  const tickerQ = useTicker();
  const prices: [string, number][] = TICKER_SYMBOLS.map(({ symbol, mint }) => {
    const live = tickerQ.data?.[mint]?.price;
    return [symbol, typeof live === 'number' && live > 0 ? live : (MOCK_PRICES[symbol] ?? 0)];
  });
  // Double for seamless loop
  const items = [...prices, ...prices];

  return (
    <div
      style={{
        overflow: 'hidden',
        width: '100%',
        borderTop: `1px solid ${tokens.line}`,
        borderBottom: `1px solid ${tokens.line}`,
        background: 'color-mix(in oklab, var(--ink) 3%, transparent)',
        ...style,
      }}
    >
      <div
        style={{
          display: 'flex',
          animation: 'tickerScroll 28s linear infinite',
          width: 'max-content',
        }}
      >
        {items.map(([symbol, price], i) => (
          <div
            key={`${symbol}-${i}`}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              padding: '12px 28px',
              whiteSpace: 'nowrap',
              fontFamily: tokens.mono,
              fontSize: 12,
              letterSpacing: '0.04em',
            }}
          >
            <span style={{ fontWeight: 600, color: 'var(--ink)' }}>{symbol}</span>
            <span style={{ color: 'color-mix(in oklab, var(--ink) 55%, transparent)' }}>
              ${price < 0.01 ? price.toFixed(7) : price.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
            </span>
            <span
              style={{
                width: 4,
                height: 4,
                borderRadius: '50%',
                background: 'color-mix(in oklab, var(--ink) 15%, transparent)',
                display: 'inline-block',
              }}
            />
          </div>
        ))}
      </div>
    </div>
  );
}

/** Shared button. variant: ghost (outline), primary (ink), accent (cobalt). */
export function Button({
  variant = 'ghost',
  children,
  onClick,
  disabled,
  size = 'md',
  type = 'button',
  style,
}: {
  variant?: 'ghost' | 'primary' | 'accent' | 'danger';
  children: ReactNode;
  onClick?: () => void;
  disabled?: boolean;
  size?: 'sm' | 'md' | 'lg';
  type?: 'button' | 'submit';
  style?: CSSProperties;
}) {
  const pad = size === 'sm' ? '6px 12px' : size === 'lg' ? '12px 22px' : '8px 14px';
  const fs = size === 'sm' ? 12 : size === 'lg' ? 14 : 13;
  const palette = {
    ghost: { bg: tokens.paper, fg: tokens.ink, bd: tokens.ink },
    primary: { bg: tokens.ink, fg: tokens.paper, bd: tokens.ink },
    accent: { bg: tokens.cobalt, fg: tokens.paper, bd: 'var(--cobalt-ink)' },
    danger: { bg: tokens.rust, fg: tokens.paper, bd: 'var(--rust-ink)' },
  }[variant];
  return (
    <motion.button
      type={type}
      onClick={onClick}
      disabled={disabled}
      whileHover={disabled ? {} : { y: -1 }}
      whileTap={disabled ? {} : { y: 0, scale: 0.98 }}
      transition={{ duration: 0.12 }}
      style={{
        fontFamily: tokens.sans,
        fontSize: fs,
        fontWeight: 500,
        padding: pad,
        borderRadius: 8,
        border: `1px solid ${palette.bd}`,
        background: palette.bg,
        color: palette.fg,
        cursor: disabled ? 'not-allowed' : 'pointer',
        opacity: disabled ? 0.5 : 1,
        ...style,
      }}
    >
      {children}
    </motion.button>
  );
}
