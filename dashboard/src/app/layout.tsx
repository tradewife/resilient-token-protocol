import type { Metadata } from "next";
import "./globals.css";
import { WalletContextProvider } from "./WalletContextProvider";

export const metadata: Metadata = {
  title: "RTP Sentinel",
  description: "Autonomous treasury protocol — governed by code, not trust.",
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
