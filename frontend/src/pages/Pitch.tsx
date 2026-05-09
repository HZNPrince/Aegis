// Pitch deck — Aegis. Lives at /pitch.
// Voice mirrors the landing page: Geist sans-bold headlines, rust spans for emphasis,
// minimal frames, the brand wordmark with the dot inside the G.
// Keyboard nav: ← → / Space / Home / End. Cmd-P prints to PDF.

import { AnimatePresence, motion } from 'framer-motion';
import { useEffect, useState } from 'react';
import type { ReactNode } from 'react';
import { SonarLogo } from '../components/SonarLogo';
import { tokens } from '../components/sonar';

const COBALT = '#3B5BDB';
const RUST = '#C44536';

const SLIDES: Array<() => JSX.Element> = [
  Cover,
  Problem,
  WhatAegisIs,
  HowAegisWorks,
  WhatYouGet,
  Telegram,
  Traction,
  Close,
];

export function Pitch() {
  const [idx, setIdx] = useState(0);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'ArrowRight' || e.key === ' ' || e.key === 'PageDown') {
        e.preventDefault();
        setIdx((i) => Math.min(i + 1, SLIDES.length - 1));
      } else if (e.key === 'ArrowLeft' || e.key === 'PageUp') {
        e.preventDefault();
        setIdx((i) => Math.max(i - 1, 0));
      } else if (e.key === 'Home') setIdx(0);
      else if (e.key === 'End') setIdx(SLIDES.length - 1);
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, []);

  const Slide = SLIDES[idx];

  return (
    <div style={{ position: 'fixed', inset: 0, background: 'var(--paper)', color: 'var(--ink)', overflow: 'hidden', fontFamily: tokens.sans }}>
      <AnimatePresence mode="wait">
        <motion.div
          key={idx}
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -8 }}
          transition={{ duration: 0.32, ease: [0.2, 0.8, 0.2, 1] }}
          style={{ width: '100%', height: '100%' }}
        >
          <Slide />
        </motion.div>
      </AnimatePresence>

      <Mark />
      <Counter idx={idx} total={SLIDES.length} />
      <NavHint />
      <ProgressBar idx={idx} total={SLIDES.length} />
    </div>
  );
}

// ─── Chrome ──────────────────────────────────────────────────────────────

function Mark() {
  return (
    <div style={{ position: 'absolute', top: 32, left: 44, display: 'flex', alignItems: 'center', gap: 12, zIndex: 10 }}>
      <SonarLogo size={22} ink="#1A1A1A" accent={COBALT} dot={RUST} />
      <span style={{ fontFamily: tokens.sans, fontWeight: 800, fontSize: 14, letterSpacing: '-0.02em' }}>
        AE<DotG />IS
      </span>
    </div>
  );
}

// "G" with a small rust dot inside — the brand mark from the landing page.
function DotG({ size = 1 }: { size?: number }) {
  return (
    <span style={{ position: 'relative', display: 'inline-flex', alignItems: 'center', justifyContent: 'center' }}>
      G
      <span
        style={{
          position: 'absolute',
          width: `${0.18 * size}em`,
          height: `${0.18 * size}em`,
          background: RUST,
          borderRadius: '50%',
          boxShadow: `0 0 ${12 * size}px ${RUST}`,
        }}
      />
    </span>
  );
}

function Counter({ idx, total }: { idx: number; total: number }) {
  return (
    <div style={{ position: 'absolute', bottom: 28, right: 44, fontFamily: tokens.mono, fontSize: 11, letterSpacing: '0.16em', color: 'rgba(26,26,26,0.45)', zIndex: 10 }}>
      {String(idx + 1).padStart(2, '0')} / {String(total).padStart(2, '0')}
    </div>
  );
}

function NavHint() {
  return (
    <div style={{ position: 'absolute', bottom: 28, left: 44, fontFamily: tokens.mono, fontSize: 10, letterSpacing: '0.14em', textTransform: 'uppercase', color: 'rgba(26,26,26,0.32)', zIndex: 10 }}>
      ← → to navigate
    </div>
  );
}

