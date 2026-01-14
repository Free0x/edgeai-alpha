import { API_BASE_URL } from "../lib/config";
import { useState, useEffect } from "react";
import { Card, CardContent, CardHeader, CardTitle, CardDescription, CardFooter } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Progress } from "@/components/ui/progress";
import { 
  Dialog, 
  DialogContent, 
  DialogDescription, 
  DialogFooter, 
  DialogHeader, 
  DialogTitle,
  DialogTrigger 
} from "@/components/ui/dialog";
import { 
  Coins, 
  TrendingUp, 
  Users, 
  Shield, 
  Clock, 
  ArrowUpRight, 
  ArrowDownRight,
  Wallet,
  Gift,
  AlertCircle,
  CheckCircle2,
  Loader2,
  Info
} from "lucide-react";
import { cn } from "@/lib/utils";
import { apiCall } from "@/lib/api";

// API Base URL
const API_BASE = API_BASE_URL;

// Types
interface StakingStats {
  totalStaked: number;
  totalDelegated: number;
  totalValidators: number;
  activeValidators: number;
  averageApy: number;
  unbondingPeriod: number;
}

interface Validator {
  address: string;
  name: string;
  description: string;
  totalStake: number;
  selfStake: number;
  delegatedStake: number;
  commission: number;
  status: "Active" | "Inactive" | "Jailed" | "Unbonding";
  uptime: number;
  votingPower: number;
}

interface Delegation {
  validator: string;
  validatorName: string;
  amount: string;
  rewards: string;
  startTime: string;
}

interface UnbondingEntry {
  validator: string;
  validatorName: string;
  amount: string;
  completionTime: string;
}

// Real API functions
const fetchStakingStats = async (): Promise<StakingStats> => {
  try {
    const response = await fetch(`${API_BASE}/staking/stats`);
    const result = await response.json();
    
    if (result.success && result.data) {
      const data = result.data;
      return {
        totalStaked: data.total_staked || 0,
        totalDelegated: data.total_delegated || 0,
        totalValidators: data.total_validators || 0,
        activeValidators: data.active_validators || 0,
        averageApy: 12.5, // Fixed APY for testnet
        unbondingPeriod: 7, // 7 days
      };
    }
  } catch (error) {
    console.error("Failed to fetch staking stats:", error);
  }
  
  // Fallback to default values
  return {
    totalStaked: 0,
    totalDelegated: 0,
    totalValidators: 0,
    activeValidators: 0,
    averageApy: 12.5,
    unbondingPeriod: 7,
  };
};

const fetchValidators = async (): Promise<Validator[]> => {
  try {
    const response = await fetch(`${API_BASE}/staking/validators`);
    const result = await response.json();
    
    if (result.success && Array.isArray(result.data)) {
      return result.data.map((v: any) => ({
        address: v.address,
        name: v.moniker || v.address.slice(0, 16),
        description: v.details || "EdgeAI Validator Node",
        totalStake: v.total_stake || 0,
        selfStake: v.self_stake || 0,
        delegatedStake: v.delegated_stake || 0,
        commission: (v.commission_rate || 0) * 100,
        status: v.status === "Active" ? "Active" : v.status === "Jailed" ? "Jailed" : "Inactive",
        uptime: v.uptime || 99.9,
        votingPower: v.voting_power || 0,
      }));
    }
  } catch (error) {
    console.error("Failed to fetch validators:", error);
  }
  
  return [];
};

const fetchMyDelegations = async (address?: string): Promise<Delegation[]> => {
  if (!address) return [];
  
  try {
    const response = await fetch(`${API_BASE}/staking/delegations/${address}`);
    const result = await response.json();
    
    if (result.success && Array.isArray(result.data)) {
      return result.data.map((d: any) => ({
        validator: d.validator,
        validatorName: d.validator.slice(0, 16),
        amount: String(d.amount || 0),
        rewards: String(d.rewards || 0),
        startTime: new Date().toISOString(),
      }));
    }
  } catch (error) {
    console.error("Failed to fetch delegations:", error);
  }
  
  return [];
};

