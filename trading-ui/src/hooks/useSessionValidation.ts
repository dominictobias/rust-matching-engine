import { useEffect } from "react";
import { useUserStore } from "../stores/userStore";
import { validateSession } from "../utils/api";

export function useSessionValidation() {
  const {
    session_id,
    sessionChecked,
    setSessionChecked,
    setAuthStatus,
    clearUser,
  } = useUserStore();

  useEffect(() => {
    const checkSession = async () => {
      // If already checked (e.g., from login), skip validation
      if (sessionChecked) {
        return;
      }

      if (!session_id) {
        setSessionChecked(true);
        setAuthStatus(false);
        return;
      }

      try {
        const isValid = await validateSession(session_id);
        if (isValid) {
          setAuthStatus(true);
        } else {
          clearUser();
          setAuthStatus(false);
        }
      } catch (error) {
        console.error("Session validation failed:", error);
        clearUser();
        setAuthStatus(false);
      } finally {
        setSessionChecked(true);
      }
    };

    checkSession();
  }, [session_id, sessionChecked, setSessionChecked, setAuthStatus, clearUser]);

  return {
    isCheckingSession: !useUserStore((state) => state.sessionChecked),
    isAuthed: useUserStore((state) => state.isAuthed),
    hasSession: !!session_id,
  };
}
