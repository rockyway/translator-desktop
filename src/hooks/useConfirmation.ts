import { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';

interface ConfirmationPayload {
  charCount: number;
}

export const useConfirmation = () => {
  const [isOpen, setIsOpen] = useState(false);
  const [charCount, setCharCount] = useState(0);

  useEffect(() => {
    const unlisten = listen<ConfirmationPayload>('show-confirmation', (event) => {
      setCharCount(event.payload.charCount);
      setIsOpen(true);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const handleClose = () => {
    setIsOpen(false);
  };

  return { isOpen, charCount, handleClose };
};
