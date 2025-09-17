import { create } from "zustand";
import { persist, subscribeWithSelector } from "zustand/middleware";
import type { User } from "../types/api";
import { getProfile } from "../utils/api";
import { notificationWebSocket } from "../utils/websocket";

interface UserState {
  // Persisted state (stored in localStorage)
  user: User | null;

  // Non-persisted state (not stored in localStorage)
  isAuthed: boolean;
  sessionChecked: boolean;
}

interface UserActions {
  setUser: (user: User) => void;
  clearUser: () => void;
  setSessionChecked: (checked: boolean) => void;
  setAuthStatus: (isAuthed: boolean) => void;
  markSessionAsValidated: () => void;
  refreshProfile: () => Promise<void>;
}

type UserStore = UserState & UserActions;

const initialState: UserState = {
  user: null,
  isAuthed: false,
  sessionChecked: false,
};

export const useUserStore = create<UserStore>()(
  subscribeWithSelector(
    persist(
      (set, get) => ({
        ...initialState,
        setUser: (user: User) => {
          set({
            user,
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
          if (!state.user?.session_id) {
            set({
              isAuthed: false,
              sessionChecked: true,
              user: null,
            });
            return;
          }

          try {
            const updatedUser = await getProfile(state.user.session_id);
            set({
              user: updatedUser,
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
            });
          }
        },
      }),
      {
        name: "user-storage",
        version: 3,
        // Only persist these fields
        partialize: (state) => ({
          user: state.user,
        }),
        // Handle store hydration from localStorage
        onRehydrateStorage: () => {
          return (state) => {
            // After rehydration, trigger onSessionIdChange if we have a session_id
            if (state?.user?.session_id) {
              console.log(
                "Store rehydrated with session_id:",
                state.user.session_id
              );
              // Use setTimeout to ensure this runs after the store is fully initialized
              setTimeout(() => {
                onSessionIdChange(state.user!.session_id);
              }, 0);
            }
          };
        },
      }
    )
  )
);

function onSessionIdChange(sessionId: string) {
  console.log("Session ID changed:", sessionId);

  if (sessionId) {
    // Connect to notifications WebSocket
    notificationWebSocket.connect(sessionId);
  } else {
    // Disconnect from WebSocket when session is cleared
    notificationWebSocket.disconnect();
  }
}

useUserStore.subscribe(
  (state) => state.user?.session_id || "",
  onSessionIdChange
);
