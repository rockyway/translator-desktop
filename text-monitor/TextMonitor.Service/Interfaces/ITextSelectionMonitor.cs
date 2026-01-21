using TextMonitor.Service.Enums;
using TextMonitor.Service.Models;

namespace TextMonitor.Service.Interfaces;

/// <summary>
/// Main coordinator for text selection monitoring.
/// Integrates hooks, state machine, cursor tracking, and text retrieval.
/// </summary>
public interface ITextSelectionMonitor : IDisposable
{
    /// <summary>
    /// Event raised when text is selected with Ctrl key held.
    /// </summary>
    event EventHandler<TextSelectionEvent>? TextSelected;

    /// <summary>
    /// Event raised when a selection is detected.
    /// </summary>
    event EventHandler<TextSelectionEvent>? SelectionDetected;

    /// <summary>
    /// Gets whether monitoring is currently active.
    /// </summary>
    bool IsMonitoring { get; }

    /// <summary>
    /// Gets the current selection state.
    /// </summary>
    SelectionState CurrentState { get; }

    /// <summary>
    /// Starts monitoring for text selections (synchronous wrapper).
    /// </summary>
    void Start();

    /// <summary>
    /// Stops monitoring for text selections (synchronous wrapper).
    /// </summary>
    void Stop();

    /// <summary>
    /// Starts monitoring for text selections.
    /// </summary>
    Task<bool> StartMonitoringAsync();

    /// <summary>
    /// Stops monitoring for text selections.
    /// </summary>
    Task StopMonitoringAsync();
}
