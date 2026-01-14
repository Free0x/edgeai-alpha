import { API_BASE_URL } from "../lib/config";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { Activity, Box, Clock, Database, Hash, Layers, ArrowLeft, RefreshCw, Search, X } from "lucide-react";
import { useEffect, useState } from "react";
import { Link } from "wouter";

// Corrected Types based on actual API response
interface BlockHeader {
  version: number;
  previous_hash: string;
  merkle_root: string;
  timestamp: string; // ISO string
  difficulty: number;
  nonce: number;
  data_entropy: number;
}

interface TransactionOutput {
  amount: number;
  recipient: string;
  data_hash: string | null;
}

interface Transaction {
  id: string;
  tx_type: string;
  timestamp: string; // ISO string
  sender: string;
  sender_public_key: string | null;
  inputs: any[]; // Assuming inputs structure, can be refined if needed
  outputs: TransactionOutput[];
  data: string | null;
  data_quality: number | null;
  gas_price: number;
  gas_limit: number;
  hash: string;
  signature: string | null;
}

interface Block {
  index: number;
  header: BlockHeader;
  transactions: Transaction[];
  hash: string;
  validator: string;
}

interface ChainInfo {
  length: number;
  total_transactions: number;
  total_accounts: number;
  difficulty: number;
  blockchain_id: string;
}

const API_URL = API_BASE_URL;

