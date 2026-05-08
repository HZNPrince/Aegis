import { useWallet } from '@solana/wallet-adapter-react';
import { motion } from 'framer-motion';
import { useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { DEMO_MODE } from '../api';
import { ArcGauge, Button, Card, Chip, Eyebrow, ProtocolBadge, Reveal, Skeleton, Stat, tokens } from '../components/sonar';
import { RepayModal } from '../components/RepayModal';
import { useAlerts, useHealth, useTicker, useLinkWallet } from '../hooks';
import { MOCK_ALERTS, MOCK_HEALTH } from '../mockData';
import type { Alert, Position, ProtocolLtv, Severity } from '../types';
import { alertWireToAlert, fmtUsd, timeAgo, walletRiskToHealth } from '../utils';

export function Dashboard() {
  const { publicKey } = useWallet();
  const wallet = publicKey?.toBase58() ?? null;
  const useLive = !DEMO_MODE && !!wallet;
  const healthQ = useHealth(useLive ? wallet : null);
  const alertsQ = useAlerts(useLive ? wallet : null);
  const tickerQ = useTicker();
  const linkWalletMut = useLinkWallet();
  const [repayPosition, setRepayPosition] = useState<Position | null>(null);

  const data = useMemo(
    () => (useLive && healthQ.data ? walletRiskToHealth(healthQ.data) : MOCK_HEALTH),
    [healthQ.data, useLive],
  );
  const alerts = useMemo(
    () => (useLive && alertsQ.data ? alertsQ.data.map(alertWireToAlert) : MOCK_ALERTS),
    [alertsQ.data, useLive],
  );

  const totalCollateral = data.positions.filter((p) => p.side === 'Collateral').reduce((s, p) => s + p.value_usd, 0);
  const totalBorrow = data.positions.filter((p) => p.side === 'Borrow').reduce((s, p) => s + p.value_usd, 0);
  const net = totalCollateral - totalBorrow;
  const watchCount = data.protocol_ltvs.filter((p) => p.ltv > p.liquidation_threshold * 0.75).length;
  const criticalCount = data.protocol_ltvs.filter((p) => p.ltv > p.liquidation_threshold * 0.9).length;
  const avgThreshold = data.protocol_ltvs.reduce((s, p) => s + p.liquidation_threshold * p.total_collateral_usd, 0) / Math.max(totalCollateral, 1);
  const firstBorrow = data.positions.find((p) => p.side === 'Borrow' && p.reserve_or_bank);

  return (
    <main style={{ padding: '64px 28px 72px', maxWidth: 1180, margin: '0 auto' }}>
      <Reveal>
        <div style={{ display: 'flex', justifyContent: 'space-between', gap: 28, alignItems: 'end', marginBottom: 26 }}>
          <div>
            <h1 style={{ fontFamily: tokens.sans, fontSize: 34, fontWeight: 700, letterSpacing: '-0.025em', margin: 0 }}>
              Overview
            </h1>
            <p style={{ fontFamily: tokens.sans, fontSize: 16, color: 'color-mix(in oklab, var(--ink) 57%, transparent)', lineHeight: 1.5, maxWidth: 620, marginTop: 8 }}>
              {data.positions.length} position legs across {data.protocol_ltvs.length} protocols. Updated from indexed Solana lending accounts.
            </p>
          </div>
          <Chip tone={criticalCount > 0 ? 'critical' : watchCount > 0 ? 'watch' : 'healthy'}>
            {criticalCount > 0 ? `${criticalCount} critical` : watchCount > 0 ? `${watchCount} watch` : 'Healthy'}
          </Chip>
        </div>
      </Reveal>

      <Reveal delay={0.05}>
        <section
          style={{
            display: 'grid',
            gridTemplateColumns: '1fr 1fr 1fr 1fr',
            border: `1px solid ${tokens.lineSoft}`,
            borderRadius: 14,
            overflow: 'hidden',
            background: 'linear-gradient(180deg, var(--surface-2) 0%, var(--surface-1) 100%)',
            boxShadow: 'var(--shadow-md)',
            marginBottom: 22,
          }}
        >
          <Stat label="Net position" value={fmtUsd(net)} delta="+2.4% 7d" tone="pos" />
          <Stat label="Total supplied" value={fmtUsd(totalCollateral)} />
          <Stat label="Total borrowed" value={fmtUsd(totalBorrow)} />
          <Stat label="Weighted health" value={(data.health_score / 50).toFixed(2)} delta={watchCount ? 'Watch' : 'Healthy'} tone={watchCount ? 'warn' : 'pos'} />
        </section>
      </Reveal>

      <Reveal delay={0.1}>
        <ProtocolOverview protocols={data.protocol_ltvs} positions={data.positions} />
      </Reveal>

      {watchCount || criticalCount ? (
        <Card pad={18} raised style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 28, borderColor: criticalCount ? 'rgba(196,69,54,0.38)' : 'rgba(198,148,35,0.42)' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 12, fontFamily: tokens.sans }}>
            <span style={{ color: criticalCount ? tokens.rust : '#8a6a1f', fontFamily: tokens.mono }}>●</span>
            <span>{criticalCount + watchCount} positions need attention.</span>
          </div>
          <Link to="/positions" style={{ textDecoration: 'none' }}>
            <Button>Review →</Button>
          </Link>
        </Card>
      ) : null}

      <div style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1.35fr) minmax(320px, 0.65fr)', gap: 22 }}>
        <div style={{ display: 'grid', gap: 20 }}>
          <Card pad={0} style={{ overflow: 'hidden' }}>
            <div style={{ padding: '18px 20px', borderBottom: `1px solid ${tokens.lineSoft}`, display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <Eyebrow>Positions</Eyebrow>
              <RefreshButton
                refetching={healthQ.isFetching || linkWalletMut.isPending}
                onClick={() => {
                  if (useLive && wallet) {
                    linkWalletMut.mutate(wallet, {
                      onSuccess: () => {
                        healthQ.refetch();
                        alertsQ.refetch();
                      }
                    });
                  } else {
                    healthQ.refetch();
                    alertsQ.refetch();
                  }
                }}
              />
            </div>
            {healthQ.isLoading && useLive ? (
              <div style={{ padding: 20, display: 'grid', gap: 10 }}>
                {[1, 2, 3].map((i) => <Skeleton key={i} height={58} />)}
              </div>
            ) : (
              <PositionGroups
                positions={data.positions}
                ticker={tickerQ.data}
                useLive={useLive}
                onRepay={setRepayPosition}
              />
            )}
          </Card>
        </div>

        <aside style={{ display: 'grid', gap: 20 }}>
          <Card pad={24} style={{ textAlign: 'center', background: 'var(--surface-1)' }}>
            <Eyebrow>Health · 30d</Eyebrow>
            <HealthLine value={(data.health_score / 50).toFixed(2)} />
          </Card>
          <RiskSummary alerts={alerts} />
        </aside>
      </div>

      {repayPosition && (
        <RepayModal
          position={repayPosition}
          totalDebtUsd={totalBorrow}
          totalCollateralUsd={totalCollateral}
          liquidationThreshold={avgThreshold || 0.85}
          onClose={() => setRepayPosition(null)}
        />
      )}

      {!useLive && firstBorrow ? null : null}
    </main>
  );
}

