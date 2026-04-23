import { ConnectionProvider, WalletProvider } from '@solana/wallet-adapter-react';
import { WalletModalProvider } from '@solana/wallet-adapter-react-ui';
import type { ReactNode } from 'react';
import { useMemo } from 'react';

import '@solana/wallet-adapter-react-ui/styles.css';

const DEFAULT_RPC = 'https://api.mainnet-beta.solana.com';

export function WalletAdapter({ children }: { children: ReactNode }) {
  const endpoint = useMemo(
    () => import.meta.env.VITE_SOLANA_RPC ?? DEFAULT_RPC,
    [],
  );
  // Standard-wallet autodiscovery covers Phantom, Backpack, Solflare, etc.
  const wallets = useMemo(() => [], []);

  return (
    <ConnectionProvider endpoint={endpoint}>
      <WalletProvider wallets={wallets} autoConnect>
        <WalletModalProvider>{children}</WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  );
}
