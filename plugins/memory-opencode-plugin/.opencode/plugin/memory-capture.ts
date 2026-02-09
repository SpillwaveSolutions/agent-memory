// memory-capture.ts
// OpenCode plugin for capturing session events into agent-memory.
//
// This plugin hooks into OpenCode lifecycle events and forwards them
// to the memory-ingest binary with agent:opencode tagging.
//
// Requires: memory-ingest binary in PATH (installed via agent-memory)
//
// Fail-open: All event capture is wrapped in try/catch. If memory-ingest
// is not available or the daemon is down, OpenCode continues normally.

import type { Plugin } from "@opencode-ai/plugin"

export const MemoryCapturePlugin: Plugin = async ({ $, directory }) => {
  const MEMORY_INGEST = process.env.MEMORY_INGEST_PATH || "memory-ingest"

  // Helper: extract session ID from various event input shapes.
  // OpenCode events provide session ID in different fields depending on event type.
  function extractSessionId(input: Record<string, unknown>): string {
    return (
      (input.id as string) ||
      (input.sessionID as string) ||
      (input.session_id as string) ||
      ((input as any).properties?.sessionID as string) ||
      "unknown"
    )
  }

  // Helper: send event to memory-ingest via stdin JSON pipe.
  // Uses fail-open pattern - silently catches all errors.
  async function captureEvent(event: {
    hook_event_name: string
    session_id: string
    message?: string
    tool_name?: string
    tool_input?: unknown
    cwd?: string
    timestamp?: string
  }): Promise<void> {
    try {
      const payload = JSON.stringify({
        ...event,
        agent: "opencode",
        cwd: event.cwd || directory,
        timestamp: event.timestamp || new Date().toISOString(),
      })
      await $`echo ${payload} | ${MEMORY_INGEST}`.quiet()
    } catch {
      // Fail-open: never block OpenCode on ingest failure
    }
  }

  return {
    // Session created - capture session start with project directory
    "session.created": async (input) => {
      await captureEvent({
        hook_event_name: "SessionStart",
        session_id: extractSessionId(input as Record<string, unknown>),
        cwd: directory,
      })
    },

    // Session idle - agent finished responding, treat as checkpoint/session end
    // Fulfills R1.4.1 (session end capture) and R1.4.2 (checkpoint capture)
    "session.idle": async (input) => {
      await captureEvent({
        hook_event_name: "Stop",
        session_id: extractSessionId(input as Record<string, unknown>),
        cwd: directory,
      })
    },

    // Message updated - capture user prompts and assistant responses
    "message.updated": async (input) => {
      const props = (input as any).properties
      const message = props?.message
      if (!message) return

      // Map role to hook event name
      const eventName =
        message.role === "user"
          ? "UserPromptSubmit"
          : message.role === "assistant"
            ? "AssistantResponse"
            : null

      if (!eventName) return

      // Handle content that may be string or array of content blocks
      const content =
        typeof message.content === "string"
          ? message.content
          : JSON.stringify(message.content)

      await captureEvent({
        hook_event_name: eventName,
        session_id: extractSessionId(input as Record<string, unknown>),
        message: content,
        cwd: directory,
      })
    },

    // Tool execution completed - capture tool results
    "tool.execute.after": async (input) => {
      const typedInput = input as Record<string, unknown>
      await captureEvent({
        hook_event_name: "PostToolUse",
        session_id: extractSessionId(typedInput),
        tool_name: typedInput.tool as string,
        tool_input: typedInput.args,
        cwd: directory,
      })
    },
  }
}
