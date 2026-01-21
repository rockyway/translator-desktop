namespace TextMonitor.Service.Models;

/// <summary>
/// Information about a focused UI element.
/// </summary>
public class FocusedElementInfo
{
    /// <summary>
    /// Name of the element.
    /// </summary>
    public string? Name { get; set; }

    /// <summary>
    /// Type of the element (e.g., "Edit", "Document", "Text").
    /// </summary>
    public string? ControlType { get; set; }

    /// <summary>
    /// Name of the application/process.
    /// </summary>
    public string? ProcessName { get; set; }

    /// <summary>
    /// Process ID.
    /// </summary>
    public int ProcessId { get; set; }

    /// <summary>
    /// Whether the element is a password field.
    /// </summary>
    public bool IsPassword { get; set; }

    /// <summary>
    /// Whether the element supports text selection.
    /// </summary>
    public bool SupportsTextPattern { get; set; }

    /// <summary>
    /// Bounding rectangle of the element.
    /// </summary>
    public System.Drawing.Rectangle Bounds { get; set; }
}
