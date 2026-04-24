import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useConnection, useWallet } from '@solana/wallet-adapter-react';
import { VersionedTransaction } from '@solana/web3.js';
import { api } from './api';
import type { BuildRepayBody, GuardRuleWire, ScenarioRequest } from './types';

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

export function useIntents(wallet: string | null) {
  return useQuery({
    queryKey: ['intents', wallet],
    queryFn: () => api.listIntents(wallet!),
    enabled: !!wallet,
    refetchInterval: 8_000,
  });
}

/// Full repay flow: ask API to build the unsigned tx + persist the intent,
/// hand the serialized bytes to the wallet, submit the signed tx, then
/// PATCH the intent status so the dashboard reflects progress.
export function useRepayIntent() {
  const qc = useQueryClient();
  const { connection } = useConnection();
  const { sendTransaction, publicKey } = useWallet();

  return useMutation({
    mutationFn: async (body: BuildRepayBody) => {
      if (!publicKey) throw new Error('wallet not connected');

      const { intent_id, unsigned } = await api.buildRepay(body);

      const txBytes = Uint8Array.from(atob(unsigned.tx_base64), (c) => c.charCodeAt(0));
      const tx = VersionedTransaction.deserialize(txBytes);

      let signature: string;
      try {
        signature = await sendTransaction(tx, connection, {
          skipPreflight: false,
          maxRetries: 3,
        });
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        await api.updateIntent(intent_id, { status: 'cancelled', error: msg });
        throw err;
      }

      await api.updateIntent(intent_id, { status: 'submitted', signature });

      // Best-effort confirmation — non-fatal if it times out.
      try {
        await connection.confirmTransaction(
          {
            signature,
            blockhash: tx.message.recentBlockhash!,
            lastValidBlockHeight: unsigned.last_valid_block_height,
          },
          'confirmed',
        );
        await api.updateIntent(intent_id, { status: 'confirmed', signature });
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        await api.updateIntent(intent_id, { status: 'submitted', signature, error: msg });
      }

      return { intent_id, signature };
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['intents'] });
      qc.invalidateQueries({ queryKey: ['health'] });
    },
  });
}

export function useCancelIntent() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (intentId: string) =>
      api.updateIntent(intentId, { status: 'cancelled' }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['intents'] });
    },
  });
}
