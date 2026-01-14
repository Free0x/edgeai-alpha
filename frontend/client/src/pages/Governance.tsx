import { API_BASE_URL } from "../lib/config";
import { useState, useEffect } from "react";
import { Card, CardContent, CardHeader, CardTitle, CardDescription, CardFooter } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Progress } from "@/components/ui/progress";
import { Textarea } from "@/components/ui/textarea";
import { 
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
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
  Vote, 
  FileText, 
  Users, 
  Clock, 
  CheckCircle2, 
  XCircle,
  AlertCircle,
  Loader2,
  Plus,
  ThumbsUp,
  ThumbsDown,
  MinusCircle,
  Ban,
  Coins,
  ArrowUpRight,
  Calendar,
  Target,
  TrendingUp
} from "lucide-react";
import { cn } from "@/lib/utils";
import { getOrCreateDemoWallet, createSignedRequest, Wallet } from "@/lib/crypto";

// Types
interface GovernanceStats {
  totalProposals: number;
  activeProposals: number;
  passedProposals: number;
  rejectedProposals: number;
  totalVotes: number;
  config: {
    minDeposit: string;
    votingPeriodDays: number;
    quorumPercentage: number;
    passThreshold: number;
    vetoThreshold: number;
    executionDelayDays: number;
  };
}

interface Proposal {
  id: number;
  proposer: string;
  title: string;
  description: string;
  proposalType: string;
  status: "Deposit" | "Voting" | "Passed" | "Rejected" | "Executed" | "Failed";
  deposit: string;
  createdAt: string;
  votingStartTime?: string;
  votingEndTime?: string;
  executionTime?: string;
  votes: {
    yes: string;
    no: string;
    abstain: string;
    noWithVeto: string;
  };
  turnout: number;
}

// Format large numbers
const formatAmount = (amount: string): string => {
  const num = BigInt(amount);
  const edge = num / BigInt(10 ** 18);
  if (edge >= BigInt(1000000)) {
    return `${(Number(edge) / 1000000).toFixed(2)}M`;
  } else if (edge >= BigInt(1000)) {
    return `${(Number(edge) / 1000).toFixed(2)}K`;
  }
  return edge.toString();
};

