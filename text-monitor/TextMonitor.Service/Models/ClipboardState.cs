namespace TextMonitor.Service.Models;

/// <summary>
/// Represents saved clipboard state.
/// </summary>
public class ClipboardState
{
    /// <summary>
    /// Text content from clipboard (if any).
    /// </summary>
    public string? Text { get; set; }

    /// <summary>
    /// Whether clipboard contained text.
    /// </summary>
    public bool ContainsText { get; set; }

    /// <summary>
    /// Whether clipboard contained other data formats.
    /// </summary>
    public bool ContainsOtherFormats { get; set; }

    /// <summary>
    /// Timestamp when state was saved.
    /// </summary>
    public DateTime SavedAt { get; set; }
}
