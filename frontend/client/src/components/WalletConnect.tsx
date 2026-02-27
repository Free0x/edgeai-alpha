import { useState } from "react";
import { useWallet } from "../contexts/WalletContext";
import {
  Wallet,
  ChevronDown,
  Copy,
  ExternalLink,
  LogOut,
  RefreshCw,
  AlertCircle,
  Check,
  Loader2,
} from "lucide-react";

// MetaMask 图标 SVG
const MetaMaskIcon = ({ className = "w-5 h-5" }: { className?: string }) => (
  <svg className={className} viewBox="0 0 35 33" fill="none" xmlns="http://www.w3.org/2000/svg">
    <path d="M32.9582 1L19.8241 10.7183L22.2665 4.99099L32.9582 1Z" fill="#E17726" stroke="#E17726" strokeWidth="0.25" strokeLinecap="round" strokeLinejoin="round"/>
    <path d="M2.66296 1L15.6886 10.809L13.3541 4.99098L2.66296 1Z" fill="#E27625" stroke="#E27625" strokeWidth="0.25" strokeLinecap="round" strokeLinejoin="round"/>
    <path d="M28.2295 23.5334L24.7346 28.872L32.2271 30.9323L34.3804 23.6501L28.2295 23.5334Z" fill="#E27625" stroke="#E27625" strokeWidth="0.25" strokeLinecap="round" strokeLinejoin="round"/>
    <path d="M1.27271 23.6501L3.41689 30.9323L10.9003 28.872L7.41449 23.5334L1.27271 23.6501Z" fill="#E27625" stroke="#E27625" strokeWidth="0.25" strokeLinecap="round" strokeLinejoin="round"/>
    <path d="M10.4706 14.5149L8.39209 17.6507L15.7886 17.9841L15.5533 9.98999L10.4706 14.5149Z" fill="#E27625" stroke="#E27625" strokeWidth="0.25" strokeLinecap="round" strokeLinejoin="round"/>
    <path d="M25.1505 14.5149L19.9859 9.89917L19.8241 17.9841L27.2206 17.6507L25.1505 14.5149Z" fill="#E27625" stroke="#E27625" strokeWidth="0.25" strokeLinecap="round" strokeLinejoin="round"/>
    <path d="M10.9003 28.8721L15.3533 26.7026L11.5116 23.6963L10.9003 28.8721Z" fill="#E27625" stroke="#E27625" strokeWidth="0.25" strokeLinecap="round" strokeLinejoin="round"/>
    <path d="M20.2677 26.7026L24.7346 28.8721L24.1094 23.6963L20.2677 26.7026Z" fill="#E27625" stroke="#E27625" strokeWidth="0.25" strokeLinecap="round" strokeLinejoin="round"/>
  </svg>
);

// EdgeAI 图标
const EdgeAIIcon = ({ className = "w-5 h-5" }: { className?: string }) => (
  <div className={`${className} bg-gradient-to-br from-purple-500 to-blue-500 rounded-full flex items-center justify-center`}>
    <span className="text-white text-xs font-bold">E</span>
  </div>
);

