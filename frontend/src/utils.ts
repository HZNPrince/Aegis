import type {
  AlertRecordWire,
  GuardRule,
  GuardRuleWire,
  HealthSnapshot,
  Position,
  Protocol,
  ProtocolLtv,
  Severity,
  WalletRisk,
  Alert,
} from './types';

export const fmtUsd = (n: number | null | undefined, decimals = 2): string => {
  if (n === null || n === undefined) return '—';
  const abs = Math.abs(n);
  if (abs >= 1e6) return `$${(n / 1e6).toFixed(2)}M`;
  if (abs >= 1e3) return `$${(n / 1e3).toFixed(1)}k`;
  return `$${n.toLocaleString('en-US', {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  })}`;
};

export const fmtPct = (n: number): string => `${(n * 100).toFixed(1)}%`;

export const truncAddr = (addr: string | null | undefined): string =>
  addr ? `${addr.slice(0, 4)}…${addr.slice(-4)}` : '—';

export const timeAgo = (iso: string): string => {
  const secs = Math.floor((Date.now() - new Date(iso).getTime()) / 1000);
  if (secs < 60) return `${secs}s ago`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m ago`;
  if (secs < 86400) return `${Math.floor(secs / 3600)}h ago`;
  return `${Math.floor(secs / 86400)}d ago`;
};

export const healthColor = (score: number): string => {
  if (score >= 65) return '#7DA87B';
  if (score >= 40) return '#E4A853';
  return '#D9604E';
};

export const severityColor = (s: Severity): string =>
  ({ Info: '#7AA2C2', Warning: '#E4A853', Critical: '#D9604E' })[s] ?? '#7AA2C2';

const KNOWN_PROTOCOLS: Protocol[] = ['Kamino', 'Save', 'Marginfi'];
const normalizeProtocol = (raw: string): Protocol => {
  const match = KNOWN_PROTOCOLS.find((p) => p.toLowerCase() === raw.toLowerCase());
  return match ?? 'Kamino';
};

export const walletRiskToHealth = (risk: WalletRisk): HealthSnapshot => {
  const positions: Position[] = [];
  const updatedAt = new Date().toISOString();
  for (const p of risk.positions) {
    const protocol = normalizeProtocol(p.protocol);
    if (p.collateral_usd > 0) {
      positions.push({
        id: `${p.pubkey}:c`,
        protocol,
        obligation_address: p.pubkey,
        asset_mint: '',
        asset_symbol: truncAddr(p.pubkey),
        side: 'Collateral',
        amount: p.collateral_usd,
        value_usd: p.collateral_usd,
        updated_at: updatedAt,
      });
    }
    if (p.debt_usd > 0) {
      positions.push({
        id: `${p.pubkey}:b`,
        protocol,
        obligation_address: p.pubkey,
        asset_mint: '',
        asset_symbol: truncAddr(p.pubkey),
        side: 'Borrow',
        amount: p.debt_usd,
        value_usd: p.debt_usd,
        updated_at: updatedAt,
      });
    }
  }
  const protocol_ltvs: ProtocolLtv[] = risk.protocols.map((ph) => ({
    protocol: normalizeProtocol(ph.protocol),
    ltv: ph.ltv,
    liquidation_threshold: ph.liquidation_threshold,
    total_collateral_usd: ph.collateral_usd,
    total_borrow_usd: ph.debt_usd,
  }));
  return {
    wallet: risk.wallet,
    health_score: risk.health_score,
    liquidation_buffer_usd: risk.liquidation_buffer_usd,
    positions,
    protocol_ltvs,
    computed_at: updatedAt,
  };
};

export const alertWireToAlert = (w: AlertRecordWire, idx: number): Alert => ({
  id: w.id ?? `alert-${idx}`,
  wallet: w.wallet,
  severity: w.severity,
  title: w.title,
  message: w.message,
  health_score: w.health_score,
  ltv: w.ltv,
  suggested_actions: w.suggested_actions,
  metadata: w.metadata,
  created_at: w.created_at ?? new Date().toISOString(),
});

export const guardRuleWireToRule = (w: GuardRuleWire, idx: number): GuardRule => ({
  id: w.id ?? `rule-${idx}`,
  wallet: w.wallet,
  protocol: w.protocol ? normalizeProtocol(w.protocol) : null,
  trigger_kind: w.trigger_kind,
  trigger_value: w.trigger_value,
  action_kind: w.action_kind,
  action_token: w.action_token,
  action_amount_usd: w.action_amount_usd,
  max_usd_per_action: w.max_usd_per_action,
  daily_limit_usd: w.daily_limit_usd,
  cooldown_seconds: w.cooldown_seconds,
  is_active: w.is_active,
  last_fired_at: w.last_fired_at ?? null,
});
