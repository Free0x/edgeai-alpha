import { useState } from "react";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Server, Cpu } from "lucide-react";

// Import existing page components
import Validators from "./Validators";
import IoT from "./IoT";

export default function Nodes() {
  const [activeTab, setActiveTab] = useState("validators");

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Nodes</h1>
      </div>

      <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
        <TabsList className="grid w-full max-w-md grid-cols-2">
          <TabsTrigger value="validators" className="flex items-center gap-2">
            <Server className="h-4 w-4" />
            Validators
          </TabsTrigger>
          <TabsTrigger value="iot" className="flex items-center gap-2">
            <Cpu className="h-4 w-4" />
            IoT Devices
          </TabsTrigger>
        </TabsList>

        <TabsContent value="validators" className="mt-4">
          <Validators />
        </TabsContent>

        <TabsContent value="iot" className="mt-4">
          <IoT />
        </TabsContent>
      </Tabs>
    </div>
  );
}
