"use client";

import React, { useMemo, useEffect } from "react";
import { ConnectionProvider, WalletProvider, useWallet } from "@solana/wallet-adapter-react";
import { PhantomWalletAdapter, SolflareWalletAdapter } from "@solana/wallet-adapter-wallets";
import { WalletModalProvider } from "@solana/wallet-adapter-react-ui";

import "@solana/wallet-adapter-react-ui/styles.css";

// Use mainnet for platform launches (Pump.fun, Bags.fm, Raydium).
// The dashboard still reads devnet treasury state directly via fetchTreasuryState.
const RPC_ENDPOINT = "https://api.mainnet-beta.solana.com";

function AutoConnectHandler({ children }: { children: React.ReactNode }) {
  // No auto-connect on page load. The user must click "Connect Wallet" explicitly.
  // Previous auto-connect caused Phantom to redirect/open on every page visit.
  return <>{children}</>;
}

export function WalletContextProvider({ children }: { children: React.ReactNode }) {
  const wallets = useMemo(
    () => [new PhantomWalletAdapter(), new SolflareWalletAdapter()],
    [],
  );

  return (
    <ConnectionProvider endpoint={RPC_ENDPOINT}>
      <WalletProvider wallets={wallets} autoConnect={false} onError={() => {}}>
        <WalletModalProvider>
          <AutoConnectHandler>{children}</AutoConnectHandler>
        </WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  );
}
