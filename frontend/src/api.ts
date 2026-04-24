import type {
  AlertRecordWire,
  BuildRepayBody,
  BuildRepayResponse,
  GuardRuleWire,
  IntentRow,
  IntentStatus,
  ScenarioRequest,
  ScenarioResponse,
  SystemStatus,
  WalletRisk,
} from './types';

export const API_URL = import.meta.env.VITE_API_URL ?? 'http://localhost:7878';
export const DEMO_MODE = import.meta.env.VITE_DEMO_MODE === 'true';

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${API_URL}${path}`, {
    headers: { 'content-type': 'application/json', ...(init?.headers ?? {}) },
    ...init,
  });
  if (!res.ok) {
    throw new Error(`${res.status} ${res.statusText}`);
  }
  // 204 No Content on intent updates
  if (res.status === 204) return undefined as T;
  return (await res.json()) as T;
}

export const api = {
  status: () => request<SystemStatus>('/api/status'),
  prices: () => request<Record<string, number>>('/api/prices'),
  ticker: () =>
    request<Record<string, { price: number; change_24h: number | null }>>(
      '/api/ticker',
    ),
  linkWallet: (wallet: string) =>
    request<{ wallet: string; backfilled_positions: number }>(
      `/api/wallets/${wallet}`,
      { method: 'POST' },
    ),
  health: (wallet: string) => request<WalletRisk>(`/api/health/${wallet}`),
  alerts: (wallet: string) => request<AlertRecordWire[]>(`/api/alerts/${wallet}`),
  guardRules: (wallet: string) =>
    request<GuardRuleWire[]>(`/api/guard-rules/${wallet}`),
  upsertGuardRule: (rule: GuardRuleWire) =>
    request<GuardRuleWire>('/api/guard-rules', {
      method: 'POST',
      body: JSON.stringify(rule),
    }),
  scenario: (req: ScenarioRequest) =>
    request<ScenarioResponse>('/api/scenario', {
      method: 'POST',
      body: JSON.stringify(req),
    }),
  buildRepay: (body: BuildRepayBody) =>
    request<BuildRepayResponse>('/api/execute/repay', {
      method: 'POST',
      body: JSON.stringify(body),
    }),
  listIntents: (wallet: string) => request<IntentRow[]>(`/api/intents/${wallet}`),
  updateIntent: (
    intentId: string,
    body: { status: IntentStatus; signature?: string; error?: string },
  ) =>
    request<void>(`/api/intents/${intentId}/status`, {
      method: 'PATCH',
      body: JSON.stringify(body),
    }),
};
