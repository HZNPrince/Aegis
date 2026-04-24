import { useWallet } from '@solana/wallet-adapter-react';
import { Fragment, useEffect, useMemo, useState } from 'react';
import { DEMO_MODE } from '../api';
import { HealthGauge } from '../components/HealthGauge';
import {
  Card,
  EmptyState,
  LtvBar,
  PriceTickerRail,
  ProtocolBadge,
  PulseDot,
  SectionLabel,
  SeverityBadge,
  SidePill,
  Skeleton,
} from '../components/ui';
import { useAlerts, useHealth, useIntents, useRepayIntent, useCancelIntent } from '../hooks';
import { MOCK_ALERTS, MOCK_HEALTH } from '../mockData';
import type { Alert, IntentRow, Position, ProtocolLtv } from '../types';
import { alertWireToAlert, fmtUsd, timeAgo, truncAddr, walletRiskToHealth } from '../utils';

export function Dashboard() {
  const { publicKey } = useWallet();
  const wallet = publicKey?.toBase58() ?? null;
  const useLive = !DEMO_MODE && !!wallet;

  const healthQ = useHealth(useLive ? wallet : null);
  const alertsQ = useAlerts(useLive ? wallet : null);
  const intentsQ = useIntents(useLive ? wallet : null);

  const data = useMemo(
    () => (useLive && healthQ.data ? walletRiskToHealth(healthQ.data) : MOCK_HEALTH),
    [useLive, healthQ.data],
  );
  const alerts = useMemo(
    () => (useLive && alertsQ.data ? alertsQ.data.map(alertWireToAlert) : MOCK_ALERTS),
    [useLive, alertsQ.data],
  );

  const loading = useLive ? healthQ.isLoading : false;
  const [elapsed, setElapsed] = useState(0);
  const [expandedRow, setExpandedRow] = useState<string | null>(null);

  useEffect(() => {
    setElapsed(0);
  }, [data.computed_at]);

  useEffect(() => {
    if (loading) return;
    const iv = setInterval(() => setElapsed((e) => e + 10), 10000);
    return () => clearInterval(iv);
  }, [loading]);

  const totalCollateral = data.positions
    .filter((p) => p.side === 'Collateral')
    .reduce((s, p) => s + p.value_usd, 0);
  const totalBorrow = data.positions
    .filter((p) => p.side === 'Borrow')
    .reduce((s, p) => s + p.value_usd, 0);

  return (
    <div style={{ padding: '88px 28px 60px', maxWidth: 1140, margin: '0 auto' }}>
      <Card style={{ marginBottom: 20, padding: '40px 40px 36px' }}>
        {loading ? (
          <div style={{ display: 'flex', gap: 40, alignItems: 'center' }}>
            <Skeleton width={220} height={220} radius="50%" />
            <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 14 }}>
              <Skeleton width="40%" height={20} />
              <Skeleton width="60%" height={48} />
              <Skeleton width="35%" height={16} />
            </div>
          </div>
        ) : (
          <div style={{ display: 'flex', gap: 40, alignItems: 'center', flexWrap: 'wrap' }}>
            <HealthGauge score={data.health_score} size={200} />
            <div style={{ flex: 1, minWidth: 200 }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
                <PulseDot />
                <span
                  style={{
                    fontFamily: "'Inter', sans-serif",
                    fontSize: 12,
                    color: 'rgba(245,244,239,0.4)',
                    letterSpacing: '0.04em',
                  }}
                >
                  updated {elapsed}s ago
                </span>
              </div>
              <div
                style={{
                  fontFamily: "'Fraunces', serif",
                  fontSize: 52,
                  fontWeight: 600,
                  color: '#F5F4EF',
                  letterSpacing: '-0.03em',
                  lineHeight: 1,
                }}
              >
                {fmtUsd(data.liquidation_buffer_usd)}
              </div>
              <div
                style={{
                  fontFamily: "'Inter', sans-serif",
                  fontSize: 14,
                  color: 'rgba(245,244,239,0.45)',
                  marginTop: 6,
                  marginBottom: 24,
                }}
              >
                Liquidation buffer
              </div>
              <div style={{ display: 'flex', gap: 32, flexWrap: 'wrap' }}>
                {[
                  { label: 'Total Collateral', value: fmtUsd(totalCollateral) },
                  { label: 'Total Borrow', value: fmtUsd(totalBorrow) },
                  { label: 'Net Value', value: fmtUsd(totalCollateral - totalBorrow) },
                ].map(({ label, value }) => (
                  <div key={label}>
                    <div
                      style={{
                        fontFamily: "'Inter', sans-serif",
                        fontSize: 11,
                        color: 'rgba(245,244,239,0.35)',
                        letterSpacing: '0.06em',
                        textTransform: 'uppercase',
                        marginBottom: 4,
                      }}
                    >
                      {label}
                    </div>
                    <div
                      style={{
                        fontFamily: "'JetBrains Mono', monospace",
                        fontSize: 18,
                        fontWeight: 600,
                        color: '#F5F4EF',
                      }}
                    >
                      {value}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}
      </Card>

      <div
        style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(3, 1fr)',
          gap: 12,
          marginBottom: 20,
        }}
      >
        {loading
          ? [1, 2, 3].map((i) => (
              <Card key={i} style={{ padding: 20 }}>
                <Skeleton width="50%" height={16} style={{ marginBottom: 12 }} />
                <Skeleton width="80%" height={28} style={{ marginBottom: 8 }} />
                <Skeleton width="100%" height={4} radius={4} />
              </Card>
            ))
          : data.protocol_ltvs.map((p) => <ProtocolCard key={p.protocol} data={p} />)}
      </div>

      <Card style={{ marginBottom: 20, padding: 0, overflow: 'hidden' }}>
        <div
          style={{
            padding: '20px 24px 16px',
            borderBottom: '1px solid rgba(255,255,255,0.06)',
          }}
        >
          <SectionLabel>Positions</SectionLabel>
        </div>
        {loading ? (
          <div style={{ padding: 24, display: 'flex', flexDirection: 'column', gap: 12 }}>
            {[1, 2, 3, 4, 5, 6].map((i) => (
              <Skeleton key={i} height={48} />
            ))}
          </div>
        ) : (
          <table style={{ width: '100%', borderCollapse: 'collapse' }}>
            <thead>
              <tr style={{ borderBottom: '1px solid rgba(255,255,255,0.06)' }}>
                {['Protocol', 'Asset', 'Side', 'Amount', 'USD Value', 'Updated', ''].map((h) => (
                  <th
                    key={h}
                    style={{
                      padding: '10px 16px',
                      textAlign: 'left',
                      fontFamily: "'Inter', sans-serif",
                      fontSize: 11,
                      fontWeight: 600,
                      color: 'rgba(245,244,239,0.3)',
                      letterSpacing: '0.07em',
                      textTransform: 'uppercase',
                    }}
                  >
                    {h}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {data.positions.map((pos) => (
                <Fragment key={pos.id}>
                  <tr
                    onClick={() => setExpandedRow(expandedRow === pos.id ? null : pos.id)}
                    style={{
                      borderBottom:
                        expandedRow === pos.id ? 'none' : '1px solid rgba(255,255,255,0.04)',
                      cursor: 'pointer',
                      transition: 'background 0.15s',
                      background: expandedRow === pos.id ? 'rgba(255,255,255,0.03)' : 'none',
                    }}
                    onMouseEnter={(e) =>
                      (e.currentTarget.style.background = 'rgba(255,255,255,0.03)')
                    }
                    onMouseLeave={(e) =>
                      (e.currentTarget.style.background =
                        expandedRow === pos.id ? 'rgba(255,255,255,0.03)' : 'none')
                    }
                  >
                    <td style={{ padding: '14px 16px' }}>
                      <ProtocolBadge protocol={pos.protocol} />
                    </td>
                    <td
                      style={{
                        padding: '14px 16px',
                        fontFamily: "'Inter', sans-serif",
                        fontSize: 14,
                        fontWeight: 600,
                        color: '#F5F4EF',
                      }}
                    >
                      {pos.asset_symbol}
                    </td>
                    <td style={{ padding: '14px 16px' }}>
                      <SidePill side={pos.side} />
                    </td>
                    <td
                      style={{
                        padding: '14px 16px',
                        fontFamily: "'JetBrains Mono', monospace",
                        fontSize: 13,
                        color: 'rgba(245,244,239,0.7)',
                      }}
                    >
                      {pos.amount.toLocaleString('en-US', { maximumFractionDigits: 4 })}
                    </td>
                    <td
                      style={{
                        padding: '14px 16px',
                        fontFamily: "'JetBrains Mono', monospace",
                        fontSize: 13,
                        color: '#F5F4EF',
                        fontWeight: 600,
                      }}
                    >
                      {fmtUsd(pos.value_usd)}
                    </td>
                    <td
                      style={{
                        padding: '14px 16px',
                        fontFamily: "'Inter', sans-serif",
                        fontSize: 12,
                        color: 'rgba(245,244,239,0.35)',
                      }}
                    >
                      {timeAgo(pos.updated_at)}
                    </td>
                    <td
                      style={{
                        padding: '14px 16px',
                        color: 'rgba(245,244,239,0.25)',
                        fontSize: 12,
                      }}
                    >
                      {expandedRow === pos.id ? '▲' : '▼'}
                    </td>
                  </tr>
                  {expandedRow === pos.id && (
                    <tr
                      style={{
                        borderBottom: '1px solid rgba(255,255,255,0.04)',
                        background: 'rgba(255,255,255,0.02)',
                      }}
                    >
                      <td colSpan={7} style={{ padding: '8px 16px 16px 16px' }}>
                        <div
                          style={{
                            display: 'flex',
                            gap: 28,
                            alignItems: 'center',
                            flexWrap: 'wrap',
                          }}
                        >
                          <div>
                            <div
                              style={{
                                fontFamily: "'Inter', sans-serif",
                                fontSize: 11,
                                color: 'rgba(245,244,239,0.35)',
                                letterSpacing: '0.07em',
                                textTransform: 'uppercase',
                                marginBottom: 4,
                              }}
                            >
                              Obligation Address
                            </div>
                            <span
                              style={{
                                fontFamily: "'JetBrains Mono', monospace",
                                fontSize: 12,
                                color: 'rgba(245,244,239,0.6)',
                              }}
                            >
                              {truncAddr(pos.obligation_address)}
                            </span>
                            <button
                              onClick={() =>
                                void navigator.clipboard?.writeText(pos.obligation_address)
                              }
                              style={{
                                background: 'none',
                                border: 'none',
                                cursor: 'pointer',
                                color: 'rgba(245,244,239,0.3)',
                                fontSize: 12,
                                marginLeft: 6,
                              }}
                            >
                              ⎘
                            </button>
                          </div>
                          <a
                            href={`https://explorer.solana.com/address/${pos.obligation_address}`}
                            target="_blank"
                            rel="noreferrer"
                            style={{
                              fontFamily: "'Inter', sans-serif",
                              fontSize: 12,
                              color: '#D97757',
                              textDecoration: 'none',
                              display: 'flex',
                              alignItems: 'center',
                              gap: 4,
                            }}
                            onClick={(e) => e.stopPropagation()}
                          >
                            View on Explorer ↗
                          </a>
                          {useLive &&
                            pos.side === 'Borrow' &&
                            pos.reserve_or_bank &&
                            pos.amount_native ? (
                            <RepayButton pos={pos} wallet={wallet!} />
                          ) : null}
                        </div>
                      </td>
                    </tr>
                  )}
                </Fragment>
              ))}
            </tbody>
          </table>
        )}
      </Card>

      {!loading && <PriceTickerRail />}

      {useLive && intentsQ.data && intentsQ.data.length > 0 ? (
        <div style={{ marginTop: 20 }}>
          <SectionLabel>Pending Actions</SectionLabel>
          <IntentsPanel intents={intentsQ.data} />
        </div>
      ) : null}

      <div style={{ marginTop: 20 }}>
        <SectionLabel>Recent Alerts</SectionLabel>
        {loading ? (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
            {[1, 2, 3].map((i) => (
              <Skeleton key={i} height={72} />
            ))}
          </div>
        ) : alerts.length === 0 ? (
          <EmptyState />
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
            {alerts.slice(0, 5).map((alert) => (
              <AlertRow key={alert.id} alert={alert} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function ProtocolCard({ data }: { data: ProtocolLtv }) {
  const ltvPct = (data.ltv * 100).toFixed(1);
  const color =
    data.ltv > data.liquidation_threshold * 0.9
      ? '#D9604E'
      : data.ltv > data.liquidation_threshold * 0.75
        ? '#E4A853'
        : '#7DA87B';
  return (
    <Card style={{ padding: 20 }}>
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'flex-start',
          marginBottom: 14,
        }}
      >
        <ProtocolBadge protocol={data.protocol} />
        <span
          style={{
            fontFamily: "'JetBrains Mono', monospace",
            fontSize: 20,
            fontWeight: 700,
            color,
            letterSpacing: '-0.02em',
          }}
        >
          {ltvPct}%
        </span>
      </div>
      <div style={{ marginBottom: 10 }}>
        <LtvBar ltv={data.ltv} threshold={data.liquidation_threshold} />
      </div>
      <div
        style={{
          fontFamily: "'Inter', sans-serif",
          fontSize: 10,
          color: 'rgba(245,244,239,0.3)',
          marginBottom: 12,
          textAlign: 'right',
        }}
      >
        liq. threshold {(data.liquidation_threshold * 100).toFixed(0)}%
      </div>
      <div style={{ display: 'flex', justifyContent: 'space-between' }}>
        <div>
          <div
            style={{
              fontFamily: "'Inter', sans-serif",
              fontSize: 10,
              color: 'rgba(245,244,239,0.3)',
              textTransform: 'uppercase',
              letterSpacing: '0.06em',
              marginBottom: 3,
            }}
          >
            Collateral
          </div>
          <div
            style={{
              fontFamily: "'JetBrains Mono', monospace",
              fontSize: 13,
              color: '#7DA87B',
            }}
          >
            {fmtUsd(data.total_collateral_usd)}
          </div>
        </div>
        <div style={{ textAlign: 'right' }}>
          <div
            style={{
              fontFamily: "'Inter', sans-serif",
              fontSize: 10,
              color: 'rgba(245,244,239,0.3)',
              textTransform: 'uppercase',
              letterSpacing: '0.06em',
              marginBottom: 3,
            }}
          >
            Borrow
          </div>
          <div
            style={{
              fontFamily: "'JetBrains Mono', monospace",
              fontSize: 13,
              color: '#E4A853',
            }}
          >
            {fmtUsd(data.total_borrow_usd)}
          </div>
        </div>
      </div>
    </Card>
  );
}

function AlertRow({ alert }: { alert: Alert }) {
  const c = { Info: '#7AA2C2', Warning: '#E4A853', Critical: '#D9604E' }[alert.severity];
  return (
    <div
      style={{
        background: '#2A2826',
        borderRadius: 14,
        border: '1px solid rgba(255,255,255,0.06)',
        borderLeft: `3px solid ${c}`,
        padding: '14px 18px',
        display: 'flex',
        gap: 14,
        alignItems: 'flex-start',
      }}
    >
      <SeverityBadge severity={alert.severity} />
      <div style={{ flex: 1, minWidth: 0 }}>
        <div
          style={{
            fontFamily: "'Inter', sans-serif",
            fontSize: 13,
            fontWeight: 600,
            color: '#F5F4EF',
            marginBottom: 2,
          }}
        >
          {alert.title}
        </div>
        <div
          style={{
            fontFamily: "'Inter', sans-serif",
            fontSize: 12,
            color: 'rgba(245,244,239,0.4)',
            whiteSpace: 'nowrap',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
          }}
        >
          {alert.message}
        </div>
      </div>
      <div
        style={{
          fontFamily: "'Inter', sans-serif",
          fontSize: 11,
          color: 'rgba(245,244,239,0.25)',
          flexShrink: 0,
        }}
      >
        {timeAgo(alert.created_at)}
      </div>
    </div>
  );
}

function RepayButton({ pos, wallet }: { pos: Position; wallet: string }) {
  const repay = useRepayIntent();
  const onClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (!pos.reserve_or_bank || !pos.amount_native) return;
    repay.mutate({
      wallet,
      obligation_or_account: pos.obligation_address,
      protocol: pos.protocol,
      reserve_or_bank: pos.reserve_or_bank,
      mint: pos.asset_mint,
      amount_native: pos.amount_native,
    });
  };
  const disabled = repay.isPending;
  const errMsg = repay.error instanceof Error ? repay.error.message : null;
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
      <button
        onClick={onClick}
        disabled={disabled}
        style={{
          padding: '6px 14px',
          borderRadius: 8,
          border: '1px solid rgba(217,119,87,0.4)',
          background: disabled ? 'rgba(217,119,87,0.15)' : 'rgba(217,119,87,0.22)',
          color: '#F5E1D6',
          fontFamily: "'Inter', sans-serif",
          fontSize: 12,
          fontWeight: 600,
          letterSpacing: '0.02em',
          cursor: disabled ? 'default' : 'pointer',
        }}
      >
        {disabled
          ? 'Signing…'
          : repay.isSuccess
            ? `Repay queued ✓`
            : `Repay ${pos.asset_symbol}`}
      </button>
      {errMsg ? (
        <span
          style={{
            fontFamily: "'Inter', sans-serif",
            fontSize: 11,
            color: '#D9604E',
            maxWidth: 320,
          }}
        >
          {errMsg}
        </span>
      ) : null}
    </div>
  );
}

function IntentsPanel({ intents }: { intents: IntentRow[] }) {
  const active = intents.filter(
    (i) => i.status === 'pending' || i.status === 'submitted',
  );
  const recent = intents
    .filter((i) => !(i.status === 'pending' || i.status === 'submitted'))
    .slice(0, 3);
  const rows = [...active, ...recent];
  if (rows.length === 0) return null;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
      {rows.map((it) => (
        <IntentRowView key={it.id} it={it} />
      ))}
    </div>
  );
}

function IntentRowView({ it }: { it: IntentRow }) {
  const cancel = useCancelIntent();
  const color =
    it.status === 'confirmed'
      ? '#7DA87B'
      : it.status === 'submitted' || it.status === 'pending'
        ? '#E4A853'
        : '#D9604E';
  // Allow clearing submitted intents too — sometimes the confirm loop can't
  // finalize (RPC timeout, blockhash lapse) and the user wants it out of the list.
  const canCancel = it.status === 'pending' || it.status === 'submitted';
  return (
    <div
      style={{
        background: '#2A2826',
        borderRadius: 12,
        border: '1px solid rgba(255,255,255,0.06)',
        borderLeft: `3px solid ${color}`,
        padding: '12px 16px',
        display: 'flex',
        gap: 16,
        alignItems: 'center',
        fontFamily: "'Inter', sans-serif",
        fontSize: 12,
      }}
    >
      <span
        style={{
          textTransform: 'uppercase',
          letterSpacing: '0.08em',
          color,
          fontWeight: 700,
          fontSize: 10,
          minWidth: 70,
        }}
      >
        {it.status}
      </span>
      <span style={{ color: '#F5F4EF', fontWeight: 600 }}>
        Repay {it.amount_native.toLocaleString()} · {it.protocol}
      </span>
      <span
        style={{
          fontFamily: "'JetBrains Mono', monospace",
          color: 'rgba(245,244,239,0.5)',
          fontSize: 11,
        }}
      >
        {truncAddr(it.mint)}
      </span>
      <div style={{ flex: 1 }} />
      {it.signature ? (
        <a
          href={`https://explorer.solana.com/tx/${it.signature}`}
          target="_blank"
          rel="noreferrer"
          style={{ color: '#D97757', textDecoration: 'none', fontSize: 11 }}
        >
          tx ↗
        </a>
      ) : null}
      <span style={{ color: 'rgba(245,244,239,0.35)', fontSize: 11 }}>
        {timeAgo(it.created_at)}
      </span>
      {canCancel ? (
        <button
          onClick={() => cancel.mutate(it.id)}
          style={{
            background: 'none',
            border: '1px solid rgba(255,255,255,0.12)',
            borderRadius: 6,
            padding: '4px 10px',
            color: 'rgba(245,244,239,0.6)',
            fontSize: 11,
            cursor: 'pointer',
          }}
        >
          dismiss
        </button>
      ) : null}
    </div>
  );
}