type TickerMap = Record<string, { price: number; change_24h: number | null }> | undefined;

const PROTOCOL_ACCENT: Record<string, string> = {
  Kamino: '#FF7A3D',
  Marginfi: '#8B5CD7',
  Save: '#5A6B47',
};

function PositionGroups({
  positions,
  ticker,
  useLive,
  onRepay,
}: {
  positions: Position[];
  ticker: TickerMap;
  useLive: boolean;
  onRepay: (p: Position) => void;
}) {
  const collateral = positions.filter((p) => p.side === 'Collateral').slice(0, 4);
  const borrow = positions.filter((p) => p.side === 'Borrow').slice(0, 4);

  return (
    <div>
      <PositionGroup
        title="Collateral"
        accent="var(--moss)"
        rows={collateral}
        ticker={ticker}
        useLive={useLive}
        onRepay={onRepay}
        empty="No collateral legs."
      />
      <PositionGroup
        title="Borrow"
        accent="var(--rust)"
        rows={borrow}
        ticker={ticker}
        useLive={useLive}
        onRepay={onRepay}
        empty="No borrow legs — you're not at risk."
        topDivider
      />
    </div>
  );
}

function PositionGroup({
  title,
  accent,
  rows,
  ticker,
  useLive,
  onRepay,
  empty,
  topDivider,
}: {
  title: string;
  accent: string;
  rows: Position[];
  ticker: TickerMap;
  useLive: boolean;
  onRepay: (p: Position) => void;
  empty: string;
  topDivider?: boolean;
}) {
  return (
    <section
      style={{
        padding: '14px 18px 18px',
        borderTop: topDivider ? `1px solid ${tokens.lineSoft}` : 'none',
      }}
    >
      {/* Section header */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 10,
          fontFamily: tokens.mono,
          fontSize: 11,
          letterSpacing: '0.18em',
          textTransform: 'uppercase',
          color: 'color-mix(in oklab, var(--ink) 65%, transparent)',
          marginBottom: 10,
        }}
      >
        <span
          aria-hidden
          style={{ width: 6, height: 6, borderRadius: '50%', background: accent }}
        />
        {title}
        <span
          style={{
            color: 'color-mix(in oklab, var(--ink) 38%, transparent)',
            marginLeft: 'auto',
            fontFamily: tokens.mono,
            fontSize: 11,
          }}
        >
          {rows.length}
        </span>
      </div>

      {rows.length === 0 ? (
        <div
          style={{
            fontFamily: tokens.sans,
            fontSize: 13,
            color: 'color-mix(in oklab, var(--ink) 45%, transparent)',
            padding: '14px 4px',
          }}
        >
          {empty}
        </div>
      ) : (
        <>
          {/* Column headers — Protocol | Asset | Amount | USD Value | Updated | (action) */}
          <div
            style={{
              display: 'grid',
              gridTemplateColumns: '38px 1.4fr 1fr 1fr 0.85fr auto',
              alignItems: 'center',
              gap: 12,
              padding: '0 12px 8px',
              fontFamily: tokens.mono,
              fontSize: 10,
              letterSpacing: '0.16em',
              textTransform: 'uppercase',
              color: 'color-mix(in oklab, var(--ink) 45%, transparent)',
            }}
          >
            <span />
            <span>Asset</span>
            <span style={{ textAlign: 'right' }}>Amount</span>
            <span style={{ textAlign: 'right' }}>USD Value</span>
            <span style={{ textAlign: 'right' }}>Updated</span>
            <span />
          </div>
          <div style={{ display: 'grid', gap: 6 }}>
            {rows.map((pos) => (
              <PositionRow
                key={pos.id}
                pos={pos}
                ticker={ticker}
                useLive={useLive}
                onRepay={onRepay}
              />
            ))}
          </div>
        </>
      )}
    </section>
  );
}

