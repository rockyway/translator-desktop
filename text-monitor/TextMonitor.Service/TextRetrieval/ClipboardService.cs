using System.Windows.Forms;
using Microsoft.Extensions.Logging;
using TextMonitor.Service.Enums;
using TextMonitor.Service.Interfaces;
using TextMonitor.Service.Models;

namespace TextMonitor.Service.TextRetrieval;

/// <summary>
/// Implementation of clipboard-based text retrieval with state preservation.
/// Uses Ctrl+C simulation to retrieve selected text.
/// </summary>
public class ClipboardService : IClipboardService
{
    private readonly ILogger<ClipboardService> _logger;
    private const int ClipboardRetryAttempts = 3;
    private const int ClipboardRetryDelayMs = 50;

    public ClipboardService(ILogger<ClipboardService> logger)
    {
        _logger = logger ?? throw new ArgumentNullException(nameof(logger));
    }

    /// <summary>
    /// Runs a function on an STA thread (required for clipboard operations).
    /// </summary>
    private async Task<T> RunOnStaThreadAsync<T>(Func<T> function)
    {
        var tcs = new TaskCompletionSource<T>();
        var thread = new Thread(() =>
        {
            try
            {
                var result = function();
                tcs.SetResult(result);
            }
            catch (Exception ex)
            {
                tcs.SetException(ex);
            }
        });

        thread.SetApartmentState(ApartmentState.STA);
        thread.IsBackground = true; // CRITICAL: Prevents process from hanging on exit
        thread.Start();
        await Task.Run(() => thread.Join()); // Wait for thread to complete

        return await tcs.Task;
    }

    /// <summary>
    /// Runs an action on an STA thread (required for clipboard operations).
    /// </summary>
    private async Task RunOnStaThreadAsync(Action action)
    {
        var tcs = new TaskCompletionSource<bool>();
        var thread = new Thread(() =>
        {
            try
            {
                action();
                tcs.SetResult(true);
            }
            catch (Exception ex)
            {
                tcs.SetException(ex);
            }
        });

        thread.SetApartmentState(ApartmentState.STA);
        thread.IsBackground = true; // CRITICAL: Prevents process from hanging on exit
        thread.Start();
        await Task.Run(() => thread.Join()); // Wait for thread to complete

        await tcs.Task;
    }

    /// <summary>
    /// Checks if clipboard operations are available.
    /// </summary>
    public bool IsAvailable()
    {
        try
        {
            // Try a simple clipboard check
            return Clipboard.ContainsText() || true; // Even if empty, clipboard is available
        }
        catch (Exception ex)
        {
            _logger.LogWarning(ex, "Clipboard is not available");
            return false;
        }
    }

    /// <summary>
    /// Saves the current clipboard state.
    /// </summary>
    public async Task<ClipboardState?> SaveClipboardStateAsync()
    {
        return await RunOnStaThreadAsync(() =>
        {
            try
            {
                var state = new ClipboardState
                {
                    SavedAt = DateTime.UtcNow,
                    ContainsText = Clipboard.ContainsText()
                };

                if (state.ContainsText)
                {
                    // Use UnicodeText format explicitly to preserve all Unicode characters
                    state.Text = Clipboard.GetText(TextDataFormat.UnicodeText);
                    _logger.LogDebug("Saved clipboard text: {Length} characters", state.Text.Length);
                }

                // Check for other formats
                var dataObject = Clipboard.GetDataObject();
                state.ContainsOtherFormats = dataObject?.GetFormats()?.Length > (state.ContainsText ? 1 : 0);

                return state;
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error saving clipboard state");
                return null;
            }
        });
    }

    /// <summary>
    /// Restores a previously saved clipboard state.
    /// </summary>
    public async Task RestoreClipboardStateAsync(ClipboardState state)
    {
        if (state == null)
        {
            _logger.LogWarning("Cannot restore null clipboard state");
            return;
        }

        await RunOnStaThreadAsync(() =>
        {
            try
            {
                if (state.ContainsText && !string.IsNullOrEmpty(state.Text))
                {
                    // Use UnicodeText format explicitly to preserve all Unicode characters
                    Clipboard.SetText(state.Text, TextDataFormat.UnicodeText);
                    _logger.LogDebug("Restored clipboard text: {Length} characters", state.Text.Length);
                }
                else
                {
                    Clipboard.Clear();
                    _logger.LogDebug("Cleared clipboard (was empty)");
                }
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error restoring clipboard state");
            }
        });
    }

