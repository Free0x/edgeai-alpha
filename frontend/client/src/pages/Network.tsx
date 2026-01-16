import { useState } from "react";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Landmark, Coins } from "lucide-react";

// Import existing page components
import Governance from "./Governance";
import Staking from "./Staking";

export default function Network() {
  const [activeTab, setActiveTab] = useState("governance");

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Network</h1>
      </div>

      <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
        <TabsList className="grid w-full max-w-md grid-cols-2">
          <TabsTrigger value="governance" className="flex items-center gap-2">
            <Landmark className="h-4 w-4" />
            Governance
          </TabsTrigger>
          <TabsTrigger value="staking" className="flex items-center gap-2">
            <Coins className="h-4 w-4" />
            Staking
          </TabsTrigger>
        </TabsList>

        <TabsContent value="governance" className="mt-4">
          <Governance />
        </TabsContent>

        <TabsContent value="staking" className="mt-4">
          <Staking />
        </TabsContent>
      </Tabs>
    </div>
  );
}
