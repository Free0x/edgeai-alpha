import React, { createContext, useContext, useState, useEffect, useCallback, ReactNode } from 'react';
import { API_BASE_URL } from '../lib/config';

// BSC Testnet 配置
const BSC_TESTNET = {
  chainId: "0x61", // 97 in hex
  chainName: "BSC Testnet",
  nativeCurrency: { name: "BNB", symbol: "tBNB", decimals: 18 },
  rpcUrls: ["https://data-seed-prebsc-1-s1.binance.org:8545"],
  blockExplorerUrls: ["https://testnet.bscscan.com"],
};

// 合约地址
const CONTRACTS = {
  wEDGE: "0xEe3131549D8727bBCd6e628D90D6b57cf99F5794",
  bridge: "0x0f72c1d37F64f0E962278A1941EC7664D4e2289B",
};

// wEDGE ABI (简化版)
const WEDGE_ABI = [
  "function balanceOf(address) view returns (uint256)",
  "function symbol() view returns (string)",
  "function decimals() view returns (uint8)",
];

// EdgeAI 钱包类型
interface EdgeAIWallet {
  address: string;
  publicKey: string;
  privateKey?: string;
}

// MetaMask 钱包状态
interface MetaMaskState {
  isInstalled: boolean;
  isConnected: boolean;
  address: string;
  chainId: number | null;
  balance: string; // BNB balance
  wEdgeBalance: string; // wEDGE balance
  isCorrectNetwork: boolean;
}

// EdgeAI 钱包状态
interface EdgeAIState {
  isConnected: boolean;
  address: string;
  balance: string; // EDGE balance
}

// Context 类型
interface WalletContextType {
  // MetaMask
  metamask: MetaMaskState;
  connectMetaMask: () => Promise<void>;
  disconnectMetaMask: () => void;
  switchToBscTestnet: () => Promise<void>;
  
  // EdgeAI
  edgeai: EdgeAIState;
  loadEdgeAIWallet: () => void;
  createEdgeAIWallet: () => Promise<EdgeAIWallet | null>;
  
  // 通用
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
  // MetaMask 状态
  const [metamask, setMetaMask] = useState<MetaMaskState>({
    isInstalled: false,
    isConnected: false,
    address: "",
    chainId: null,
    balance: "0",
    wEdgeBalance: "0",
    isCorrectNetwork: false,
  });

  // EdgeAI 状态
  const [edgeai, setEdgeAI] = useState<EdgeAIState>({
    isConnected: false,
    address: "",
    balance: "0",
  });

  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState("");

  // 检查 MetaMask 安装
  useEffect(() => {
    const checkMetaMask = async () => {
      if (typeof window.ethereum !== "undefined") {
        setMetaMask(prev => ({ ...prev, isInstalled: true }));

        // 检查是否已连接
        try {
          const accounts = await window.ethereum.request({ method: "eth_accounts" });
          if (accounts.length > 0) {
            const chainId = await window.ethereum.request({ method: "eth_chainId" });
            setMetaMask(prev => ({
              ...prev,
              isConnected: true,
              address: accounts[0],
              chainId: parseInt(chainId, 16),
              isCorrectNetwork: parseInt(chainId, 16) === 97,
            }));
          }
        } catch (err) {
          console.error("Failed to check MetaMask connection:", err);
        }

        // 监听账户变化
        window.ethereum.on("accountsChanged", (accounts: string[]) => {
          if (accounts.length > 0) {
            setMetaMask(prev => ({ ...prev, isConnected: true, address: accounts[0] }));
          } else {
            setMetaMask(prev => ({
              ...prev,
              isConnected: false,
              address: "",
              balance: "0",
              wEdgeBalance: "0",
            }));
          }
        });

        // 监听网络变化
        window.ethereum.on("chainChanged", (chainId: string) => {
          const id = parseInt(chainId, 16);
          setMetaMask(prev => ({
            ...prev,
            chainId: id,
            isCorrectNetwork: id === 97,
          }));
        });
      }
    };

    checkMetaMask();
  }, []);

