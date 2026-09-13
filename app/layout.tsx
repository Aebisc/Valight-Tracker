import type { Metadata, Viewport } from "next";
import { DM_Sans, JetBrains_Mono } from "next/font/google";
import "./globals.css";
import { ToastProvider } from "../components/toast";
import UpdaterCard from "../components/updater-card";

const dmSans = DM_Sans({
  subsets: ["latin"],
  weight: ["300", "400", "500", "600", "700"],
  variable: "--font-dm-sans",
  display: "swap",
});

const jetbrainsMono = JetBrains_Mono({
  subsets: ["latin"],
  weight: ["400", "500", "600"],
  variable: "--font-jetbrains-mono",
  display: "swap",
});

export const viewport: Viewport = {
  width: "device-width",
  initialScale: 1,
  themeColor: "#060609",
};

export const metadata: Metadata = {
  title: {
    default: "VaLight Tracker",
    template: "%s | VaLight Tracker",
  },
  description:
    "Track your Valorant stats, match history, and rank progression in real time.",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className={`${dmSans.variable} ${jetbrainsMono.variable}`}>
      <body suppressHydrationWarning>
        <ToastProvider>
          {children}
          <UpdaterCard />
        </ToastProvider>
      </body>
    </html>
  );
}