function ProgressBar({ idx, total }: { idx: number; total: number }) {
  return (
    <div style={{ position: 'absolute', top: 0, left: 0, right: 0, height: 2, background: 'rgba(26,26,26,0.06)', zIndex: 10 }}>
      <motion.div
        initial={false}
        animate={{ width: `${((idx + 1) / total) * 100}%` }}
        transition={{ duration: 0.4, ease: [0.2, 0.8, 0.2, 1] }}
        style={{ height: '100%', background: COBALT }}
      />
    </div>
  );
}

// ─── Slide primitives ────────────────────────────────────────────────────

function Shell({ children, align = 'left' }: { children: ReactNode; align?: 'left' | 'center' }) {
  return (
    <div
      style={{
        width: '100%',
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'center',
        alignItems: align === 'center' ? 'center' : 'flex-start',
        padding: '110px clamp(48px, 7vw, 120px) 88px',
        textAlign: align === 'center' ? 'center' : 'left',
        maxWidth: 1320,
        margin: '0 auto',
      }}
    >
      {children}
    </div>
  );
}

function Eyebrow({ children }: { children: ReactNode }) {
  return (
    <div style={{ fontFamily: tokens.mono, fontSize: 11, letterSpacing: '0.18em', textTransform: 'uppercase', color: 'rgba(26,26,26,0.5)', marginBottom: 22 }}>
      {children}
    </div>
  );
}

function Headline({ children, size = 72 }: { children: ReactNode; size?: number }) {
  return (
    <h1
      style={{
        fontFamily: tokens.sans,
        fontWeight: 750,
        fontSize: size,
        lineHeight: 1.05,
        letterSpacing: '-0.035em',
        margin: 0,
        maxWidth: 1100,
      }}
    >
      {children}
    </h1>
  );
}

function Rust({ children }: { children: ReactNode }) {
  return <span style={{ color: RUST }}>{children}</span>;
}

function Cobalt({ children }: { children: ReactNode }) {
  return <span style={{ color: COBALT }}>{children}</span>;
}

function Lead({ children }: { children: ReactNode }) {
  return (
    <p style={{ fontFamily: tokens.sans, fontWeight: 500, fontSize: 19, lineHeight: 1.55, color: 'color-mix(in oklab, var(--ink) 55%, transparent)', margin: '32px 0 0', maxWidth: 760 }}>
      {children}
    </p>
  );
}

// ─── 1. Cover ────────────────────────────────────────────────────────────

function Cover() {
  return (
    <Shell align="left">
      <motion.div
        initial={{ opacity: 0, y: 16 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.7, ease: [0.2, 0.8, 0.2, 1] }}
        style={{
          fontFamily: tokens.sans,
          fontWeight: 800,
          fontSize: 'clamp(96px, 14vw, 200px)',
          lineHeight: 0.9,
          letterSpacing: '-0.04em',
          display: 'flex',
          alignItems: 'center',
          gap: '0.06em',
        }}
      >
        <span>A</span>
        <span>E</span>
        <span style={{ position: 'relative', display: 'inline-flex', alignItems: 'center', justifyContent: 'center' }}>
          G
          <span style={{ position: 'absolute', width: '0.18em', height: '0.18em', background: RUST, borderRadius: '50%', boxShadow: `0 0 36px ${RUST}` }} />
        </span>
        <span>I</span>
        <span>S</span>
      </motion.div>

      <p style={{ fontFamily: tokens.sans, fontWeight: 600, fontSize: 22, lineHeight: 1.5, color: 'color-mix(in oklab, var(--ink) 50%, transparent)', maxWidth: 640, marginTop: 56 }}>
        A unified watchtower for your Solana lending positions — with guardrails and an AI sentinel that only speaks when it matters.
      </p>

      <div style={{ width: 40, height: 1, background: 'rgba(26,26,26,0.22)', marginTop: 40 }} />
      <div style={{ fontFamily: tokens.mono, fontSize: 11, letterSpacing: '0.18em', textTransform: 'uppercase', color: 'rgba(26,26,26,0.4)', marginTop: 18 }}>
        Tracking lending across the Solana ecosystem
      </div>
    </Shell>
  );
}

