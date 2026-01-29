import React from 'react';
import { invoke } from '@tauri-apps/api/core';
import { FiAlertCircle, FiCheck, FiX } from 'react-icons/fi';

interface ConfirmationDialogProps {
  isOpen: boolean;
  charCount: number;
  onClose: () => void;
}

export const ConfirmationDialog: React.FC<ConfirmationDialogProps> = ({
  isOpen,
  charCount,
  onClose,
}) => {
  if (!isOpen) return null;

  const handleConfirm = async () => {
    await invoke('respond_to_confirmation', { confirmed: true });
    onClose();
  };

  const handleCancel = async () => {
    await invoke('respond_to_confirmation', { confirmed: false });
    onClose();
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow-xl max-w-md w-full mx-4 p-6 border border-gray-200 dark:border-gray-700">
        <div className="flex items-start gap-4">
          <div className="flex-shrink-0">
            <FiAlertCircle className="w-6 h-6 text-amber-500" />
          </div>
          <div className="flex-1">
            <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-2">
              Confirm Translation
            </h3>
            <p className="text-sm text-gray-600 dark:text-gray-400">
              Selected text is <strong>{charCount}</strong> characters long.
            </p>
            <p className="text-sm text-gray-600 dark:text-gray-400 mt-2">
              Do you want to translate it?
            </p>
          </div>
        </div>

        <div className="flex gap-3 mt-6">
          <button
            onClick={handleCancel}
            className="flex-1 px-4 py-2.5 rounded-lg font-medium
              bg-gray-100 dark:bg-gray-700
              text-gray-700 dark:text-gray-300
              hover:bg-gray-200 dark:hover:bg-gray-600
              transition-colors flex items-center justify-center gap-2"
          >
            <FiX className="w-4 h-4" />
            Cancel
          </button>
          <button
            onClick={handleConfirm}
            autoFocus
            className="flex-1 px-4 py-2.5 rounded-lg font-medium
              bg-amber-500 hover:bg-amber-600
              text-white transition-colors
              flex items-center justify-center gap-2"
          >
            <FiCheck className="w-4 h-4" />
            Translate
          </button>
        </div>
      </div>
    </div>
  );
};
