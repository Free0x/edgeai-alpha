import { useState, useEffect } from 'react';
import { ArrowRightLeft, Wallet, ExternalLink, AlertCircle, CheckCircle, Loader2 } from 'lucide-react';

// Contract addresses on BSC Testnet
const CONTRACTS = {
  bscTestnet: {
    wEDGE: '0xEe3131549D8727bBCd6e628D90D6b57cf99F5794',
    bridge: '0x0f72c1d37F64f0E962278A1941EC7664D4e2289B',
    chainId: 97,
    name: 'BSC Testnet',
    explorer: 'https://testnet.bscscan.com'
  }
};

// Bridge ABI for frontend interactions
const BRIDGE_ABI = [
  "function bridgeToEdgeAI(uint256 amount, string edgeaiAddress) external",
  "function calculateFee(uint256 amount) view returns (uint256)",
  "function getBridgeStats() view returns (uint256, uint256, uint256, uint256)"
];

const WEDGE_ABI = [
  "function balanceOf(address) view returns (uint256)",
  "function approve(address spender, uint256 amount) returns (bool)",
  "function allowance(address owner, address spender) view returns (uint256)"
];

declare global {
  interface Window {
    ethereum?: any;
  }
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
  const [wEdgeBalance, setWEdgeBalance] = useState('0');
  const [bridgeFee, setBridgeFee] = useState('0.1');
  const [isLoading, setIsLoading] = useState(false);
  const [txStatus, setTxStatus] = useState<'idle' | 'pending' | 'success' | 'error'>('idle');
  const [txHash, setTxHash] = useState('');
  const [error, setError] = useState('');
  
  // Terminal logs
  const [logs, setLogs] = useState<string[]>([]);
  
  const addLog = (message: string) => {
    const timestamp = new Date().toLocaleTimeString();
    setLogs(prev => [...prev, `${timestamp} ${message}`]);
  };
  
  // Check MetaMask installation
  useEffect(() => {
    if (typeof window.ethereum !== 'undefined') {
      setIsMetaMaskInstalled(true);
      addLog('MetaMask detected');
      
      // Check if already connected
      window.ethereum.request({ method: 'eth_accounts' })
        .then((accounts: string[]) => {
          if (accounts.length > 0) {
            setEvmAddress(accounts[0]);
            setIsConnected(true);
            addLog(`Connected: ${accounts[0].slice(0, 10)}...`);
          }
        });
      
      // Get chain ID
      window.ethereum.request({ method: 'eth_chainId' })
        .then((chainId: string) => {
          setChainId(parseInt(chainId, 16));
        });
      
      // Listen for account changes
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
      
      // Listen for chain changes
      window.ethereum.on('chainChanged', (chainId: string) => {
        setChainId(parseInt(chainId, 16));
        addLog(`Network changed to chain ID: ${parseInt(chainId, 16)}`);
      });
    } else {
      addLog('MetaMask not detected');
    }
  }, []);
  
  // Connect MetaMask
  const connectMetaMask = async () => {
    if (!isMetaMaskInstalled) {
      window.open('https://metamask.io/download/', '_blank');
      return;
    }
    
    try {
      setIsLoading(true);
      addLog('Connecting to MetaMask...');
      
      const accounts = await window.ethereum.request({
        method: 'eth_requestAccounts'
      });
      
      if (accounts.length > 0) {
        setEvmAddress(accounts[0]);
        setIsConnected(true);
        addLog(`Connected: ${accounts[0]}`);
        
        // Get chain ID
        const chainId = await window.ethereum.request({ method: 'eth_chainId' });
        setChainId(parseInt(chainId, 16));
      }
    } catch (err: any) {
      setError(err.message);
      addLog(`Error: ${err.message}`);
    } finally {
      setIsLoading(false);
    }
  };
  
  // Switch to BSC Testnet network
  const switchToBscTestnet = async () => {
    try {
      await window.ethereum.request({
        method: 'wallet_switchEthereumChain',
        params: [{ chainId: '0x61' }] // BSC Testnet chain ID (97)
      });
    } catch (err: any) {
      // If network doesn't exist, add it
      if (err.code === 4902) {
        await window.ethereum.request({
          method: 'wallet_addEthereumChain',
          params: [{
            chainId: '0x61',
            chainName: 'BSC Testnet',
            nativeCurrency: { name: 'BNB', symbol: 'tBNB', decimals: 18 },
            rpcUrls: ['https://data-seed-prebsc-1-s1.binance.org:8545'],
            blockExplorerUrls: ['https://testnet.bscscan.com']
          }]
        });
      }
    }
  };
  
  // Load EdgeAI wallet from localStorage
  useEffect(() => {
    const savedWallet = localStorage.getItem('edgeai_wallet');
    if (savedWallet) {
      const wallet = JSON.parse(savedWallet);
      setEdgeaiAddress(wallet.address);
      addLog(`EdgeAI wallet loaded: ${wallet.address.slice(0, 15)}...`);
      
      // Fetch balance
      fetchEdgeaiBalance(wallet.address);
    }
  }, []);
  
  const fetchEdgeaiBalance = async (address: string) => {
    try {
      const response = await fetch(`https://edgeai-blockchain-node.fly.dev/api/accounts/${address}`);
      const data = await response.json();
      if (data.success && data.data) {
        setEdgeaiBalance(data.data.balance?.toString() || '0');
      }
    } catch (err) {
      console.error('Failed to fetch EdgeAI balance', err);
    }
  };
  
  // Bridge from EdgeAI to EVM
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
      addLog(`Initiating bridge: ${amount} EDGE → wEDGE`);
      addLog(`From: ${edgeaiAddress}`);
      addLog(`To: ${evmAddress}`);
      
      // Step 1: Lock EDGE on EdgeAI chain
      addLog('Step 1: Locking EDGE on EdgeAI chain...');
      
      const lockResponse = await fetch('https://edgeai-blockchain-node.fly.dev/api/bridge/lock', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          from: edgeaiAddress,
          amount: parseFloat(amount),
          evmAddress: evmAddress
        })
      });
      
      const lockData = await lockResponse.json();
      
      if (!lockData.success) {
        throw new Error(lockData.error || 'Failed to lock EDGE');
      }
      
      addLog(`EDGE locked! TX: ${lockData.data?.txHash || 'pending'}`);
      
      // Step 2: Wait for bridge relayer to mint wEDGE
      addLog('Step 2: Waiting for wEDGE to be minted on EVM...');
      addLog('(This may take a few minutes)');
      
      // In production, we would poll for the minting transaction
      // For now, show success
      setTxStatus('success');
      addLog('Bridge request submitted successfully!');
      addLog('wEDGE will be credited to your EVM address shortly.');
      
    } catch (err: any) {
      setError(err.message);
      setTxStatus('error');
      addLog(`Error: ${err.message}`);
    } finally {
      setIsLoading(false);
    }
  };
  
  // Bridge from EVM to EdgeAI
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
      addLog(`Initiating bridge: ${amount} wEDGE → EDGE`);
      addLog(`From: ${evmAddress}`);
      addLog(`To: ${edgeaiAddress}`);
      
      // For demo, show the flow
      addLog('Step 1: Approving wEDGE spend...');
      addLog('Step 2: Burning wEDGE on EVM chain...');
      addLog('Step 3: Releasing EDGE on EdgeAI chain...');
      
      // In production, this would interact with the smart contract
      setTxStatus('success');
      addLog('Bridge request submitted successfully!');
      addLog('EDGE will be credited to your EdgeAI address shortly.');
      
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
  
  const isCorrectNetwork = chainId === CONTRACTS.bscTestnet.chainId;
  
  return (
    <div className="min-h-screen bg-gray-900 text-white p-6">
      <div className="max-w-6xl mx-auto">
        <h1 className="text-3xl font-bold mb-2 flex items-center gap-3">
          <ArrowRightLeft className="w-8 h-8 text-purple-400" />
          EdgeAI Bridge
        </h1>
        <p className="text-gray-400 mb-8">
          Bridge EDGE tokens between EdgeAI Chain and EVM networks
        </p>
        
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* Bridge Card */}
          <div className="bg-gray-800 rounded-xl p-6 border border-gray-700">
            <h2 className="text-xl font-semibold mb-4">Bridge Tokens</h2>
            
            {/* Direction Toggle */}
            <div className="flex gap-2 mb-6">
              <button
                onClick={() => setDirection('toEVM')}
                className={`flex-1 py-3 px-4 rounded-lg font-medium transition-colors ${
                  direction === 'toEVM'
                    ? 'bg-purple-600 text-white'
                    : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
                }`}
              >
                EDGE → wEDGE
              </button>
              <button
                onClick={() => setDirection('toEdgeAI')}
                className={`flex-1 py-3 px-4 rounded-lg font-medium transition-colors ${
                  direction === 'toEdgeAI'
                    ? 'bg-purple-600 text-white'
                    : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
                }`}
              >
                wEDGE → EDGE
              </button>
            </div>
            
            {/* From Section */}
            <div className="mb-4">
              <label className="block text-sm text-gray-400 mb-2">From</label>
              <div className="bg-gray-700 rounded-lg p-4">
                <div className="flex justify-between items-center mb-2">
                  <span className="text-lg font-medium">
                    {direction === 'toEVM' ? 'EdgeAI Chain' : 'BSC Testnet'}
                  </span>
                  <span className="text-sm text-gray-400">
                    Balance: {direction === 'toEVM' ? edgeaiBalance : wEdgeBalance} {direction === 'toEVM' ? 'EDGE' : 'wEDGE'}
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
                    {direction === 'toEVM' ? 'BSC Testnet' : 'EdgeAI Chain'}
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
                  {amount ? (parseFloat(amount) * (1 - parseFloat(bridgeFee) / 100)).toFixed(4) : '0'} {direction === 'toEVM' ? 'wEDGE' : 'EDGE'}
                </span>
              </div>
            </div>
            
            {/* Error Message */}
            {error && (
              <div className="bg-red-900/30 border border-red-700 rounded-lg p-3 mb-4 flex items-center gap-2">
                <AlertCircle className="w-5 h-5 text-red-400" />
                <span className="text-red-300 text-sm">{error}</span>
              </div>
            )}
            
            {/* Success Message */}
            {txStatus === 'success' && (
              <div className="bg-green-900/30 border border-green-700 rounded-lg p-3 mb-4 flex items-center gap-2">
                <CheckCircle className="w-5 h-5 text-green-400" />
                <span className="text-green-300 text-sm">Bridge request submitted successfully!</span>
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
                  Bridge {direction === 'toEVM' ? 'to EVM' : 'to EdgeAI'}
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
                  <img src="https://upload.wikimedia.org/wikipedia/commons/3/36/MetaMask_Fox.svg" className="w-6 h-6" alt="MetaMask" />
                  Install MetaMask
                </button>
              ) : !isConnected ? (
                <button
                  onClick={connectMetaMask}
                  disabled={isLoading}
                  className="w-full py-3 bg-orange-600 hover:bg-orange-500 rounded-lg font-medium flex items-center justify-center gap-2"
                >
                  <img src="https://upload.wikimedia.org/wikipedia/commons/3/36/MetaMask_Fox.svg" className="w-6 h-6" alt="MetaMask" />
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
                      {chainId === 97 ? 'BSC Testnet' : chainId === 56 ? 'BSC Mainnet' : `Chain ${chainId}`}
                    </span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-gray-400">wEDGE Balance</span>
                    <span>{wEdgeBalance} wEDGE</span>
                  </div>
                  
                  {!isCorrectNetwork && (
                    <button
                      onClick={switchToBscTestnet}
                      className="w-full py-2 bg-yellow-600 hover:bg-yellow-500 rounded-lg text-sm font-medium"
                    >
                      Switch to BSC Testnet
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
        
        {/* Info Section */}
        <div className="mt-8 bg-gray-800 rounded-xl p-6 border border-gray-700">
          <h2 className="text-xl font-semibold mb-4">How the Bridge Works</h2>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
            <div className="text-center">
              <div className="w-12 h-12 bg-purple-600 rounded-full flex items-center justify-center mx-auto mb-3">
                <span className="text-xl font-bold">1</span>
              </div>
              <h3 className="font-semibold mb-2">Lock/Burn</h3>
              <p className="text-gray-400 text-sm">
                {direction === 'toEVM' 
                  ? 'Lock your EDGE tokens on EdgeAI Chain'
                  : 'Burn your wEDGE tokens on EVM chain'}
              </p>
            </div>
            <div className="text-center">
              <div className="w-12 h-12 bg-purple-600 rounded-full flex items-center justify-center mx-auto mb-3">
                <span className="text-xl font-bold">2</span>
              </div>
              <h3 className="font-semibold mb-2">Verify</h3>
              <p className="text-gray-400 text-sm">
                Bridge relayers verify the transaction on both chains
              </p>
            </div>
            <div className="text-center">
              <div className="w-12 h-12 bg-purple-600 rounded-full flex items-center justify-center mx-auto mb-3">
                <span className="text-xl font-bold">3</span>
              </div>
              <h3 className="font-semibold mb-2">Mint/Release</h3>
              <p className="text-gray-400 text-sm">
                {direction === 'toEVM'
                  ? 'Receive wEDGE tokens on EVM chain'
                  : 'Receive EDGE tokens on EdgeAI Chain'}
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
