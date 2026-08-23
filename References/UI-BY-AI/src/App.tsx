import { AppProvider, useApp } from "./store/AppContext";
import { Sidebar } from "./components/Sidebar";
import { Header } from "./components/Header";
import { Dashboard } from "./pages/Dashboard";
import { ModulePage } from "./pages/ModulePage";
import { cn } from "./utils/cn";
import { LogIn, ShieldCheck } from "lucide-react";

function SignedOut() {
  const { signIn } = useApp();
  return (
    <div className="grid min-h-screen place-items-center bg-gradient-to-b from-[#191c3c] to-[#0e1024] p-6">
      <div className="fade-up w-full max-w-sm rounded-3xl border border-white/10 bg-white/[0.04] p-8 text-center shadow-[var(--shadow-lg)] backdrop-blur">
        <div className="mx-auto mb-5 grid size-16 place-items-center rounded-3xl bg-gradient-to-br from-[#e7bd75] to-[#c8923c] shadow-[0_12px_28px_-8px_rgba(220,167,87,.6)]">
          <svg viewBox="0 0 24 24" className="size-8 text-[#21254E]" fill="currentColor" aria-hidden>
            <path d="M12 2 2.5 9.5 12 22l9.5-12.5L12 2Zm0 3.1 5.4 4.4L12 17.2 6.6 9.5 12 5.1Z" />
          </svg>
        </div>
        <h1 className="text-xl font-extrabold text-white">نوین پرداز</h1>
        <p className="mt-1.5 text-xs text-white/55">نشست شما پایان یافت. برای ادامه وارد شوید.</p>
        <button
          onClick={signIn}
          className="mt-6 inline-flex w-full items-center justify-center gap-2 rounded-xl bg-accent py-3 text-sm font-extrabold text-[#241c3d] transition-transform hover:scale-[1.02] active:scale-95"
        >
          <LogIn className="size-4" aria-hidden />
          ورود به حساب کاربری
        </button>
        <p className="mt-4 flex items-center justify-center gap-1.5 text-[10px] text-white/40">
          <ShieldCheck className="size-3.5" aria-hidden />
          ارتباط امن · نسخه ۷٫۲
        </p>
      </div>
    </div>
  );
}

function Shell() {
  const { collapsed, route, signedIn } = useApp();

  if (!signedIn) return <SignedOut />;

  return (
    <div className="min-h-screen bg-bg text-text">
      <a
        href="#main"
        className="sr-only focus:not-sr-only focus:absolute focus:start-4 focus:top-3 focus:z-[60] focus:rounded-lg focus:bg-accent focus:px-4 focus:py-2 focus:text-xs focus:font-bold focus:text-[#241c3d]"
      >
        پرش به محتوای اصلی
      </a>
      <Sidebar />
      <div
        className={cn(
          "flex min-h-screen min-w-0 flex-col transition-[margin] duration-300",
          collapsed ? "lg:ms-[84px]" : "lg:ms-[272px]",
        )}
      >
        <Header />
        <main id="main" className="min-w-0 flex-1">
          {route === "dashboard" ? <Dashboard /> : <ModulePage />}
        </main>
        <footer className="border-t border-border px-5 py-4 text-center text-[10.5px] text-faint">
          نرم‌افزار حسابداری نوین پرداز · نسخه ۷٫۲ · تمامی قیمت‌ها به تومان است
        </footer>
      </div>
    </div>
  );
}

export default function App() {
  return (
    <AppProvider>
      <Shell />
    </AppProvider>
  );
}
