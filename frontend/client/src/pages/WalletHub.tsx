import { useState } from "react";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Wallet, Link2, Gift } from "lucide-react";

// Import existing page components
import WalletPage from "./Wallet";
import Bridge from "./Bridge";
import Rewards from "./Rewards";

export default function WalletHub() {
  const [activeTab, setActiveTab] = useState("wallet");

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Wallet</h1>
      </div>

      <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
        <TabsList className="grid w-full max-w-lg grid-cols-3">
          <TabsTrigger value="wallet" className="flex items-center gap-2">
            <Wallet className="h-4 w-4" />
            Assets
          </TabsTrigger>
          <TabsTrigger value="bridge" className="flex items-center gap-2">
            <Link2 className="h-4 w-4" />
            Bridge
          </TabsTrigger>
          <TabsTrigger value="rewards" className="flex items-center gap-2">
            <Gift className="h-4 w-4" />
            Rewards
          </TabsTrigger>
        </TabsList>

        <TabsContent value="wallet" className="mt-4">
          <WalletPage />
        </TabsContent>

        <TabsContent value="bridge" className="mt-4">
          <Bridge />
        </TabsContent>

        <TabsContent value="rewards" className="mt-4">
          <Rewards />
        </TabsContent>
      </Tabs>
    </div>
  );
}
