import { useState } from "react";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { BarChart3, Store } from "lucide-react";

// Import existing page components
import DEX from "./DEX";
import Marketplace from "./Marketplace";

export default function Trade() {
  const [activeTab, setActiveTab] = useState("dex");

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Trade</h1>
      </div>

      <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
        <TabsList className="grid w-full max-w-md grid-cols-2">
          <TabsTrigger value="dex" className="flex items-center gap-2">
            <BarChart3 className="h-4 w-4" />
            DEX
          </TabsTrigger>
          <TabsTrigger value="marketplace" className="flex items-center gap-2">
            <Store className="h-4 w-4" />
            Marketplace
          </TabsTrigger>
        </TabsList>

        <TabsContent value="dex" className="mt-4">
          <DEX />
        </TabsContent>

        <TabsContent value="marketplace" className="mt-4">
          <Marketplace />
        </TabsContent>
      </Tabs>
    </div>
  );
}
