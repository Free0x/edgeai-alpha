import { API_BASE_URL } from "@/lib/config";
import { StepCard } from "@/components/StepCard";
import { Terminal } from "@/components/Terminal";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { cn } from "@/lib/utils";
import { Activity, ArrowRight, CheckCircle2, Copy, ExternalLink, RefreshCw, ShieldCheck, Wallet } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { Link } from "wouter";

interface TerminalLine {
  type: "input" | "output" | "error" | "success" | "info";
  content: string;
  timestamp?: string;
}

export default function Home() {
  const [activeStep, setActiveStep] = useState(1);
  const [terminalLines, setTerminalLines] = useState<TerminalLine[]>([
    { type: "info", content: "EdgeAI Blockchain Interface initialized..." },
    { type: "info", content: "Connected to node: https://edgeai-blockchain-node.fly.dev" },
    { type: "output", content: "Ready for user input." }
  ]);
  const [isLoading, setIsLoading] = useState(false);
  
  // Wallet State
  const [wallet, setWallet] = useState<{address: string, public_key: string, private_key: string} | null>(null);
  const [balance, setBalance] = useState<number | null>(null);
  const [recipient, setRecipient] = useState("bob");
  const [amount, setAmount] = useState("100");

  const addLog = (type: TerminalLine["type"], content: string) => {
    setTerminalLines(prev => [...prev, { type, content, timestamp: new Date().toLocaleTimeString() }]);
  };

  const copyToClipboard = (text: string, label: string) => {
    navigator.clipboard.writeText(text);
    toast.success(`${label} copied to clipboard`);
  };

  // Step 1: Generate Wallet
  const handleGenerateWallet = async () => {
    setIsLoading(true);
    addLog("input", "edgeai-cli wallet generate");
    
    try {
      const response = await fetch(`${API_BASE_URL}/wallet/generate`, {
        method: "POST"
      });
      const data = await response.json();
      
      setWallet(data);
      addLog("success", "Wallet generated successfully.");
      addLog("output", `Address: ${data.address}`);
      addLog("info", "⚠️ SAVE YOUR PRIVATE KEY! It cannot be recovered.");
      
      setActiveStep(2);
    } catch (error) {
      addLog("error", "Failed to generate wallet. Network error?");
    } finally {
      setIsLoading(false);
    }
  };

  // Step 2: Check Balance (and fund if needed)
  const handleCheckBalance = async () => {
    if (!wallet) return;
    setIsLoading(true);
    addLog("input", `edgeai-cli balance ${wallet.address}`);
    
    try {
      const response = await fetch(`${API_BASE_URL}/accounts/${wallet.address}/balance`);
      const data = await response.json();
      
      setBalance(data.balance);
      addLog("output", `Balance: ${data.balance} EDGE`);
      
      if (data.balance === 0) {
        addLog("info", "Balance is 0. Requesting faucet funds...");
        // Auto-fund for demo purposes
        await fetch(`${API_BASE_URL}/transactions/transfer`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            from: "genesis",
            to: wallet.address,
            amount: 1000
          })
        });
        
        // Mine a block to confirm
        await fetch(`${API_BASE_URL}/mine`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ validator: "faucet_miner" })
        });
        
        addLog("success", "Faucet transfer initiated and mined.");
        
        // Check balance again
        const newBalanceRes = await fetch(`${API_BASE_URL}/accounts/${wallet.address}/balance`);
        const newBalanceData = await newBalanceRes.json();
        setBalance(newBalanceData.balance);
        addLog("output", `New Balance: ${newBalanceData.balance} EDGE`);
      }
      
      setActiveStep(3);
    } catch (error) {
      addLog("error", "Failed to check balance.");
    } finally {
      setIsLoading(false);
    }
  };

  // Step 3: Signed Transfer
  const handleTransfer = async () => {
    if (!wallet) return;
    setIsLoading(true);
    addLog("input", `edgeai-cli transfer --to ${recipient} --amount ${amount} --sign`);
    
    try {
      // 1. Prepare
      addLog("info", "Preparing transaction...");
      const prepareRes = await fetch(`${API_BASE_URL}/wallet/transfer/prepare`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          from: wallet.address,
          to: recipient,
          amount: parseInt(amount)
        })
      });
      const prepareData = await prepareRes.json();
      addLog("output", `Message to sign: ${prepareData.message}`);
      
      // 2. Sign (Client-side simulation using the API for demo, normally done locally)
      // In a real app, we would use ed25519-dalek here. For this demo, we use the server's sign endpoint 
      // (which is available in the demo wallet API for testing) or simulate it.
      // Since we can't import rust crypto easily in browser without WASM, we'll use the server's sign endpoint
      // which was added for the CLI tool.
      
      addLog("info", "Signing with private key...");
      const signRes = await fetch(`${API_BASE_URL}/wallet/sign`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          private_key: wallet.private_key,
          message: prepareData.message
        })
      });
      const signData = await signRes.json();
      addLog("output", `Signature: ${signData.signature.substring(0, 20)}...`);
      
      // 3. Submit
      addLog("info", "Submitting signed transaction...");
      const submitRes = await fetch(`${API_BASE_URL}/wallet/transfer/submit`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          from: wallet.address,
          to: recipient,
          amount: parseInt(amount),
          signature: signData.signature,
          public_key: wallet.public_key
        })
      });
      
      if (!submitRes.ok) throw new Error("Transfer failed");
      
      const submitData = await submitRes.json();
      addLog("success", `Transaction submitted! ID: ${submitData.transaction_id}`);
      
      setActiveStep(4);
    } catch (error) {
      addLog("error", "Transfer failed. Check console for details.");
      console.error(error);
    } finally {
      setIsLoading(false);
    }
  };

  // Step 4: Mine & Confirm
  const handleMine = async () => {
    setIsLoading(true);
    addLog("input", "edgeai-cli mine");
    
    try {
      const response = await fetch(`${API_BASE_URL}/mine`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ validator: wallet?.address || "miner" })
      });
      const data = await response.json();
      
      addLog("success", `Block #${data.index} mined successfully!`);
      addLog("output", `Transactions: ${data.transactions.length}`);
      addLog("output", `Hash: ${data.hash.substring(0, 20)}...`);
      
      setActiveStep(5);
    } catch (error) {
      addLog("error", "Mining failed.");
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="min-h-screen bg-background text-foreground flex flex-col lg:flex-row overflow-hidden">
      {/* Left Panel: Guide */}
      <div className="flex-1 flex flex-col h-screen overflow-y-auto border-r border-border">
        <header className="p-6 border-b border-border bg-card/50 backdrop-blur-sm sticky top-0 z-10">
          <div className="flex items-center justify-between mb-2">
            <div className="flex items-center gap-3">
              <div className="w-8 h-8 bg-primary rounded flex items-center justify-center">
                <Wallet className="text-primary-foreground w-5 h-5" />
              </div>
              <h1 className="text-2xl font-display font-bold tracking-tight">EdgeAI <span className="text-primary">Wallet Guide</span></h1>
            </div>
            <Link href="/explorer" className="flex items-center gap-2 text-sm text-muted-foreground hover:text-primary transition-colors border border-border rounded-full px-3 py-1 hover:bg-muted/50">
              <Activity className="w-4 h-4" />
              Explorer
            </Link>
          </div>
          <p className="text-muted-foreground">Interactive tutorial for key management and signed transactions.</p>
        </header>

        <div className="p-6 space-y-8 pb-20">
          <StepCard
            step={1}
            title="Generate Identity"
            description="Create a secure Ed25519 keypair. Your address is derived from your public key."
            isActive={activeStep === 1}
            isCompleted={activeStep > 1}
            onAction={handleGenerateWallet}
            isLoading={isLoading}
            actionLabel="Generate Keypair"
          >
            <div className="bg-muted/30 p-4 rounded-md border border-border space-y-2">
              <p className="text-sm text-muted-foreground">
                Unlike traditional accounts, blockchain identities are cryptographic keypairs. 
                You don't "register" - you generate keys locally.
              </p>
            </div>
          </StepCard>

          <StepCard
            step={2}
            title="Verify & Fund"
            description="Check your balance. For this demo, we'll auto-fund your wallet from the faucet."
            isActive={activeStep === 2}
            isCompleted={activeStep > 2}
            onAction={handleCheckBalance}
            isLoading={isLoading}
            actionLabel="Check Balance"
          >
            {wallet && (
              <div className="space-y-3">
                <div className="space-y-1">
                  <Label className="text-xs uppercase text-muted-foreground">Your Address</Label>
                  <div className="flex gap-2">
                    <Input value={wallet.address} readOnly className="font-mono text-xs bg-muted/50" />
                    <Button size="icon" variant="outline" onClick={() => copyToClipboard(wallet.address, "Address")}>
                      <Copy className="w-4 h-4" />
                    </Button>
                  </div>
                </div>
                <div className="space-y-1">
                  <Label className="text-xs uppercase text-destructive">Private Key (SAVE THIS)</Label>
                  <div className="flex gap-2">
                    <Input value={wallet.private_key} readOnly type="password" className="font-mono text-xs bg-muted/50" />
                    <Button size="icon" variant="outline" onClick={() => copyToClipboard(wallet.private_key, "Private Key")}>
                      <Copy className="w-4 h-4" />
                    </Button>
                  </div>
                </div>
              </div>
            )}
          </StepCard>

          <StepCard
            step={3}
            title="Signed Transfer"
            description="Execute a secure transaction signed with your private key."
            isActive={activeStep === 3}
            isCompleted={activeStep > 3}
            onAction={handleTransfer}
            isLoading={isLoading}
            actionLabel="Sign & Send"
          >
            <div className="grid gap-4 p-4 border border-border rounded-md bg-muted/20">
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label>Recipient</Label>
                  <Input value={recipient} onChange={(e) => setRecipient(e.target.value)} className="font-mono" />
                </div>
                <div className="space-y-2">
                  <Label>Amount (EDGE)</Label>
                  <Input type="number" value={amount} onChange={(e) => setAmount(e.target.value)} className="font-mono" />
                </div>
              </div>
              <div className="flex items-center gap-2 text-xs text-muted-foreground">
                <ShieldCheck className="w-4 h-4 text-green-500" />
                <span>Transaction will be cryptographically signed before submission.</span>
              </div>
            </div>
          </StepCard>

          <StepCard
            step={4}
            title="Mine Block"
            description="Confirm your transaction by mining a new block."
            isActive={activeStep === 4}
            isCompleted={activeStep > 4}
            onAction={handleMine}
            isLoading={isLoading}
            actionLabel="Mine Block"
          >
            <div className="bg-muted/30 p-4 rounded-md border border-border">
              <p className="text-sm text-muted-foreground">
                Transactions enter a "pending" state until a validator mines them into a block. 
                In this demo, you act as the validator.
              </p>
            </div>
          </StepCard>

          <StepCard
            step={5}
            title="Tutorial Complete"
            description="You have successfully mastered the EdgeAI wallet workflow."
            isActive={activeStep === 5}
            isCompleted={activeStep > 5}
          >
            <div className="space-y-4">
              <div className="p-4 bg-green-500/10 border border-green-500/20 rounded-md flex items-start gap-3">
                <CheckCircle2 className="w-5 h-5 text-green-500 mt-0.5" />
                <div>
                  <h4 className="font-bold text-green-500">Success!</h4>
                  <p className="text-sm text-green-400/80">
                    Your transaction is now immutable on the blockchain.
                  </p>
                </div>
              </div>
              <div className="flex gap-3">
                <Button variant="outline" className="flex-1" onClick={() => window.location.reload()}>
                  <RefreshCw className="mr-2 w-4 h-4" /> Reset Demo
                </Button>
                <Button className="flex-1" asChild>
                  <a href="https://edgeai-blockchain-node.fly.dev" target="_blank" rel="noreferrer">
                    Explorer <ExternalLink className="ml-2 w-4 h-4" />
                  </a>
                </Button>
              </div>
            </div>
          </StepCard>
        </div>
      </div>

      {/* Right Panel: Terminal */}
      <div className="hidden lg:flex flex-1 bg-black p-6 flex-col gap-4 border-l border-border">
        <div className="flex items-center justify-between">
          <Badge variant="outline" className="font-mono text-primary border-primary/50">
            LIVE SESSION
          </Badge>
          <div className="flex items-center gap-2 text-xs text-muted-foreground">
            <div className="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
            Node Online
          </div>
        </div>
        
        <Terminal lines={terminalLines} className="flex-1 shadow-2xl shadow-primary/5" />
        
        <div className="grid grid-cols-3 gap-4">
          <div className="p-4 rounded-md border border-border bg-card/50">
            <div className="text-xs text-muted-foreground uppercase tracking-wider mb-1">Network</div>
            <div className="font-mono text-lg">Testnet</div>
          </div>
          <div className="p-4 rounded-md border border-border bg-card/50">
            <div className="text-xs text-muted-foreground uppercase tracking-wider mb-1">Block Height</div>
            <div className="font-mono text-lg text-primary">
              {activeStep > 4 ? "Updated" : "Synced"}
            </div>
          </div>
          <div className="p-4 rounded-md border border-border bg-card/50">
            <div className="text-xs text-muted-foreground uppercase tracking-wider mb-1">Latency</div>
            <div className="font-mono text-lg text-green-500">24ms</div>
          </div>
        </div>
      </div>
    </div>
  );
}
