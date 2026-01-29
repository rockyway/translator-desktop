import { StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./index.css";
import App from "./App.tsx";
import { PopupWindow } from "./features/popup/PopupWindow.tsx";
import { ConfirmationWindow } from "./features/confirmation";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60 * 5, // 5 minutes
      retry: 1,
    },
  },
});

/**
 * Root component that renders the appropriate UI based on window label.
 * - "popup" -> PopupWindow (compact translation overlay)
 * - "main" or others -> Full App
 */
function Root() {
  const [windowLabel, setWindowLabel] = useState<string | null>(null);

  useEffect(() => {
    // Get the current window's label to determine which UI to render
    const currentWindow = getCurrentWindow();
    setWindowLabel(currentWindow.label);
  }, []);

  // Show loading state while determining window type
  if (windowLabel === null) {
    return null;
  }

  // Route to popup, confirmation, or main app based on window label
  if (windowLabel === "popup") {
    return <PopupWindow />;
  }

  if (windowLabel === "confirmation") {
    return <ConfirmationWindow />;
  }

  return <App />;
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <Root />
    </QueryClientProvider>
  </StrictMode>
);
