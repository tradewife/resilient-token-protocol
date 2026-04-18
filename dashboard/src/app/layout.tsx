import type { Metadata } from "next";
import "./globals.css";
import { WalletContextProvider } from "./WalletContextProvider";

export const metadata: Metadata = {
  title: "Resilient Token Protocol",
  description: "Autonomous treasury protocol — governed by code, not trust.",
  icons: {
    icon: [
      { url: "/icon.svg", type: "image/svg+xml" },
      { url: "/favicon.ico", sizes: "any" },
    ],
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