  // 加载 EdgeAI 钱包
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
        // 获取余额
        fetchEdgeAIBalance(wallet.address);
      } catch (err) {
        console.error("Failed to load EdgeAI wallet:", err);
        localStorage.removeItem("edgeai_wallet");
      }
    }
  }, []);

  // 初始化时加载 EdgeAI 钱包
  useEffect(() => {
    loadEdgeAIWallet();
  }, [loadEdgeAIWallet]);

  // 获取 EdgeAI 余额
  const fetchEdgeAIBalance = async (address: string) => {
    try {
      const response = await fetch(
        `${API_BASE_URL}/accounts/${address}`
      );
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

  // 获取 MetaMask 余额
  const fetchMetaMaskBalances = async (address: string) => {
    if (!window.ethereum) return;

    try {
      // 获取 BNB 余额
      const balanceHex = await window.ethereum.request({
        method: "eth_getBalance",
        params: [address, "latest"],
      });
      const balanceWei = BigInt(balanceHex);
      const balanceEth = Number(balanceWei) / 1e18;

      // 获取 wEDGE 余额 (简化实现)
      let wEdgeBalance = "0";
      try {
        const data = `0x70a08231000000000000000000000000${address.slice(2)}`;
        const result = await window.ethereum.request({
          method: "eth_call",
          params: [{ to: CONTRACTS.wEDGE, data }, "latest"],
        });
        const wEdgeWei = BigInt(result);
        wEdgeBalance = (Number(wEdgeWei) / 1e18).toFixed(4);
      } catch (err) {
        console.error("Failed to fetch wEDGE balance:", err);
      }

      setMetaMask(prev => ({
        ...prev,
        balance: balanceEth.toFixed(4),
        wEdgeBalance,
      }));
    } catch (err) {
      console.error("Failed to fetch MetaMask balances:", err);
    }
  };

  // 当 MetaMask 连接状态变化时获取余额
  useEffect(() => {
    if (metamask.isConnected && metamask.address) {
      fetchMetaMaskBalances(metamask.address);
    }
  }, [metamask.isConnected, metamask.address, metamask.chainId]);

  // 连接 MetaMask
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
          isCorrectNetwork: parseInt(chainId, 16) === 97,
        }));
      }
    } catch (err: any) {
      setError(err.message || "Failed to connect MetaMask");
    } finally {
      setIsLoading(false);
    }
  };

  // 断开 MetaMask
  const disconnectMetaMask = () => {
    setMetaMask(prev => ({
      ...prev,
      isConnected: false,
      address: "",
      balance: "0",
      wEdgeBalance: "0",
    }));
  };

  // 切换到 BSC Testnet
  const switchToBscTestnet = async () => {
    if (!window.ethereum) return;

    try {
      await window.ethereum.request({
        method: "wallet_switchEthereumChain",
        params: [{ chainId: BSC_TESTNET.chainId }],
      });
    } catch (err: any) {
      // 如果网络不存在，添加它
      if (err.code === 4902) {
        await window.ethereum.request({
          method: "wallet_addEthereumChain",
          params: [BSC_TESTNET],
        });
      } else {
        setError(err.message || "Failed to switch network");
      }
    }
  };

  // 创建 EdgeAI 钱包
  const createEdgeAIWallet = async (): Promise<EdgeAIWallet | null> => {
    setIsLoading(true);
    setError("");

    try {
      const response = await fetch(
        `${API_BASE_URL}/wallet/generate`,
        { method: "POST" }
      );
      const data = await response.json();

      if (data.success && data.data) {
        const wallet: EdgeAIWallet = {
          address: data.data.address,
          publicKey: data.data.public_key,
          privateKey: data.data.private_key,
        };

        // 保存到 localStorage
        localStorage.setItem("edgeai_wallet", JSON.stringify(wallet));

        setEdgeAI({
          isConnected: true,
          address: wallet.address,
          balance: "0",
        });

        // 从 Faucet 获取测试代币
        try {
          await fetch(`${API_BASE_URL}/faucet`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ address: wallet.address }),
          });
          // 等待一下再获取余额
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

  // 刷新所有余额
  const refreshBalances = async () => {
    if (metamask.isConnected && metamask.address) {
      await fetchMetaMaskBalances(metamask.address);
    }
    if (edgeai.isConnected && edgeai.address) {
      await fetchEdgeAIBalance(edgeai.address);
    }
  };

  // 清除错误
  const clearError = () => setError("");

  return (
    <WalletContext.Provider
      value={{
        metamask,
        connectMetaMask,
        disconnectMetaMask,
        switchToBscTestnet,
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

// 导出合约地址供其他组件使用
export { CONTRACTS, BSC_TESTNET };
