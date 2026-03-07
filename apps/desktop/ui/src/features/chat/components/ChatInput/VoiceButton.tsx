/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { Mic, Square } from "lucide-react";
import styles from "./ChatInput.module.css";
import { Tooltip } from "@/components";

interface VoiceButtonProps {
  onTranscript: (text: string, isFinal: boolean) => void;
  disabled?: boolean;
}

interface SttEvent {
  type: "transcription" | "status" | "error";
  text?: string;
  is_final?: boolean;
  status?: string;
  message?: string;
}

export const VoiceButton: React.FC<VoiceButtonProps> = ({
  onTranscript,
  disabled,
}) => {
  const [isRecording, setIsRecording] = useState(false);

  const onTranscriptRef = useRef(onTranscript);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  const micButtonRef = useRef<HTMLButtonElement>(null);
  const [showMicButtonTooltip, setShowMicButtonTooltip] = useState(false);

  useEffect(() => {
    onTranscriptRef.current = onTranscript;
  }, [onTranscript]);

  useEffect(() => {
    let isMounted = true;

    const setupListener = async () => {
      try {
        const unlisten = await listen<SttEvent>("stt_event", (event) => {
          const payload = event.payload;

          if (payload.type === "transcription" && payload.text) {
            onTranscriptRef.current(payload.text, payload.is_final || false);
          } else if (payload.type === "status") {
            console.log("[STT] Status:", payload.status);
            if (payload.status === "ready") {
              // ready
            }
          } else if (payload.type === "error") {
            console.error("[STT] Error:", payload.message);
            setIsRecording(false);
          }
        });

        if (isMounted) {
          unlistenRef.current = unlisten;
        } else {
          unlisten();
        }
      } catch (err) {
        console.error("Failed to setup STT listener:", err);
      }
    };

    setupListener();

    return () => {
      isMounted = false;
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
    };
  }, []);

  const toggleRecording = useCallback(async () => {
    if (disabled) return;

    if (isRecording) {
      try {
        await invoke("stop_stt");
      } catch (err) {
        console.error("Failed to stop STT:", err);
      } finally {
        setIsRecording(false);
      }
    } else {
      try {
        setIsRecording(true);
        await invoke("start_stt", {
          model: "ggml-tiny.en.bin",
          language: "en",
        });
      } catch (err) {
        console.error("Failed to start STT:", err);
        setIsRecording(false);
      }
    }
  }, [disabled, isRecording]);

  return (
    <div className={styles.voiceContainer}>
      <button
        className={`${styles.micButton} ${isRecording ? styles.recording : ""}`}
        onClick={toggleRecording}
        disabled={disabled}
        type="button"
        ref={micButtonRef}
        onMouseEnter={() => setShowMicButtonTooltip(true)}
        onMouseLeave={() => setShowMicButtonTooltip(false)}
      >
        {isRecording ? (
          <Square className={styles.icon} fill="currentColor" />
        ) : (
          <Mic className={styles.icon} />
        )}
      </button>
      <Tooltip
        text={isRecording ? "Stop recording" : "Start recording"}
        parentRef={micButtonRef}
        show={showMicButtonTooltip}
        above
      />
    </div>
  );
};