function PositionRow({
  pos,
  ticker,
  useLive,
  onRepay,
}: {
  pos: Position;
  ticker: TickerMap;
  useLive: boolean;
  onRepay: (p: Position) => void;
}) {
  const accent = PROTOCOL_ACCENT[pos.protocol] ?? tokens.ink2;
  const tick = pos.asset_mint ? ticker?.[pos.asset_mint] : undefined;
  const priceLabel = tick?.price
    ? `$${tick.price.toLocaleString('en-US', { maximumFractionDigits: tick.price < 1 ? 6 : 4 })}`
    : '—';
  const explorerUrl = pos.obligation_address
    ? `https://solscan.io/account/${pos.obligation_address}`
    : null;

  // Trim trailing zeros for token amounts so 25.500000 reads as 25.5.
  const amountLabel = Number(pos.amount.toFixed(6))
    .toString()
    .replace(/(\.\d*?[1-9])0+$/, '$1')
    .replace(/\.0+$/, '');

  return (
    <div
      style={{
        display: 'grid',
        gridTemplateColumns: '38px 1.4fr 1fr 1fr 0.85fr auto',
        alignItems: 'center',
        gap: 12,
        padding: '10px 12px',
        borderRadius: 10,
        borderLeft: `3px solid ${accent}`,
        background: 'color-mix(in oklab, var(--ink) 3%, transparent)',
        transition: 'background 0.15s',
      }}
      onMouseEnter={(e) => { e.currentTarget.style.background = 'color-mix(in oklab, var(--ink) 6%, transparent)'; }}
      onMouseLeave={(e) => { e.currentTarget.style.background = 'color-mix(in oklab, var(--ink) 3%, transparent)'; }}
    >
      {/* Protocol — logo only, native browser tooltip on hover */}
      <span title={pos.protocol} aria-label={pos.protocol} style={{ display: 'inline-flex' }}>
        <ProtocolBadge protocol={pos.protocol} size={28} />
      </span>

      {/* Asset + live price */}
      <div style={{ minWidth: 0 }}>
        {explorerUrl ? (
          <a
            href={explorerUrl}
            target="_blank"
            rel="noreferrer"
            title={`Open obligation on Solscan — ${pos.obligation_address}`}
            style={{
              fontFamily: tokens.sans,
              fontWeight: 600,
              fontSize: 14,
              color: tokens.ink,
              textDecoration: 'none',
              display: 'inline-flex',
              alignItems: 'center',
              gap: 5,
            }}
            onMouseEnter={(e) => { e.currentTarget.style.color = 'var(--cobalt)'; }}
            onMouseLeave={(e) => { e.currentTarget.style.color = tokens.ink; }}
          >
            {pos.asset_symbol}
            <svg
              width="10"
              height="10"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2.4"
              strokeLinecap="round"
              strokeLinejoin="round"
              aria-hidden
              style={{ opacity: 0.55 }}
            >
              <path d="M7 17 17 7M9 7h8v8" />
            </svg>
          </a>
        ) : (
          <div style={{ fontFamily: tokens.sans, fontWeight: 600, fontSize: 14, color: tokens.ink }}>
            {pos.asset_symbol}
          </div>
        )}
        <div
          style={{
            fontFamily: tokens.mono,
            fontSize: 11,
            color: 'color-mix(in oklab, var(--ink) 50%, transparent)',
            marginTop: 2,
            fontVariantNumeric: 'tabular-nums',
          }}
        >
          {priceLabel}
          {tick?.change_24h != null ? (
            <span
              style={{
                marginLeft: 6,
                color: tick.change_24h >= 0 ? 'var(--moss)' : 'var(--rust)',
              }}
            >
              {tick.change_24h >= 0 ? '+' : ''}
              {(tick.change_24h * 100).toFixed(2)}%
            </span>
          ) : null}
        </div>
      </div>

      {/* Amount (token-native) */}
      <div
        style={{
          fontFamily: tokens.mono,
          fontSize: 13,
          fontVariantNumeric: 'tabular-nums',
          color: 'color-mix(in oklab, var(--ink) 80%, transparent)',
          textAlign: 'right',
        }}
      >
        {amountLabel}
      </div>

      {/* USD value */}
      <div
        style={{
          fontFamily: tokens.mono,
          fontWeight: 700,
          fontSize: 14,
          fontVariantNumeric: 'tabular-nums',
          color: tokens.ink,
          textAlign: 'right',
        }}
      >
        {fmtUsd(pos.value_usd)}
      </div>

      {/* Last updated — relative time, hover for absolute */}
      <div
        title={pos.updated_at ? new Date(pos.updated_at).toLocaleString() : ''}
        style={{
          fontFamily: tokens.mono,
          fontSize: 11.5,
          color: 'color-mix(in oklab, var(--ink) 55%, transparent)',
          textAlign: 'right',
          fontVariantNumeric: 'tabular-nums',
        }}
      >
        {pos.updated_at ? timeAgo(pos.updated_at) : '—'}
      </div>

      {/* Action — only for repayable borrows */}
      <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
        {pos.side === 'Borrow' && pos.reserve_or_bank && pos.amount > 0 && useLive ? (
          <RepayPill onClick={() => onRepay(pos)} />
        ) : null}
      </div>
    </div>
  );
}

