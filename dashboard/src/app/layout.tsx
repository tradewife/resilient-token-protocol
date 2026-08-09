import type { Metadata } from "next";
import "./globals.css";
import { WalletContextProvider } from "./WalletContextProvider";

export const metadata: Metadata = {
  title: "Resilient Token Protocol: Bespoke Trading Engines on Solana",
  description: "Your terms in, a bespoke engine out. One client, one strategy — engineered around your risk budget, drawdown limit and horizon, priced at live venue fees measured on-chain, run on self-custodied rails. Start with the on-chain compatibility check.",
  icons: {
    icon: "/icon.svg",
    apple: "/apple-touch-icon.png",
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body>
        <WalletContextProvider>{children}</WalletContextProvider>
      </body>
    </html>
  );
}
