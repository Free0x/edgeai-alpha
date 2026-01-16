import { Switch, Route, Link, useLocation } from "wouter";
import { Toaster } from "@/components/ui/sonner";
import { ThemeProvider } from "@/components/theme-provider";
import { WalletProvider } from "@/contexts/WalletContext";
import WalletConnect from "@/components/WalletConnect";

import { Sheet, SheetContent, SheetTrigger, SheetTitle, SheetDescription } from "@/components/ui/sheet";
import { Button } from "@/components/ui/button";
import { Menu, Loader2, Home, Search, Server, Network, BarChart3, Wallet } from "lucide-react";
import { ThemeToggle } from "@/components/ThemeToggle";
import { useState, Suspense, lazy } from "react";
import { cn } from "@/lib/utils";

// Lazy load pages for better initial load performance
const Dashboard = lazy(() => import("@/pages/Dashboard"));
const Explorer = lazy(() => import("@/pages/Explorer"));
const BlockDetails = lazy(() => import("@/pages/BlockDetails"));
const TransactionDetails = lazy(() => import("@/pages/TransactionDetails"));
const Nodes = lazy(() => import("@/pages/Nodes"));
const NetworkPage = lazy(() => import("@/pages/Network"));
const Trade = lazy(() => import("@/pages/Trade"));
const WalletHub = lazy(() => import("@/pages/WalletHub"));
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
    case "explorer": return <Search {...iconProps} />;
    case "nodes": return <Server {...iconProps} />;
    case "network": return <Network {...iconProps} />;
    case "trade": return <BarChart3 {...iconProps} />;
    case "wallet": return <Wallet {...iconProps} />;
    default: return null;
  }
};

function Layout({ children }: { children: React.ReactNode }) {
  const [location] = useLocation();
  const [isOpen, setIsOpen] = useState(false);

  // Simplified navigation - 6 items only
  const navItems = [
    { path: "/", label: "Dashboard", icon: "home" },
    { path: "/explorer", label: "Explorer", icon: "explorer" },
    { path: "/nodes", label: "Nodes", icon: "nodes" },
    { path: "/network", label: "Network", icon: "network" },
    { path: "/trade", label: "Trade", icon: "trade" },
    { path: "/wallet", label: "Wallet", icon: "wallet" },
  ];

  const isActive = (path: string) => {
    if (path === "/") return location === "/";
    return location.startsWith(path);
  };

  return (
    <div className="min-h-screen bg-background text-foreground font-sans">
      <header className="sticky top-0 z-50 w-full border-b border-border/40 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
        <div className="container flex h-14 sm:h-16 max-w-screen-2xl items-center justify-between px-3 sm:px-4 lg:px-8">
          {/* Logo */}
          <Link href="/" className="flex items-center gap-2 shrink-0">
            <img 
              src="/images/logo_transparent.png" 
              alt="EdgeAI" 
              className="h-8 sm:h-10 w-auto object-contain" 
              loading="lazy" 
            />
            <span className="hidden sm:inline-block text-lg font-bold text-foreground">
              EdgeAI
            </span>
          </Link>
          
          {/* Desktop Navigation - Clean and Simple */}
          <nav className="hidden md:flex items-center gap-1">
            {navItems.map((item) => (
              <Link 
                key={item.path} 
                href={item.path} 
                className={cn(
                  "flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-colors hover:bg-accent hover:text-accent-foreground cursor-pointer",
                  isActive(item.path)
                    ? "bg-accent text-accent-foreground"
                    : "text-muted-foreground"
                )}
              >
                <NavIcon name={item.icon} />
                {item.label}
              </Link>
            ))}
          </nav>

          {/* Right side: Wallet Connect + Theme Toggle */}
          <div className="hidden md:flex items-center gap-2">
            <WalletConnect />
            <ThemeToggle />
          </div>

          {/* Mobile Navigation */}
          <div className="md:hidden flex items-center gap-2">
            <WalletConnect />
            <ThemeToggle />
            <Sheet open={isOpen} onOpenChange={setIsOpen}>
              <SheetTrigger asChild>
                <Button variant="ghost" size="icon" className="h-9 w-9">
                  <Menu className="h-5 w-5" />
                  <span className="sr-only">Toggle menu</span>
                </Button>
              </SheetTrigger>
              <SheetContent side="right" className="w-[240px] p-4">
                <SheetTitle className="text-base font-semibold mb-4">Menu</SheetTitle>
                <SheetDescription className="sr-only">
                  Navigation menu
                </SheetDescription>
                <nav className="flex flex-col gap-1">
                  {navItems.map((item) => (
                    <Link 
                      key={item.path} 
                      href={item.path} 
                      onClick={() => setIsOpen(false)}
                      className={cn(
                        "flex items-center gap-3 px-4 py-3 rounded-lg text-sm font-medium transition-colors hover:bg-accent hover:text-accent-foreground cursor-pointer",
                        isActive(item.path)
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
            <Route path="/explorer" component={Explorer} />
            <Route path="/block/:id" component={BlockDetails} />
            <Route path="/tx/:id" component={TransactionDetails} />
            <Route path="/nodes" component={Nodes} />
            <Route path="/network" component={NetworkPage} />
            <Route path="/trade" component={Trade} />
            <Route path="/wallet" component={WalletHub} />
            <Route component={NotFound} />
          </Switch>
        </Layout>
      </WalletProvider>
    </ThemeProvider>
  );
}

export default App;