function RepayPill({ onClick }: { onClick: () => void }) {
  return (
    <motion.button
      type="button"
      onClick={onClick}
      whileHover={{ y: -1 }}
      whileTap={{ y: 0, scale: 0.97 }}
      transition={{ duration: 0.12 }}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: 6,
        fontFamily: tokens.sans,
        fontSize: 12.5,
        fontWeight: 600,
        padding: '6px 14px',
        borderRadius: 999,
        border: `1px solid color-mix(in oklab, var(--rust) 55%, transparent)`,
        background: 'color-mix(in oklab, var(--rust) 14%, transparent)',
        color: 'var(--rust)',
        cursor: 'pointer',
        letterSpacing: '0.01em',
        transition: 'background 0.18s, border-color 0.18s, box-shadow 0.18s',
      }}
      onMouseEnter={(e) => {
        e.currentTarget.style.background = 'color-mix(in oklab, var(--rust) 22%, transparent)';
        e.currentTarget.style.boxShadow = '0 0 0 4px color-mix(in oklab, var(--rust) 8%, transparent)';
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.background = 'color-mix(in oklab, var(--rust) 14%, transparent)';
        e.currentTarget.style.boxShadow = 'none';
      }}
    >
      <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
        <path d="M3 12a9 9 0 1 0 3.95-7.46" />
        <polyline points="3 4 3 10 9 10" />
      </svg>
      Repay
    </motion.button>
  );
}