export default function Explorer() {
  const [chainInfo, setChainInfo] = useState<ChainInfo | null>(null);
  const [blocks, setBlocks] = useState<Block[]>([]);
  const [loading, setLoading] = useState(true);
  const [lastUpdated, setLastUpdated] = useState<Date>(new Date());
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<{ blocks: Block[], transactions: Transaction[] } | null>(null);

  const fetchData = async () => {
    try {
      setLoading(true);
      // Fetch chain info
      const infoRes = await fetch(`${API_URL}/chain`, {
        mode: 'cors',
        headers: {
          'Accept': 'application/json'
        }
      });
      if (!infoRes.ok) throw new Error(`Chain info fetch failed: ${infoRes.status}`);
      const infoResponse = await infoRes.json();
      // Handle API wrapper { success: true, data: ... }
      const infoData = infoResponse.data || infoResponse; 
      setChainInfo(infoData);

      // Fetch blocks
      const blocksRes = await fetch(`${API_URL}/blocks`, {
        mode: 'cors',
        headers: {
          'Accept': 'application/json'
        }
      });
      if (!blocksRes.ok) throw new Error(`Blocks fetch failed: ${blocksRes.status}`);
      const blocksResponse = await blocksRes.json();
      // Handle API wrapper { success: true, data: ... }
      const blocksData = Array.isArray(blocksResponse.data) ? blocksResponse.data : (Array.isArray(blocksResponse) ? blocksResponse : []);
      
      // Sort by index descending
      setBlocks(blocksData.sort((a: Block, b: Block) => b.index - a.index));
      
      setLastUpdated(new Date());
    } catch (error) {
      console.error("Failed to fetch blockchain data", error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchData();
    // Auto-refresh every 10 seconds
    const interval = setInterval(fetchData, 10000);
    return () => clearInterval(interval);
  }, []);

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    if (!searchQuery.trim()) {
      setSearchResults(null);
      return;
    }

    const query = searchQuery.trim().toLowerCase();
    
    // Search in local blocks
    const matchedBlocks = blocks.filter(block => 
      block.hash.toLowerCase().includes(query) || 
      block.index.toString() === query
    );

    // Search in transactions (flattened from blocks)
    const allTransactions = blocks.flatMap(b => b.transactions);
    const matchedTransactions = allTransactions.filter(tx => 
      tx.id.toLowerCase().includes(query) || 
      tx.sender.toLowerCase().includes(query) ||
      (tx.outputs[0]?.recipient || "").toLowerCase().includes(query)
    );

    setSearchResults({
      blocks: matchedBlocks,
      transactions: matchedTransactions
    });
  };

  const clearSearch = () => {
    setSearchQuery("");
    setSearchResults(null);
  };

  return (
    <div className="min-h-screen bg-background text-foreground p-6 md:p-12 font-mono selection:bg-primary selection:text-primary-foreground">
      <div className="max-w-7xl mx-auto space-y-8">
        {/* Header */}
        <div className="flex flex-col md:flex-row justify-between items-start md:items-center gap-4">
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <Link href="/" className="p-2 hover:bg-muted rounded-full transition-colors">
                <ArrowLeft className="w-5 h-5 text-muted-foreground" />
              </Link>
              <h1 className="text-3xl font-bold tracking-tighter text-primary">
                EdgeAI Explorer
              </h1>
            </div>
            <p className="text-muted-foreground pl-11">
              Real-time blockchain surveillance system.
            </p>
          </div>
          
          <div className="flex items-center gap-4">
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              <Clock className="w-3 h-3" />
              <span>Updated: {lastUpdated.toLocaleTimeString()}</span>
            </div>
            <button 
              onClick={fetchData}
              disabled={loading}
              className={cn(
                "p-2 rounded-md border border-border bg-card hover:bg-accent transition-all",
                loading && "opacity-50 cursor-not-allowed"
              )}
            >
              <RefreshCw className={cn("w-4 h-4", loading && "animate-spin")} />
            </button>
          </div>
        </div>

        {/* Search Bar */}
        <div className="w-full max-w-2xl mx-auto">
          <form onSubmit={handleSearch} className="relative flex items-center gap-2">
            <div className="relative flex-1">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
              <Input 
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="Search by Block Hash, Tx ID, or Address..." 
                className="pl-9 pr-9 bg-card/50 border-border focus:border-primary transition-colors"
              />
              {searchQuery && (
                <button 
                  type="button"
                  onClick={clearSearch}
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                >
                  <X className="w-4 h-4" />
                </button>
              )}
            </div>
            <Button type="submit" variant="secondary">
              Search
            </Button>
          </form>
        </div>

        {/* Search Results */}
        {searchResults && (
          <div className="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-300">
            <div className="flex items-center justify-between">
              <h2 className="text-xl font-semibold flex items-center gap-2">
                <Search className="w-5 h-5 text-primary" />
                Search Results
              </h2>
              <Button variant="ghost" size="sm" onClick={clearSearch} className="text-muted-foreground hover:text-foreground">
                Clear Results
              </Button>
            </div>

            {searchResults.blocks.length === 0 && searchResults.transactions.length === 0 ? (
              <div className="text-center py-12 border border-dashed border-border rounded-lg bg-card/30">
                <p className="text-muted-foreground">No results found for "{searchQuery}"</p>
              </div>
            ) : (
              <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
                {searchResults.blocks.length > 0 && (
                  <div className="space-y-4">
                    <h3 className="text-sm font-medium text-muted-foreground uppercase tracking-wider">Blocks ({searchResults.blocks.length})</h3>
                    <div className="space-y-4">
                      {searchResults.blocks.map(block => (
                        <BlockCard key={block.hash} block={block} />
                      ))}
                    </div>
                  </div>
                )}
                
                {searchResults.transactions.length > 0 && (
                  <div className="space-y-4">
                    <h3 className="text-sm font-medium text-muted-foreground uppercase tracking-wider">Transactions ({searchResults.transactions.length})</h3>
                    <div className="space-y-4">
                      {searchResults.transactions.map(tx => (
                        <TransactionCard key={tx.id} tx={tx} />
                      ))}
                    </div>
                  </div>
                )}
              </div>
            )}
            <Separator className="my-8" />
          </div>
        )}

        {/* Stats Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          <StatsCard 
            title="Block Height" 
            value={chainInfo?.length?.toString() ?? "-"} 
            icon={<Layers className="w-4 h-4 text-blue-400" />}
            loading={loading}
          />
          <StatsCard 
            title="Total Transactions" 
            value={chainInfo?.total_transactions?.toString() ?? "-"} 
            icon={<Activity className="w-4 h-4 text-green-400" />}
            loading={loading}
          />
          <StatsCard 
            title="Active Accounts" 
            value={chainInfo?.total_accounts?.toString() ?? "-"} 
            icon={<Hash className="w-4 h-4 text-purple-400" />}
            loading={loading}
          />
          <StatsCard 
            title="Network Difficulty" 
            value={chainInfo?.difficulty?.toString() ?? "-"} 
            icon={<Database className="w-4 h-4 text-orange-400" />}
            loading={loading}
          />
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
          {/* Blocks List */}
          <div className="lg:col-span-2 space-y-4">
            <div className="flex items-center justify-between">
              <h2 className="text-xl font-semibold flex items-center gap-2">
                <Box className="w-5 h-5 text-primary" />
                Latest Blocks
              </h2>
              <Badge variant="outline" className="font-mono">
                {blocks.length} Total
              </Badge>
            </div>
            
            <ScrollArea className="h-[600px] rounded-lg border border-border bg-card/50 backdrop-blur-sm">
              <div className="p-4 space-y-4">
                {blocks.map((block) => (
                  <BlockCard key={block.hash} block={block} />
                ))}
              </div>
            </ScrollArea>
          </div>

          {/* Recent Transactions (Flattened from blocks) */}
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h2 className="text-xl font-semibold flex items-center gap-2">
                <Activity className="w-5 h-5 text-primary" />
                Recent Activity
              </h2>
            </div>
            
            <ScrollArea className="h-[600px] rounded-lg border border-border bg-card/50 backdrop-blur-sm">
              <div className="p-4 space-y-4">
                {blocks.flatMap(b => b.transactions).slice(0, 20).map((tx) => (
                  <TransactionCard key={tx.id} tx={tx} />
                ))}
                {blocks.flatMap(b => b.transactions).length === 0 && (
                  <div className="text-center text-muted-foreground py-8">
                    No transactions found
                  </div>
                )}
              </div>
            </ScrollArea>
          </div>
        </div>
      </div>
    </div>
  );
}

