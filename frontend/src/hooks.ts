// React Query hooks — wraps API calls for data fetching and mutations.
// Provides automatic caching, refetch intervals, and state management for all API operations.

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { api } from './api';
import type { CreateRepayIntentRequest, GuardRuleWire, ScenarioRequest } from './types';

export function useStatus() {
  return useQuery({
    queryKey: ['status'],
    queryFn: api.status,
    refetchInterval: 10_000,
  });
}

export function usePrices() {
  return useQuery({
    queryKey: ['prices'],
    queryFn: api.prices,
    refetchInterval: 15_000,
  });
}

export function useTicker() {
  return useQuery({
    queryKey: ['ticker'],
    queryFn: api.ticker,
    refetchInterval: 15_000,
  });
}

export function useHealth(wallet: string | null) {
  return useQuery({
    queryKey: ['health', wallet],
    queryFn: () => api.health(wallet!),
    enabled: !!wallet,
    refetchInterval: 10_000,
  });
}

export function useAlerts(wallet: string | null) {
  return useQuery({
    queryKey: ['alerts', wallet],
    queryFn: () => api.alerts(wallet!),
    enabled: !!wallet,
    refetchInterval: 20_000,
  });
}

export function useGuardRules(wallet: string | null) {
  return useQuery({
    queryKey: ['guard-rules', wallet],
    queryFn: () => api.guardRules(wallet!),
    enabled: !!wallet,
  });
}

export function useUpsertGuardRule() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (rule: GuardRuleWire) => api.upsertGuardRule(rule),
    onSuccess: (saved) => {
      qc.invalidateQueries({ queryKey: ['guard-rules', saved.wallet] });
    },
  });
}

export function useScenario() {
  return useMutation({
    mutationFn: (req: ScenarioRequest) => api.scenario(req),
  });
}

export function useLinkWallet() {
  return useMutation({
    mutationFn: (wallet: string) => api.linkWallet(wallet),
  });
}

export function useWalletSettings(wallet: string | null, pollMs?: number) {
  return useQuery({
    queryKey: ['wallet-settings', wallet],
    queryFn: () => api.getWalletSettings(wallet!),
    enabled: !!wallet,
    refetchInterval: pollMs,
  });
}

export function useCreateTelegramLinkCode() {
  return useMutation({
    mutationFn: (wallet: string) => api.createTelegramLinkCode(wallet),
  });
}

export function useUnlinkTelegram() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (wallet: string) => api.unlinkTelegram(wallet),
    onSuccess: (_, wallet) =>
      qc.invalidateQueries({ queryKey: ['wallet-settings', wallet] }),
  });
}

export function useLinkTelegram() {
  return useMutation({
    mutationFn: ({ wallet, chatId }: { wallet: string; chatId: number }) =>
      api.linkTelegram(wallet, chatId),
  });
}

export function useDeleteGuardRule() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (ruleId: string) => api.deleteGuardRule(ruleId),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['guard-rules'] });
    },
  });
}

export function useLinkEmail() {
  return useMutation({
    mutationFn: ({ wallet, email }: { wallet: string; email: string }) =>
      api.linkEmail(wallet, email),
  });
}

export function useCreateRepayIntent() {
  return useMutation({
    mutationFn: (req: CreateRepayIntentRequest) => api.createRepayIntent(req),
  });
}

export function useSubmitIntent() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ intentId, signedTxBase64 }: { intentId: string; signedTxBase64: string }) =>
      api.submitIntent(intentId, signedTxBase64),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['health'] });
    },
  });
}

export function useIntent(intentId: string | null) {
  return useQuery({
    queryKey: ['intent', intentId],
    queryFn: () => api.getIntent(intentId!),
    enabled: !!intentId,
    refetchInterval: (q) =>
      q.state.data?.status === 'submitted' ? 4_000 : false,
  });
}
