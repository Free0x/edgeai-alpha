import React, { createContext, useContext, useState, useEffect, useCallback, ReactNode } from 'react';
import { API_BASE_URL } from '../lib/config';

// BSC Mainnet configuration
const BSC_MAINNET = {
  chainId: "0x38", // 56 in hex
  chainName: "BNB Smart Chain",
  nativeCurrency: { name: "BNB", symbol: "BNB", decimals: 18 },
  rpcUrls: ["https://bsc-dataseed1.binance.org"],
  blockExplorerUrls: ["https://bscscan.com"],
};

// Contract addresses on BSC Mainnet
const CONTRACTS = {
  EDGEAI: "0x276b792D11B9e3712FE6A78A460a0DEb416baB0A",
};

// EdgeAI wallet type
interface EdgeAIWallet {
  address: string;
  publicKey: string;
  privateKey?: string;
}

// MetaMask wallet state
interface MetaMaskState {
  isInstalled: boolean;
  isConnected: boolean;
  address: string;
  chainId: number | null;
  balance: string; // BNB balance
  edgeaiTokenBalance: string; // EDGEAI token balance on BSC
  isCorrectNetwork: boolean;
}

// EdgeAI wallet state
interface EdgeAIState {
  isConnected: boolean;
  address: string;
  balance: string; // EDGE balance on native chain
}

// Context type
interface WalletContextType {
  // MetaMask
  metamask: MetaMaskState;
  connectMetaMask: () => Promise<void>;
  disconnectMetaMask: () => void;
  switchToBscMainnet: () => Promise<void>;

  // EdgeAI
  edgeai: EdgeAIState;
  loadEdgeAIWallet: () => void;
  createEdgeAIWallet: () => Promise<EdgeAIWallet | null>;

  // General
  isLoading: boolean;
  error: string;
  clearError: () => void;
  refreshBalances: () => Promise<void>;
}

const WalletContext = createContext<WalletContextType | undefined>(undefined);

declare global {
  interface Window {
    ethereum?: any;
  }
}

