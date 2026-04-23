import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { api } from './api';
import type { GuardRuleWire, ScenarioRequest } from './types';

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
