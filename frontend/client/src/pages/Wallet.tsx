import { API_BASE_URL } from "@/lib/config";
import { StepCard } from "@/components/StepCard";
import { Terminal } from "@/components/Terminal";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Activity, CheckCircle2, Copy, Download, ExternalLink, RefreshCw, ShieldCheck, Wallet as WalletIcon } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { Link } from "wouter";

interface TerminalLine {
  type: "input" | "output" | "error" | "success" | "info";
  content: string;
  timestamp?: string;
}

export default function Wallet() {
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

  const downloadKeyFile = () => {
    if (!wallet) return;
    const element = document.createElement("a");
    const file = new Blob([JSON.stringify(wallet, null, 2)], {type: 'application/json'});
    element.href = URL.createObjectURL(file);
    element.download = `edgeai-wallet-${wallet.address.substring(0, 8)}.json`;
    document.body.appendChild(element);
    element.click();
    document.body.removeChild(element);
    toast.success("Keyfile downloaded successfully");
  };

  // Step 1: Generate Wallet
  const handleGenerateWallet = async () => {
    setIsLoading(true);
    addLog("input", "edgeai-cli wallet generate");
    
    try {
      const response = await fetch("${API_BASE_URL}/wallet/generate", {
        method: "POST"
      });
      const result = await response.json();
      
      // API returns { success: true, data: { address, public_key, secret_key } }
      if (result.success && result.data) {
        const walletData = {
          address: result.data.address,
          public_key: result.data.public_key,
          private_key: result.data.secret_key
        };
        setWallet(walletData);
        addLog("success", "Wallet generated successfully.");
        addLog("output", `Address: ${walletData.address}`);
        addLog("info", "⚠️ SAVE YOUR PRIVATE KEY! It cannot be recovered.");
        setActiveStep(2);
      } else {
        addLog("error", result.error || "Failed to generate wallet.");
      }
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
      const result = await response.json();
      
      // API returns { success: true, data: { address, balance } }
      const currentBalance = result.data?.balance ?? result.balance ?? 0;
      setBalance(currentBalance);
      addLog("output", `Balance: ${currentBalance} EDGE`);
      
      if (currentBalance === 0) {
        addLog("info", "Balance is 0. Requesting faucet funds...");
        
        // Use the new faucet API
        const faucetRes = await fetch("${API_BASE_URL}/faucet", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            address: wallet.address,
            amount: 1000
          })
        });
        const faucetResult = await faucetRes.json();
        
        if (faucetResult.success) {
          addLog("success", `Faucet sent ${faucetResult.data?.amount || 1000} EDGE!`);
          addLog("output", `Transaction: ${faucetResult.data?.transaction_hash?.substring(0, 16)}...`);
          
          // Wait a moment for the transaction to be processed
          await new Promise(resolve => setTimeout(resolve, 1000));
          
          // Check balance again
          const newBalanceRes = await fetch(`${API_BASE_URL}/accounts/${wallet.address}/balance`);
          const newBalanceResult = await newBalanceRes.json();
          const newBalance = newBalanceResult.data?.balance ?? newBalanceResult.balance ?? 0;
          setBalance(newBalance);
          addLog("output", `New Balance: ${newBalance} EDGE`);
        } else {
          addLog("error", faucetResult.error || "Faucet request failed.");
        }
      }
      
      setActiveStep(3);
    } catch (error) {
      addLog("error", "Failed to check balance.");
      console.error(error);
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
      // 1. Prepare - get the message to sign
      addLog("info", "Preparing transaction...");
      const prepareRes = await fetch("${API_BASE_URL}/wallet/prepare-transfer", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          from: wallet.address,
          to: recipient,
          amount: parseInt(amount)
        })
      });
      const prepareResult = await prepareRes.json();
      const messageToSign = prepareResult.data?.message_to_sign || prepareResult.message_to_sign;
      addLog("output", `Message to sign: ${messageToSign?.substring(0, 32)}...`);
      
      // 2. Sign the message with private key
      addLog("info", "Signing with private key...");
      const signRes = await fetch("${API_BASE_URL}/wallet/sign", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          secret_key: wallet.private_key,
          message: messageToSign
        })
      });
      const signResult = await signRes.json();
      const signature = signResult.data?.signature || signResult.signature;
      addLog("output", `Signature: ${signature?.substring(0, 20)}...`);
      
      // 3. Submit the signed transaction
      addLog("info", "Submitting signed transaction...");
      const submitRes = await fetch("${API_BASE_URL}/wallet/transfer", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          from: wallet.address,
          to: recipient,
          amount: parseInt(amount),
          signature: signature,
          public_key: wallet.public_key
        })
      });
      
      const submitResult = await submitRes.json();
      
      if (submitResult.success) {
        const txHash = submitResult.data || submitResult.transaction_id;
        addLog("success", `Transaction submitted! Hash: ${txHash?.substring(0, 16)}...`);
        toast.success("Transfer successful!");
        setActiveStep(4);
      } else {
        addLog("error", submitResult.error || "Transfer failed.");
        toast.error(submitResult.error || "Transfer failed");
      }
    } catch (error) {
      addLog("error", "Transfer failed. Check console for details.");
      toast.error("Transfer failed");
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
      const response = await fetch("${API_BASE_URL}/mine", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ validator: wallet?.address || "miner" })
      });
      const result = await response.json();
      
      // API returns { success: true, data: { index, hash, transactions, ... } }
      const blockData = result.data || result;
      
      if (result.success || blockData.index !== undefined) {
        addLog("success", `Block #${blockData.index} mined successfully!`);
        addLog("output", `Transactions: ${blockData.transactions?.length || 0}`);
        addLog("output", `Hash: ${blockData.hash?.substring(0, 20)}...`);
        toast.success(`Block #${blockData.index} mined!`);
        setActiveStep(5);
      } else {
        addLog("error", result.error || "Mining failed.");
        toast.error("Mining failed");
      }
    } catch (error) {
      addLog("error", "Mining failed.");
      toast.error("Mining failed");
      console.error(error);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="min-h-[calc(100vh-4rem)] bg-background text-foreground flex flex-col lg:flex-row overflow-hidden rounded-lg border border-border">
      {/* Left Panel: Guide */}
      <div className="flex-1 flex flex-col h-[calc(100vh-4rem)] overflow-y-auto border-r border-border">
        <header className="p-6 border-b border-border bg-card/50 backdrop-blur-sm sticky top-0 z-10">
          <div className="flex items-center justify-between mb-2">
            <div className="flex items-center gap-3">
              <div className="w-8 h-8 bg-primary rounded flex items-center justify-center">
                <WalletIcon className="text-primary-foreground w-5 h-5" />
              </div>
              <h1 className="text-2xl font-display font-bold tracking-tight">EdgeAI <span className="text-primary">Wallet Guide</span></h1>
            </div>
            <Link href="/" className="flex items-center gap-2 text-sm text-muted-foreground hover:text-primary transition-colors border border-border rounded-full px-3 py-1 hover:bg-muted/50">
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
            {wallet && activeStep === 2 && (
              <div className="mt-4">
                <Button variant="secondary" size="sm" onClick={downloadKeyFile} className="w-full">
                  <Download className="mr-2 h-4 w-4" /> Download Keyfile
                </Button>
              </div>
            )}
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
                  <Link href="/">
                    Explorer <ExternalLink className="ml-2 w-4 h-4" />
                  </Link>
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
