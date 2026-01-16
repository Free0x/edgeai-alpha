import { Switch, Route, Link, useLocation } from "wouter";
import { Toaster } from "@/components/ui/sonner";
import { ThemeProvider } from "@/components/theme-provider";
import { WalletProvider } from "@/contexts/WalletContext";
import WalletConnect from "@/components/WalletConnect";

import { Sheet, SheetContent, SheetTrigger, SheetTitle, SheetDescription } from "@/components/ui/sheet";
import { Button } from "@/components/ui/button";
import { Menu, Loader2, Home, Blocks, ArrowLeftRight, Server, Coins, Landmark, BarChart3, Store, Wallet, Cpu, Gift, Link2 } from "lucide-react";
import { ThemeToggle } from "@/components/ThemeToggle";
import { useState, Suspense, lazy } from "react";
import { cn } from "@/lib/utils";

// Lazy load pages for better initial load performance
const Dashboard = lazy(() => import("@/pages/Dashboard"));
const BlocksPage = lazy(() => import("@/pages/Blocks"));
const BlockDetails = lazy(() => import("@/pages/BlockDetails"));
const Transactions = lazy(() => import("@/pages/Transactions"));
const TransactionDetails = lazy(() => import("@/pages/TransactionDetails"));
const Validators = lazy(() => import("@/pages/Validators"));
const Staking = lazy(() => import("@/pages/Staking"));
const Governance = lazy(() => import("@/pages/Governance"));
const Marketplace = lazy(() => import("@/pages/Marketplace"));
const DEX = lazy(() => import("@/pages/DEX"));
const WalletPage = lazy(() => import("@/pages/Wallet"));
const BridgePage = lazy(() => import("@/pages/Bridge"));
const IoT = lazy(() => import("@/pages/IoT"));
const Rewards = lazy(() => import("@/pages/Rewards"));
const NotFound = lazy(() => import("@/pages/NotFound"));

// Loading fallback component
function PageLoader() {
  return (
    <div className="flex items-center justify-center min-h-[60vh]">
      <div className="flex flex-col items-center gap-4">
        <Loader2 className="h-8 w-8 animate-spin text-primary" />
        <p className="text-sm text-muted-foreground">Loading...</p>
      </div>
    </div>
  );
}

// Navigation icons mapping
const NavIcon = ({ name, className }: { name: string; className?: string }) => {
  const iconProps = { className: cn("h-4 w-4", className) };
  switch (name) {
    case "home": return <Home {...iconProps} />;
    case "blocks": return <Blocks {...iconProps} />;
    case "transactions": return <ArrowLeftRight {...iconProps} />;
    case "validators": return <Server {...iconProps} />;
    case "staking": return <Coins {...iconProps} />;
    case "governance": return <Landmark {...iconProps} />;
    case "dex": return <BarChart3 {...iconProps} />;
    case "marketplace": return <Store {...iconProps} />;
    case "wallet": return <Wallet {...iconProps} />;
    case "bridge": return <Link2 {...iconProps} />;
    case "iot": return <Cpu {...iconProps} />;
    case "rewards": return <Gift {...iconProps} />;
    default: return null;
  }
};

