/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import type { Citation, PendingAssistantTurn, ToolStep } from "./types";
import type { ProviderStreamEvent } from "./types";

type ToolEventHandlerOptions = {
  onResetText?: () => void;
  setToolStatus: (value: string | null) => void;
  setStreamingToolSteps: (steps: ToolStep[]) => void;
  setStreamingCitations: (citations: Citation[]) => void;
  updatePendingAssistantTurn: (
    updater: (turn: PendingAssistantTurn) => PendingAssistantTurn,
  ) => void;
  mapToolStatusText: (
    status: string,
  ) =>
    | { type: "set"; text: string }
    | { type: "clear" }
    | { type: "skip" };
  getDefaultProgressText: (
    requestKind: PendingAssistantTurn["requestKind"],
  ) => string;
};

export function createToolEventHandler(options: ToolEventHandlerOptions) {
  let steps: ToolStep[] = [];
  let citations: Citation[] = [];

  const mergeCitations = (incoming: Citation[]) => {
    if (!incoming.length) return;

    const byUrl = new Map<string, Citation>();
    for (const citation of citations) byUrl.set(citation.url, citation);
    for (const citation of incoming) byUrl.set(citation.url, citation);
    citations = Array.from(byUrl.values());
  };

  const parseCitationsFromResult = (result: unknown): Citation[] => {
    if (!result || typeof result !== "object") return [];

    const payload = result as { sources?: unknown };
    if (!Array.isArray(payload.sources)) return [];

    const parsed: Citation[] = [];

    for (const entry of payload.sources) {
      const source = entry as {
        title?: string;
        url?: string;
        summary?: string;
        favicon?: string;
      };

      if (!source.url || !source.title) {
        continue;
      }

      parsed.push({
        title: String(source.title),
        url: String(source.url),
        summary: String(source.summary || ""),
        favicon:
          typeof source.favicon === "string" && source.favicon.trim().length > 0
            ? source.favicon
            : undefined,
      });
    }

    return parsed;
  };

  const onEvent = (event: ProviderStreamEvent) => {
    if (!event?.type) return;

    if (event.type === "reset") {
      options.onResetText?.();
      return;
    }

    if (event.type === "tool_status") {
      const mappedStatus = options.mapToolStatusText(event.message);
      if (mappedStatus.type === "set") {
        options.setToolStatus(mappedStatus.text);
        options.updatePendingAssistantTurn((turn) => ({
          ...turn,
          progressText: mappedStatus.text,
        }));
      } else if (mappedStatus.type === "clear") {
        options.setToolStatus(null);
        options.updatePendingAssistantTurn((turn) => ({
          ...turn,
          progressText: options.getDefaultProgressText(turn.requestKind),
        }));
      }
      return;
    }

    if (event.type === "tool_start") {
      const startedAtMs = Date.now();
      const next: ToolStep = {
        id: String(event.id || `tool-${Date.now()}`),
        name: String(event.name || "tool"),
        status: "running",
        args: event.args || {},
        message: event.message || "",
        startedAtMs,
      };
      steps = [...steps, next];
      options.setStreamingToolSteps([...steps]);
      options.updatePendingAssistantTurn((turn) => ({
        ...turn,
        toolSteps: [...steps],
      }));
      return;
    }

    if (event.type === "tool_end") {
      const id = String(event.id || "");
      const endedAtMs = Date.now();
      let matched = false;

      steps = steps.map((step) => {
        if (step.id !== id) return step;
        matched = true;
        const startedAtMs = step.startedAtMs || endedAtMs;
        return {
          ...step,
          status: event.status === "error" ? "error" : "done",
          endedAtMs,
          message: `Thought for ${Math.max(
            1,
            Math.round((endedAtMs - startedAtMs) / 1000),
          )}s`,
        };
      });

      if (!matched) {
        steps = [
          ...steps,
          {
            id: id || `tool-${endedAtMs}`,
            name: String(event.name || "tool"),
            status: event.status === "error" ? "error" : "done",
            args: {},
            startedAtMs: endedAtMs,
            endedAtMs,
            message: "Thought for 1s",
          },
        ];
      }

      options.setStreamingToolSteps([...steps]);

      const parsedCitations = parseCitationsFromResult(event.result);
      if (parsedCitations.length > 0) {
        mergeCitations(parsedCitations);
        options.setStreamingCitations([...citations]);
      }

      options.updatePendingAssistantTurn((turn) => ({
        ...turn,
        toolSteps: [...steps],
        pendingCitations: [...citations],
        visibleCitations:
          turn.phase === "complete" || turn.phase === "stopped" || turn.stopped
            ? [...citations]
            : turn.visibleCitations,
      }));
    }
  };

  return {
    onEvent,
    snapshot: () => ({
      steps: [...steps],
      citations: [...citations],
    }),
  };
}