    /// <summary>
    /// Retrieves selected text using clipboard simulation with Ctrl+C.
    /// </summary>
    public async Task<TextRetrievalResult> GetSelectedTextViaClipboardAsync()
    {
        var startTime = DateTime.UtcNow;
        ClipboardState? originalState = null;

        try
        {
            _logger.LogDebug("Attempting clipboard-based text retrieval");

            // Save original clipboard state
            originalState = await SaveClipboardStateAsync();
            if (originalState == null)
            {
                return CreateFailureResult("Failed to save clipboard state", startTime);
            }

            // Give user's Ctrl+C operation time to complete
            _logger.LogDebug("Waiting for potential user clipboard operation to complete");
            await Task.Delay(200);

            // Check if clipboard was updated by user
            var currentClipboardState = await SaveClipboardStateAsync();
            if (currentClipboardState != null &&
                currentClipboardState.ContainsText &&
                !string.IsNullOrEmpty(currentClipboardState.Text))
            {
                // Check if clipboard changed since we started
                if (!originalState.ContainsText ||
                    originalState.Text != currentClipboardState.Text)
                {
                    _logger.LogInformation(
                        "Clipboard already updated by user or application, using existing content ({Length} chars)",
                        currentClipboardState.Text.Length);

                    var earlyResult = TextRetrievalResult.CreateSuccess(
                        currentClipboardState.Text,
                        RetrievalMethod.ClipboardSimulation);
                    earlyResult.RetrievalDuration = DateTime.UtcNow - startTime;

                    // Restore original clipboard if it was different
                    if (originalState.ContainsText && originalState.Text != currentClipboardState.Text)
                    {
                        await RestoreClipboardStateAsync(originalState);
                    }

                    return earlyResult;
                }
            }

            // Clipboard wasn't updated by user - proceed with simulation
            _logger.LogDebug("No user clipboard operation detected, proceeding with Ctrl+C simulation");

            // Clear clipboard to ensure we're getting fresh data
            await RunOnStaThreadAsync(() => Clipboard.Clear());
            await Task.Delay(ClipboardRetryDelayMs);

            // Simulate Ctrl+C
            _logger.LogDebug("Sending Ctrl+C simulation");
            await RunOnStaThreadAsync(() => SendKeys.SendWait("^c"));

            // Wait for clipboard to be populated
            await Task.Delay(ClipboardRetryDelayMs * 2);

            // Try to retrieve text from clipboard with retries
            string? selectedText = null;

            for (int attempt = 0; attempt < ClipboardRetryAttempts; attempt++)
            {
                try
                {
                    var clipboardData = await RunOnStaThreadAsync(() =>
                    {
                        if (Clipboard.ContainsText())
                        {
                            // Use UnicodeText format explicitly
                            var text = Clipboard.GetText(TextDataFormat.UnicodeText);
                            if (!string.IsNullOrEmpty(text))
                            {
                                return text;
                            }
                        }
                        return (string?)null;
                    });

                    if (!string.IsNullOrEmpty(clipboardData))
                    {
                        selectedText = clipboardData;
                        break;
                    }

                    if (attempt < ClipboardRetryAttempts - 1)
                    {
                        _logger.LogDebug("Clipboard empty, retry attempt {Attempt}", attempt + 1);
                        await Task.Delay(ClipboardRetryDelayMs);
                    }
                }
                catch (Exception ex)
                {
                    _logger.LogWarning(ex, "Error reading clipboard, attempt {Attempt}", attempt + 1);
                    if (attempt < ClipboardRetryAttempts - 1)
                    {
                        await Task.Delay(ClipboardRetryDelayMs);
                    }
                }
            }

            // Restore original clipboard
            if (originalState != null)
            {
                await RestoreClipboardStateAsync(originalState);
            }

            // Check if we got text
            if (string.IsNullOrEmpty(selectedText))
            {
                _logger.LogWarning("No text retrieved via clipboard");
                return CreateFailureResult("No text in clipboard after Ctrl+C", startTime);
            }

            // Create success result
            var result = TextRetrievalResult.CreateSuccess(selectedText, RetrievalMethod.ClipboardSimulation);
            result.RetrievalDuration = DateTime.UtcNow - startTime;

            _logger.LogInformation("Successfully retrieved {Length} characters via clipboard",
                selectedText.Length);

            return result;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Unexpected error during clipboard text retrieval");

            // Attempt to restore clipboard even on failure
            try
            {
                if (originalState != null)
                {
                    await RestoreClipboardStateAsync(originalState);
                }
            }
            catch (Exception restoreEx)
            {
                _logger.LogError(restoreEx, "Failed to restore clipboard after error");
            }

            return CreateFailureResult($"Unexpected error: {ex.Message}", startTime);
        }
    }

    private TextRetrievalResult CreateFailureResult(string errorMessage, DateTime startTime)
    {
        var result = TextRetrievalResult.CreateFailure(errorMessage, RetrievalMethod.ClipboardSimulation);
        result.RetrievalDuration = DateTime.UtcNow - startTime;
        return result;
    }
}