export function WalletProvider({ children }: { children: React.ReactNode }) {
  const [metamask, setMetaMask] = useState<MetaMaskState>({
    isInstalled: false,
    isConnected: false,
    address: "",
    chainId: null,
    balance: "0",
    edgeaiTokenBalance: "0",
    isCorrectNetwork: false,
  });

  const [edgeai, setEdgeAI] = useState<EdgeAIState>({
    isConnected: false,
    address: "",
    balance: "0",
  });

  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState("");

  // Check MetaMask installation
  useEffect(() => {
    const checkMetaMask = async () => {
      if (typeof window.ethereum !== "undefined") {
        setMetaMask(prev => ({ ...prev, isInstalled: true }));

        try {
          const accounts = await window.ethereum.request({ method: "eth_accounts" });
          if (accounts.length > 0) {
            const chainId = await window.ethereum.request({ method: "eth_chainId" });
            setMetaMask(prev => ({
              ...prev,
              isConnected: true,
              address: accounts[0],
              chainId: parseInt(chainId, 16),
              isCorrectNetwork: parseInt(chainId, 16) === 56,
            }));
          }
        } catch (err) {
          console.error("Failed to check MetaMask connection:", err);
        }

        window.ethereum.on("accountsChanged", (accounts: string[]) => {
          if (accounts.length > 0) {
            setMetaMask(prev => ({ ...prev, isConnected: true, address: accounts[0] }));
          } else {
            setMetaMask(prev => ({
              ...prev,
              isConnected: false,
              address: "",
              balance: "0",
              edgeaiTokenBalance: "0",
            }));
          }
        });

        window.ethereum.on("chainChanged", (chainId: string) => {
          const id = parseInt(chainId, 16);
          setMetaMask(prev => ({
            ...prev,
            chainId: id,
            isCorrectNetwork: id === 56,
          }));
        });
      }
    };

    checkMetaMask();
  }, []);

  // Load EdgeAI wallet
  const loadEdgeAIWallet = useCallback(() => {
    const savedWallet = localStorage.getItem("edgeai_wallet");
    if (savedWallet) {
      try {
        const wallet = JSON.parse(savedWallet);
        setEdgeAI(prev => ({
          ...prev,
          isConnected: true,
          address: wallet.address,
        }));
        fetchEdgeAIBalance(wallet.address);
      } catch (err) {
        console.error("Failed to load EdgeAI wallet:", err);
        localStorage.removeItem("edgeai_wallet");
      }
    }
  }, []);

  useEffect(() => {
    loadEdgeAIWallet();
  }, [loadEdgeAIWallet]);

  // Fetch EdgeAI native balance
  const fetchEdgeAIBalance = async (address: string) => {
    try {
      const response = await fetch(`${API_BASE_URL}/accounts/${address}`);
      const data = await response.json();
      if (data.success && data.data) {
        setEdgeAI(prev => ({
          ...prev,
          balance: data.data.balance?.toString() || "0",
        }));
      }
    } catch (err) {
      console.error("Failed to fetch EdgeAI balance:", err);
    }
  };

  // Fetch MetaMask balances (BNB + EDGEAI token)
  const fetchMetaMaskBalances = async (address: string) => {
    if (!window.ethereum) return;

    try {
      // Get BNB balance
      const balanceHex = await window.ethereum.request({
        method: "eth_getBalance",
        params: [address, "latest"],
      });
      const balanceWei = BigInt(balanceHex);
      const balanceEth = Number(balanceWei) / 1e18;

      // Get EDGEAI token balance
      let edgeaiTokenBalance = "0";
      try {
        const data = `0x70a08231000000000000000000000000${address.slice(2)}`;
        const result = await window.ethereum.request({
          method: "eth_call",
          params: [{ to: CONTRACTS.EDGEAI, data }, "latest"],
        });
        const tokenWei = BigInt(result);
        edgeaiTokenBalance = (Number(tokenWei) / 1e18).toFixed(4);
      } catch (err) {
        console.error("Failed to fetch EDGEAI token balance:", err);
      }

      setMetaMask(prev => ({
        ...prev,
        balance: balanceEth.toFixed(4),
        edgeaiTokenBalance,
      }));
    } catch (err) {
      console.error("Failed to fetch MetaMask balances:", err);
    }
  };

  useEffect(() => {
    if (metamask.isConnected && metamask.address) {
      fetchMetaMaskBalances(metamask.address);
    }
  }, [metamask.isConnected, metamask.address, metamask.chainId]);

  // Connect MetaMask
  const connectMetaMask = async () => {
    if (!metamask.isInstalled) {
      window.open("https://metamask.io/download/", "_blank");
      return;
    }

    setIsLoading(true);
    setError("");

    try {
      const accounts = await window.ethereum.request({
        method: "eth_requestAccounts",
      });

      if (accounts.length > 0) {
        const chainId = await window.ethereum.request({ method: "eth_chainId" });
        setMetaMask(prev => ({
          ...prev,
          isConnected: true,
          address: accounts[0],
          chainId: parseInt(chainId, 16),
          isCorrectNetwork: parseInt(chainId, 16) === 56,
        }));
      }
    } catch (err: any) {
      setError(err.message || "Failed to connect MetaMask");
    } finally {
      setIsLoading(false);
    }
  };

  // Disconnect MetaMask
  const disconnectMetaMask = () => {
    setMetaMask(prev => ({
      ...prev,
      isConnected: false,
      address: "",
      balance: "0",
      edgeaiTokenBalance: "0",
    }));
  };

  // Switch to BSC Mainnet
  const switchToBscMainnet = async () => {
    if (!window.ethereum) return;

    try {
      await window.ethereum.request({
        method: "wallet_switchEthereumChain",
        params: [{ chainId: BSC_MAINNET.chainId }],
      });
    } catch (err: any) {
      if (err.code === 4902) {
        await window.ethereum.request({
          method: "wallet_addEthereumChain",
          params: [BSC_MAINNET],
        });
      } else {
        setError(err.message || "Failed to switch network");
      }
    }
  };

  // Create EdgeAI wallet
  const createEdgeAIWallet = async (): Promise<EdgeAIWallet | null> => {
    setIsLoading(true);
    setError("");

    try {
      const response = await fetch(`${API_BASE_URL}/wallet/generate`, { method: "POST" });
      const data = await response.json();

      if (data.success && data.data) {
        const wallet: EdgeAIWallet = {
          address: data.data.address,
          publicKey: data.data.public_key,
          privateKey: data.data.private_key,
        };

        localStorage.setItem("edgeai_wallet", JSON.stringify(wallet));

        setEdgeAI({
          isConnected: true,
          address: wallet.address,
          balance: "0",
        });

        try {
          await fetch(`${API_BASE_URL}/faucet`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ address: wallet.address }),
          });
          setTimeout(() => fetchEdgeAIBalance(wallet.address), 2000);
        } catch (err) {
          console.error("Faucet request failed:", err);
        }

        return wallet;
      } else {
        throw new Error(data.error || "Failed to generate wallet");
      }
    } catch (err: any) {
      setError(err.message || "Failed to create wallet");
      return null;
    } finally {
      setIsLoading(false);
    }
  };

  // Refresh all balances
  const refreshBalances = async () => {
    if (metamask.isConnected && metamask.address) {
      await fetchMetaMaskBalances(metamask.address);
    }
    if (edgeai.isConnected && edgeai.address) {
      await fetchEdgeAIBalance(edgeai.address);
    }
  };

  const clearError = () => setError("");

  return (
    <WalletContext.Provider
      value={{
        metamask,
        connectMetaMask,
        disconnectMetaMask,
        switchToBscMainnet,
        edgeai,
        loadEdgeAIWallet,
        createEdgeAIWallet,
        isLoading,
        error,
        clearError,
        refreshBalances,
      }}
    >
      {children}
    </WalletContext.Provider>
  );
}

export function useWallet() {
  const context = useContext(WalletContext);
  if (!context) {
    throw new Error("useWallet must be used within WalletProvider");
  }
  return context;
}

// Export contract addresses and config for other components
export { CONTRACTS, BSC_MAINNET };
