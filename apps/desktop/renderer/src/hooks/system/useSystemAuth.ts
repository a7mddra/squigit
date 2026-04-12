/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { commands } from "@/core/api/tauri";
import { listen } from "@tauri-apps/api/event";
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

type AuthStage = "LOADING" | "LOGIN" | "AUTHENTICATED";
let hasLoggedMissingAuthConfig = false;

type AuthFailurePayload = {
  error: string;
  cancelled: boolean;
};

export const useAuth = () => {
  const [authStage, setAuthStage] = useState<AuthStage>("LOADING");

  useEffect(() => {
    const checkAuthStatus = async () => {
      try {
        const activeProfile = await invoke<any>("get_active_profile");
        if (activeProfile) {
          setAuthStage("AUTHENTICATED");
        } else {
          const hasProfiles = await invoke<boolean>("has_profiles");
          if (hasProfiles) {
            setAuthStage("LOGIN");
          } else {
            setAuthStage("LOGIN");
          }
        }
      } catch (e) {
        console.error("Auth check failed:", e);
        setAuthStage("LOGIN");
      }
    };

    checkAuthStatus();
  }, []);

  const login = () => {
    setAuthStage("AUTHENTICATED");
  };

  const logout = () => {
    setAuthStage("LOGIN");
  };

  return {
    authStage,
    isAuthenticated: authStage === "AUTHENTICATED",
    login,
    logout,
  };
};

export const useSystemAuth = (
  setSwitchingProfileId: (id: string | null) => void,
) => {
  useEffect(() => {
    let cancelled = false;

    const attachFailureListener = async () => {
      const unlisten = await listen<AuthFailurePayload>(
        "auth-failure",
        (event) => {
          if (cancelled) return;

          setSwitchingProfileId(null);

          if (!event.payload?.cancelled) {
            console.error("Google auth failed:", event.payload?.error);
          }
        },
      );

      if (cancelled) {
        unlisten();
      }

      return unlisten;
    };

    let cleanup: (() => void) | undefined;
    attachFailureListener().then((unlisten) => {
      cleanup = unlisten;
    });

    return () => {
      cancelled = true;
      cleanup?.();
    };
  }, [setSwitchingProfileId]);

  const addAccount = async () => {
    setSwitchingProfileId("creating_account");

    const performAuth = async () => {
      const timeoutPromise = new Promise((_, reject) =>
        setTimeout(() => reject(new Error("Auth Timeout")), 120000),
      );
      await Promise.race([commands.startGoogleAuth(), timeoutPromise]);
    };

    try {
      await performAuth();
    } catch (e: any) {
      const errorMsg = String(e);
      if (
        errorMsg.includes("Authentication already in progress") ||
        errorMsg.includes("Address in use") ||
        errorMsg.includes("OSError")
      ) {
        console.warn("Auth blocked, attempting to cleanup and retry...");
        try {
          await commands.cancelGoogleAuth();
          await new Promise((resolve) => setTimeout(resolve, 500));
          await performAuth();
        } catch (retryErr) {
          console.error("Failed to restart auth:", retryErr);
          setSwitchingProfileId(null);
        }
      } else {
        const missingConfig = errorMsg.includes(
          "Google authentication is not configured in this build.",
        );
        if (missingConfig) {
          if (!hasLoggedMissingAuthConfig) {
            hasLoggedMissingAuthConfig = true;
            console.warn(
              "[auth] Google auth is not configured in this build. See VS Code terminal for setup steps.",
            );
          }
        } else {
          console.error("Failed to start auth:", e);
        }

        if (errorMsg.includes("Auth Timeout")) {
          console.warn("Auth timed out, cancelling backend process...");
          try {
            await commands.cancelGoogleAuth();
          } catch (cancelErr) {
            console.error("Failed to cancel timed out auth:", cancelErr);
          }
        }
        setSwitchingProfileId(null);
      }
    }
  };

  const cancelAuth = async () => {
    try {
      await commands.cancelGoogleAuth();
    } catch (e) {
      console.error("Failed to cancel auth:", e);
    } finally {
      setSwitchingProfileId(null);
    }
  };

  return { addAccount, cancelAuth };
};
