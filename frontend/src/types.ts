export type Protocol = 'Kamino' | 'Save' | 'Marginfi';
export type Severity = 'Info' | 'Warning' | 'Critical';
export type Side = 'Collateral' | 'Borrow';

export type TriggerKind = 'HealthBelow' | 'LtvAbove' | 'DebtAboveUsd';
export type ActionKind = 'NotifyOnly' | 'AddCollateral' | 'RepayDebt' | 'Deleverage';

// Mock / legacy UI shape (per-asset rows). Kept for demo mode.
export interface Position {
  id: string;
  protocol: Protocol;
  obligation_address: string;
  asset_mint: string;
  asset_symbol: string;
  side: Side;
  amount: number;
  value_usd: number;
  updated_at: string;
}

export interface ProtocolLtv {
  protocol: Protocol;
  ltv: number;
  liquidation_threshold: number;
  total_collateral_usd: number;
  total_borrow_usd: number;
}

export interface HealthSnapshot {
  wallet: string;
  health_score: number;
  liquidation_buffer_usd: number;
  positions: Position[];
  protocol_ltvs: ProtocolLtv[];
  computed_at: string;
}

export interface Alert {
  id: string;
  wallet: string;
  severity: Severity;
  title: string;
  message: string;
  health_score: number;
  ltv: number;
  suggested_actions: string[];
  metadata: Record<string, unknown> & { protocol?: Protocol };
  created_at: string;
}

export interface GuardRule {
  id: string;
  wallet: string;
  protocol: Protocol | null;
  trigger_kind: TriggerKind;
  trigger_value: number;
  action_kind: ActionKind;
  action_token: string | null;
  action_amount_usd: number | null;
  max_usd_per_action: number;
  daily_limit_usd: number;
  cooldown_seconds: number;
  is_active: boolean;
  last_fired_at: string | null;
}

export interface SystemStatus {
  positions_cached: number;
  prices_loaded: number;
  wallets_monitored: number;
  bank_cache_size: number;
}

// ── Backend wire shapes (from aegis-api) ──

export interface PositionLeg {
  side: Side;
  asset_mint: string;
  asset_symbol: string;
  amount_native: number;
  amount_ui: number;
  value_usd: number;
  reserve_or_bank: string;
}

export interface PositionUpdate {
  pubkey: string;
  owner: string;
  protocol: string;
  collateral_usd: number;
  debt_usd: number;
  slot: number;
  legs?: PositionLeg[];
}

export interface ProtocolHealth {
  protocol: string;
  collateral_usd: number;
  debt_usd: number;
  ltv: number;
  liquidation_threshold: number;
}

export interface WalletRisk {
  wallet: string;
  health_score: number;
  severity: Severity;
  total_collateral_usd: number;
  total_debt_usd: number;
  ltv: number;
  liquidation_threshold: number;
  liquidation_buffer_usd: number;
  protocols: ProtocolHealth[];
  positions: PositionUpdate[];
}

export interface ScenarioRequest {
  wallet: string;
  collateral_shock_pct?: number;
  debt_shock_pct?: number;
  protocol_overrides: Record<string, number>;
}

export interface ScenarioResponse {
  base: WalletRisk;
  shocked: WalletRisk;
  collateral_change_usd: number;
  debt_change_usd: number;
  breached: boolean;
}

// Backend GuardRule shape (id/last_fired_at can be absent; protocol is string)
export interface GuardRuleWire {
  id?: string | null;
  wallet: string;
  protocol: string | null;
  trigger_kind: TriggerKind;
  trigger_value: number;
  action_kind: ActionKind;
  action_token: string | null;
  action_amount_usd: number | null;
  max_usd_per_action: number;
  daily_limit_usd: number;
  cooldown_seconds: number;
  is_active: boolean;
  created_at?: string | null;
  updated_at?: string | null;
  last_fired_at?: string | null;
}

export interface AlertRecordWire {
  id?: string | null;
  wallet: string;
  severity: Severity;
  title: string;
  message: string;
  health_score: number;
  ltv: number;
  suggested_actions: string[];
  metadata: Record<string, unknown>;
  created_at?: string | null;
}
