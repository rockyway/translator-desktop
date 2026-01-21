namespace TextMonitor.Service.Models;

/// <summary>
/// Represents the screen coordinates and bounds of a text selection.
/// </summary>
public class SelectionCoordinates
{
    /// <summary>
    /// X coordinate where selection started (screen coordinates).
    /// </summary>
    public int StartX { get; set; }

    /// <summary>
    /// Y coordinate where selection started (screen coordinates).
    /// </summary>
    public int StartY { get; set; }

    /// <summary>
    /// X coordinate where selection ended (screen coordinates).
    /// </summary>
    public int EndX { get; set; }

    /// <summary>
    /// Y coordinate where selection ended (screen coordinates).
    /// </summary>
    public int EndY { get; set; }

    /// <summary>
    /// Width of the selection bounding box.
    /// </summary>
    public int Width => Math.Abs(EndX - StartX);

    /// <summary>
    /// Height of the selection bounding box.
    /// </summary>
    public int Height => Math.Abs(EndY - StartY);

    /// <summary>
    /// Top-left X coordinate of bounding box.
    /// </summary>
    public int Left => Math.Min(StartX, EndX);

    /// <summary>
    /// Top-left Y coordinate of bounding box.
    /// </summary>
    public int Top => Math.Min(StartY, EndY);

    /// <summary>
    /// Bottom-right X coordinate of bounding box.
    /// </summary>
    public int Right => Math.Max(StartX, EndX);

    /// <summary>
    /// Bottom-right Y coordinate of bounding box.
    /// </summary>
    public int Bottom => Math.Max(StartY, EndY);

    /// <summary>
    /// Timestamp when selection started.
    /// </summary>
    public DateTime SelectionStartTime { get; set; }

    /// <summary>
    /// Timestamp when selection completed.
    /// </summary>
    public DateTime SelectionEndTime { get; set; }

    /// <summary>
    /// Duration of selection drag operation.
    /// </summary>
    public TimeSpan SelectionDuration => SelectionEndTime - SelectionStartTime;
}
