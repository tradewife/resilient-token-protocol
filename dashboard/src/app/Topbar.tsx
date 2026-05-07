"use client";

import React, { useState } from "react";
import Link from "next/link";
import { useWallet } from "@solana/wallet-adapter-react";
import { useWalletModal } from "@solana/wallet-adapter-react-ui";

interface TopbarProps {
  activePage?: "dashboard" | "launch" | "research" | "system" | "docs";
}

const NAV_ITEMS = [
  { href: "/", label: "Dashboard", key: "dashboard" },
  { href: "/system", label: "System", key: "system" },
  { href: "/launch", label: "Launch", key: "launch" },
  { href: "/docs", label: "Docs", key: "docs" },
] as const;

export default function Topbar({ activePage }: TopbarProps) {
  const { publicKey, connected, disconnect } = useWallet();
  const { setVisible } = useWalletModal();
  const [menuOpen, setMenuOpen] = useState(false);

  const addr = publicKey
    ? `${publicKey.toBase58().slice(0, 4)}...${publicKey.toBase58().slice(-4)}`
    : null;

  return (
    <header className="topbar">
      <div className="brand">
        <img className="brand-icon" src="/icon.svg" alt="RTP" />
        <Link href="/" className="brand-name" style={{ textDecoration: "none", color: "inherit" }}>
          RESILIENT TOKEN PROTOCOL
        </Link>
      </div>
      <div className="topbar-actions">
        <span className="network-badge">Devnet</span>
        <span style={{
          fontSize: "0.5625rem", fontWeight: 600, letterSpacing: "0.08em",
          color: "#fff", background: "var(--emerald)", padding: "2px 8px",
          borderRadius: 3, textTransform: "uppercase", lineHeight: 1.6,
        }}>Mainnet</span>
        {NAV_ITEMS.map((item) => (
          <Link
            key={item.key}
            href={item.href}
            className="btn-connect nav-link"
            style={{
              textDecoration: "none",
              fontSize: "0.8125rem",
              padding: "6px 14px",
              ...(activePage === item.key
                ? { borderColor: "var(--coral-dim)", color: "var(--coral)" }
                : {}),
            }}
          >
            {item.label}
          </Link>
        ))}
        {connected && publicKey ? (
          <div className="wallet-pill">
            <span className="wallet-indicator" />
            <span className="wallet-addr">{addr}</span>
            <button className="btn-disconnect" onClick={disconnect} title="Disconnect">&times;</button>
          </div>
        ) : (
          <button className="btn-connect" onClick={() => setVisible(true)}>Connect Wallet</button>
        )}
        <button
          className={`hamburger${menuOpen ? " open" : ""}`}
          onClick={() => setMenuOpen(!menuOpen)}
          aria-label="Menu"
        >
          <span /><span /><span />
        </button>
      </div>
      {menuOpen && (
        <nav className="mobile-menu">
          {NAV_ITEMS.map((item) => (
            <Link key={item.key} href={item.href} onClick={() => setMenuOpen(false)}>
              {item.label}
            </Link>
          ))}
          <span className="menu-badge">Devnet</span>
          <span style={{
            fontSize: "0.5625rem", fontWeight: 600, letterSpacing: "0.08em",
            color: "#fff", background: "var(--emerald)", padding: "2px 8px",
            borderRadius: 3, textTransform: "uppercase", lineHeight: 1.6,
          }}>Mainnet</span>
        </nav>
      )}
    </header>
  );
}