function StatsCard({ title, value, icon, loading }: { title: string, value: string, icon: React.ReactNode, loading: boolean }) {
  return (
    <Card className="bg-card/50 backdrop-blur-sm border-border">
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium text-muted-foreground">
          {title}
        </CardTitle>
        {icon}
      </CardHeader>
      <CardContent>
        <div className={cn("text-2xl font-bold font-mono", loading && "opacity-50")}>
          {value}
        </div>
      </CardContent>
    </Card>
  );
}

function BlockCard({ block }: { block: Block }) {
  // Safe access to nested properties
  const timestamp = block.header?.timestamp ? new Date(block.header.timestamp).toLocaleString() : "Unknown";
  const difficulty = block.header?.difficulty ?? 0;
  const entropy = block.header?.data_entropy ?? 0;
  const txCount = block.transactions?.length ?? 0;
  const validator = block.validator || "Unknown";

  return (
    <div className="group relative rounded-lg border border-border bg-card p-4 hover:bg-accent/50 transition-all">
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div className="space-y-1">
          <div className="flex items-center gap-2">
            <span className="text-lg font-bold text-primary">#{block.index}</span>
            <span className="text-xs text-muted-foreground font-mono truncate max-w-[100px] md:max-w-[200px]">
              {block.hash}
            </span>
          </div>
          <div className="flex items-center gap-4 text-xs text-muted-foreground">
            <span>Tx: {txCount}</span>
            <span>Validator: {validator.substring(0, 8)}...</span>
            <span>Entropy: {entropy.toFixed(4)}</span>
          </div>
        </div>
        <div className="text-right">
          <div className="text-xs text-muted-foreground">
            {timestamp}
          </div>
          <Badge variant="secondary" className="mt-1 text-[10px]">
            Diff: {difficulty}
          </Badge>
        </div>
      </div>
    </div>
  );
}

function TransactionCard({ tx }: { tx: Transaction }) {
  const isData = !!tx.data;
  const timestamp = tx.timestamp ? new Date(tx.timestamp).toLocaleTimeString() : "Unknown";
  
  // Extract recipient and amount from outputs if available
  const output = tx.outputs && tx.outputs.length > 0 ? tx.outputs[0] : null;
  const recipient = output?.recipient || "Unknown";
  const amount = output?.amount || 0;
  
  return (
    <div className="rounded-lg border border-border bg-card p-3 text-sm hover:border-primary/50 transition-colors">
      <div className="flex justify-between items-start mb-2">
        <Badge variant={isData ? "default" : "outline"} className="text-[10px]">
          {isData ? "DATA" : "TRANSFER"}
        </Badge>
        <span className="text-[10px] text-muted-foreground">
          {timestamp}
        </span>
      </div>
      
      <div className="space-y-1 font-mono text-xs">
        <div className="flex justify-between">
          <span className="text-muted-foreground">From:</span>
          <span className="truncate max-w-[120px]">{tx.sender}</span>
        </div>
        <div className="flex justify-between">
          <span className="text-muted-foreground">To:</span>
          <span className="truncate max-w-[120px]">{recipient}</span>
        </div>
        
        <Separator className="my-2" />
        
        {isData ? (
          <div className="bg-muted/50 p-2 rounded text-[10px] break-all">
            {tx.data}
          </div>
        ) : (
          <div className="flex justify-between items-center font-bold text-primary">
            <span>Amount:</span>
            <span>{amount} EDGE</span>
          </div>
        )}
      </div>
    </div>
  );
}
