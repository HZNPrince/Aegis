import { useWallet } from '@solana/wallet-adapter-react';
import { useMemo, useState } from 'react';
import { DEMO_MODE } from '../api';
import { Card, Chip, Eyebrow, ProtocolBadge, Reveal, Skeleton, tokens } from '../components/sonar';
import { useHealth, useTicker } from '../hooks';
import { MOCK_HEALTH } from '../mockData';
import type { Position, Protocol } from '../types';
import { fmtUsd, timeAgo, truncAddr, walletRiskToHealth } from '../utils';

const PROTOCOLS: Array<Protocol | 'All'> = ['All', 'Kamino', 'Save', 'Marginfi'];
const SIDES: Array<Position['side'] | 'All'> = ['All', 'Collateral', 'Borrow'];

export function Positions() {
  const { publicKey } = useWallet();
  const wallet = publicKey?.toBase58() ?? null;
  const useLive = !DEMO_MODE && !!wallet;
  const healthQ = useHealth(useLive ? wallet : null);
  const tickerQ = useTicker();

  const data = useMemo(
    () => (useLive && healthQ.data ? walletRiskToHealth(healthQ.data) : MOCK_HEALTH),
    [healthQ.data, useLive],
  );

  const [protocol, setProtocol] = useState<Protocol | 'All'>('All');
  const [side, setSide] = useState<Position['side'] | 'All'>('All');

  const rows = data.positions.filter((p) => {
    if (p.amount <= 0 || p.value_usd < 0.01) return false;
    if (protocol !== 'All' && p.protocol !== protocol) return false;
    if (side !== 'All' && p.side !== side) return false;
    return true;
  });

  return (
    <main style={{ padding: '64px 28px 72px', maxWidth: 1180, margin: '0 auto' }}>
      <Reveal>
        <div style={{ display: 'flex', alignItems: 'end', justifyContent: 'space-between', gap: 24, marginBottom: 28 }}>
          <div>
            <Eyebrow>Positions</Eyebrow>
            <h1 style={{ fontFamily: tokens.serif, fontSize: 52, fontWeight: 500, letterSpacing: '-0.02em', marginTop: 10 }}>
              Every lending leg, in one place.
            </h1>
            <p style={{ fontFamily: tokens.sans, fontSize: 15, color: tokens.ink2, marginTop: 10, maxWidth: 620, lineHeight: 1.55 }}>
              Track collateral, debt, prices, and liquidation exposure across Kamino, Save, and MarginFi.
            </p>
          </div>
          <Chip tone={data.health_score < 45 ? 'critical' : data.health_score < 70 ? 'watch' : 'healthy'}>
            Weighted health {data.health_score}
          </Chip>
        </div>
      </Reveal>

      <Card pad={0} style={{ overflow: 'hidden' }}>
        <div style={{ padding: 18, borderBottom: `1px solid ${tokens.lineSoft}`, display: 'flex', alignItems: 'center', gap: 10, flexWrap: 'wrap' }}>
          {PROTOCOLS.map((p) => (
            <FilterButton key={p} active={protocol === p} onClick={() => setProtocol(p)}>
              {p}
            </FilterButton>
          ))}
          <span style={{ width: 1, height: 24, background: tokens.line, margin: '0 4px' }} />
          {SIDES.map((s) => (
            <FilterButton key={s} active={side === s} onClick={() => setSide(s)}>
              {s}
            </FilterButton>
          ))}
        </div>

        {healthQ.isLoading && useLive ? (
          <div style={{ padding: 22, display: 'grid', gap: 10 }}>
            {[1, 2, 3, 4].map((i) => <Skeleton key={i} height={58} />)}
          </div>
        ) : rows.length === 0 ? (
          <div style={{ padding: 72, textAlign: 'center' }}>
            <Eyebrow>No rows</Eyebrow>
            <p style={{ fontFamily: tokens.serif, fontSize: 28, marginTop: 10 }}>No positions match this filter.</p>
          </div>
        ) : (
          <div style={{ overflowX: 'auto' }}>
            <table style={{ width: '100%', borderCollapse: 'collapse', minWidth: 820 }}>
              <thead>
                <tr>
                  {['Protocol', 'Asset', 'Side', 'Amount', 'Value', 'Price', 'Updated'].map((h) => (
                    <th key={h} style={{ textAlign: 'left', padding: '14px 18px', borderBottom: `1px solid ${tokens.lineSoft}` }}>
                      <Eyebrow>{h}</Eyebrow>
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {rows.map((pos, idx) => {
                  const tick = pos.asset_mint ? tickerQ.data?.[pos.asset_mint] : undefined;
                  const accent = pos.protocol === 'Kamino' ? '#FF7A3D' : pos.protocol === 'Marginfi' ? '#8B5CD7' : pos.protocol === 'Save' ? '#5A6B47' : tokens.ink2;
                  const sideTint = pos.side === 'Borrow' ? 'rgba(196,69,54,0.035)' : 'rgba(90,107,71,0.035)';
                  const zebra = idx % 2 === 0 ? 'transparent' : 'color-mix(in oklab, var(--ink) 2%, transparent)';
                  return (
                    <tr
                      key={pos.id}
                      style={{
                        borderBottom: `1px solid ${tokens.lineSoft}`,
                        background: `linear-gradient(90deg, ${sideTint} 0%, ${zebra} 60%)`,
                      }}
                    >
                      <td style={{ padding: '16px 18px', borderLeft: `3px solid ${accent}` }}>
                        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                          <ProtocolBadge protocol={pos.protocol} size={24} />
                          <span style={{ fontFamily: tokens.sans, fontWeight: 500 }}>{pos.protocol}</span>
                        </div>
                      </td>
                      <td style={{ padding: '16px 18px' }}>
                        <div style={{ fontFamily: tokens.mono, fontSize: 14 }}>{pos.asset_symbol}</div>
                        <div style={{ fontFamily: tokens.mono, fontSize: 11, color: 'color-mix(in oklab, var(--ink) 52%, transparent)', marginTop: 3 }}>
                          {truncAddr(pos.asset_mint || pos.obligation_address)}
                        </div>
                      </td>
                      <td style={{ padding: '16px 18px' }}>
                        <Chip tone={pos.side === 'Collateral' ? 'healthy' : 'watch'}>{pos.side}</Chip>
                      </td>
                      <td style={{ padding: '16px 18px', fontFamily: tokens.mono }}>{pos.amount.toLocaleString('en-US', { maximumFractionDigits: 4 })}</td>
                      <td style={{ padding: '16px 18px', fontFamily: tokens.mono, fontWeight: 600 }}>{fmtUsd(pos.value_usd)}</td>
                      <td style={{ padding: '16px 18px', fontFamily: tokens.mono }}>
                        {tick ? `$${tick.price.toLocaleString('en-US', { maximumFractionDigits: 4 })}` : '—'}
                      </td>
                      <td style={{ padding: '16px 18px', color: tokens.ink2, fontSize: 13 }}>{timeAgo(pos.updated_at)}</td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </Card>
    </main>
  );
}

function FilterButton({ active, children, onClick }: { active: boolean; children: string; onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      style={{
        minHeight: 36,
        padding: '0 13px',
        borderRadius: 8,
        border: `1px solid ${active ? tokens.ink : tokens.line}`,
        background: active ? tokens.ink : 'var(--surface-1)',
        color: active ? tokens.paper : tokens.ink,
        fontFamily: tokens.sans,
        fontSize: 13,
        cursor: 'pointer',
        transition: 'background 0.15s, transform 0.12s',
      }}
      onMouseEnter={(e) => { if (!active) e.currentTarget.style.background = 'var(--surface-2)'; }}
      onMouseLeave={(e) => { if (!active) e.currentTarget.style.background = 'var(--surface-1)'; }}
    >
      {children}
    </button>
  );
}
