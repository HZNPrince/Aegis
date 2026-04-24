import { useState } from 'react';
import type { CSSProperties, ReactNode } from 'react';
import type { Protocol, Severity, Side } from '../types';
import { severityColor } from '../utils';
import { useTicker } from '../hooks';
import { MOCK_WALLET_FULL, MOCK_WALLET_SHORT } from '../mockData';

// Curated ticker list: symbol → mainnet mint. Kept small + recognizable;
// we look these up in the backend's live price cache (Jupiter-polled).
const TICKER_TOKENS: Array<{ symbol: string; mint: string }> = [
  { symbol: 'SOL', mint: 'So11111111111111111111111111111111111111112' },
  { symbol: 'JitoSOL', mint: 'J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn' },
  { symbol: 'USDC', mint: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v' },
  { symbol: 'USDT', mint: 'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB' },
  { symbol: 'BONK', mint: 'DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263' },
  { symbol: 'WIF', mint: 'EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm' },
  { symbol: 'JUP', mint: 'JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN' },
  { symbol: 'PYTH', mint: 'HZ1JovNiVvGrGNiiYvEozEVgZ58xaU3RKwX8eACQBCt3' },
];

export function Card({
  children,
  style = {},
  onClick,
}: {
  children: ReactNode;
  style?: CSSProperties;
  onClick?: () => void;
}) {
  return (
    <div
      onClick={onClick}
      style={{
        background: '#2A2826',
        borderRadius: 20,
        border: '1px solid rgba(255,255,255,0.07)',
        boxShadow: '0 2px 24px rgba(0,0,0,0.35)',
        padding: 24,
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
  radius = 8,
  style = {},
}: {
  width?: number | string;
  height?: number | string;
  radius?: number | string;
  style?: CSSProperties;
}) {
  return (
    <div
      style={{
        width,
        height,
        borderRadius: radius,
        background: 'rgba(255,255,255,0.06)',
        backgroundImage:
          'linear-gradient(90deg, rgba(255,255,255,0.0) 0%, rgba(255,255,255,0.06) 50%, rgba(255,255,255,0.0) 100%)',
        backgroundSize: '200% 100%',
        animation: 'shimmer 1.6s ease-in-out infinite',
        ...style,
      }}
    />
  );
}

export function SeverityBadge({ severity }: { severity: Severity }) {
  const c = severityColor(severity);
  return (
    <span
      style={{
        fontFamily: "'Inter', sans-serif",
        fontSize: 11,
        fontWeight: 600,
        color: c,
        background: `${c}18`,
        border: `1px solid ${c}33`,
        padding: '2px 8px',
        borderRadius: 100,
        letterSpacing: '0.06em',
        textTransform: 'uppercase',
      }}
    >
      {severity}
    </span>
  );
}

const PROTOCOL_COLORS: Record<Protocol, string> = {
  Kamino: '#A78BFA',
  Save: '#34D399',
  Marginfi: '#60A5FA',
};

export function ProtocolBadge({ protocol }: { protocol: Protocol }) {
  const c = PROTOCOL_COLORS[protocol] ?? '#888';
  return (
    <span
      style={{
        fontFamily: "'Inter', sans-serif",
        fontSize: 11,
        fontWeight: 600,
        color: c,
        background: `${c}18`,
        padding: '2px 8px',
        borderRadius: 100,
      }}
    >
      {protocol}
    </span>
  );
}

export function SidePill({ side }: { side: Side }) {
  const isCollateral = side === 'Collateral';
  return (
    <span
      style={{
        fontFamily: "'Inter', sans-serif",
        fontSize: 11,
        fontWeight: 600,
        color: isCollateral ? '#7DA87B' : '#E4A853',
        background: isCollateral ? '#7DA87B18' : '#E4A85318',
        padding: '2px 8px',
        borderRadius: 100,
      }}
    >
      {side}
    </span>
  );
}

export function Toggle({
  checked,
  onChange,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <div
      onClick={() => onChange(!checked)}
      style={{
        width: 44,
        height: 24,
        borderRadius: 12,
        cursor: 'pointer',
        position: 'relative',
        background: checked ? '#D97757' : 'rgba(255,255,255,0.12)',
        transition: 'background 0.25s',
        flexShrink: 0,
      }}
    >
      <div
        style={{
          position: 'absolute',
          top: 3,
          left: checked ? 22 : 3,
          width: 18,
          height: 18,
          borderRadius: '50%',
          background: '#F5F4EF',
          transition: 'left 0.25s',
          boxShadow: '0 1px 4px rgba(0,0,0,0.4)',
        }}
      />
    </div>
  );
}

export function PulseDot() {
  return (
    <span style={{ position: 'relative', display: 'inline-flex', width: 8, height: 8 }}>
      <span
        style={{
          position: 'absolute',
          inset: 0,
          borderRadius: '50%',
          background: '#7DA87B',
          animation: 'ping 1.5s ease-in-out infinite',
          opacity: 0.6,
        }}
      />
      <span
        style={{
          borderRadius: '50%',
          width: 8,
          height: 8,
          background: '#7DA87B',
          display: 'inline-block',
        }}
      />
    </span>
  );
}

export function LtvBar({ ltv, threshold }: { ltv: number; threshold: number }) {
  const pct = Math.min(ltv / threshold, 1) * 100;
  const color =
    ltv > threshold * 0.9 ? '#D9604E' : ltv > threshold * 0.75 ? '#E4A853' : '#7DA87B';
  return (
    <div
      style={{
        height: 4,
        borderRadius: 4,
        background: 'rgba(255,255,255,0.1)',
        position: 'relative',
        overflow: 'visible',
      }}
    >
      <div
        style={{
          height: '100%',
          width: `${pct}%`,
          background: color,
          borderRadius: 4,
          transition: 'width 0.6s ease',
          maxWidth: '100%',
        }}
      />
      <div
        style={{
          position: 'absolute',
          top: -3,
          left: 'calc(100% - 1px)',
          width: 2,
          height: 10,
          background: 'rgba(255,255,255,0.25)',
          borderRadius: 1,
        }}
      />
    </div>
  );
}

export function EmptyState({ text = "All quiet. You're safe." }: { text?: string }) {
  return (
    <div style={{ padding: '80px 24px', textAlign: 'center' }}>
      <p
        style={{
          fontFamily: "'Fraunces', serif",
          fontSize: 22,
          color: 'rgba(245,244,239,0.4)',
          fontWeight: 300,
          letterSpacing: '-0.01em',
        }}
      >
        {text}
      </p>
    </div>
  );
}

export function SectionLabel({ children }: { children: ReactNode }) {
  return (
    <div
      style={{
        fontFamily: "'Inter', sans-serif",
        fontSize: 11,
        fontWeight: 600,
        color: 'rgba(245,244,239,0.35)',
        letterSpacing: '0.1em',
        textTransform: 'uppercase',
        marginBottom: 14,
      }}
    >
      {children}
    </div>
  );
}

export function PriceTickerRail() {
  const tickerQ = useTicker();

  // Only render tokens present in the backend cache. The delta is Jupiter's
  // real 24h change — no more session-delta placeholder or Math.random().
  const rows = TICKER_TOKENS.filter(
    ({ mint }) => tickerQ.data?.[mint]?.price !== undefined,
  ).map(({ symbol, mint }) => {
    const entry = tickerQ.data![mint];
    return {
      symbol,
      price: entry.price,
      delta: entry.change_24h ?? 0,
    };
  });

  const doubled = [...rows, ...rows];
  return (
    <div
      style={{
        overflow: 'hidden',
        borderTop: '1px solid rgba(255,255,255,0.06)',
        borderBottom: '1px solid rgba(255,255,255,0.06)',
        padding: '10px 0',
        background: 'rgba(0,0,0,0.15)',
      }}
    >
      <div
        style={{
          display: 'flex',
          gap: 0,
          animation: 'tickerScroll 28s linear infinite',
          width: 'max-content',
        }}
      >
        {doubled.map(({ symbol, price, delta }, i) => {
          const display =
            price < 0.01
              ? price.toFixed(8)
              : price < 10
                ? price.toFixed(4)
                : price.toFixed(2);
          const deltaColor = delta > 0 ? '#7DA87B' : delta < 0 ? '#D9604E' : 'rgba(245,244,239,0.35)';
          const deltaGlyph = delta > 0 ? '▲' : delta < 0 ? '▼' : '·';
          return (
            <div
              key={i}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 8,
                padding: '0 28px',
                borderRight: '1px solid rgba(255,255,255,0.06)',
                flexShrink: 0,
              }}
            >
              <span
                style={{
                  fontFamily: "'Inter', sans-serif",
                  fontSize: 12,
                  color: 'rgba(245,244,239,0.45)',
                  fontWeight: 500,
                }}
              >
                {symbol}
              </span>
              <span
                style={{
                  fontFamily: "'JetBrains Mono', monospace",
                  fontSize: 12,
                  color: '#F5F4EF',
                  fontWeight: 500,
                }}
              >
                {display}
              </span>
              <span style={{ fontSize: 10, color: deltaColor }}>
                {deltaGlyph} {Math.abs(delta).toFixed(2)}%
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

export function WalletChip({ onClick }: { onClick?: () => void }) {
  const [copied, setCopied] = useState(false);
  const copy = (e: React.MouseEvent) => {
    e.stopPropagation();
    void navigator.clipboard?.writeText(MOCK_WALLET_FULL);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 6,
        background: 'rgba(255,255,255,0.06)',
        border: '1px solid rgba(255,255,255,0.1)',
        borderRadius: 100,
        padding: '5px 12px 5px 8px',
        cursor: 'pointer',
      }}
      onClick={onClick}
    >
      <div
        style={{
          width: 8,
          height: 8,
          borderRadius: '50%',
          background: '#7DA87B',
          boxShadow: '0 0 6px #7DA87B88',
        }}
      />
      <span
        style={{
          fontFamily: "'JetBrains Mono', monospace",
          fontSize: 12,
          color: 'rgba(245,244,239,0.7)',
          letterSpacing: '0.02em',
        }}
      >
        {MOCK_WALLET_SHORT}
      </span>
      <button
        onClick={copy}
        style={{
          background: 'none',
          border: 'none',
          cursor: 'pointer',
          padding: 0,
          color: 'rgba(245,244,239,0.35)',
          fontSize: 11,
        }}
      >
        {copied ? '✓' : '⎘'}
      </button>
    </div>
  );
}