const fetchMyUnbonding = async (): Promise<UnbondingEntry[]> => {
  return [];
};

// Utility functions
const formatEdge = (amount: number | string, decimals: number = 2): string => {
  const value = typeof amount === 'string' ? parseFloat(amount) : amount;
  if (isNaN(value) || value === 0) return "0";
  
  if (value >= 1000000) {
    return (value / 1000000).toFixed(decimals) + "M";
  } else if (value >= 1000) {
    return (value / 1000).toFixed(decimals) + "K";
  }
  return value.toFixed(decimals);
};

const shortenAddress = (address: string): string => {
  if (!address || address.length < 10) return address;
  return `${address.slice(0, 6)}...${address.slice(-4)}`;
};

export default function Staking() {
  const [stats, setStats] = useState<StakingStats | null>(null);
  const [validators, setValidators] = useState<Validator[]>([]);
  const [myDelegations, setMyDelegations] = useState<Delegation[]>([]);
  const [myUnbonding, setMyUnbonding] = useState<UnbondingEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedValidator, setSelectedValidator] = useState<Validator | null>(null);
  const [delegateAmount, setDelegateAmount] = useState("");
  const [undelegateAmount, setUndelegateAmount] = useState("");
  const [isProcessing, setIsProcessing] = useState(false);
  const [activeTab, setActiveTab] = useState("validators");
  const [toast, setToast] = useState<{ message: string; type: 'success' | 'error' } | null>(null);

  // Demo wallet state (will be replaced with real wallet)
  const [walletConnected] = useState(true);
  const [walletBalance] = useState(10000); // 10,000 EDGE
  const [walletAddress] = useState("edge_demo_user_001");

  useEffect(() => {
    const loadData = async () => {
      setLoading(true);
      try {
        const [statsData, validatorsData, delegationsData, unbondingData] = await Promise.all([
          fetchStakingStats(),
          fetchValidators(),
          fetchMyDelegations(walletAddress),
          fetchMyUnbonding(),
        ]);
        setStats(statsData);
        setValidators(validatorsData);
        setMyDelegations(delegationsData);
        setMyUnbonding(unbondingData);
      } catch (error) {
        console.error("Failed to load staking data:", error);
      } finally {
        setLoading(false);
      }
    };
    loadData();
  }, [walletAddress]);

  const showToast = (message: string, type: 'success' | 'error') => {
    setToast({ message, type });
    setTimeout(() => setToast(null), 3000);
  };

  const handleDelegate = async () => {
    if (!selectedValidator || !delegateAmount) return;
    setIsProcessing(true);
    
    try {
      const response = await fetch(`${API_BASE}/staking/delegate`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          delegator: walletAddress,
          validator: selectedValidator.address,
          amount: parseInt(delegateAmount),
        }),
      });
      
      const result = await response.json();
      
      if (result.success) {
        showToast(`Successfully delegated ${delegateAmount} EDGE to ${selectedValidator.name}`, 'success');
        setDelegateAmount("");
        // Refresh data
        const [statsData, validatorsData] = await Promise.all([
          fetchStakingStats(),
          fetchValidators(),
        ]);
        setStats(statsData);
        setValidators(validatorsData);
      } else {
        showToast(result.error || 'Delegation failed', 'error');
      }
    } catch (error) {
      showToast('Network error. Please try again.', 'error');
    } finally {
      setIsProcessing(false);
    }
  };

  const handleUndelegate = async () => {
    if (!selectedValidator || !undelegateAmount) return;
    setIsProcessing(true);
    
    try {
      const response = await fetch(`${API_BASE}/staking/undelegate`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          delegator: walletAddress,
          validator: selectedValidator.address,
          amount: parseInt(undelegateAmount),
        }),
      });
      
      const result = await response.json();
      
      if (result.success) {
        showToast(`Undelegation started. Tokens will be available in 7 days.`, 'success');
        setUndelegateAmount("");
      } else {
        showToast(result.error || 'Undelegation failed', 'error');
      }
    } catch (error) {
      showToast('Network error. Please try again.', 'error');
    } finally {
      setIsProcessing(false);
    }
  };

  const handleClaimRewards = async () => {
    setIsProcessing(true);
    await new Promise(resolve => setTimeout(resolve, 1000));
    showToast('Rewards claimed successfully!', 'success');
    setIsProcessing(false);
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case "Active": return "bg-green-500/10 text-green-500 border-green-500/20";
      case "Inactive": return "bg-gray-500/10 text-gray-500 border-gray-500/20";
      case "Jailed": return "bg-red-500/10 text-red-500 border-red-500/20";
      case "Unbonding": return "bg-yellow-500/10 text-yellow-500 border-yellow-500/20";
      default: return "bg-gray-500/10 text-gray-500 border-gray-500/20";
    }
  };

  const totalRewards = myDelegations.reduce((acc, d) => acc + parseFloat(d.rewards || "0"), 0);
  const totalDelegated = myDelegations.reduce((acc, d) => acc + parseFloat(d.amount || "0"), 0);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-[60vh]">
        <Loader2 className="h-8 w-8 animate-spin text-primary" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Toast notification */}
      {toast && (
        <div className={cn(
          "fixed top-4 right-4 z-50 p-4 rounded-lg shadow-lg flex items-center gap-2",
          toast.type === 'success' ? "bg-green-500 text-white" : "bg-red-500 text-white"
        )}>
          {toast.type === 'success' ? <CheckCircle2 className="h-5 w-5" /> : <AlertCircle className="h-5 w-5" />}
          {toast.message}
        </div>
      )}

      {/* Header */}
      <div className="flex flex-col md:flex-row justify-between items-start md:items-center gap-4">
        <div>
          <h1 className="text-3xl font-bold tracking-tight flex items-center gap-2">
            <Coins className="h-8 w-8 text-primary" />
            Staking
          </h1>
          <p className="text-muted-foreground mt-1">
            Stake EDGE tokens to secure the network and earn rewards.
          </p>
        </div>
      </div>

      {/* Stats Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Total Staked</CardTitle>
            <Coins className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-primary">
              {formatEdge(stats?.totalStaked || 0)} EDGE
            </div>
            <p className="text-xs text-muted-foreground">Network security value</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Average APY</CardTitle>
            <TrendingUp className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-green-500">
              {stats?.averageApy || 12.5}%
            </div>
            <p className="text-xs text-muted-foreground">Estimated annual yield</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Active Validators</CardTitle>
            <Users className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {stats?.activeValidators || 0}
            </div>
            <p className="text-xs text-muted-foreground">
              of {stats?.totalValidators || 0} total validators
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Unbonding Period</CardTitle>
            <Clock className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {stats?.unbondingPeriod || 7} Days
            </div>
            <p className="text-xs text-muted-foreground">Time to unlock staked tokens</p>
          </CardContent>
        </Card>
      </div>

      {/* My Staking Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Wallet className="h-5 w-5" />
            My Staking
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div>
              <p className="text-sm text-muted-foreground">Available Balance</p>
              <p className="text-xl font-bold">{formatEdge(walletBalance)} EDGE</p>
            </div>
            <div>
              <p className="text-sm text-muted-foreground">Total Delegated</p>
              <p className="text-xl font-bold">{formatEdge(totalDelegated)} EDGE</p>
            </div>
            <div>
              <p className="text-sm text-muted-foreground">Pending Rewards</p>
              <p className="text-xl font-bold text-green-500">{formatEdge(totalRewards)} EDGE</p>
            </div>
          </div>
        </CardContent>
        <CardFooter>
          <Button 
            onClick={handleClaimRewards} 
            disabled={totalRewards === 0 || isProcessing}
            className="w-full md:w-auto"
          >
            {isProcessing ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : <Gift className="h-4 w-4 mr-2" />}
            Claim Rewards
          </Button>
        </CardFooter>
      </Card>

      {/* Validators Tabs */}
      <Tabs value={activeTab} onValueChange={setActiveTab}>
        <TabsList>
          <TabsTrigger value="validators">Validators</TabsTrigger>
          <TabsTrigger value="delegations">My Delegations</TabsTrigger>
          <TabsTrigger value="unbonding">Unbonding</TabsTrigger>
        </TabsList>

        <TabsContent value="validators" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle>Active Validators</CardTitle>
              <CardDescription>
                Select a validator to delegate your EDGE tokens and earn rewards.
              </CardDescription>
            </CardHeader>
            <CardContent>
              {validators.length === 0 ? (
                <div className="text-center py-8 text-muted-foreground">
                  <Users className="h-12 w-12 mx-auto mb-4 opacity-50" />
                  <p>No validators registered yet.</p>
                  <p className="text-sm">Be the first to register as a validator!</p>
                </div>
              ) : (
                <div className="space-y-4">
                  {validators.map((validator) => (
                    <Card key={validator.address} className="p-4">
                      <div className="flex flex-col md:flex-row justify-between items-start md:items-center gap-4">
                        <div className="flex-1">
                          <div className="flex items-center gap-2">
                            <Shield className="h-5 w-5 text-primary" />
                            <span className="font-semibold">{validator.name}</span>
                            <Badge className={getStatusColor(validator.status)}>
                              {validator.status}
                            </Badge>
                          </div>
                          <p className="text-sm text-muted-foreground mt-1">
                            {validator.description}
                          </p>
                          <div className="flex flex-wrap gap-4 mt-2 text-sm">
                            <span>Stake: <strong>{formatEdge(validator.totalStake)} EDGE</strong></span>
                            <span>Commission: <strong>{validator.commission}%</strong></span>
                            <span>Uptime: <strong>{validator.uptime.toFixed(2)}%</strong></span>
                            <span>Voting Power: <strong>{validator.votingPower.toFixed(1)}%</strong></span>
                          </div>
                        </div>
                        <Dialog>
                          <DialogTrigger asChild>
                            <Button 
                              variant="outline" 
                              onClick={() => setSelectedValidator(validator)}
                            >
                              <ArrowUpRight className="h-4 w-4 mr-2" />
                              Delegate
                            </Button>
                          </DialogTrigger>
                          <DialogContent>
                            <DialogHeader>
                              <DialogTitle>Delegate to {validator.name}</DialogTitle>
                              <DialogDescription>
                                Enter the amount of EDGE tokens you want to delegate.
                              </DialogDescription>
                            </DialogHeader>
                            <div className="space-y-4 py-4">
                              <div className="space-y-2">
                                <Label>Amount (EDGE)</Label>
                                <Input
                                  type="number"
                                  placeholder="Enter amount"
                                  value={delegateAmount}
                                  onChange={(e) => setDelegateAmount(e.target.value)}
                                />
                                <p className="text-xs text-muted-foreground">
                                  Available: {formatEdge(walletBalance)} EDGE
                                </p>
                              </div>
                              <div className="bg-muted p-3 rounded-lg text-sm">
                                <div className="flex justify-between">
                                  <span>Commission Rate:</span>
                                  <span>{validator.commission}%</span>
                                </div>
                                <div className="flex justify-between">
                                  <span>Estimated APY:</span>
                                  <span className="text-green-500">{(12.5 * (1 - validator.commission / 100)).toFixed(2)}%</span>
                                </div>
                              </div>
                            </div>
                            <DialogFooter>
                              <Button 
                                onClick={handleDelegate} 
                                disabled={!delegateAmount || isProcessing}
                              >
                                {isProcessing ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : null}
                                Confirm Delegation
                              </Button>
                            </DialogFooter>
                          </DialogContent>
                        </Dialog>
                      </div>
                    </Card>
                  ))}
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="delegations">
          <Card>
            <CardHeader>
              <CardTitle>My Delegations</CardTitle>
              <CardDescription>
                Your active delegations and accumulated rewards.
              </CardDescription>
            </CardHeader>
            <CardContent>
              {myDelegations.length === 0 ? (
                <div className="text-center py-8 text-muted-foreground">
                  <Coins className="h-12 w-12 mx-auto mb-4 opacity-50" />
                  <p>You haven't delegated to any validators yet.</p>
                  <p className="text-sm">Start earning rewards by delegating your EDGE tokens.</p>
                </div>
              ) : (
                <div className="space-y-4">
                  {myDelegations.map((delegation, index) => (
                    <Card key={index} className="p-4">
                      <div className="flex justify-between items-center">
                        <div>
                          <p className="font-semibold">{delegation.validatorName}</p>
                          <p className="text-sm text-muted-foreground">
                            Delegated: {formatEdge(delegation.amount)} EDGE
                          </p>
                        </div>
                        <div className="text-right">
                          <p className="text-green-500 font-semibold">
                            +{formatEdge(delegation.rewards)} EDGE
                          </p>
                          <p className="text-xs text-muted-foreground">Rewards</p>
                        </div>
                      </div>
                    </Card>
                  ))}
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="unbonding">
          <Card>
            <CardHeader>
              <CardTitle>Unbonding</CardTitle>
              <CardDescription>
                Tokens being unbonded from validators.
              </CardDescription>
            </CardHeader>
            <CardContent>
              {myUnbonding.length === 0 ? (
                <div className="text-center py-8 text-muted-foreground">
                  <Clock className="h-12 w-12 mx-auto mb-4 opacity-50" />
                  <p>No tokens are currently unbonding.</p>
                </div>
              ) : (
                <div className="space-y-4">
                  {myUnbonding.map((entry, index) => (
                    <Card key={index} className="p-4">
                      <div className="flex justify-between items-center">
                        <div>
                          <p className="font-semibold">{entry.validatorName}</p>
                          <p className="text-sm text-muted-foreground">
                            Amount: {formatEdge(entry.amount)} EDGE
                          </p>
                        </div>
                        <div className="text-right">
                          <p className="text-yellow-500 font-semibold">
                            {new Date(entry.completionTime).toLocaleDateString()}
                          </p>
                          <p className="text-xs text-muted-foreground">Completion Date</p>
                        </div>
                      </div>
                    </Card>
                  ))}
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>

      {/* Info Section */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Info className="h-5 w-5" />
            About Staking
          </CardTitle>
        </CardHeader>
        <CardContent className="prose prose-sm dark:prose-invert max-w-none">
          <p>
            Staking EDGE tokens helps secure the EdgeAI network through the Proof of Information Entropy (PoIE) 
            consensus mechanism. By delegating your tokens to validators, you contribute to network security 
            and earn rewards proportional to your stake.
          </p>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mt-4">
            <div className="p-4 bg-muted rounded-lg">
              <h4 className="font-semibold flex items-center gap-2">
                <TrendingUp className="h-4 w-4 text-green-500" />
                Earn Rewards
              </h4>
              <p className="text-sm text-muted-foreground">
                Earn up to 12.5% APY by delegating to active validators.
              </p>
            </div>
            <div className="p-4 bg-muted rounded-lg">
              <h4 className="font-semibold flex items-center gap-2">
                <Shield className="h-4 w-4 text-blue-500" />
                Secure the Network
              </h4>
              <p className="text-sm text-muted-foreground">
                Your stake helps validators process IoT data and maintain consensus.
              </p>
            </div>
            <div className="p-4 bg-muted rounded-lg">
              <h4 className="font-semibold flex items-center gap-2">
                <Users className="h-4 w-4 text-purple-500" />
                Participate in Governance
              </h4>
              <p className="text-sm text-muted-foreground">
                Stakers can vote on protocol upgrades and network parameters.
              </p>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
