import { Route, Switch, useLocation } from "wouter";
import { useEffect } from "react";
import LoginPage from "./pages/LoginPage";
import IndexPage from "./pages/IndexPage";
import TradePage from "./pages/TradePage";
import { useUserStore } from "./stores/userStore";

export function App() {
  const { sessionChecked, isAuthed, refreshProfile } = useUserStore();
  const [location, setLocation] = useLocation();

  // Session validation effect
  useEffect(() => {
    const checkSession = async () => {
      // If already checked, skip validation
      if (sessionChecked) {
        return;
      }

      // Use refreshProfile to validate session and update profile
      await refreshProfile();
    };

    checkSession();
  }, [sessionChecked, refreshProfile]);

  // Redirect logic
  useEffect(() => {
    if (sessionChecked) {
      if (isAuthed && location === "/login") {
        setLocation("/");
      } else if (!isAuthed && location !== "/login") {
        setLocation("/login");
      }
    }
  }, [sessionChecked, isAuthed, location, setLocation]);

  // Show loading spinner while checking session
  if (!sessionChecked) {
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
