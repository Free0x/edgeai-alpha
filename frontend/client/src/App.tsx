import { Switch, Route, Link, useLocation } from "wouter";
import { Toaster } from "@/components/ui/sonner";
import { ThemeProvider } from "@/components/theme-provider";
import { WalletProvider } from "@/contexts/WalletContext";
import WalletConnect from "@/components/WalletConnect";

import { Sheet, SheetContent, SheetTrigger, SheetTitle, SheetDescription } from "@/components/ui/sheet";
import { Button } from "@/components/ui/button";
import { Menu } from "lucide-react";
import { ThemeToggle } from "@/components/ThemeToggle";
import { useState } from "react";
import Dashboard from "@/pages/Dashboard";
import Blocks from "@/pages/Blocks";
import BlockDetails from "@/pages/BlockDetails";
import Transactions from "@/pages/Transactions";
import TransactionDetails from "@/pages/TransactionDetails";
import Validators from "@/pages/Validators";
import Staking from "@/pages/Staking";
import Governance from "@/pages/Governance";
import Marketplace from "@/pages/Marketplace";
import DEX from "@/pages/DEX";
import Wallet from "@/pages/Wallet";
import Bridge from "@/pages/Bridge";
import NotFound from "@/pages/NotFound";
import { cn } from "@/lib/utils";

function Layout({ children }: { children: React.ReactNode }) {
  const [location] = useLocation();
  const [isOpen, setIsOpen] = useState(false);

  const navItems = [
    { path: "/", label: "Dashboard", icon: "fas fa-chart-line" },
    { path: "/blocks", label: "Blocks", icon: "fas fa-cubes" },
    { path: "/transactions", label: "Transactions", icon: "fas fa-exchange-alt" },
    { path: "/validators", label: "Validators", icon: "fas fa-server" },
    { path: "/staking", label: "Staking", icon: "fas fa-coins" },
    { path: "/governance", label: "Governance", icon: "fas fa-landmark" },
    { path: "/dex", label: "DEX", icon: "fas fa-chart-bar" },
    { path: "/marketplace", label: "Market Place", icon: "fas fa-store" },
    { path: "/wallet", label: "Wallet", icon: "fas fa-wallet" },
    { path: "/bridge", label: "Bridge", icon: "fas fa-exchange-alt" },
  ];

  return (
    <div className="min-h-screen bg-background text-foreground font-sans">
      <header className="sticky top-0 z-50 w-full border-b border-border/40 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
        <div className="container flex h-20 max-w-screen-2xl items-center justify-between">
          <div className="flex items-center gap-2">
            <img src="/images/logo_transparent.png" alt="EdgeAI Blockchain" className="h-16 w-auto object-contain" />
          </div>
          
          {/* Desktop Navigation */}
          <nav className="hidden lg:flex items-center gap-1">
            {navItems.map((item) => (
              <Link key={item.path} href={item.path} className={cn(
                  "flex items-center gap-2 px-3 py-2 rounded-md text-sm font-medium transition-colors hover:bg-accent hover:text-accent-foreground cursor-pointer",
                  location === item.path || (item.path !== "/" && location.startsWith(item.path))
                    ? "bg-accent text-accent-foreground"
                    : "text-muted-foreground"
                )}>
                  <i className={item.icon}></i>
                  {item.label}
              </Link>
            ))}
          </nav>

          {/* Right side: Wallet Connect + Theme Toggle */}
          <div className="hidden lg:flex items-center gap-3">
            <WalletConnect />
            <ThemeToggle />
          </div>

          {/* Mobile Navigation */}
          <div className="lg:hidden flex items-center gap-2">
            <WalletConnect />
            <ThemeToggle />
            <Sheet open={isOpen} onOpenChange={setIsOpen}>
              <SheetTrigger asChild>
                <Button variant="ghost" size="icon" className="h-10 w-10">
                  <Menu className="h-6 w-6" />
                  <span className="sr-only">Toggle menu</span>
                </Button>
              </SheetTrigger>
              <SheetContent side="right" className="w-[240px] sm:w-[300px]">
                <SheetTitle className="sr-only">Navigation Menu</SheetTitle>
                <SheetDescription className="sr-only">
                  Mobile navigation menu for accessing different sections of the application.
                </SheetDescription>
                <nav className="flex flex-col gap-4 mt-8">
                  {navItems.map((item) => (
                    <Link 
                      key={item.path} 
                      href={item.path} 
                      onClick={() => setIsOpen(false)}
                      className={cn(
                        "flex items-center gap-2 px-4 py-2 rounded-md text-sm font-medium transition-colors hover:bg-accent hover:text-accent-foreground cursor-pointer",
                        location === item.path || (item.path !== "/" && location.startsWith(item.path))
                          ? "bg-accent text-accent-foreground"
                          : "text-muted-foreground"
                      )}>
                        <i className={item.icon}></i>
                        {item.label}
                    </Link>
                  ))}
                </nav>
              </SheetContent>
            </Sheet>
          </div>
        </div>
      </header>

      <main className="container py-6 max-w-screen-2xl">
        {children}
      </main>
      <Toaster />
    </div>
  );
}

function App() {
  return (
    <ThemeProvider defaultTheme="dark" storageKey="vite-ui-theme">
      <WalletProvider>
        <Layout>
          <Switch>
            <Route path="/" component={Dashboard} />
            <Route path="/blocks" component={Blocks} />
            <Route path="/block/:id" component={BlockDetails} />
            <Route path="/transactions" component={Transactions} />
            <Route path="/tx/:id" component={TransactionDetails} />
            <Route path="/validators" component={Validators} />
            <Route path="/staking" component={Staking} />
            <Route path="/governance" component={Governance} />
            <Route path="/dex" component={DEX} />
            <Route path="/marketplace" component={Marketplace} />
            <Route path="/wallet" component={Wallet} />
            <Route path="/bridge" component={Bridge} />
            <Route component={NotFound} />
          </Switch>
        </Layout>
      </WalletProvider>
    </ThemeProvider>
  );
}

export default App;
