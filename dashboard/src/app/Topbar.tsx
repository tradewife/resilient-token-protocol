"use client";

import React, { useState } from "react";
import Link from "next/link";

interface TopbarProps {
  activePage?: "dashboard" | "architecture" | "pipeline" | "diagnostic" | "launch" | "docs" | "research";
}

const NAV_ITEMS = [
  { href: "/#live", label: "Live Console", key: "dashboard" },
  { href: "/#trust", label: "Architecture", key: "architecture" },
  { href: "/#pipeline", label: "Research Pipeline", key: "pipeline" },
  { href: "/diagnostic", label: "Diagnostic", key: "diagnostic" },
  { href: "/docs", label: "Docs", key: "docs" },
] as const;

export default function Topbar({ activePage }: TopbarProps) {
  const [menuOpen, setMenuOpen] = useState(false);

  return (
    <header className="topbar">
      <div className="brand">
        <img className="brand-icon" src="/icon.svg" alt="RTP" />
        <Link href="/" className="brand-name" style={{ textDecoration: "none", color: "inherit" }}>
          RESILIENT TOKEN PROTOCOL
        </Link>
      </div>
      <div className="topbar-actions">
        <span className="network-badge" title="Resilient Token Protocol is currently in closed beta">Closed Beta</span>
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
          <span className="menu-badge">Closed Beta</span>
        </nav>
      )}
    </header>
  );
}
