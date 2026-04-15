"use client";

import React, { useMemo, useEffect } from "react";
import { ConnectionProvider, WalletProvider, useWallet } from "@solana/wallet-adapter-react";
import { PhantomWalletAdapter, SolflareWalletAdapter } from "@solana/wallet-adapter-wallets";
import { WalletModalProvider } from "@solana/wallet-adapter-react-ui";

import "@solana/wallet-adapter-react-ui/styles.css";

const DEVNET_RPC = "https://api.devnet.solana.com";

function AutoConnectHandler({ children }: { children: React.ReactNode }) {
  const { wallet, connected, connect } = useWallet();

  // When a wallet is selected from the modal, connect immediately.
  // This replaces `autoConnect` which fires on page load and fails
  // when no prior wallet is cached, leaving the adapter in a bad state.
  useEffect(() => {
    if (wallet && !connected) {
      connect().catch(() => {});
    }
  }, [wallet]);

  return <>{children}</>;
}

export function WalletContextProvider({ children }: { children: React.ReactNode }) {
  const wallets = useMemo(
    () => [new PhantomWalletAdapter(), new SolflareWalletAdapter()],
    [],
  );

  return (
    <ConnectionProvider endpoint={DEVNET_RPC}>
      <WalletProvider wallets={wallets} autoConnect={false} onError={() => {}}>
        <WalletModalProvider>
          <AutoConnectHandler>{children}</AutoConnectHandler>
        </WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  );
}
