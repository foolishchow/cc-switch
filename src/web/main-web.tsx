/**
 * Web 控制台：直接挂真实 `<App />`（桌面同源），经 alias shim → REST。
 *
 * provider 栈对齐桌面 main.tsx：FrontendErrorBoundary → QueryClient →
 * ThemeProvider → UpdateProvider → App。TokenGate 先写 token 再渲染，确保
 * App 的 query 发起时 Bearer 已就位。未映射命令返回 null（warn）不炸渲染；
 * 控制面 39 命令走 REST。非控制面 UI（目录/mcp/skills…）降级为空。
 */

import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import "@/index.css";
import "@/i18n";
import App from "@/App";
import { FrontendErrorBoundary } from "@/components/FrontendErrorBoundary";
import { ThemeProvider } from "@/components/theme-provider";
import { UpdateProvider } from "@/contexts/UpdateContext";
import { Toaster } from "@/components/ui/sonner";

const queryClient = new QueryClient();

function TokenGate({ children }: { children: React.ReactNode }) {
  // token 从 hash fragment 读取（#token=xxx），不进服务端 log / Referer
  const tok =
    new URLSearchParams(location.hash.slice(1)).get("token") ??
    localStorage.getItem("ccs.restToken") ??
    "";
  // 同步写入 localStorage（render 阶段），确保子组件 useEffect 发起 fetch 时已就位
  if (tok) localStorage.setItem("ccs.restToken", tok);
  const [input, setInput] = useState("");
  const [has, setHas] = useState(!!tok);
  // 清除 hash 在 useEffect（不影响 render，只洗浏览器历史）
  useEffect(() => {
    if (tok)
      history.replaceState(null, "", location.pathname + location.search);
  }, [tok]);
  if (has) return <>{children}</>;
  return (
    <div className="min-h-screen bg-background text-foreground font-sans flex items-center justify-center">
      <div className="glass-card rounded-xl p-6 w-full max-w-sm space-y-3">
        <h2 className="text-xl font-semibold">cc-switch Web Console</h2>
        <p className="text-sm text-muted-foreground">
          需要 REST Bearer token：
        </p>
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder="token"
          className="w-full px-3 py-2 rounded-md border border-input bg-background text-sm"
        />
        <button
          onClick={() => {
            localStorage.setItem("ccs.restToken", input);
            setHas(true);
          }}
          className="w-full px-4 py-2 rounded-md bg-primary text-primary-foreground text-sm hover:opacity-90"
        >
          进入
        </button>
      </div>
    </div>
  );
}

createRoot(document.getElementById("root")!).render(
  <FrontendErrorBoundary>
    <QueryClientProvider client={queryClient}>
      <ThemeProvider defaultTheme="system" storageKey="cc-switch-theme">
        <UpdateProvider>
          <TokenGate>
            <App />
          </TokenGate>
          <Toaster />
        </UpdateProvider>
      </ThemeProvider>
    </QueryClientProvider>
  </FrontendErrorBoundary>,
);