// ─── 2. Problem ──────────────────────────────────────────────────────────

function Problem() {
  return (
    <Shell align="left">
      <Eyebrow>The problem</Eyebrow>
      <h1
        style={{
          fontFamily: tokens.sans,
          fontWeight: 750,
          fontSize: 'clamp(36px, 4.6vw, 60px)',
          lineHeight: 1.08,
          letterSpacing: '-0.03em',
          margin: 0,
          maxWidth: 1040,
        }}
      >
        Liquidations don't send a <Rust>heads-up</Rust>.
        <br />
        You find out after you've been liquidated.
      </h1>

      <p style={{ fontFamily: tokens.sans, fontWeight: 500, fontSize: 17, lineHeight: 1.6, color: 'color-mix(in oklab, var(--ink) 55%, transparent)', margin: '28px 0 0', maxWidth: 720 }}>
        If you borrow on Solana, you're watching dashboards in multiple tabs. When SOL drops 8% at
        3 a.m., nobody pages you. The bots already moved.
      </p>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 0, marginTop: 48, width: '100%', maxWidth: 880, border: '1px solid rgba(26,26,26,0.18)' }}>
        <Cell n="Many" label="dashboards to check" />
        <Cell n="<1s" label="bot reaction time" border />
        <Cell n="0" label="warning by default" border tint />
      </div>
    </Shell>
  );
}

function Cell({ n, label, border, tint }: { n: string; label: string; border?: boolean; tint?: boolean }) {
  return (
    <div style={{ padding: '24px 22px', borderLeft: border ? '1px solid rgba(26,26,26,0.18)' : 'none' }}>
      <div style={{ fontFamily: tokens.sans, fontWeight: 800, fontSize: 44, letterSpacing: '-0.03em', lineHeight: 1, color: tint ? RUST : 'var(--ink)' }}>
        {n}
      </div>
      <div style={{ fontFamily: tokens.mono, fontSize: 10.5, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'rgba(26,26,26,0.55)', marginTop: 12 }}>
        {label}
      </div>
    </div>
  );
}

// ─── 3. What Aegis is ────────────────────────────────────────────────────

function WhatAegisIs() {
  return (
    <Shell align="left">
      <Eyebrow>What Aegis is</Eyebrow>
      <Headline size={64}>
        A <Rust>watchtower</Rust> for your Solana lending positions.
      </Headline>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 56, marginTop: 56, width: '100%', maxWidth: 1120 }}>
        <p style={{ fontFamily: tokens.sans, fontWeight: 500, fontSize: 18, lineHeight: 1.6, color: 'var(--ink-2)', margin: 0 }}>
          Connect your wallet once. You get a single unified view across every supported lending venue —
          real-time prices, weighted health score, every collateral and borrow leg, with risk computed
          live from on-chain data.
        </p>
        <p style={{ fontFamily: tokens.sans, fontWeight: 500, fontSize: 18, lineHeight: 1.6, color: 'var(--ink-2)', margin: 0 }}>
          And it isn't just a dashboard. An AI sentinel watches 24/7. If your health crosses a threshold,
          you get a Telegram alert <Rust>before liquidation, not after</Rust>. From the same bot, you can
          repay in one tap.
        </p>
      </div>
    </Shell>
  );
}

// ─── 4. How Aegis works (system overview) ───────────────────────────────