export default function WalletConnect() {
  const {
    metamask,
    edgeai,
    connectMetaMask,
    disconnectMetaMask,
    switchToBscMainnet,
    createEdgeAIWallet,
    isLoading,
    error,
    clearError,
    refreshBalances,
  } = useWallet();

  const [isDropdownOpen, setIsDropdownOpen] = useState(false);
  const [copiedAddress, setCopiedAddress] = useState<string | null>(null);

  // 复制地址
  const copyAddress = (address: string) => {
    navigator.clipboard.writeText(address);
    setCopiedAddress(address);
    setTimeout(() => setCopiedAddress(null), 2000);
  };

  // 格式化地址 - 移动端更短
  const formatAddress = (address: string, short: boolean = false) => {
    if (!address) return "";
    if (short) return `${address.slice(0, 4)}...${address.slice(-3)}`;
    return `${address.slice(0, 6)}...${address.slice(-4)}`;
  };

  // 格式化余额
  const formatBalance = (balance: string, decimals: number = 4) => {
    const num = parseFloat(balance);
    if (isNaN(num)) return "0";
    if (num >= 1000000) return `${(num / 1000000).toFixed(2)}M`;
    if (num >= 1000) return `${(num / 1000).toFixed(2)}K`;
    return num.toFixed(decimals);
  };

  // 获取连接状态
  const isAnyWalletConnected = metamask.isConnected || edgeai.isConnected;

  return (
    <div className="relative">
      {/* 主按钮 - 响应式设计 */}
      <button
        onClick={() => setIsDropdownOpen(!isDropdownOpen)}
        className={`flex items-center gap-1.5 sm:gap-2 px-2 sm:px-3 lg:px-4 py-1.5 sm:py-2 rounded-lg font-medium transition-all text-sm ${
          isAnyWalletConnected
            ? "bg-gray-800 hover:bg-gray-700 text-white border border-gray-600"
            : "bg-purple-600 hover:bg-purple-500 text-white"
        }`}
      >
        {isLoading ? (
          <Loader2 className="w-4 h-4 animate-spin" />
        ) : (
          <Wallet className="w-4 h-4" />
        )}
        
        {isAnyWalletConnected ? (
          <div className="flex items-center gap-1 sm:gap-2">
            {/* 移动端只显示图标和短地址 */}
            {metamask.isConnected && (
              <div className="flex items-center gap-1">
                <MetaMaskIcon className="w-4 h-4 hidden sm:block" />
                <span className="text-xs sm:text-sm">
                  <span className="sm:hidden">{formatAddress(metamask.address, true)}</span>
                  <span className="hidden sm:inline">{formatAddress(metamask.address)}</span>
                </span>
              </div>
            )}
            {metamask.isConnected && edgeai.isConnected && (
              <span className="text-gray-500 hidden sm:inline">|</span>
            )}
            {edgeai.isConnected && !metamask.isConnected && (
              <div className="flex items-center gap-1">
                <EdgeAIIcon className="w-4 h-4 hidden sm:block" />
                <span className="text-xs sm:text-sm">
                  <span className="sm:hidden">{formatAddress(edgeai.address, true)}</span>
                  <span className="hidden sm:inline">{formatAddress(edgeai.address)}</span>
                </span>
              </div>
            )}
            {edgeai.isConnected && metamask.isConnected && (
              <div className="hidden sm:flex items-center gap-1">
                <EdgeAIIcon className="w-4 h-4" />
                <span className="text-sm">{formatAddress(edgeai.address)}</span>
              </div>
            )}
          </div>
        ) : (
          <>
            <span className="hidden sm:inline">Connect Wallet</span>
            <span className="sm:hidden">Connect</span>
          </>
        )}
        
        <ChevronDown className={`w-3 h-3 sm:w-4 sm:h-4 transition-transform ${isDropdownOpen ? "rotate-180" : ""}`} />
      </button>

      {/* 下拉菜单 - 响应式宽度 */}
      {isDropdownOpen && (
        <div className="absolute right-0 top-full mt-2 w-[calc(100vw-24px)] sm:w-80 max-w-[320px] bg-gray-800 border border-gray-700 rounded-xl shadow-xl z-50">
          {/* 错误提示 */}
          {error && (
            <div className="p-2 sm:p-3 bg-red-900/50 border-b border-gray-700 flex items-center gap-2">
              <AlertCircle className="w-4 h-4 text-red-400 shrink-0" />
              <span className="text-xs sm:text-sm text-red-400 flex-1 line-clamp-2">{error}</span>
              <button onClick={clearError} className="text-gray-400 hover:text-white shrink-0">
                ×
              </button>
            </div>
          )}

          {/* MetaMask 部分 */}
          <div className="p-3 sm:p-4 border-b border-gray-700">
            <div className="flex items-center justify-between mb-2 sm:mb-3">
              <div className="flex items-center gap-2">
                <MetaMaskIcon className="w-4 h-4 sm:w-5 sm:h-5" />
                <span className="font-medium text-white text-sm sm:text-base">MetaMask</span>
              </div>
              {metamask.isConnected && (
                <span className={`text-xs px-1.5 sm:px-2 py-0.5 sm:py-1 rounded ${
                  metamask.isCorrectNetwork 
                    ? "bg-green-900/50 text-green-400" 
                    : "bg-yellow-900/50 text-yellow-400"
                }`}>
                  {metamask.isCorrectNetwork ? "BSC Mainnet" : `Chain ${metamask.chainId}`}
                </span>
              )}
            </div>

            {metamask.isConnected ? (
              <div className="space-y-2 sm:space-y-3">
                {/* 地址 */}
                <div className="flex items-center justify-between bg-gray-900 rounded-lg p-2">
                  <span className="text-xs sm:text-sm text-gray-300">{formatAddress(metamask.address)}</span>
                  <div className="flex items-center gap-1">
                    <button
                      onClick={() => copyAddress(metamask.address)}
                      className="p-1 hover:bg-gray-700 rounded"
                      title="Copy address"
                    >
                      {copiedAddress === metamask.address ? (
                        <Check className="w-3 h-3 sm:w-4 sm:h-4 text-green-400" />
                      ) : (
                        <Copy className="w-3 h-3 sm:w-4 sm:h-4 text-gray-400" />
                      )}
                    </button>
                    <a
                      href={`https://bscscan.com/address/${metamask.address}`}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="p-1 hover:bg-gray-700 rounded"
                      title="View on BSCScan"
                    >
                      <ExternalLink className="w-3 h-3 sm:w-4 sm:h-4 text-gray-400" />
                    </a>
                  </div>
                </div>

                {/* 余额 */}
                <div className="grid grid-cols-2 gap-2 text-sm">
                  <div className="bg-gray-900 rounded-lg p-2">
                    <div className="text-gray-500 text-xs">BNB</div>
                    <div className="text-white font-medium text-sm">{formatBalance(metamask.balance)}</div>
                  </div>
                  <div className="bg-gray-900 rounded-lg p-2">
                    <div className="text-gray-500 text-xs">EDGEAI</div>
                    <div className="text-purple-400 font-medium text-sm">{formatBalance(metamask.edgeaiTokenBalance)}</div>
                  </div>
                </div>

                {/* 网络切换 */}
                {!metamask.isCorrectNetwork && (
                  <button
                    onClick={switchToBscMainnet}
                    className="w-full py-2 bg-yellow-600 hover:bg-yellow-500 rounded-lg text-xs sm:text-sm font-medium text-white"
                  >
                    Switch to BSC Mainnet
                  </button>
                )}

                {/* 断开连接 */}
                <button
                  onClick={disconnectMetaMask}
                  className="w-full py-2 bg-gray-700 hover:bg-gray-600 rounded-lg text-xs sm:text-sm font-medium text-gray-300 flex items-center justify-center gap-2"
                >
                  <LogOut className="w-3 h-3 sm:w-4 sm:h-4" />
                  Disconnect
                </button>
              </div>
            ) : (
              <button
                onClick={connectMetaMask}
                disabled={isLoading}
                className="w-full py-2.5 sm:py-3 bg-orange-600 hover:bg-orange-500 rounded-lg font-medium text-white flex items-center justify-center gap-2 text-sm"
              >
                {isLoading ? (
                  <Loader2 className="w-4 h-4 animate-spin" />
                ) : (
                  <>
                    <MetaMaskIcon className="w-4 h-4 sm:w-5 sm:h-5" />
                    {metamask.isInstalled ? "Connect MetaMask" : "Install MetaMask"}
                  </>
                )}
              </button>
            )}
          </div>

          {/* EdgeAI 部分 */}
          <div className="p-3 sm:p-4">
            <div className="flex items-center justify-between mb-2 sm:mb-3">
              <div className="flex items-center gap-2">
                <EdgeAIIcon className="w-4 h-4 sm:w-5 sm:h-5" />
                <span className="font-medium text-white text-sm sm:text-base">EdgeAI Wallet</span>
              </div>
              {edgeai.isConnected && (
                <span className="text-xs px-1.5 sm:px-2 py-0.5 sm:py-1 rounded bg-purple-900/50 text-purple-400">
                  Native Chain
                </span>
              )}
            </div>

            {edgeai.isConnected ? (
              <div className="space-y-2 sm:space-y-3">
                {/* 地址 */}
                <div className="flex items-center justify-between bg-gray-900 rounded-lg p-2">
                  <span className="text-xs sm:text-sm text-gray-300">{formatAddress(edgeai.address)}</span>
                  <button
                    onClick={() => copyAddress(edgeai.address)}
                    className="p-1 hover:bg-gray-700 rounded"
                    title="Copy address"
                  >
                    {copiedAddress === edgeai.address ? (
                      <Check className="w-3 h-3 sm:w-4 sm:h-4 text-green-400" />
                    ) : (
                      <Copy className="w-3 h-3 sm:w-4 sm:h-4 text-gray-400" />
                    )}
                  </button>
                </div>

                {/* 余额 */}
                <div className="bg-gray-900 rounded-lg p-2">
                  <div className="text-gray-500 text-xs">EDGE Balance</div>
                  <div className="text-purple-400 font-medium text-base sm:text-lg">
                    {formatBalance(edgeai.balance, 2)} EDGE
                  </div>
                </div>
              </div>
            ) : (
              <button
                onClick={createEdgeAIWallet}
                disabled={isLoading}
                className="w-full py-2.5 sm:py-3 bg-purple-600 hover:bg-purple-500 rounded-lg font-medium text-white flex items-center justify-center gap-2 text-sm"
              >
                {isLoading ? (
                  <Loader2 className="w-4 h-4 animate-spin" />
                ) : (
                  <>
                    <Wallet className="w-4 h-4" />
                    Create EdgeAI Wallet
                  </>
                )}
              </button>
            )}
          </div>

          {/* 刷新按钮 */}
          {isAnyWalletConnected && (
            <div className="p-2 sm:p-3 border-t border-gray-700">
              <button
                onClick={refreshBalances}
                disabled={isLoading}
                className="w-full py-2 bg-gray-700 hover:bg-gray-600 rounded-lg text-xs sm:text-sm font-medium text-gray-300 flex items-center justify-center gap-2"
              >
                <RefreshCw className={`w-3 h-3 sm:w-4 sm:h-4 ${isLoading ? "animate-spin" : ""}`} />
                Refresh Balances
              </button>
            </div>
          )}
        </div>
      )}

      {/* 点击外部关闭 */}
      {isDropdownOpen && (
        <div
          className="fixed inset-0 z-40"
          onClick={() => setIsDropdownOpen(false)}
        />
      )}
    </div>
  );
}
