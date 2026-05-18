import { create } from "zustand";
import { persist } from "zustand/middleware";

export type WalletPlatform =
  | "metamask" | "okx" | "trustwallet" | "walletconnect"
  | "coinbase" | "phantom" | "rabby" | "rainbow"
  | "ledger" | "trezor" | "binance_pay" | "paypal"
  | "apple_pay" | "google_pay";

export type ConnectMethod = "extension" | "qr_code" | "api_key" | "oauth" | "usb" | "manual";

export type PaymentMode = "auto" | "manual";

export interface WalletConnection {
  id: string;
  platform: WalletPlatform;
  label: string;
  address: string;
  chain: string;
  balance: string;
  connected: boolean;
  agent_controlled: boolean;
  created_by_agent: boolean;
}

export interface PaymentMethodSummary {
  id: string;
  platform: WalletPlatform;
  label: string;
  address: string;
  chain: string;
  balance: string;
  is_active: boolean;
  agent_controlled: boolean;
  connection_method: ConnectMethod;
  connection_data: string | null;
}

export const CONNECT_METHOD_LABELS: Record<ConnectMethod, string> = {
  extension: "Browser Extension",
  qr_code: "QR Code",
  api_key: "API Key",
  oauth: "OAuth",
  usb: "USB",
  manual: "Manual",
};

export const CONNECT_METHOD_SIMPLES: ConnectMethod[] = ["extension", "qr_code"];

export const PLATFORM_META: Record<WalletPlatform, { label: string; icon: string; chains: string[]; agentCreate: boolean; simpleMethod: ConnectMethod }> = {
  metamask:     { label: "MetaMask", icon: "🦊", chains: ["Ethereum", "Polygon", "Arbitrum", "Optimism", "Base", "BNB Chain"], agentCreate: true, simpleMethod: "extension" },
  okx:          { label: "OKX Wallet", icon: "ⓞ", chains: ["Ethereum", "Solana", "Polygon", "Arbitrum", "BNB Chain", "Bitcoin", "Tron"], agentCreate: true, simpleMethod: "extension" },
  trustwallet:  { label: "TrustWallet", icon: "🛡️", chains: ["Ethereum", "Solana", "Polygon", "BNB Chain", "Bitcoin", "Tron", "Cosmos"], agentCreate: true, simpleMethod: "qr_code" },
  walletconnect:{ label: "WalletConnect", icon: "🔗", chains: ["Ethereum", "Solana", "Polygon", "Arbitrum", "Optimism", "Base"], agentCreate: false, simpleMethod: "qr_code" },
  coinbase:     { label: "Coinbase Wallet", icon: "🔵", chains: ["Ethereum", "Base", "Polygon", "Arbitrum"], agentCreate: true, simpleMethod: "extension" },
  phantom:      { label: "Phantom", icon: "👻", chains: ["Solana", "Ethereum", "Polygon"], agentCreate: true, simpleMethod: "extension" },
  rabby:        { label: "Rabby", icon: "🐰", chains: ["Ethereum", "Polygon", "Arbitrum", "Optimism", "Base"], agentCreate: false, simpleMethod: "extension" },
  rainbow:      { label: "Rainbow", icon: "🌈", chains: ["Ethereum", "Polygon", "Arbitrum", "Optimism", "Base"], agentCreate: false, simpleMethod: "extension" },
  ledger:       { label: "Ledger Live", icon: "💼", chains: ["Ethereum", "Bitcoin", "Solana", "Polygon"], agentCreate: false, simpleMethod: "usb" },
  trezor:       { label: "Trezor", icon: "🔒", chains: ["Ethereum", "Bitcoin", "Solana", "Polygon"], agentCreate: false, simpleMethod: "usb" },
  binance_pay:  { label: "Binance Pay", icon: "💰", chains: ["BNB Chain", "Ethereum"], agentCreate: false, simpleMethod: "api_key" },
  paypal:       { label: "PayPal", icon: "💳", chains: ["Fiat"], agentCreate: false, simpleMethod: "oauth" },
  apple_pay:    { label: "Apple Pay", icon: "🍎", chains: ["Fiat"], agentCreate: false, simpleMethod: "oauth" },
  google_pay:   { label: "Google Pay", icon: "📱", chains: ["Fiat"], agentCreate: false, simpleMethod: "oauth" },
};

interface PaymentStoreState {
  mode: PaymentMode;
  methods: PaymentMethodSummary[];
  activeMethodId: string | null;
  walletsConfigured: boolean;

  setMode: (mode: PaymentMode) => void;
  setMethods: (methods: PaymentMethodSummary[]) => void;
  setActiveMethodId: (id: string) => void;
  setWalletsConfigured: (v: boolean) => void;
}

export const usePaymentStore = create<PaymentStoreState>()(
  persist(
    (set) => ({
      mode: "auto",
      methods: [],
      activeMethodId: null,
      walletsConfigured: false,

      setMode: (mode) => set({ mode }),
      setMethods: (methods) => set({ methods }),
      setActiveMethodId: (id) => set({ activeMethodId: id }),
      setWalletsConfigured: (v) => set({ walletsConfigured: v }),
    }),
    { name: "prime-payment-store" },
  ),
);
