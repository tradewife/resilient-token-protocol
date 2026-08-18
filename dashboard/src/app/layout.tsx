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
    // `dark` is the shadcn/EvilCharts theme class — this site is dark-only,
    // so it is static. It activates the dark palette block in globals.css
    // and EvilCharts' dark chart-color variables.
    <html lang="en" className="dark">
      <body>
        <WalletContextProvider>{children}</WalletContextProvider>
      </body>
    </html>
  );
}