// Format date
const formatDate = (timestamp: string): string => {
  return new Date(timestamp).toLocaleDateString("en-US", {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
};

// Calculate time remaining
const getTimeRemaining = (endTime: string): string => {
  const end = new Date(endTime).getTime();
  const now = Date.now();
  const diff = end - now;
  
  if (diff <= 0) return "Ended";
  
  const days = Math.floor(diff / (1000 * 60 * 60 * 24));
  const hours = Math.floor((diff % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60));
  
  if (days > 0) return `${days}d ${hours}h remaining`;
  return `${hours}h remaining`;
};

// Status badge component
const StatusBadge = ({ status }: { status: Proposal["status"] }) => {
  const variants: Record<Proposal["status"], { color: string; icon: React.ReactNode }> = {
    Deposit: { color: "bg-yellow-500/10 text-yellow-500 border-yellow-500/20", icon: <Coins className="w-3 h-3" /> },
    Voting: { color: "bg-blue-500/10 text-blue-500 border-blue-500/20", icon: <Vote className="w-3 h-3" /> },
    Passed: { color: "bg-green-500/10 text-green-500 border-green-500/20", icon: <CheckCircle2 className="w-3 h-3" /> },
    Rejected: { color: "bg-red-500/10 text-red-500 border-red-500/20", icon: <XCircle className="w-3 h-3" /> },
    Executed: { color: "bg-purple-500/10 text-purple-500 border-purple-500/20", icon: <CheckCircle2 className="w-3 h-3" /> },
    Failed: { color: "bg-gray-500/10 text-gray-500 border-gray-500/20", icon: <AlertCircle className="w-3 h-3" /> },
  };
  
  const { color, icon } = variants[status];
  
  return (
    <Badge variant="outline" className={cn("flex items-center gap-1", color)}>
      {icon}
      {status}
    </Badge>
  );
};

// Vote distribution bar
const VoteBar = ({ votes, total }: { votes: Proposal["votes"]; total: string }) => {
  const totalVotes = BigInt(votes.yes) + BigInt(votes.no) + BigInt(votes.abstain) + BigInt(votes.noWithVeto);
  
  if (totalVotes === BigInt(0)) {
    return (
      <div className="w-full h-3 bg-muted rounded-full overflow-hidden">
        <div className="h-full bg-muted-foreground/20 w-full" />
      </div>
    );
  }
  
  const yesPercent = Number((BigInt(votes.yes) * BigInt(100)) / totalVotes);
  const noPercent = Number((BigInt(votes.no) * BigInt(100)) / totalVotes);
  const abstainPercent = Number((BigInt(votes.abstain) * BigInt(100)) / totalVotes);
  const vetoPercent = Number((BigInt(votes.noWithVeto) * BigInt(100)) / totalVotes);
  
  return (
    <div className="space-y-2">
      <div className="w-full h-3 bg-muted rounded-full overflow-hidden flex">
        <div className="h-full bg-green-500" style={{ width: `${yesPercent}%` }} />
        <div className="h-full bg-red-500" style={{ width: `${noPercent}%` }} />
        <div className="h-full bg-gray-400" style={{ width: `${abstainPercent}%` }} />
        <div className="h-full bg-orange-500" style={{ width: `${vetoPercent}%` }} />
      </div>
      <div className="flex justify-between text-xs text-muted-foreground">
        <span className="flex items-center gap-1">
          <div className="w-2 h-2 rounded-full bg-green-500" />
          Yes {yesPercent.toFixed(1)}%
        </span>
        <span className="flex items-center gap-1">
          <div className="w-2 h-2 rounded-full bg-red-500" />
          No {noPercent.toFixed(1)}%
        </span>
        <span className="flex items-center gap-1">
          <div className="w-2 h-2 rounded-full bg-gray-400" />
          Abstain {abstainPercent.toFixed(1)}%
        </span>
        <span className="flex items-center gap-1">
          <div className="w-2 h-2 rounded-full bg-orange-500" />
          Veto {vetoPercent.toFixed(1)}%
        </span>
      </div>
    </div>
  );
};

// Proposal Card Component
const ProposalCard = ({ 
  proposal, 
  onVote, 
  onDeposit 
}: { 
  proposal: Proposal; 
  onVote: (id: number, vote: string) => void;
  onDeposit: (id: number, amount: string) => void;
}) => {
  const [voteDialogOpen, setVoteDialogOpen] = useState(false);
  const [depositDialogOpen, setDepositDialogOpen] = useState(false);
  const [depositAmount, setDepositAmount] = useState("");
  const [selectedVote, setSelectedVote] = useState<string | null>(null);
  
  return (
    <Card className="hover:border-primary/50 transition-colors">
      <CardHeader>
        <div className="flex items-start justify-between">
          <div className="space-y-1">
            <div className="flex items-center gap-2">
              <span className="text-sm text-muted-foreground">#{proposal.id}</span>
              <StatusBadge status={proposal.status} />
              <Badge variant="secondary" className="text-xs">
                {proposal.proposalType}
              </Badge>
            </div>
            <CardTitle className="text-lg">{proposal.title}</CardTitle>
          </div>
        </div>
        <CardDescription className="line-clamp-2">
          {proposal.description}
        </CardDescription>
      </CardHeader>
      
      <CardContent className="space-y-4">
        {/* Vote Distribution */}
        {(proposal.status === "Voting" || proposal.status === "Passed" || proposal.status === "Rejected") && (
          <VoteBar votes={proposal.votes} total="0" />
        )}
        
        {/* Proposal Details */}
        <div className="grid grid-cols-2 gap-4 text-sm">
          <div className="space-y-1">
            <span className="text-muted-foreground">Proposer</span>
            <p className="font-mono text-xs truncate">{proposal.proposer}</p>
          </div>
          <div className="space-y-1">
            <span className="text-muted-foreground">Deposit</span>
            <p className="font-medium">{formatAmount(proposal.deposit)} EDGE</p>
          </div>
          {proposal.votingEndTime && (
            <div className="space-y-1">
              <span className="text-muted-foreground">Voting Ends</span>
              <p className="font-medium">{getTimeRemaining(proposal.votingEndTime)}</p>
            </div>
          )}
          <div className="space-y-1">
            <span className="text-muted-foreground">Turnout</span>
            <p className="font-medium">{proposal.turnout.toFixed(1)}%</p>
          </div>
        </div>
      </CardContent>
      
      <CardFooter className="flex gap-2">
        {proposal.status === "Deposit" && (
          <Dialog open={depositDialogOpen} onOpenChange={setDepositDialogOpen}>
            <DialogTrigger asChild>
              <Button variant="outline" className="flex-1">
                <Coins className="w-4 h-4 mr-2" />
                Add Deposit
              </Button>
            </DialogTrigger>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>Add Deposit</DialogTitle>
                <DialogDescription>
                  Add deposit to help this proposal reach the minimum threshold.
                </DialogDescription>
              </DialogHeader>
              <div className="space-y-4 py-4">
                <div className="space-y-2">
                  <Label>Amount (EDGE)</Label>
                  <Input
                    type="number"
                    placeholder="Enter amount"
                    value={depositAmount}
                    onChange={(e) => setDepositAmount(e.target.value)}
                  />
                </div>
              </div>
              <DialogFooter>
                <Button variant="outline" onClick={() => setDepositDialogOpen(false)}>
                  Cancel
                </Button>
                <Button onClick={() => {
                  onDeposit(proposal.id, depositAmount);
                  setDepositDialogOpen(false);
                  setDepositAmount("");
                }}>
                  Deposit
                </Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>
        )}
        
        {proposal.status === "Voting" && (
          <Dialog open={voteDialogOpen} onOpenChange={setVoteDialogOpen}>
            <DialogTrigger asChild>
              <Button className="flex-1">
                <Vote className="w-4 h-4 mr-2" />
                Vote
              </Button>
            </DialogTrigger>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>Cast Your Vote</DialogTitle>
                <DialogDescription>
                  Your vote will be weighted by your staked EDGE tokens.
                </DialogDescription>
              </DialogHeader>
              <div className="grid grid-cols-2 gap-3 py-4">
                <Button
                  variant={selectedVote === "yes" ? "default" : "outline"}
                  className={cn(
                    "h-20 flex-col gap-2",
                    selectedVote === "yes" && "bg-green-600 hover:bg-green-700"
                  )}
                  onClick={() => setSelectedVote("yes")}
                >
                  <ThumbsUp className="w-6 h-6" />
                  Yes
                </Button>
                <Button
                  variant={selectedVote === "no" ? "default" : "outline"}
                  className={cn(
                    "h-20 flex-col gap-2",
                    selectedVote === "no" && "bg-red-600 hover:bg-red-700"
                  )}
                  onClick={() => setSelectedVote("no")}
                >
                  <ThumbsDown className="w-6 h-6" />
                  No
                </Button>
                <Button
                  variant={selectedVote === "abstain" ? "default" : "outline"}
                  className={cn(
                    "h-20 flex-col gap-2",
                    selectedVote === "abstain" && "bg-gray-600 hover:bg-gray-700"
                  )}
                  onClick={() => setSelectedVote("abstain")}
                >
                  <MinusCircle className="w-6 h-6" />
                  Abstain
                </Button>
                <Button
                  variant={selectedVote === "no_with_veto" ? "default" : "outline"}
                  className={cn(
                    "h-20 flex-col gap-2",
                    selectedVote === "no_with_veto" && "bg-orange-600 hover:bg-orange-700"
                  )}
                  onClick={() => setSelectedVote("no_with_veto")}
                >
                  <Ban className="w-6 h-6" />
                  No with Veto
                </Button>
              </div>
              <DialogFooter>
                <Button variant="outline" onClick={() => setVoteDialogOpen(false)}>
                  Cancel
                </Button>
                <Button 
                  disabled={!selectedVote}
                  onClick={() => {
                    if (selectedVote) {
                      onVote(proposal.id, selectedVote);
                      setVoteDialogOpen(false);
                      setSelectedVote(null);
                    }
                  }}
                >
                  Submit Vote
                </Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>
        )}
        
        <Button variant="ghost" size="sm">
          <ArrowUpRight className="w-4 h-4" />
          Details
        </Button>
      </CardFooter>
    </Card>
  );
};

// Create Proposal Dialog
const CreateProposalDialog = ({ 
  open, 
  onOpenChange, 
  onSubmit 
}: { 
  open: boolean; 
  onOpenChange: (open: boolean) => void;
  onSubmit: (data: any) => void;
}) => {
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [proposalType, setProposalType] = useState("text");
  const [deposit, setDeposit] = useState("");
  const [textContent, setTextContent] = useState("");
  
  const handleSubmit = () => {
    onSubmit({
      title,
      description,
      proposalType,
      deposit,
      content: textContent,
    });
    onOpenChange(false);
    // Reset form
    setTitle("");
    setDescription("");
    setProposalType("text");
    setDeposit("");
    setTextContent("");
  };
  
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>Create New Proposal</DialogTitle>
          <DialogDescription>
            Submit a new governance proposal for the community to vote on.
          </DialogDescription>
        </DialogHeader>
        
        <div className="space-y-4 py-4 max-h-[60vh] overflow-y-auto">
          <div className="space-y-2">
            <Label>Proposal Type</Label>
            <Select value={proposalType} onValueChange={setProposalType}>
              <SelectTrigger>
                <SelectValue placeholder="Select type" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="text">Text Proposal</SelectItem>
                <SelectItem value="parameter_change">Parameter Change</SelectItem>
                <SelectItem value="software_upgrade">Software Upgrade</SelectItem>
                <SelectItem value="treasury_spend">Treasury Spend</SelectItem>
                <SelectItem value="validator_change">Validator Change</SelectItem>
                <SelectItem value="emergency">Emergency</SelectItem>
              </SelectContent>
            </Select>
          </div>
          
          <div className="space-y-2">
            <Label>Title</Label>
            <Input
              placeholder="Enter proposal title"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
            />
          </div>
          
          <div className="space-y-2">
            <Label>Description</Label>
            <Textarea
              placeholder="Describe your proposal in detail..."
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              rows={4}
            />
          </div>
          
          {proposalType === "text" && (
            <div className="space-y-2">
              <Label>Proposal Content</Label>
              <Textarea
                placeholder="Enter the full text content of your proposal..."
                value={textContent}
                onChange={(e) => setTextContent(e.target.value)}
                rows={6}
              />
            </div>
          )}
          
          <div className="space-y-2">
            <Label>Initial Deposit (EDGE)</Label>
            <Input
              type="number"
              placeholder="Minimum: 10,000 EDGE"
              value={deposit}
              onChange={(e) => setDeposit(e.target.value)}
            />
            <p className="text-xs text-muted-foreground">
              Proposals require a minimum deposit to enter the voting period.
            </p>
          </div>
        </div>
        
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={handleSubmit} disabled={!title || !description || !deposit}>
            Submit Proposal
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

// Main Governance Page
export default function Governance() {
  const [stats, setStats] = useState<GovernanceStats | null>(null);
  const [proposals, setProposals] = useState<Proposal[]>([]);
  const [loading, setLoading] = useState(true);
  const [activeTab, setActiveTab] = useState("all");
  const [createDialogOpen, setCreateDialogOpen] = useState(false);
  
  // Fetch data
  useEffect(() => {
    const fetchData = async () => {
      try {
        // Fetch governance stats
        const statsRes = await fetch("${API_BASE_URL}/governance/stats");
        const statsData = await statsRes.json();
        setStats({
          totalProposals: statsData.total_proposals || 0,
          activeProposals: statsData.active_proposals || 0,
          passedProposals: statsData.passed_proposals || 0,
          rejectedProposals: statsData.rejected_proposals || 0,
          totalVotes: statsData.total_votes || 0,
          config: {
            minDeposit: statsData.config?.min_deposit || "10000000000000000000000",
            votingPeriodDays: statsData.config?.voting_period_days || 7,
            quorumPercentage: statsData.config?.quorum_percentage || 33,
            passThreshold: statsData.config?.pass_threshold || 50,
            vetoThreshold: statsData.config?.veto_threshold || 33,
            executionDelayDays: statsData.config?.execution_delay_days || 2,
          },
        });
        
        // Fetch proposals
        const proposalsRes = await fetch("${API_BASE_URL}/governance/proposals");
        const proposalsData = await proposalsRes.json();
        setProposals(proposalsData.proposals || []);
      } catch (error) {
        console.error("Failed to fetch governance data:", error);
      } finally {
        setLoading(false);
      }
    };
    
    fetchData();
    const interval = setInterval(fetchData, 30000);
    return () => clearInterval(interval);
  }, []);
  
  // Get or create demo wallet
  const [wallet, setWallet] = useState<Wallet | null>(null);
  
  useEffect(() => {
    getOrCreateDemoWallet().then(setWallet);
  }, []);
  
  // Handle vote with signature authentication
  const handleVote = async (proposalId: number, vote: string) => {
    if (!wallet) {
      console.error("No wallet available");
      return;
    }
    
    try {
      const voteData = {
        voter: wallet.address,
        option: vote,
        voting_power: "1000000000000000000", // 1 EDGE
      };
      
      const signedRequest = await createSignedRequest(wallet, voteData);
      
      const response = await fetch(
        `${API_BASE_URL}/governance/proposals/${proposalId}/vote`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(signedRequest),
        }
      );
      
      const result = await response.json();
      if (result.success) {
        console.log("Vote cast successfully");
        // Refresh proposals
        window.location.reload();
      } else {
        console.error("Vote failed:", result.error);
      }
    } catch (error) {
      console.error("Failed to vote:", error);
    }
  };
  
  // Handle deposit
  const handleDeposit = async (proposalId: number, amount: string) => {
    console.log(`Depositing ${amount} on proposal ${proposalId}`);
    // Deposit doesn't require signature authentication
  };
  
  // Handle create proposal with signature authentication
  const handleCreateProposal = async (data: any) => {
    if (!wallet) {
      console.error("No wallet available");
      return;
    }
    
    try {
      const proposalData = {
        proposer: wallet.address,
        title: data.title,
        description: data.description,
        proposal_type: { type: "text", content: data.content || data.description },
        initial_deposit: "10000000000000000000", // 10 EDGE
      };
      
      const signedRequest = await createSignedRequest(wallet, proposalData);
      
      const response = await fetch(
        "${API_BASE_URL}/governance/proposals",
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(signedRequest),
        }
      );
      
      const result = await response.json();
      if (result.success) {
        console.log("Proposal created:", result.proposal_id);
        setCreateDialogOpen(false);
        // Refresh proposals
        window.location.reload();
      } else {
        console.error("Proposal creation failed:", result.error);
      }
    } catch (error) {
      console.error("Failed to create proposal:", error);
    }
  };
  
  // Filter proposals
  const filteredProposals = proposals.filter((p) => {
    if (activeTab === "all") return true;
    if (activeTab === "active") return p.status === "Voting" || p.status === "Deposit";
    if (activeTab === "passed") return p.status === "Passed" || p.status === "Executed";
    if (activeTab === "rejected") return p.status === "Rejected" || p.status === "Failed";
    return true;
  });
  
  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-[60vh]">
        <Loader2 className="w-8 h-8 animate-spin text-primary" />
      </div>
    );
  }
  
  return (
    <div className="container mx-auto py-8 space-y-8">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Governance</h1>
          <p className="text-muted-foreground">
            Participate in on-chain governance and shape the future of EdgeAI
          </p>
        </div>
        <Button onClick={() => setCreateDialogOpen(true)}>
          <Plus className="w-4 h-4 mr-2" />
          New Proposal
        </Button>
      </div>
      
      {/* Stats Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">
              Total Proposals
            </CardTitle>
            <FileText className="w-4 h-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{stats?.totalProposals || 0}</div>
            <p className="text-xs text-muted-foreground">
              {stats?.activeProposals || 0} active
            </p>
          </CardContent>
        </Card>
        
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">
              Passed
            </CardTitle>
            <CheckCircle2 className="w-4 h-4 text-green-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-green-500">
              {stats?.passedProposals || 0}
            </div>
            <p className="text-xs text-muted-foreground">
              {stats?.totalProposals ? ((stats.passedProposals / stats.totalProposals) * 100).toFixed(1) : 0}% pass rate
            </p>
          </CardContent>
        </Card>
        
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">
              Rejected
            </CardTitle>
            <XCircle className="w-4 h-4 text-red-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-red-500">
              {stats?.rejectedProposals || 0}
            </div>
            <p className="text-xs text-muted-foreground">
              {stats?.config?.vetoThreshold || 33}% veto threshold
            </p>
          </CardContent>
        </Card>
        
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">
              Total Votes
            </CardTitle>
            <Vote className="w-4 h-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{stats?.totalVotes || 0}</div>
            <p className="text-xs text-muted-foreground">
              {stats?.config?.quorumPercentage || 33}% quorum required
            </p>
          </CardContent>
        </Card>
      </div>
      
      {/* Governance Parameters */}
      <Card>
        <CardHeader>
          <CardTitle className="text-lg">Governance Parameters</CardTitle>
          <CardDescription>Current on-chain governance configuration</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4">
            <div className="space-y-1">
              <span className="text-sm text-muted-foreground">Min Deposit</span>
              <p className="font-medium">{formatAmount(stats?.config?.minDeposit || "0")} EDGE</p>
            </div>
            <div className="space-y-1">
              <span className="text-sm text-muted-foreground">Voting Period</span>
              <p className="font-medium">{stats?.config?.votingPeriodDays || 7} days</p>
            </div>
            <div className="space-y-1">
              <span className="text-sm text-muted-foreground">Quorum</span>
              <p className="font-medium">{stats?.config?.quorumPercentage || 33}%</p>
            </div>
            <div className="space-y-1">
              <span className="text-sm text-muted-foreground">Pass Threshold</span>
              <p className="font-medium">{stats?.config?.passThreshold || 50}%</p>
            </div>
            <div className="space-y-1">
              <span className="text-sm text-muted-foreground">Veto Threshold</span>
              <p className="font-medium">{stats?.config?.vetoThreshold || 33}%</p>
            </div>
            <div className="space-y-1">
              <span className="text-sm text-muted-foreground">Execution Delay</span>
              <p className="font-medium">{stats?.config?.executionDelayDays || 2} days</p>
            </div>
          </div>
        </CardContent>
      </Card>
      
      {/* Proposals */}
      <div className="space-y-4">
        <Tabs value={activeTab} onValueChange={setActiveTab}>
          <TabsList>
            <TabsTrigger value="all">All Proposals</TabsTrigger>
            <TabsTrigger value="active">Active</TabsTrigger>
            <TabsTrigger value="passed">Passed</TabsTrigger>
            <TabsTrigger value="rejected">Rejected</TabsTrigger>
          </TabsList>
          
          <TabsContent value={activeTab} className="mt-4">
            {filteredProposals.length === 0 ? (
              <Card className="py-12">
                <CardContent className="flex flex-col items-center justify-center text-center">
                  <FileText className="w-12 h-12 text-muted-foreground mb-4" />
                  <h3 className="text-lg font-medium">No Proposals Found</h3>
                  <p className="text-muted-foreground mb-4">
                    {activeTab === "all" 
                      ? "Be the first to create a governance proposal!"
                      : `No ${activeTab} proposals at the moment.`}
                  </p>
                  {activeTab === "all" && (
                    <Button onClick={() => setCreateDialogOpen(true)}>
                      <Plus className="w-4 h-4 mr-2" />
                      Create Proposal
                    </Button>
                  )}
                </CardContent>
              </Card>
            ) : (
              <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
                {filteredProposals.map((proposal) => (
                  <ProposalCard
                    key={proposal.id}
                    proposal={proposal}
                    onVote={handleVote}
                    onDeposit={handleDeposit}
                  />
                ))}
              </div>
            )}
          </TabsContent>
        </Tabs>
      </div>
      
      {/* Create Proposal Dialog */}
      <CreateProposalDialog
        open={createDialogOpen}
        onOpenChange={setCreateDialogOpen}
        onSubmit={handleCreateProposal}
      />
    </div>
  );
}
