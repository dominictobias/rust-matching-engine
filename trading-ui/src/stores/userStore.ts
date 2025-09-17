import { create } from "zustand";
import { persist, subscribeWithSelector } from "zustand/middleware";
import type { AuthenticatedUser } from "../types/api";
import { getProfile } from "../utils/api";

interface UserState {
  // Persisted state (stored in localStorage)
  email: string | null;
  sessionId: string | null;
  user: AuthenticatedUser | null;

  // Non-persisted state (not stored in localStorage)
  isAuthed: boolean;
  sessionChecked: boolean;
}

interface UserActions {
  setUser: (user: AuthenticatedUser, sessionId: string) => void;
  clearUser: () => void;
  setSessionChecked: (checked: boolean) => void;
  setAuthStatus: (isAuthed: boolean) => void;
  markSessionAsValidated: () => void;
  refreshProfile: () => Promise<void>;
}

type UserStore = UserState & UserActions;

const initialState: UserState = {
  email: null,
  sessionId: null,
  user: null,
  isAuthed: false,
  sessionChecked: false,
};

export const useUserStore = create<UserStore>()(
  subscribeWithSelector(
    persist(
      (set, get) => ({
        ...initialState,
        setUser: (user: AuthenticatedUser, sessionId: string) => {
          set({
            user,
            email: user.email,
            sessionId,
            isAuthed: true,
            sessionChecked: true, // Mark as checked since we just authenticated
          });
        },
        clearUser: () => {
          set(initialState);
        },
        setSessionChecked: (checked: boolean) => {
          set({ sessionChecked: checked });
        },
        setAuthStatus: (isAuthed: boolean) => {
          set({ isAuthed });
        },
        markSessionAsValidated: () => {
          set({ sessionChecked: true });
        },
        refreshProfile: async () => {
          const state = get();

          // If no session_id, mark as checked and unauthenticated
          if (!state.sessionId) {
            set({
              isAuthed: false,
              sessionChecked: true,
              user: null,
              email: null,
            });
            return;
          }

          try {
            const updatedUser = await getProfile(state.sessionId);
            set({
              user: updatedUser,
              email: updatedUser.email,
              isAuthed: true,
              sessionChecked: true,
            });
          } catch (error) {
            console.error(
              "Failed to refresh profile - session may be invalid:",
              error
            );
            // If profile refresh fails, the session is likely invalid
            set({
              isAuthed: false,
              sessionChecked: true,
              user: null,
              email: null,
            });
          }
        },
      }),
      {
        name: "user-storage",
        version: 1,
        // Only persist these fields
        partialize: (state) => ({
          email: state.email,
          sessionId: state.sessionId,
          user: state.user,
        }),
      }
    )
  )
);

function onSessionIdChange(sessionId: string) {
  console.log("Session ID changed:", sessionId);
}

useUserStore.subscribe(
  (state) => state.user?.session_id || "",
  onSessionIdChange
);
