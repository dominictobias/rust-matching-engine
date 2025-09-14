import { Route, Switch, useLocation } from "wouter";
import { useEffect } from "react";
import LoginPage from "./pages/LoginPage";
import IndexPage from "./pages/IndexPage";
import TradePage from "./pages/TradePage";
import { useSessionValidation } from "./hooks/useSessionValidation";

export function App() {
  const { isCheckingSession, isAuthed } = useSessionValidation();
  const [location, setLocation] = useLocation();

  // Redirect logic
  useEffect(() => {
    if (!isCheckingSession) {
      if (isAuthed && location === "/login") {
        setLocation("/");
      } else if (!isAuthed && location !== "/login") {
        setLocation("/login");
      }
    }
  }, [isCheckingSession, isAuthed, location, setLocation]);

  // Show loading spinner while checking session
  if (isCheckingSession) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="animate-spin rounded-full h-32 w-32 border-b-2 border-blue-500"></div>
      </div>
    );
  }

  return (
    <Switch>
      <Route path="/login" component={LoginPage} />
      <Route path="/trade/:assetId" component={TradePage} />
      <Route path="/" component={IndexPage} />
    </Switch>
  );
}