function RefreshButton({ refetching, onClick }: { refetching: boolean; onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={refetching}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: 6,
        fontFamily: tokens.sans,
        fontSize: 12.5,
        fontWeight: 500,
        padding: '6px 11px',
        borderRadius: 999,
        border: `1px solid ${tokens.line}`,
        background: 'var(--surface-2)',
        color: tokens.ink,
        cursor: refetching ? 'progress' : 'pointer',
        transition: 'background 0.15s, transform 0.12s',
      }}
      onMouseEnter={(e) => { if (!refetching) e.currentTarget.style.background = 'var(--surface-3)'; }}
      onMouseLeave={(e) => { e.currentTarget.style.background = 'var(--surface-2)'; }}
    >
      <motion.span
        aria-hidden
        animate={refetching ? { rotate: 360 } : { rotate: 0 }}
        transition={refetching ? { duration: 0.9, repeat: Infinity, ease: 'linear' } : { duration: 0.2 }}
        style={{ display: 'inline-flex' }}
      >
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round">
          <path d="M21 12a9 9 0 0 1-15.36 6.36M3 12a9 9 0 0 1 15.36-6.36" />
          <polyline points="21 4 21 9 16 9" />
          <polyline points="3 20 3 15 8 15" />
        </svg>
      </motion.span>
      {refetching ? 'Refreshing…' : 'Refresh'}
    </button>
  );
}

function ProtocolOverview({ protocols, positions }: { protocols: ProtocolLtv[]; positions: Position[] }) {
  const accentMap: Record<string, string> = { Kamino: '#FF7A3D', Save: '#5A6B47', Marginfi: '#8B5CD7' };
  return (
    <section style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 14, marginBottom: 22 }}>
      {protocols.map((p) => {
        const accent = accentMap[p.protocol] ?? tokens.cobalt;
        const legs = positions.filter((pos) => pos.protocol === p.protocol).length;
        return (
          <motion.div
            key={p.protocol}
            whileHover={{ y: -3 }}
            transition={{ type: 'spring', stiffness: 320, damping: 26 }}
          >
            <Link to={`/protocol/${encodeURIComponent(p.protocol)}`} style={{ textDecoration: 'none', color: 'inherit' }}>
              <Card
                pad={20}
                style={{
                  position: 'relative',
                  overflow: 'hidden',
                  cursor: 'pointer',
                  minHeight: 168,
                  background: `linear-gradient(180deg, var(--surface-2) 0%, var(--surface-1) 100%)`,
                  borderTop: `2px solid ${accent}`,
                  boxShadow: 'var(--shadow-md)',
                }}
              >
                <div style={{ position: 'absolute', inset: 0, background: `radial-gradient(180px 120px at 100% 0%, ${accent}22, transparent 70%)`, pointerEvents: 'none' }} />
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'start', position: 'relative' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                    <ProtocolBadge protocol={p.protocol} size={36} />
                    <div>
                      <div style={{ fontFamily: tokens.serif, fontSize: 22, fontWeight: 500, lineHeight: 1.05 }}>{p.protocol}</div>
                      <div style={{ fontFamily: tokens.mono, fontSize: 11, color: 'rgba(128,128,128,0.78)', marginTop: 4 }}>
                        {legs} leg{legs === 1 ? '' : 's'}
                      </div>
                    </div>
                  </div>
                  <ArcGauge value={Math.min(1, p.ltv / Math.max(p.liquidation_threshold, 0.01))} size={84} />
                </div>
                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 14, marginTop: 14, position: 'relative' }}>
                  <div>
                    <Eyebrow>Collateral</Eyebrow>
                    <div style={{ fontFamily: tokens.mono, fontSize: 17, fontWeight: 600, marginTop: 4 }}>{fmtUsd(p.total_collateral_usd)}</div>
                  </div>
                  <div>
                    <Eyebrow>Borrowed</Eyebrow>
                    <div style={{ fontFamily: tokens.mono, fontSize: 17, fontWeight: 600, marginTop: 4 }}>{fmtUsd(p.total_borrow_usd)}</div>
                  </div>
                </div>
                <div style={{ marginTop: 14, height: 5, background: 'var(--surface-3)', borderRadius: 3, position: 'relative', overflow: 'hidden' }}>
                  <motion.div
                    initial={{ width: 0 }}
                    animate={{ width: `${Math.min(100, (p.ltv / p.liquidation_threshold) * 100)}%` }}
                    transition={{ duration: 0.9, ease: 'easeOut' }}
                    style={{
                      height: '100%',
                      background: p.ltv > p.liquidation_threshold * 0.9 ? tokens.rust : p.ltv > p.liquidation_threshold * 0.75 ? '#c69423' : tokens.moss,
                    }}
                  />
                </div>
              </Card>
            </Link>
          </motion.div>
        );
      })}
    </section>
  );
}

