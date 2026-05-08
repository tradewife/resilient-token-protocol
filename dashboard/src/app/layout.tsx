import type { Metadata } from "next";
import "./globals.css";
import { WalletContextProvider } from "./WalletContextProvider";

export const metadata: Metadata = {
  title: "Resilient Token Protocol: On-Chain Treasury Yield for Every Token",
  description: "Token projects route trading fees to RTP → the swarm generates yield via on-chain perps → yield flows back to holders. 70/20/10 split, enforced on-chain. No RTP token. Pure infrastructure.",
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
