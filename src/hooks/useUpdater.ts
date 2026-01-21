import { check, Update, DownloadEvent } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { useState, useEffect, useCallback, useRef } from 'react';

export interface UpdateStatus {
  available: boolean;
  version?: string;
  currentVersion?: string;
  body?: string;
  date?: string;
  downloading: boolean;
  progress: number;
  downloaded: number;
  total: number;
  error?: string;
  checking: boolean;
}

const initialStatus: UpdateStatus = {
  available: false,
  downloading: false,
  progress: 0,
  downloaded: 0,
  total: 0,
  checking: false,
};

export function useUpdater(checkOnMount = true) {
  const [status, setStatus] = useState<UpdateStatus>(initialStatus);
  const updateRef = useRef<Update | null>(null);

  const checkForUpdates = useCallback(async () => {
    setStatus(prev => ({ ...prev, checking: true, error: undefined }));

    try {
      const update = await check();

      if (update) {
        updateRef.current = update;
        setStatus({
          available: true,
          version: update.version,
          currentVersion: update.currentVersion,
          body: update.body ?? undefined,
          date: update.date ?? undefined,
          downloading: false,
          progress: 0,
          downloaded: 0,
          total: 0,
          checking: false,
        });
        return update;
      }

      setStatus({ ...initialStatus, checking: false });
      return null;
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Update check failed';
      setStatus({
        ...initialStatus,
        checking: false,
        error: errorMessage,
      });
      return null;
    }
  }, []);

  const downloadAndInstall = useCallback(async () => {
    const update = updateRef.current;
    if (!update) {
      // Try to check again if we don't have an update ref
      const freshUpdate = await check();
      if (!freshUpdate) return;
      updateRef.current = freshUpdate;
    }

    const currentUpdate = updateRef.current;
    if (!currentUpdate) return;

    setStatus(prev => ({
      ...prev,
      downloading: true,
      progress: 0,
      downloaded: 0,
      total: 0,
      error: undefined,
    }));

    try {
      await currentUpdate.downloadAndInstall((event: DownloadEvent) => {
        switch (event.event) {
          case 'Started':
            setStatus(prev => ({
              ...prev,
              total: event.data.contentLength ?? 0,
            }));
            break;
          case 'Progress':
            setStatus(prev => {
              const downloaded = prev.downloaded + event.data.chunkLength;
              const progress = prev.total > 0 ? (downloaded / prev.total) * 100 : 0;
              return {
                ...prev,
                downloaded,
                progress,
              };
            });
            break;
          case 'Finished':
            setStatus(prev => ({
              ...prev,
              progress: 100,
              downloading: false,
            }));
            break;
        }
      });

      // Relaunch the app after successful update
      await relaunch();
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Download failed';
      setStatus(prev => ({
        ...prev,
        downloading: false,
        error: errorMessage,
      }));
    }
  }, []);

  const dismissUpdate = useCallback(() => {
    setStatus(initialStatus);
    updateRef.current = null;
  }, []);

  useEffect(() => {
    if (checkOnMount) {
      // Small delay to let the app initialize
      const timer = setTimeout(() => {
        checkForUpdates();
      }, 3000);

      return () => clearTimeout(timer);
    }
  }, [checkOnMount, checkForUpdates]);

  return {
    ...status,
    checkForUpdates,
    downloadAndInstall,
    dismissUpdate,
  };
}
