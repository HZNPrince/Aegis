// API client — thin typed wrapper around fetch calls to Aegis backend.
// All paths are relative to API_URL env var; handles request/response serialization and error handling.

import type {
  AlertRecordWire,
  CreateRepayIntentRequest,
  CreateRepayIntentResponse,
  GuardRuleWire,
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
    request<GuardRuleWire[]>(`/api/wallets/${wallet}/guard-rules`),
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
  getWalletSettings: (wallet: string) =>
    request<{ telegram_chat_id: number | null }>(`/api/wallets/${wallet}`),
  linkTelegram: (wallet: string, chatId: number) =>
    request<void>(`/api/wallets/${wallet}/telegram`, {
      method: 'PATCH',
      body: JSON.stringify({ chat_id: chatId }),
    }),
  unlinkTelegram: (wallet: string) =>
    request<void>(`/api/wallets/${wallet}/telegram`, { method: 'DELETE' }),
  createTelegramLinkCode: (wallet: string) =>
    request<{ code: string; expires_at: string; deep_link: string }>(
      `/api/wallets/${wallet}/telegram/code`,
      { method: 'POST' },
    ),
  deleteGuardRule: (ruleId: string) =>
    request<void>(`/api/guard-rules/${ruleId}`, { method: 'DELETE' }),
  linkEmail: (wallet: string, email: string) =>
    request<void>(`/api/wallets/${wallet}/email`, {
      method: 'PATCH',
      body: JSON.stringify({ email }),
    }),
  createRepayIntent: (req: CreateRepayIntentRequest) =>
    request<CreateRepayIntentResponse>('/api/intents/repay', {
      method: 'POST',
      body: JSON.stringify(req),
    }),
  submitIntent: (intentId: string, signedTxBase64: string) =>
    request<{ signature: string }>(`/api/intents/${intentId}/submit`, {
      method: 'POST',
      body: JSON.stringify({ signed_tx_base64: signedTxBase64 }),
    }),
  getIntent: (intentId: string) =>
    request<IntentStatus>(`/api/intents/${intentId}`),
};
