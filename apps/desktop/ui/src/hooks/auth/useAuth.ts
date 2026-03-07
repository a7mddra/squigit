/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

type AuthStage = "LOADING" | "LOGIN" | "AUTHENTICATED";

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
