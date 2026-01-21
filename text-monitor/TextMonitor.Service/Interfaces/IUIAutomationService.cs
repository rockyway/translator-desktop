using TextMonitor.Service.Models;

namespace TextMonitor.Service.Interfaces;

/// <summary>
/// Service for interacting with Windows UI Automation API to retrieve selected text.
/// </summary>
public interface IUIAutomationService
{
    /// <summary>
    /// Attempts to retrieve selected text from the currently focused element.
    /// </summary>
    /// <returns>Text retrieval result with metadata.</returns>
    Task<TextRetrievalResult> GetSelectedTextAsync();

    /// <summary>
    /// Gets information about the currently focused UI element.
    /// </summary>
    /// <returns>Element information or null if no element is focused.</returns>
    Task<FocusedElementInfo?> GetFocusedElementInfoAsync();

    /// <summary>
    /// Checks if UI Automation is available and accessible.
    /// </summary>
    /// <returns>True if UI Automation can be used, false otherwise.</returns>
    bool IsAvailable();
}