function RiskSummary({ alerts }: { alerts: Alert[] }) {
  const dots: Record<Severity, string> = { Info: 'color-mix(in oklab, var(--ink) 55%, transparent)', Warning: '#c69423', Critical: 'var(--rust)' };
  return (
    <Card pad={24}>
      <Eyebrow>AI risk summary</Eyebrow>
      <div style={{ display: 'grid', gap: 16, marginTop: 20 }}>
        {alerts.slice(0, 3).map((a) => (
          <div key={a.id} style={{ display: 'grid', gridTemplateColumns: '14px 1fr', gap: 10 }}>
            <span style={{ color: dots[a.severity], fontFamily: tokens.mono }}>●</span>
            <div>
              <div style={{ fontFamily: tokens.sans, fontSize: 15, lineHeight: 1.45 }}>{a.title}</div>
              <div style={{ fontFamily: tokens.mono, fontSize: 11, color: 'color-mix(in oklab, var(--ink) 52%, transparent)', marginTop: 5 }}>{timeAgo(a.created_at)}</div>
            </div>
          </div>
        ))}
      </div>
      <Link to="/alerts" style={{ display: 'inline-flex', marginTop: 20, color: tokens.ink2, fontFamily: tokens.sans, textDecoration: 'none' }}>
        Why? →
      </Link>
    </Card>
  );
}

/** Enhanced health chart with gradient fill and red pulsing dot at the head */
function HealthLine({ value }: { value: string }) {
  // SVG path data for a health trend line
  const pathD = 'M8 66 C46 28 78 34 112 72 S184 94 222 58 284 50 312 88';
  // End point of the curve
  const endX = 312;
  const endY = 88;

  return (
    <div style={{ paddingTop: 20 }}>
      <svg width="100%" height="120" viewBox="0 0 320 120" fill="none" aria-label="30 day health trend">
        {/* Gradient definition */}
        <defs>
          <linearGradient id="healthGradient" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="var(--cobalt)" stopOpacity="0.2" />
            <stop offset="100%" stopColor="var(--cobalt)" stopOpacity="0.02" />
          </linearGradient>
        </defs>

        {/* Gradient fill area under the curve */}
        <path
          d={`${pathD} L 312 120 L 8 120 Z`}
          fill="url(#healthGradient)"
        />

        {/* Main line */}
        <motion.path
          d={pathD}
          stroke="var(--cobalt)"
          strokeWidth="2.5"
          fill="none"
          strokeLinecap="round"
          initial={{ pathLength: 0 }}
          animate={{ pathLength: 1 }}
          transition={{ duration: 1.2, ease: 'easeOut' }}
        />

        {/* Pulsing red ring at the head */}
        <motion.circle
          cx={endX}
          cy={endY}
          r="6"
          fill="none"
          stroke="var(--rust)"
          strokeWidth="1.5"
          initial={{ r: 4, opacity: 0.6 }}
          animate={{ r: 12, opacity: 0 }}
          transition={{ duration: 1.5, repeat: Infinity, ease: 'easeOut' }}
        />

        {/* Solid red dot at the head */}
        <circle
          cx={endX}
          cy={endY}
          r="4"
          fill="var(--rust)"
        />
      </svg>
      <div style={{ display: 'flex', justifyContent: 'space-between', fontFamily: tokens.sans, color: 'color-mix(in oklab, var(--ink) 57%, transparent)' }}>
        <span>Now</span>
        <strong style={{ fontFamily: tokens.mono, color: tokens.ink }}>{value}</strong>
      </div>
    </div>
  );
}
