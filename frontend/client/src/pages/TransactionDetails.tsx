import { useState, useEffect } from "react";
import { useRoute } from "wouter";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { apiCall, Transaction } from "@/lib/api";
import { generateIoTTransaction, IoTTransaction } from "@/lib/iotGenerator";
import { Badge } from "@/components/ui/badge";
import { ArrowLeft, Clock, CheckCircle, Database, Hash, Wallet, ArrowRight, Building2, Factory, Leaf, HeartPulse, MapPin, Cpu, ShieldCheck } from "lucide-react";
import { Link } from "wouter";
import QRCode from "react-qr-code";

export default function TransactionDetails() {
  const [, params] = useRoute("/tx/:id");
  const id = params?.id;
  const [transaction, setTransaction] = useState<IoTTransaction | Transaction | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  useEffect(() => {
    const fetchTransaction = async () => {
      if (!id) return;
      
      try {
        // For this demo, we prioritize the generated IoT data to ensure consistency
        // with the Transactions list page.
        
        // We use the hash to seed a generator to get consistent details
        // We take a slice of the hash to create a seed number
        const seedStr = id.replace('0x', '').substring(0, 8);
        const seed = parseInt(seedStr, 16);
        
        // Generate the transaction
        // Note: In a real app, we would query by ID. Here we regenerate deterministically.
        const generatedTx = generateIoTTransaction(isNaN(seed) ? 0 : seed);
        
        // Override the ID/Hash to match the requested one exactly
        generatedTx.id = id;
        generatedTx.hash = id;
        
        setTransaction(generatedTx);
      } catch (err) {
        setError("Failed to load transaction details");
      } finally {
        setLoading(false);
      }
    };

    fetchTransaction();
  }, [id]);

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-[50vh]">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
      </div>
    );
  }

  if (error || !transaction) {
    return (
      <div className="space-y-6">
        <div className="flex items-center gap-4">
          <Link href="/transactions">
            <a className="p-2 hover:bg-secondary rounded-full transition-colors">
              <ArrowLeft className="h-5 w-5" />
            </a>
          </Link>
          <h1 className="text-3xl font-bold tracking-tight">Transaction Not Found</h1>
        </div>
        <Card>
          <CardContent className="p-6">
            <p className="text-muted-foreground">
              The transaction with ID <span className="font-mono text-foreground">{id}</span> could not be found.
            </p>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-4">
        <Link href="/transactions">
          <a className="p-2 hover:bg-secondary rounded-full transition-colors">
            <ArrowLeft className="h-5 w-5" />
          </a>
        </Link>
        <h1 className="text-3xl font-bold tracking-tight">Transaction Details</h1>
      </div>

      <div className="grid gap-6 md:grid-cols-3">
        {/* Main Info Column */}
        <div className="md:col-span-2 space-y-6">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Hash className="h-5 w-5 text-primary" />
                Transaction Hash
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="p-4 bg-secondary/50 rounded-lg font-mono text-sm break-all flex items-center justify-between gap-4">
                <span>{transaction.id}</span>
                <Badge variant="outline" className="text-green-500 border-green-500 flex items-center gap-1 whitespace-nowrap">
                  <CheckCircle className="h-3 w-3" /> Confirmed
                </Badge>
              </div>
            </CardContent>
          </Card>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2 text-base">
                  <Wallet className="h-4 w-4 text-blue-400" />
                  From
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="font-mono text-sm break-all text-muted-foreground">
                  {transaction.from}
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2 text-base">
                  <ArrowRight className="h-4 w-4 text-green-400" />
                  To
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="font-mono text-sm break-all text-muted-foreground">
                  {transaction.to || 'Contract Creation'}
                </div>
              </CardContent>
            </Card>
          </div>

          {/* IoT Data Section */}
          {'sector' in transaction && (
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Database className="h-5 w-5 text-purple-400" />
                  IoT Data Payload
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-4">
                  <div className="p-4 bg-secondary/10 rounded-lg border border-border">
                    <div className="text-xs text-muted-foreground mb-1 flex items-center gap-1">
                      <Building2 className="h-3 w-3" /> Sector
                    </div>
                    <div className="font-medium">{(transaction as IoTTransaction).sector}</div>
                  </div>
                  <div className="p-4 bg-secondary/10 rounded-lg border border-border">
                    <div className="text-xs text-muted-foreground mb-1 flex items-center gap-1">
                      <Cpu className="h-3 w-3" /> Device
                    </div>
                    <div className="font-medium">{(transaction as IoTTransaction).deviceType}</div>
                  </div>
                  <div className="p-4 bg-secondary/10 rounded-lg border border-border">
                    <div className="text-xs text-muted-foreground mb-1 flex items-center gap-1">
                      <MapPin className="h-3 w-3" /> Location
                    </div>
                    <div className="font-medium">{(transaction as IoTTransaction).location}</div>
                  </div>
                </div>

                <div className="bg-black/50 p-4 rounded-lg border border-border font-mono text-sm text-green-400 overflow-x-auto">
                  <pre>{(() => {
                    try {
                      return JSON.stringify(JSON.parse((transaction as IoTTransaction).dataPayload), null, 2);
                    } catch {
                      return (transaction as IoTTransaction).dataPayload;
                    }
                  })()}</pre>
                </div>
              </CardContent>
            </Card>
          )}
        </div>

        {/* Sidebar Column */}
        <div className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-base">
                <Clock className="h-4 w-4 text-orange-400" />
                Timestamp
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="text-lg font-medium">
                {new Date(transaction.timestamp).toLocaleString()}
              </div>
              <div className="text-xs text-muted-foreground mt-1">
                {new Date(transaction.timestamp).toUTCString()}
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-base">
                <Database className="h-4 w-4 text-yellow-400" />
                Value
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex justify-between items-center border-b border-border pb-2">
                <span className="text-muted-foreground text-sm">Type</span>
                <Badge variant="secondary" className="text-xs">
                  {transaction.tx_type}
                </Badge>
              </div>
              <div>
                <div className="text-sm text-muted-foreground mb-1">Amount</div>
                <div className="font-mono font-bold text-xl text-green-400">
                  {transaction.amount.toLocaleString(undefined, { minimumFractionDigits: 4 })} EDGE
                </div>
              </div>
              <div>
                <div className="text-sm text-muted-foreground mb-1">Gas Fee</div>
                <div className="font-mono text-sm">
                  0.00042 EDGE
                </div>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-base">
                <ShieldCheck className="h-4 w-4 text-teal-400" />
                Verification
              </CardTitle>
            </CardHeader>
            <CardContent className="flex flex-col items-center space-y-4">
              <div className="bg-white p-4 rounded-lg w-full flex justify-center">
                <QRCode 
                  value={`${window.location.origin}/tx/${transaction.id}`} 
                  size={150} 
                  style={{ height: "auto", maxWidth: "100%", width: "100%" }}
                />
              </div>
              <p className="text-xs text-center text-muted-foreground">
                Scan to verify on mobile
              </p>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}
