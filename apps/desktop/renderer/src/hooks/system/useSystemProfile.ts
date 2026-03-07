/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useState } from "react";
import { Profile, commands } from "@/lib";

export const useSystemProfile = () => {
  const [activeProfile, setActiveProfile] = useState<Profile | null>(null);
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [profileLoaded, setProfileLoaded] = useState(false);
  const [showExistingProfileDialog, setShowExistingProfileDialog] =
    useState(false);
  const [switchingProfileId, setSwitchingProfileId] = useState<string | null>(
    null,
  );

  const [userName, setUserName] = useState("");
  const [userEmail, setUserEmail] = useState("");
  const [avatarSrc, setAvatarSrc] = useState("");
  const [originalPicture, setOriginalPicture] = useState<string | null>(null);

  const updateUserData = (data: {
    name: string;
    email: string;
    avatar: string;
    original_picture?: string;
  }) => {
    setUserName(data.name);
    setUserEmail(data.email);
    setAvatarSrc(data.avatar);
    if (data.original_picture) {
      setOriginalPicture(data.original_picture);
    }
  };

  const deleteProfile = async (profileId: string) => {
    await commands.deleteProfile(profileId);
  };

  return {
    activeProfile,
    setActiveProfile,
    profiles,
    setProfiles,
    profileLoaded,
    setProfileLoaded,
    showExistingProfileDialog,
    setShowExistingProfileDialog,
    switchingProfileId,
    setSwitchingProfileId,

    userName,
    setUserName,
    userEmail,
    setUserEmail,
    avatarSrc,
    setAvatarSrc,
    originalPicture,
    setOriginalPicture,
    updateUserData,

    deleteProfile,
  };
};
