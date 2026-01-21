using TextMonitor.Service.Models;

namespace TextMonitor.Service.Interfaces;

/// <summary>
/// Service for clipboard-based text retrieval with state preservation.
/// </summary>
public interface IClipboardService
{
    /// <summary>
    /// Retrieves selected text using clipboard simulation (Ctrl+C).
    /// Preserves and restores the original clipboard content.
    /// </summary>
    /// <returns>Text retrieval result.</returns>
    Task<TextRetrievalResult> GetSelectedTextViaClipboardAsync();

    /// <summary>
    /// Saves the current clipboard state.
    /// </summary>
    /// <returns>Handle to saved clipboard data or null if clipboard is empty.</returns>
    Task<ClipboardState?> SaveClipboardStateAsync();

    /// <summary>
    /// Restores a previously saved clipboard state.
    /// </summary>
    /// <param name="state">The saved clipboard state.</param>
    Task RestoreClipboardStateAsync(ClipboardState state);

    /// <summary>
    /// Checks if clipboard operations are available.
    /// </summary>
    bool IsAvailable();
}