function Layout({ children }: { children: React.ReactNode }) {
  const [location] = useLocation();
  const [isOpen, setIsOpen] = useState(false);

  const navItems = [
    { path: "/", label: "Dashboard", icon: "home" },
    { path: "/blocks", label: "Blocks", icon: "blocks" },
    { path: "/transactions", label: "Transactions", icon: "transactions" },
    { path: "/validators", label: "Validators", icon: "validators" },
    { path: "/staking", label: "Staking", icon: "staking" },
    { path: "/governance", label: "Governance", icon: "governance" },
    { path: "/dex", label: "DEX", icon: "dex" },
    { path: "/marketplace", label: "Market Place", icon: "marketplace" },
    { path: "/wallet", label: "Wallet", icon: "wallet" },
    { path: "/bridge", label: "Bridge", icon: "bridge" },
    { path: "/iot", label: "IoT Hub", icon: "iot" },
    { path: "/rewards", label: "Rewards", icon: "rewards" },
  ];

  return (
    <div className="min-h-screen bg-background text-foreground font-sans">
      <header className="sticky top-0 z-50 w-full border-b border-border/40 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
        <div className="container flex h-14 sm:h-16 lg:h-20 max-w-screen-2xl items-center justify-between px-3 sm:px-4 lg:px-8">
          {/* Logo - responsive sizing */}
          <Link href="/" className="flex items-center gap-2 shrink-0">
            <img 
              src="/images/logo_transparent.png" 
              alt="EdgeAI" 
              className="h-8 sm:h-12 lg:h-16 w-auto object-contain" 
              loading="lazy" 
            />
            <span className="hidden sm:inline-block text-lg font-bold text-foreground">
              EdgeAI
            </span>
          </Link>
          
          {/* Desktop Navigation */}
          <nav className="hidden xl:flex items-center gap-1">
            {navItems.map((item) => (
              <Link 
                key={item.path} 
                href={item.path} 
                className={cn(
                  "flex items-center gap-1.5 px-2.5 py-1.5 rounded-md text-xs font-medium transition-colors hover:bg-accent hover:text-accent-foreground cursor-pointer whitespace-nowrap",
                  location === item.path || (item.path !== "/" && location.startsWith(item.path))
                    ? "bg-accent text-accent-foreground"
                    : "text-muted-foreground"
                )}
              >
                <NavIcon name={item.icon} />
                {item.label}
              </Link>
            ))}
          </nav>

          {/* Tablet Navigation - Compact */}
          <nav className="hidden lg:flex xl:hidden items-center gap-0.5">
            {navItems.slice(0, 8).map((item) => (
              <Link 
                key={item.path} 
                href={item.path} 
                className={cn(
                  "flex items-center justify-center p-2 rounded-md transition-colors hover:bg-accent hover:text-accent-foreground cursor-pointer",
                  location === item.path || (item.path !== "/" && location.startsWith(item.path))
                    ? "bg-accent text-accent-foreground"
                    : "text-muted-foreground"
                )}
                title={item.label}
              >
                <NavIcon name={item.icon} />
              </Link>
            ))}
            {/* More menu for remaining items */}
            <Sheet>
              <SheetTrigger asChild>
                <Button variant="ghost" size="icon" className="h-8 w-8">
                  <Menu className="h-4 w-4" />
                </Button>
              </SheetTrigger>
              <SheetContent side="right" className="w-[200px]">
                <SheetTitle className="text-sm">More</SheetTitle>
                <SheetDescription className="sr-only">Additional navigation items</SheetDescription>
                <nav className="flex flex-col gap-2 mt-4">
                  {navItems.slice(8).map((item) => (
                    <Link 
                      key={item.path} 
                      href={item.path}
                      className={cn(
                        "flex items-center gap-2 px-3 py-2 rounded-md text-sm font-medium transition-colors hover:bg-accent hover:text-accent-foreground cursor-pointer",
                        location === item.path || (item.path !== "/" && location.startsWith(item.path))
                          ? "bg-accent text-accent-foreground"
                          : "text-muted-foreground"
                      )}
                    >
                      <NavIcon name={item.icon} />
                      {item.label}
                    </Link>
                  ))}
                </nav>
              </SheetContent>
            </Sheet>
          </nav>

          {/* Right side: Wallet Connect + Theme Toggle */}
          <div className="hidden lg:flex items-center gap-2">
            <WalletConnect />
            <ThemeToggle />
          </div>

          {/* Mobile Navigation */}
          <div className="lg:hidden flex items-center gap-1 sm:gap-2">
            <div className="scale-90 sm:scale-100">
              <WalletConnect />
            </div>
            <ThemeToggle />
            <Sheet open={isOpen} onOpenChange={setIsOpen}>
              <SheetTrigger asChild>
                <Button variant="ghost" size="icon" className="h-9 w-9 sm:h-10 sm:w-10">
                  <Menu className="h-5 w-5 sm:h-6 sm:w-6" />
                  <span className="sr-only">Toggle menu</span>
                </Button>
              </SheetTrigger>
              <SheetContent side="right" className="w-[260px] sm:w-[300px] p-4">
                <SheetTitle className="text-base font-semibold mb-2">Navigation</SheetTitle>
                <SheetDescription className="sr-only">
                  Mobile navigation menu for accessing different sections of the application.
                </SheetDescription>
                <nav className="flex flex-col gap-1 mt-4">
                  {navItems.map((item) => (
                    <Link 
                      key={item.path} 
                      href={item.path} 
                      onClick={() => setIsOpen(false)}
                      className={cn(
                        "flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-colors hover:bg-accent hover:text-accent-foreground cursor-pointer",
                        location === item.path || (item.path !== "/" && location.startsWith(item.path))
                          ? "bg-accent text-accent-foreground"
                          : "text-muted-foreground"
                      )}
                    >
                      <NavIcon name={item.icon} className="h-5 w-5" />
                      {item.label}
                    </Link>
                  ))}
                </nav>
              </SheetContent>
            </Sheet>
          </div>
        </div>
      </header>

      <main className="container py-4 sm:py-6 max-w-screen-2xl px-3 sm:px-4 lg:px-8">
        <Suspense fallback={<PageLoader />}>
          {children}
        </Suspense>
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
            <Route path="/blocks" component={BlocksPage} />
            <Route path="/block/:id" component={BlockDetails} />
            <Route path="/transactions" component={Transactions} />
            <Route path="/tx/:id" component={TransactionDetails} />
            <Route path="/validators" component={Validators} />
            <Route path="/staking" component={Staking} />
            <Route path="/governance" component={Governance} />
            <Route path="/dex" component={DEX} />
            <Route path="/marketplace" component={Marketplace} />
            <Route path="/wallet" component={WalletPage} />
            <Route path="/bridge" component={BridgePage} />
            <Route path="/iot" component={IoT} />
            <Route path="/rewards" component={Rewards} />
            <Route component={NotFound} />
          </Switch>
        </Layout>
      </WalletProvider>
    </ThemeProvider>
  );
}

export default App;
