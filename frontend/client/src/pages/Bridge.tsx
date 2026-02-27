import { API_BASE_URL } from "../lib/config";
import { useState, useEffect, useCallback } from 'react';
import { ArrowRightLeft, Wallet, AlertCircle, CheckCircle, Loader2, Clock, ExternalLink } from 'lucide-react';

// BSC Mainnet configuration
const BSC_CONFIG = {
  chainId: 56,
  chainIdHex: '0x38',
  name: 'BNB Smart Chain',
  explorer: 'https://bscscan.com',
  edgeaiToken: '0x276b792D11B9e3712FE6A78A460a0DEb416baB0A',
  rpcUrl: 'https://bsc-dataseed1.binance.org',
};

declare global {
  interface Window {
    ethereum?: any;
  }
}

interface BridgeRequest {
  request_id: string;
  edge_address: string;
  evm_address: string;
  amount: number;
  fee: number;
  net_amount: number;
  status: string;
  target_chain: string;
  created_at: string;
  completed_at?: string;
  evm_tx_hash?: string;
}

export default function Bridge() {
  // Wallet state
  const [isMetaMaskInstalled, setIsMetaMaskInstalled] = useState(false);
  const [isConnected, setIsConnected] = useState(false);
  const [evmAddress, setEvmAddress] = useState('');
  const [chainId, setChainId] = useState<number | null>(null);

  // EdgeAI wallet state
  const [edgeaiAddress, setEdgeaiAddress] = useState('');
  const [edgeaiBalance, setEdgeaiBalance] = useState('0');

  // Bridge state
  const [direction, setDirection] = useState<'toEVM' | 'toEdgeAI'>('toEVM');
  const [amount, setAmount] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [txStatus, setTxStatus] = useState<'idle' | 'pending' | 'success' | 'error'>('idle');
  const [error, setError] = useState('');
  const [requestId, setRequestId] = useState('');

  // Bridge history
  const [history, setHistory] = useState<BridgeRequest[]>([]);
  const [stats, setStats] = useState<any>(null);

  // Terminal logs
  const [logs, setLogs] = useState<string[]>([]);

  const addLog = useCallback((message: string) => {
    const timestamp = new Date().toLocaleTimeString();
    setLogs(prev => [...prev.slice(-49), `${timestamp} ${message}`]);
  }, []);

  // Check MetaMask installation
  useEffect(() => {
    if (typeof window.ethereum !== 'undefined') {
      setIsMetaMaskInstalled(true);
      addLog('MetaMask detected');

      window.ethereum.request({ method: 'eth_accounts' })
        .then((accounts: string[]) => {
          if (accounts.length > 0) {
            setEvmAddress(accounts[0]);
            setIsConnected(true);
            addLog(`Connected: ${accounts[0].slice(0, 10)}...`);
          }
        });

      window.ethereum.request({ method: 'eth_chainId' })
        .then((id: string) => setChainId(parseInt(id, 16)));

      window.ethereum.on('accountsChanged', (accounts: string[]) => {
        if (accounts.length > 0) {
          setEvmAddress(accounts[0]);
          setIsConnected(true);
          addLog(`Account changed: ${accounts[0].slice(0, 10)}...`);
        } else {
          setEvmAddress('');
          setIsConnected(false);
          addLog('Wallet disconnected');
        }
      });

      window.ethereum.on('chainChanged', (id: string) => {
        setChainId(parseInt(id, 16));
        addLog(`Network changed to chain ID: ${parseInt(id, 16)}`);
      });
    } else {
      addLog('MetaMask not detected');
    }
  }, [addLog]);

  // Connect MetaMask
  const connectMetaMask = async () => {
    if (!isMetaMaskInstalled) {
      window.open('https://metamask.io/download/', '_blank');
      return;
    }

    try {
      setIsLoading(true);
      addLog('Connecting to MetaMask...');
      const accounts = await window.ethereum.request({ method: 'eth_requestAccounts' });
      if (accounts.length > 0) {
        setEvmAddress(accounts[0]);
        setIsConnected(true);
        addLog(`Connected: ${accounts[0]}`);
        const id = await window.ethereum.request({ method: 'eth_chainId' });
        setChainId(parseInt(id, 16));
      }
    } catch (err: any) {
      setError(err.message);
      addLog(`Error: ${err.message}`);
    } finally {
      setIsLoading(false);
    }
  };

  // Switch to BSC Mainnet
  const switchToBscMainnet = async () => {
    try {
      await window.ethereum.request({
        method: 'wallet_switchEthereumChain',
        params: [{ chainId: BSC_CONFIG.chainIdHex }]
      });
    } catch (err: any) {
      if (err.code === 4902) {
        await window.ethereum.request({
          method: 'wallet_addEthereumChain',
          params: [{
            chainId: BSC_CONFIG.chainIdHex,
            chainName: BSC_CONFIG.name,
            nativeCurrency: { name: 'BNB', symbol: 'BNB', decimals: 18 },
            rpcUrls: [BSC_CONFIG.rpcUrl],
            blockExplorerUrls: [BSC_CONFIG.explorer]
          }]
        });
      }
    }
  };

  // Load EdgeAI wallet from localStorage
  useEffect(() => {
    const savedWallet = localStorage.getItem('edgeai_wallet');
    if (savedWallet) {
      try {
        const wallet = JSON.parse(savedWallet);
        setEdgeaiAddress(wallet.address);
        addLog(`EdgeAI wallet loaded: ${wallet.address.slice(0, 15)}...`);
        fetchEdgeaiBalance(wallet.address);
      } catch {
        addLog('Failed to load EdgeAI wallet');
        localStorage.removeItem('edgeai_wallet');
      }
    }
  }, [addLog]);

  const fetchEdgeaiBalance = async (address: string) => {
    try {
      const response = await fetch(`${API_BASE_URL}/accounts/${address}`);
      const data = await response.json();
      if (data.success && data.data) {
        setEdgeaiBalance(data.data.balance?.toString() || '0');
      }
    } catch (err) {
      console.error('Failed to fetch EdgeAI balance', err);
    }
  };

  // Fetch bridge stats
  useEffect(() => {
    const fetchStats = async () => {
      try {
        const res = await fetch(`${API_BASE_URL}/bridge/stats`);
        const data = await res.json();
        if (data.success) setStats(data.data);
      } catch { /* ignore */ }
    };
    fetchStats();
  }, []);

  // Fetch bridge history for current wallet
  useEffect(() => {
    if (!edgeaiAddress) return;
    const fetchHistory = async () => {
      try {
        const res = await fetch(`${API_BASE_URL}/bridge/history?address=${edgeaiAddress}`);
        const data = await res.json();
        if (data.success) setHistory(data.data || []);
      } catch { /* ignore */ }
    };
    fetchHistory();
  }, [edgeaiAddress, txStatus]);

  // Bridge from EdgeAI to BSC (lock EDGE → receive EDGEAI)
  const bridgeToEVM = async () => {
    if (!amount || parseFloat(amount) <= 0) {
      setError('Please enter a valid amount');
      return;
    }
    if (!evmAddress) {
      setError('Please connect MetaMask first');
      return;
    }
    if (!edgeaiAddress) {
      setError('Please create an EdgeAI wallet first');
      return;
    }

    setIsLoading(true);
    setTxStatus('pending');
    setError('');

    try {
      addLog(`Initiating bridge: ${amount} EDGE → EDGEAI`);
      addLog(`From: ${edgeaiAddress} (EdgeAI Chain)`);
      addLog(`To: ${evmAddress} (BSC Mainnet)`);
      addLog('Locking EDGE on EdgeAI Chain...');

      const lockResponse = await fetch(`${API_BASE_URL}/bridge/lock`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          edge_address: edgeaiAddress,
          evm_address: evmAddress,
          amount: parseFloat(amount),
          target_chain: 'bsc_mainnet'
        })
      });

      const lockData = await lockResponse.json();

      if (!lockData.success) {
        throw new Error(lockData.error || 'Failed to lock EDGE');
      }

      const rid = lockData.data?.request_id || '';
      setRequestId(rid);
      addLog(`EDGE locked successfully!`);
      addLog(`Request ID: ${rid}`);
      addLog('Your EDGEAI will be sent to your BSC address.');
      addLog('Processing time: typically within 24 hours.');

      setTxStatus('success');
      setAmount('');

      // Refresh balance
      fetchEdgeaiBalance(edgeaiAddress);
    } catch (err: any) {
      setError(err.message);
      setTxStatus('error');
      addLog(`Error: ${err.message}`);
    } finally {
      setIsLoading(false);
    }
  };

  // Bridge from BSC to EdgeAI (send EDGEAI → receive EDGE)
  const bridgeToEdgeAI = async () => {
    if (!amount || parseFloat(amount) <= 0) {
      setError('Please enter a valid amount');
      return;
    }
    if (!evmAddress) {
      setError('Please connect MetaMask first');
      return;
    }
    if (!edgeaiAddress) {
      setError('Please enter your EdgeAI address');
      return;
    }

    setIsLoading(true);
    setTxStatus('pending');
    setError('');

    try {
      addLog(`Initiating bridge: ${amount} EDGEAI → EDGE`);
      addLog(`From: ${evmAddress} (BSC Mainnet)`);
      addLog(`To: ${edgeaiAddress} (EdgeAI Chain)`);
      addLog('Step 1: Transfer EDGEAI to the bridge reserve wallet');
      addLog('Step 2: Once confirmed, EDGE will be released on EdgeAI Chain');
      addLog('');
      addLog('Please transfer EDGEAI to the bridge reserve address.');
      addLog('After transfer, your EDGE will be released within 24 hours.');

      setTxStatus('success');
    } catch (err: any) {
      setError(err.message);
      setTxStatus('error');
      addLog(`Error: ${err.message}`);
    } finally {
      setIsLoading(false);
    }
  };

  const handleBridge = () => {
    if (direction === 'toEVM') {
      bridgeToEVM();
    } else {
      bridgeToEdgeAI();
    }
  };

  const isCorrectNetwork = chainId === BSC_CONFIG.chainId;
  const bridgeFee = 0.1;
  const netAmount = amount ? (parseFloat(amount) * (1 - bridgeFee / 100)).toFixed(4) : '0';

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'completed': return 'text-green-400';
      case 'pending': return 'text-yellow-400';
      case 'cancelled': return 'text-red-400';
      default: return 'text-gray-400';
    }
  };

  return (
    <div className="min-h-screen bg-gray-900 text-white p-6">
      <div className="max-w-6xl mx-auto">
        <h1 className="text-3xl font-bold mb-2 flex items-center gap-3">
          <ArrowRightLeft className="w-8 h-8 text-purple-400" />
          EdgeAI Bridge
        </h1>
        <p className="text-gray-400 mb-8">
          Bridge EDGE tokens between EdgeAI Chain and BNB Smart Chain
        </p>

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* Bridge Card */}
          <div className="bg-gray-800 rounded-xl p-6 border border-gray-700">
            <h2 className="text-xl font-semibold mb-4">Bridge Tokens</h2>

            {/* Direction Toggle */}
            <div className="flex gap-2 mb-6">
              <button
                onClick={() => { setDirection('toEVM'); setTxStatus('idle'); setError(''); }}
                className={`flex-1 py-3 px-4 rounded-lg font-medium transition-colors ${
                  direction === 'toEVM'
                    ? 'bg-purple-600 text-white'
                    : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
                }`}
              >
                EDGE → EDGEAI
              </button>
              <button
                onClick={() => { setDirection('toEdgeAI'); setTxStatus('idle'); setError(''); }}
                className={`flex-1 py-3 px-4 rounded-lg font-medium transition-colors ${
                  direction === 'toEdgeAI'
                    ? 'bg-purple-600 text-white'
                    : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
                }`}
              >
                EDGEAI → EDGE
              </button>
            </div>

            {/* From Section */}
            <div className="mb-4">
              <label className="block text-sm text-gray-400 mb-2">From</label>
              <div className="bg-gray-700 rounded-lg p-4">
                <div className="flex justify-between items-center mb-2">
                  <span className="text-lg font-medium">
                    {direction === 'toEVM' ? 'EdgeAI Chain' : 'BNB Smart Chain'}
                  </span>
                  <span className="text-sm text-gray-400">
                    {direction === 'toEVM' ? `Balance: ${edgeaiBalance} EDGE` : ''}
                  </span>
                </div>
                <input
                  type="number"
                  value={amount}
                  onChange={(e) => setAmount(e.target.value)}
                  placeholder="0.0"
                  className="w-full bg-transparent text-2xl font-mono outline-none"
                />
              </div>
            </div>

            {/* Arrow */}
            <div className="flex justify-center my-2">
              <div className="bg-gray-700 p-2 rounded-full">
                <ArrowRightLeft className="w-5 h-5 text-purple-400" />
              </div>
            </div>

            {/* To Section */}
            <div className="mb-4">
              <label className="block text-sm text-gray-400 mb-2">To</label>
              <div className="bg-gray-700 rounded-lg p-4">
                <div className="flex justify-between items-center mb-2">
                  <span className="text-lg font-medium">
                    {direction === 'toEVM' ? 'BNB Smart Chain' : 'EdgeAI Chain'}
                  </span>
                </div>
                {direction === 'toEVM' ? (
                  <div className="text-sm text-gray-400 truncate">
                    {evmAddress || 'Connect MetaMask'}
                  </div>
                ) : (
                  <input
                    type="text"
                    value={edgeaiAddress}
                    onChange={(e) => setEdgeaiAddress(e.target.value)}
                    placeholder="Enter EdgeAI address"
                    className="w-full bg-transparent text-sm font-mono outline-none"
                  />
                )}
              </div>
            </div>

            {/* Fee Info */}
            <div className="bg-gray-700/50 rounded-lg p-3 mb-4">
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">Bridge Fee</span>
                <span>{bridgeFee}%</span>
              </div>
              <div className="flex justify-between text-sm mt-1">
                <span className="text-gray-400">You will receive</span>
                <span className="text-green-400">
                  {netAmount} {direction === 'toEVM' ? 'EDGEAI' : 'EDGE'}
                </span>
              </div>
              <div className="flex justify-between text-sm mt-1">
                <span className="text-gray-400">Processing time</span>
                <span className="text-gray-300">Up to 24 hours</span>
              </div>
            </div>

            {/* Error Message */}
            {error && (
              <div className="bg-red-900/30 border border-red-700 rounded-lg p-3 mb-4 flex items-center gap-2">
                <AlertCircle className="w-5 h-5 text-red-400 flex-shrink-0" />
                <span className="text-red-300 text-sm">{error}</span>
              </div>
            )}

            {/* Success Message */}
            {txStatus === 'success' && (
              <div className="bg-green-900/30 border border-green-700 rounded-lg p-3 mb-4 flex items-center gap-2">
                <CheckCircle className="w-5 h-5 text-green-400 flex-shrink-0" />
                <div className="text-green-300 text-sm">
                  <p>Bridge request submitted successfully!</p>
                  {requestId && <p className="text-xs mt-1 font-mono">ID: {requestId}</p>}
                </div>
              </div>
            )}

            {/* Bridge Button */}
            <button
              onClick={handleBridge}
              disabled={isLoading || !amount || (direction === 'toEVM' && !isConnected)}
              className="w-full py-4 bg-gradient-to-r from-purple-600 to-blue-600 rounded-lg font-semibold text-lg hover:from-purple-500 hover:to-blue-500 transition-all disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
            >
              {isLoading ? (
                <>
                  <Loader2 className="w-5 h-5 animate-spin" />
                  Processing...
                </>
              ) : (
                <>
                  <ArrowRightLeft className="w-5 h-5" />
                  {direction === 'toEVM' ? 'Bridge to BSC' : 'Bridge to EdgeAI'}
                </>
              )}
            </button>
          </div>

          {/* Wallet & Info Panel */}
          <div className="space-y-6">
            {/* MetaMask Connection */}
            <div className="bg-gray-800 rounded-xl p-6 border border-gray-700">
              <h2 className="text-xl font-semibold mb-4 flex items-center gap-2">
                <Wallet className="w-5 h-5" />
                EVM Wallet
              </h2>

              {!isMetaMaskInstalled ? (
                <button
                  onClick={connectMetaMask}
                  className="w-full py-3 bg-orange-600 hover:bg-orange-500 rounded-lg font-medium flex items-center justify-center gap-2"
                >
                  Install MetaMask
                </button>
              ) : !isConnected ? (
                <button
                  onClick={connectMetaMask}
                  disabled={isLoading}
                  className="w-full py-3 bg-orange-600 hover:bg-orange-500 rounded-lg font-medium flex items-center justify-center gap-2"
                >
                  Connect MetaMask
                </button>
              ) : (
                <div className="space-y-3">
                  <div className="flex items-center justify-between">
                    <span className="text-gray-400">Address</span>
                    <span className="font-mono text-sm">{evmAddress.slice(0, 8)}...{evmAddress.slice(-6)}</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-gray-400">Network</span>
                    <span className={isCorrectNetwork ? 'text-green-400' : 'text-yellow-400'}>
                      {chainId === 56 ? 'BNB Smart Chain' : chainId === 97 ? 'BSC Testnet' : `Chain ${chainId}`}
                    </span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-gray-400">EDGEAI Token</span>
                    <a
                      href={`${BSC_CONFIG.explorer}/token/${BSC_CONFIG.edgeaiToken}`}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-purple-400 hover:text-purple-300 text-sm flex items-center gap-1"
                    >
                      View on BSCScan <ExternalLink className="w-3 h-3" />
                    </a>
                  </div>

                  {!isCorrectNetwork && (
                    <button
                      onClick={switchToBscMainnet}
                      className="w-full py-2 bg-yellow-600 hover:bg-yellow-500 rounded-lg text-sm font-medium"
                    >
                      Switch to BNB Smart Chain
                    </button>
                  )}
                </div>
              )}
            </div>

            {/* EdgeAI Wallet */}
            <div className="bg-gray-800 rounded-xl p-6 border border-gray-700">
              <h2 className="text-xl font-semibold mb-4">EdgeAI Wallet</h2>
              {edgeaiAddress ? (
                <div className="space-y-3">
                  <div className="flex items-center justify-between">
                    <span className="text-gray-400">Address</span>
                    <span className="font-mono text-sm">{edgeaiAddress.slice(0, 12)}...</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-gray-400">Balance</span>
                    <span>{edgeaiBalance} EDGE</span>
                  </div>
                </div>
              ) : (
                <div className="text-center py-4">
                  <p className="text-gray-400 mb-3">No EdgeAI wallet found</p>
                  <a
                    href="/wallet"
                    className="inline-block py-2 px-4 bg-purple-600 hover:bg-purple-500 rounded-lg text-sm font-medium"
                  >
                    Create Wallet
                  </a>
                </div>
              )}
            </div>

            {/* Terminal */}
            <div className="bg-gray-800 rounded-xl p-6 border border-gray-700">
              <h2 className="text-xl font-semibold mb-4">Bridge Logs</h2>
              <div className="bg-black rounded-lg p-4 h-48 overflow-y-auto font-mono text-sm">
                {logs.length === 0 ? (
                  <span className="text-gray-500">Waiting for bridge activity...</span>
                ) : (
                  logs.map((log, i) => (
                    <div key={i} className="text-green-400">{log}</div>
                  ))
                )}
              </div>
            </div>
          </div>
        </div>

        {/* Bridge History */}
        {history.length > 0 && (
          <div className="mt-8 bg-gray-800 rounded-xl p-6 border border-gray-700">
            <h2 className="text-xl font-semibold mb-4 flex items-center gap-2">
              <Clock className="w-5 h-5" />
              Bridge History
            </h2>
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="text-gray-400 border-b border-gray-700">
                    <th className="text-left py-3 px-2">Request ID</th>
                    <th className="text-left py-3 px-2">Amount</th>
                    <th className="text-left py-3 px-2">Direction</th>
                    <th className="text-left py-3 px-2">Status</th>
                    <th className="text-left py-3 px-2">Time</th>
                    <th className="text-left py-3 px-2">BSC TX</th>
                  </tr>
                </thead>
                <tbody>
                  {history.map((req) => (
                    <tr key={req.request_id} className="border-b border-gray-700/50 hover:bg-gray-700/30">
                      <td className="py-3 px-2 font-mono text-xs">{req.request_id.slice(0, 12)}...</td>
                      <td className="py-3 px-2">{req.net_amount.toLocaleString()} EDGE</td>
                      <td className="py-3 px-2">EDGE → EDGEAI</td>
                      <td className={`py-3 px-2 font-medium ${getStatusColor(req.status)}`}>
                        {req.status.charAt(0).toUpperCase() + req.status.slice(1)}
                      </td>
                      <td className="py-3 px-2 text-gray-400">{new Date(req.created_at).toLocaleDateString()}</td>
                      <td className="py-3 px-2">
                        {req.evm_tx_hash ? (
                          <a
                            href={`${BSC_CONFIG.explorer}/tx/${req.evm_tx_hash}`}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="text-purple-400 hover:text-purple-300 flex items-center gap-1"
                          >
                            View <ExternalLink className="w-3 h-3" />
                          </a>
                        ) : (
                          <span className="text-gray-500">Pending</span>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}

        {/* How it Works */}
        <div className="mt-8 bg-gray-800 rounded-xl p-6 border border-gray-700">
          <h2 className="text-xl font-semibold mb-4">How the Bridge Works</h2>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
            <div className="text-center">
              <div className="w-12 h-12 bg-purple-600 rounded-full flex items-center justify-center mx-auto mb-3">
                <span className="text-xl font-bold">1</span>
              </div>
              <h3 className="font-semibold mb-2">
                {direction === 'toEVM' ? 'Lock EDGE' : 'Transfer EDGEAI'}
              </h3>
              <p className="text-gray-400 text-sm">
                {direction === 'toEVM'
                  ? 'Lock your EDGE tokens on EdgeAI Chain via the bridge'
                  : 'Transfer your EDGEAI tokens to the bridge reserve on BSC'}
              </p>
            </div>
            <div className="text-center">
              <div className="w-12 h-12 bg-purple-600 rounded-full flex items-center justify-center mx-auto mb-3">
                <span className="text-xl font-bold">2</span>
              </div>
              <h3 className="font-semibold mb-2">Verify</h3>
              <p className="text-gray-400 text-sm">
                Bridge operators verify the transaction on both chains
              </p>
            </div>
            <div className="text-center">
              <div className="w-12 h-12 bg-purple-600 rounded-full flex items-center justify-center mx-auto mb-3">
                <span className="text-xl font-bold">3</span>
              </div>
              <h3 className="font-semibold mb-2">
                {direction === 'toEVM' ? 'Receive EDGEAI' : 'Receive EDGE'}
              </h3>
              <p className="text-gray-400 text-sm">
                {direction === 'toEVM'
                  ? 'Receive EDGEAI tokens on BNB Smart Chain'
                  : 'Receive EDGE tokens on EdgeAI Chain'}
              </p>
            </div>
          </div>
        </div>

        {/* Bridge Stats */}
        {stats && (
          <div className="mt-8 bg-gray-800 rounded-xl p-6 border border-gray-700">
            <h2 className="text-xl font-semibold mb-4">Bridge Statistics</h2>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
              <div className="bg-gray-700/50 rounded-lg p-4 text-center">
                <div className="text-2xl font-bold text-purple-400">{stats.total_requests || 0}</div>
                <div className="text-sm text-gray-400 mt-1">Total Requests</div>
              </div>
              <div className="bg-gray-700/50 rounded-lg p-4 text-center">
                <div className="text-2xl font-bold text-green-400">{stats.completed || 0}</div>
                <div className="text-sm text-gray-400 mt-1">Completed</div>
              </div>
              <div className="bg-gray-700/50 rounded-lg p-4 text-center">
                <div className="text-2xl font-bold text-yellow-400">{stats.pending || 0}</div>
                <div className="text-sm text-gray-400 mt-1">Pending</div>
              </div>
              <div className="bg-gray-700/50 rounded-lg p-4 text-center">
                <div className="text-2xl font-bold text-blue-400">{(stats.total_volume || 0).toLocaleString()}</div>
                <div className="text-sm text-gray-400 mt-1">Total Volume (EDGE)</div>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
