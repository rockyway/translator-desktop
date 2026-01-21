namespace TextMonitor.Service.Enums;

/// <summary>
/// Represents the current state of text selection monitoring.
/// </summary>
public enum SelectionState
{
    /// <summary>
    /// No selection activity. Waiting for user to start selecting text.
    /// </summary>
    Idle,

    /// <summary>
    /// User is actively dragging mouse to select text.
    /// </summary>
    Selecting,

    /// <summary>
    /// Selection complete. Text retrieval in progress.
    /// </summary>
    Selected,

    /// <summary>
    /// Text retrieval failed or was cancelled.
    /// </summary>
    Failed
}
