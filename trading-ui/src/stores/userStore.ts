import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { AuthenticatedUser } from "../types/api";

interface UserState {
  // Persisted state (stored in localStorage)
  email: string | null;
  session_id: string | null;
  user: AuthenticatedUser | null;

  // Non-persisted state (not stored in localStorage)
  isAuthed: boolean;
  sessionChecked: boolean;
}

interface UserActions {
  setUser: (user: AuthenticatedUser, session_id: string) => void;
  clearUser: () => void;
  setSessionChecked: (checked: boolean) => void;
  setAuthStatus: (isAuthed: boolean) => void;
  markSessionAsValidated: () => void;
}

type UserStore = UserState & UserActions;

const initialState: UserState = {
  email: null,
  session_id: null,
  user: null,
  isAuthed: false,
  sessionChecked: false,
};

export const useUserStore = create<UserStore>()(
  persist(
    (set) => ({
      ...initialState,
      setUser: (user: AuthenticatedUser, session_id: string) => {
        set({
          user,
          email: user.email,
          session_id,
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
    }),
    {
      name: "user-storage",
      // Only persist these fields
      partialize: (state) => ({
        email: state.email,
        session_id: state.session_id,
        user: state.user,
      }),
    }
  )
);