function HowAegisWorks() {
  const stages = [
    { label: 'Wallet', sub: 'Read-only · non-custodial' },
    { label: 'Indexer', sub: 'Direct RPC · 60s poll' },
    { label: 'Risk + AI', sub: 'Weighted health · LLM' },
    { label: 'Telegram + Web', sub: 'Critical pings · dashboard' },
    { label: 'One-tap repay', sub: 'Wallet signs locally' },
  ];
  return (
    <Shell align="left">
      <Eyebrow>How Aegis works</Eyebrow>
      <Headline size={64}>
        From <Cobalt>on-chain</Cobalt> to your phone, in under a minute.
      </Headline>

      <div style={{ marginTop: 72, width: '100%' }}>
        <div style={{ display: 'flex', alignItems: 'stretch', gap: 0 }}>
          {stages.map((s, i) => (
            <FlowItem key={s.label} index={i} total={stages.length} label={s.label} sub={s.sub} highlight={i === 2} />
          ))}
        </div>
      </div>

      <p style={{ fontFamily: tokens.sans, fontSize: 14.5, color: 'var(--ink-2)', lineHeight: 1.6, marginTop: 56, maxWidth: 980 }}>
        Direct on-chain reads — no third-party indexer in the hot path. The AI sentinel narrates the
        <i> why</i>, not just the number. We never hold keys; every action is signed by your wallet.
      </p>
    </Shell>
  );
}

function FlowItem({ index, total, label, sub, highlight }: { index: number; total: number; label: string; sub: string; highlight?: boolean }) {
  const isLast = index === total - 1;
  return (
    <div style={{ flex: 1, display: 'flex', alignItems: 'center', gap: 0, minWidth: 0 }}>
      <div
        style={{
          flex: 1,
          padding: '22px 14px',
          border: highlight ? `1.5px solid ${COBALT}` : '1px solid rgba(26,26,26,0.18)',
          background: highlight ? 'rgba(59,91,219,0.05)' : 'var(--paper-2)',
          textAlign: 'center',
          minHeight: 100,
          display: 'flex',
          flexDirection: 'column',
          justifyContent: 'center',
        }}
      >
        <div style={{ fontFamily: tokens.sans, fontWeight: 700, fontSize: 17, letterSpacing: '-0.01em', color: highlight ? COBALT : 'var(--ink)' }}>
          {label}
        </div>
        <div style={{ fontFamily: tokens.mono, fontSize: 10, letterSpacing: '0.14em', color: 'rgba(26,26,26,0.5)', marginTop: 8, textTransform: 'uppercase' }}>
          {sub}
        </div>
      </div>
      {!isLast && (
        <div style={{ width: 28, display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'rgba(26,26,26,0.32)', flexShrink: 0 }}>
          <svg width="22" height="8" viewBox="0 0 22 8">
            <line x1="0" y1="4" x2="18" y2="4" stroke="currentColor" strokeWidth="1" />
            <polyline points="14,1 18,4 14,7" fill="none" stroke="currentColor" strokeWidth="1" />
          </svg>
        </div>
      )}
    </div>
  );
}

// ─── 4. What you get ─────────────────────────────────────────────────────

function WhatYouGet() {
  const features = [
    { title: 'Unified watchtower', body: 'One dashboard. Multiple protocols. Cross-protocol weighted health so you stop juggling tabs.' },
    { title: 'AI alerts that earn the ping', body: 'LLM summarizes the why — collateral price drop, oracle drift, rate spike — not just the number.' },
    { title: 'Guardrails that act', body: 'Pre-built repay, deleverage, and add-collateral rules. Triggered on-chain via your wallet, with cooldowns and daily caps.' },
    { title: 'Telegram, with intent', body: 'Critical alerts ship with inline Repay / Mute buttons. Anywhere you have a phone, you have your defense.' },
  ];
  return (
    <Shell align="left">
      <Eyebrow>What you get</Eyebrow>
      <Headline size={64}>
        Built for the moment <Rust>before</Rust> the trouble.
      </Headline>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: 0, marginTop: 56, width: '100%', border: '1px solid rgba(26,26,26,0.18)', background: 'var(--paper)' }}>
        {features.map((f, i) => (
          <div
            key={f.title}
            style={{
              padding: '32px 30px',
              borderRight: i % 2 === 0 ? '1px solid rgba(26,26,26,0.18)' : 'none',
              borderTop: i >= 2 ? '1px solid rgba(26,26,26,0.18)' : 'none',
              minHeight: 160,
            }}
          >
            <div style={{ fontFamily: tokens.sans, fontWeight: 700, fontSize: 22, letterSpacing: '-0.02em' }}>{f.title}</div>
            <p style={{ fontFamily: tokens.sans, fontSize: 14.5, color: 'var(--ink-2)', lineHeight: 1.6, marginTop: 12, maxWidth: 460 }}>{f.body}</p>
          </div>
        ))}
      </div>
    </Shell>
  );
}

