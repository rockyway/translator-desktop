using System.Runtime.InteropServices;
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
    private const int ClipboardPollIntervalMs = 25;
    private const int ClipboardTimeoutMs = 300;
    private const int AltReleaseTimeoutMs = 2000;
    private const int VkMenu = 0x12;

    [DllImport("user32.dll")]
    private static extern short GetAsyncKeyState(int virtualKey);

    [DllImport("user32.dll")]
    private static extern uint GetClipboardSequenceNumber();

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

            // Capture changes while saving state and waiting for Alt release.
            var originalSequence = GetClipboardSequenceNumber();
            originalState = await SaveClipboardStateAsync();
            if (originalState == null)
            {
                return CreateFailureResult("Failed to save clipboard state", startTime);
            }

            // A physical Alt key would turn the simulated Ctrl+C into Ctrl+Alt+C.
            // Never synthesize an Alt release: wait for the user instead.
            var altWait = System.Diagnostics.Stopwatch.StartNew();
            while ((GetAsyncKeyState(VkMenu) & 0x8000) != 0)
            {
                if (altWait.ElapsedMilliseconds >= AltReleaseTimeoutMs)
                {
                    _logger.LogWarning("Clipboard copy cancelled: Alt was not released within {Timeout} ms", AltReleaseTimeoutMs);
                    return CreateFailureResult("Release Alt to copy the selected text", startTime);
                }

                await Task.Delay(ClipboardPollIntervalMs);
            }

            _logger.LogInformation("Clipboard copy: Alt released after {WaitMs} ms", altWait.ElapsedMilliseconds);

            // If the user or target app copied while we waited, use that fresh text
            // without overwriting their clipboard.
            if (GetClipboardSequenceNumber() != originalSequence)
            {
                var currentState = await SaveClipboardStateAsync();
                if (currentState?.ContainsText == true && !string.IsNullOrEmpty(currentState.Text))
                {
                    _logger.LogInformation("Clipboard changed during retrieval; using fresh text ({Length} chars)", currentState.Text.Length);
                    var earlyResult = TextRetrievalResult.CreateSuccess(currentState.Text, RetrievalMethod.ClipboardSimulation);
                    earlyResult.RetrievalDuration = DateTime.UtcNow - startTime;
                    return earlyResult;
                }
            }

            // Clear clipboard to ensure we're getting fresh data
            await RunOnStaThreadAsync(() => Clipboard.Clear());

            // Simulate Ctrl+C
            _logger.LogDebug("Sending Ctrl+C simulation");
            await RunOnStaThreadAsync(() => SendKeys.SendWait("^c"));

            // Poll immediately, returning as soon as the target app copies the text.
            string? selectedText = null;
            var copyWait = System.Diagnostics.Stopwatch.StartNew();
            var attempt = 0;
            while (true)
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

                }
                catch (Exception ex)
                {
                    _logger.LogWarning(ex, "Error reading clipboard, attempt {Attempt}", attempt + 1);
                }

                if (copyWait.ElapsedMilliseconds >= ClipboardTimeoutMs)
                    break;

                attempt++;
                await Task.Delay(ClipboardPollIntervalMs);
            }
            _logger.LogInformation("Clipboard copy completed in {WaitMs} ms; text found: {Found}", copyWait.ElapsedMilliseconds, !string.IsNullOrEmpty(selectedText));

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
