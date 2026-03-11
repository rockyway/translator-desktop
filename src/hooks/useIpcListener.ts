import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/** Payload structure for text-selected events from the IPC bridge */
interface TextSelectedPayload {
  text: string;
  cursorX: number;
  cursorY: number;
  sourceApp: string;
  windowTitle: string | null;
  timestamp: string;
}

/** Payload structure for connection status events */
interface ConnectionStatusPayload {
  connected: boolean;
  timestamp: string;
}

/** Payload structure for text monitor version events */
interface TextMonitorVersionPayload {
  version: string;
  timestamp: string;
}

/** Return type for the useIpcListener hook */
export interface UseIpcListenerResult {
  selectedText: string | null;
  isConnected: boolean;
  textMonitorVersion: string | null;
  needsAccessibilityPermission: boolean;
  clearSelectedText: () => void;
}

/**
 * Hook to listen for IPC events from the Tauri backend.
 * Handles text-selected, ipc-connected, and ipc-disconnected events.
 */
export function useIpcListener(): UseIpcListenerResult {
  const [selectedText, setSelectedText] = useState<string | null>(null);
  const [isConnected, setIsConnected] = useState(false);
  const [textMonitorVersion, setTextMonitorVersion] = useState<string | null>(null);
  const [needsAccessibilityPermission, setNeedsAccessibilityPermission] = useState(false);

  const clearSelectedText = useCallback(() => {
    setSelectedText(null);
  }, []);

  useEffect(() => {
    let unlistenTextSelected: (() => void) | undefined;
    let unlistenConnected: (() => void) | undefined;
    let unlistenDisconnected: (() => void) | undefined;
    let unlistenVersion: (() => void) | undefined;
    let unlistenAccessibility: (() => void) | undefined;
    let unlistenAccessibilityGranted: (() => void) | undefined;

    const setupListeners = async () => {
      try {
        // Query initial IPC connection status from backend
        // This ensures correct status display even after page refresh
        try {
          const initialStatus = await invoke<boolean>("get_ipc_status");
          console.log("Initial IPC status:", initialStatus);
          setIsConnected(initialStatus);
        } catch (error) {
          console.warn("Failed to get initial IPC status:", error);
        }

        // Listen for text-selected events
        unlistenTextSelected = await listen<TextSelectedPayload>(
          "text-selected",
          (event) => {
            console.log("Received text-selected event:", event.payload);
            setSelectedText(event.payload.text);
          }
        );

        // Listen for IPC connected events
        unlistenConnected = await listen<ConnectionStatusPayload>(
          "ipc-connected",
          (event) => {
            console.log("IPC connected:", event.payload);
            setIsConnected(true);
          }
        );

        // Listen for IPC disconnected events
        unlistenDisconnected = await listen<ConnectionStatusPayload>(
          "ipc-disconnected",
          (event) => {
            console.log("IPC disconnected:", event.payload);
            setIsConnected(false);
          }
        );

        // Listen for text monitor version events
        unlistenVersion = await listen<TextMonitorVersionPayload>(
          "text-monitor-version",
          (event) => {
            console.log("Text Monitor version:", event.payload.version);
            setTextMonitorVersion(event.payload.version);
          }
        );

        // Listen for macOS accessibility permission needed event
        unlistenAccessibility = await listen(
          "accessibility-permission-needed",
          () => {
            console.log("Accessibility permission needed");
            setNeedsAccessibilityPermission(true);
          }
        );

        // Listen for macOS accessibility permission granted (after user enables it)
        unlistenAccessibilityGranted = await listen(
          "accessibility-permission-granted",
          () => {
            console.log("Accessibility permission granted");
            setNeedsAccessibilityPermission(false);
          }
        );

        console.log("IPC listeners registered");
      } catch (error) {
        console.error("Failed to setup IPC listeners:", error);
      }
    };

    setupListeners();

    return () => {
      if (unlistenTextSelected) {
        unlistenTextSelected();
      }
      if (unlistenConnected) {
        unlistenConnected();
      }
      if (unlistenDisconnected) {
        unlistenDisconnected();
      }
      if (unlistenVersion) {
        unlistenVersion();
      }
      if (unlistenAccessibility) {
        unlistenAccessibility();
      }
      if (unlistenAccessibilityGranted) {
        unlistenAccessibilityGranted();
      }
      console.log("IPC listeners unregistered");
    };
  }, []);

  return {
    selectedText,
    isConnected,
    textMonitorVersion,
    needsAccessibilityPermission,
    clearSelectedText,
  };
}