// ─── 5. Telegram ─────────────────────────────────────────────────────────

function Telegram() {
  return (
    <Shell align="left">
      <Eyebrow>Telegram integration</Eyebrow>
      <Headline size={64}>
        Alerts that reach you <Cobalt>anywhere.</Cobalt>
      </Headline>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 24, marginTop: 56, width: '100%' }}>
        {/* Bot preview */}
        <div style={{ border: '1px solid rgba(26,26,26,0.18)', background: 'var(--paper-2)', padding: 22 }}>
          <div style={{ fontFamily: tokens.mono, fontSize: 11, letterSpacing: '0.16em', color: 'rgba(26,26,26,0.5)', textTransform: 'uppercase', marginBottom: 18 }}>
            Preview · @AegisBot
          </div>

          <div style={{ border: '1px solid rgba(196,69,54,0.28)', borderRadius: 10, padding: 14, background: 'rgba(196,69,54,0.04)' }}>
            <div style={{ fontFamily: tokens.sans, color: RUST, fontWeight: 600, fontSize: 12.5 }}>
              ● CRITICAL · MarginFi mSOL/USDT
            </div>
            <div style={{ fontFamily: tokens.sans, fontSize: 13.5, lineHeight: 1.45, marginTop: 8 }}>
              Health 1.04. Liq $154.20, current $161.80 (-6.8% / 24h).
            </div>
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 6, marginTop: 12 }}>
              <Btn>Repay 50%</Btn>
              <Btn>Custom</Btn>
              <Btn>Mute 1h</Btn>
            </div>
          </div>

          <Bubble align="right">/health</Bubble>
          <Bubble>Weighted health: 1.42{'\n'}4 positions · 2 watch · 1 critical</Bubble>
        </div>

        {/* Commands */}
        <div style={{ border: '1px solid rgba(26,26,26,0.18)', background: 'var(--paper-2)', overflow: 'hidden' }}>
          <div style={{ padding: '18px 22px', borderBottom: '1px solid rgba(26,26,26,0.10)' }}>
            <div style={{ fontFamily: tokens.mono, fontSize: 11, letterSpacing: '0.16em', color: 'rgba(26,26,26,0.5)', textTransform: 'uppercase' }}>
              Bot commands
            </div>
          </div>
          {[
            ['/positions', 'List tracked positions with health and LTV.'],
            ['/health', 'Return weighted portfolio health.'],
            ['/repay <pos> <amt>', 'Create a repay confirmation.'],
            ['/rules', 'Show your active guardrails.'],
            ['/alerts', 'Show your last 10 alerts.'],
            ['/mute <hours>', 'Pause non-critical alerts.'],
          ].map(([cmd, desc]) => (
            <div key={cmd} style={{ display: 'grid', gridTemplateColumns: '160px 1fr', gap: 12, padding: '12px 22px', borderBottom: '1px solid rgba(26,26,26,0.10)' }}>
              <code style={{ fontFamily: tokens.mono, fontSize: 12.5 }}>{cmd}</code>
              <span style={{ fontFamily: tokens.sans, fontSize: 12.5, color: 'var(--ink-2)', lineHeight: 1.5 }}>{desc}</span>
            </div>
          ))}
        </div>
      </div>
    </Shell>
  );
}

