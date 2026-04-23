import type {
  Alert,
  GuardRule,
  HealthSnapshot,
  SystemStatus,
} from './types';

export const MOCK_WALLET_SHORT = '7xKX…mPq3';
export const MOCK_WALLET_FULL = '7xKXaBc123def456gHiJkLmNoPqRsTuVwXyZmPq3';

export const MOCK_PRICES: Record<string, number> = {
  SOL: 142.3,
  USDC: 1.0,
  mSOL: 152.4,
  JitoSOL: 154.2,
  USDT: 1.0,
  BONK: 0.0000182,
  WIF: 1.84,
  JUP: 0.92,
  RAY: 2.31,
};

export const MOCK_HEALTH: HealthSnapshot = {
  wallet: MOCK_WALLET_FULL,
  health_score: 73,
  liquidation_buffer_usd: 4250,
  positions: [
    { id: 'p1', protocol: 'Kamino', obligation_address: 'KmNoBLiGaTioN1aBcDeF2gHiJ', asset_mint: 'So11111111111111111111111111111111111111112', asset_symbol: 'SOL', side: 'Collateral', amount: 25.5, value_usd: 3625.65, updated_at: '2026-04-20T10:00:01Z' },
    { id: 'p2', protocol: 'Kamino', obligation_address: 'KmNoBLiGaTioN1aBcDeF2gHiJ', asset_mint: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', asset_symbol: 'USDC', side: 'Borrow', amount: 2000, value_usd: 2000.0, updated_at: '2026-04-20T10:00:01Z' },
    { id: 'p3', protocol: 'Save', obligation_address: 'SaVeObLiGaTioN2cDeFgHiJkL', asset_mint: 'mSoLzYCxHdYgdzU16g5QSh3i5K3z1Zbk7Jo47ZvZEuQ', asset_symbol: 'mSOL', side: 'Collateral', amount: 10, value_usd: 1524.0, updated_at: '2026-04-20T09:59:58Z' },
    { id: 'p4', protocol: 'Save', obligation_address: 'SaVeObLiGaTioN2cDeFgHiJkL', asset_mint: 'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB', asset_symbol: 'USDT', side: 'Borrow', amount: 800, value_usd: 800.0, updated_at: '2026-04-20T09:59:58Z' },
    { id: 'p5', protocol: 'Marginfi', obligation_address: 'MfObLiGaTioN3eFgHiJkLmNoP', asset_mint: 'J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn', asset_symbol: 'JitoSOL', side: 'Collateral', amount: 5, value_usd: 771.0, updated_at: '2026-04-20T10:00:03Z' },
    { id: 'p6', protocol: 'Marginfi', obligation_address: 'MfObLiGaTioN3eFgHiJkLmNoP', asset_mint: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', asset_symbol: 'USDC', side: 'Borrow', amount: 500, value_usd: 500.0, updated_at: '2026-04-20T10:00:03Z' },
  ],
  protocol_ltvs: [
    { protocol: 'Kamino', ltv: 0.552, liquidation_threshold: 0.75, total_collateral_usd: 3625.65, total_borrow_usd: 2000.0 },
    { protocol: 'Save', ltv: 0.525, liquidation_threshold: 0.7, total_collateral_usd: 1524.0, total_borrow_usd: 800.0 },
    { protocol: 'Marginfi', ltv: 0.649, liquidation_threshold: 0.8, total_collateral_usd: 771.0, total_borrow_usd: 500.0 },
  ],
  computed_at: '2026-04-20T10:00:03Z',
};

export const MOCK_ALERTS: Alert[] = [
  { id: 'a1', wallet: MOCK_WALLET_FULL, severity: 'Warning', title: 'Marginfi LTV approaching danger zone', message: 'Your JitoSOL/USDC position on Marginfi has an LTV of 64.9%, closing in on the 80% liquidation threshold. A 10% drop in SOL price would push you into the critical range. Consider adding collateral or partially repaying your USDC borrow.', health_score: 73, ltv: 0.649, suggested_actions: ['Add JitoSOL collateral', 'Repay 200 USDC', 'View position'], metadata: { protocol: 'Marginfi', triggered_price: 154.2 }, created_at: '2026-04-20T09:45:00Z' },
  { id: 'a2', wallet: MOCK_WALLET_FULL, severity: 'Info', title: 'Kamino collateral value increased', message: 'SOL price rose 3.2% in the last hour, improving your Kamino health. Your liquidation buffer is now $4,250. No action needed.', health_score: 73, ltv: 0.552, suggested_actions: ['View Kamino position'], metadata: { protocol: 'Kamino', price_change: 0.032 }, created_at: '2026-04-20T08:30:00Z' },
  { id: 'a3', wallet: MOCK_WALLET_FULL, severity: 'Warning', title: 'Save protocol oracle lag detected', message: 'Save Finance oracle prices are lagging by ~45 seconds. Your on-chain LTV may not reflect current market prices. Monitor closely.', health_score: 71, ltv: 0.525, suggested_actions: ['Check Save dashboard', 'Set tighter guard rule'], metadata: { protocol: 'Save', oracle_lag_ms: 45000 }, created_at: '2026-04-20T07:15:00Z' },
  { id: 'a4', wallet: MOCK_WALLET_FULL, severity: 'Critical', title: 'Health score dropped below 60 — action taken', message: "Health score fell to 57 at 03:12 UTC as SOL dropped 8.4%. Your Guard Rule 'Repay USDC on health < 60' automatically repaid 500 USDC on Kamino. Health recovered to 68.", health_score: 57, ltv: 0.71, suggested_actions: ['Review transaction', 'Adjust guard rule'], metadata: { protocol: 'Kamino', auto_action: 'RepayDebt', tx: '5kLmN...xYz9' }, created_at: '2026-04-20T03:12:00Z' },
  { id: 'a5', wallet: MOCK_WALLET_FULL, severity: 'Info', title: 'New position detected on Marginfi', message: 'A new JitoSOL collateral deposit of 5 tokens ($771) was detected on Marginfi. Aegis is now monitoring this position.', health_score: 80, ltv: 0.45, suggested_actions: ['Set guard rule for Marginfi'], metadata: { protocol: 'Marginfi' }, created_at: '2026-04-19T22:00:00Z' },
];

export const MOCK_GUARD_RULES: GuardRule[] = [
  { id: 'gr1', wallet: MOCK_WALLET_FULL, protocol: null, trigger_kind: 'HealthBelow', trigger_value: 60, action_kind: 'RepayDebt', action_token: 'USDC', action_amount_usd: 500, max_usd_per_action: 500, daily_limit_usd: 1000, cooldown_seconds: 3600, is_active: true, last_fired_at: '2026-04-20T03:12:00Z' },
  { id: 'gr2', wallet: MOCK_WALLET_FULL, protocol: 'Marginfi', trigger_kind: 'LtvAbove', trigger_value: 0.7, action_kind: 'NotifyOnly', action_token: null, action_amount_usd: null, max_usd_per_action: 0, daily_limit_usd: 0, cooldown_seconds: 1800, is_active: true, last_fired_at: null },
  { id: 'gr3', wallet: MOCK_WALLET_FULL, protocol: 'Kamino', trigger_kind: 'DebtAboveUsd', trigger_value: 2500, action_kind: 'AddCollateral', action_token: 'SOL', action_amount_usd: 300, max_usd_per_action: 300, daily_limit_usd: 600, cooldown_seconds: 7200, is_active: false, last_fired_at: null },
];

export const MOCK_STATUS: SystemStatus = {
  positions_cached: 6,
  prices_loaded: 9,
  wallets_monitored: 1,
  bank_cache_size: 42,
};
