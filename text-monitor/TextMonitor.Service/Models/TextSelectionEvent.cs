using TextMonitor.Service.Enums;

namespace TextMonitor.Service.Models;

/// <summary>
/// Event data for text selection detection.
/// </summary>
public class TextSelectionEvent
{
    /// <summary>
    /// Unique identifier for this selection event.
    /// </summary>
    public Guid EventId { get; set; } = Guid.NewGuid();

    /// <summary>
    /// Coordinates and bounds of the selection.
    /// Nullable for reactive path (late Ctrl detection) where coordinates aren't tracked.
    /// </summary>
    public SelectionCoordinates? Coordinates { get; set; }

    /// <summary>
    /// Result of text retrieval operation.
    /// </summary>
    public TextRetrievalResult RetrievalResult { get; set; } = new();

    /// <summary>
    /// Current state of the selection.
    /// </summary>
    public SelectionState State { get; set; }

    /// <summary>
    /// Timestamp when this event was created.
    /// </summary>
    public DateTime EventTime { get; set; } = DateTime.UtcNow;

    /// <summary>
    /// Whether this event should be processed (false if cancelled or invalid).
    /// </summary>
    public bool ShouldProcess { get; set; } = true;

    /// <summary>
    /// Reason for cancellation if ShouldProcess is false.
    /// </summary>
    public string? CancellationReason { get; set; }

    /// <summary>
    /// Detection path used: "Proactive" (Ctrl at press) or "Reactive" (Ctrl at release).
    /// Used for metrics and monitoring to track which path is being utilized.
    /// </summary>
    public string? DetectionPath { get; set; }
}