function Btn({ children }: { children: ReactNode }) {
  return (
    <span style={{ fontFamily: tokens.sans, fontSize: 11, padding: '6px 8px', textAlign: 'center', border: '1px solid rgba(26,26,26,0.22)', background: 'var(--paper)' }}>
      {children}
    </span>
  );
}

function Bubble({ children, align = 'left' }: { children: ReactNode; align?: 'left' | 'right' }) {
  const isRight = align === 'right';
  return (
    <div
      style={{
        marginTop: 12,
        marginLeft: isRight ? 'auto' : 0,
        maxWidth: 280,
        padding: '8px 12px',
        background: isRight ? 'var(--ink)' : 'var(--paper)',
        color: isRight ? 'var(--paper)' : 'var(--ink)',
        border: '1px solid rgba(26,26,26,0.16)',
        borderRadius: 10,
        fontFamily: tokens.mono,
        fontSize: 11.5,
        lineHeight: 1.45,
        whiteSpace: 'pre-line',
      }}
    >
      {children}
    </div>
  );
}

// ─── 6. Traction ─────────────────────────────────────────────────────────

function Traction() {
  return (
    <Shell align="left">
      <Eyebrow>Where we are</Eyebrow>
      <Headline size={64}>
        Live in <Cobalt>production</Cobalt>, today.
      </Headline>

      <div style={{ marginTop: 56, width: '100%', maxWidth: 1080 }}>
        <Row label="Live URL" value="aegisalert.xyz" detail="Vercel frontend + DigitalOcean Rust backend, TLS via certbot" />
        <Row label="On-chain repay" value="Verified" detail="Real USDC repay on Kamino mainnet, confirmed on-chain" />
        <Row label="Protocols indexed" value="Live" detail="Kamino, Save, MarginFi shipping today — more in flight, direct-RPC architecture" />
        <Row label="Telegram bot" value="Live" detail="Inline Repay buttons, /health · /positions · /rules" />
        <Row label="Built by" value="1 founder" detail="Solo build, ~10 weeks, shipped end-April 2026" />
      </div>
    </Shell>
  );
}

function Row({ label, value, detail }: { label: string; value: string; detail: string }) {
  return (
    <div style={{ display: 'grid', gridTemplateColumns: '1.2fr 1fr 2.6fr', alignItems: 'center', padding: '20px 0', borderBottom: '1px solid rgba(26,26,26,0.12)' }}>
      <span style={{ fontFamily: tokens.mono, fontSize: 11, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'rgba(26,26,26,0.5)' }}>{label}</span>
      <span style={{ fontFamily: tokens.sans, fontWeight: 700, fontSize: 26, letterSpacing: '-0.02em' }}>{value}</span>
      <span style={{ fontFamily: tokens.sans, fontSize: 14, color: 'var(--ink-2)' }}>{detail}</span>
    </div>
  );
}

// ─── 7. Close ────────────────────────────────────────────────────────────

function Close() {
  return (
    <Shell align="center">
      <SonarLogo size={140} ink="#1A1A1A" accent={COBALT} dot={RUST} animate />
      <h2
        style={{
          fontFamily: tokens.sans,
          fontWeight: 800,
          fontSize: 'clamp(40px, 6vw, 72px)',
          letterSpacing: '-0.03em',
          lineHeight: 1.05,
          maxWidth: 900,
          margin: '40px auto 0',
        }}
      >
        One dashboard. <Rust>Multiple venues.</Rust> Zero surprises.
      </h2>
      <div style={{ width: 60, height: 1, background: 'rgba(26,26,26,0.22)', margin: '40px auto 24px' }} />
      <div style={{ fontFamily: tokens.mono, fontSize: 12, letterSpacing: '0.18em', textTransform: 'uppercase', color: 'rgba(26,26,26,0.5)' }}>
        aegisalert.xyz
      </div>
    </Shell>
  );
}
