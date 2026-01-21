namespace TextMonitor.Service.Models;

/// <summary>
/// Wrapper for mouse event data from SharpHook.
/// </summary>
public class MouseEventData
{
    /// <summary>
    /// Mouse button that triggered the event (1=Left, 2=Right, 3=Middle).
    /// </summary>
    public ushort Button { get; set; }

    /// <summary>
    /// X coordinate in screen space.
    /// </summary>
    public short X { get; set; }

    /// <summary>
    /// Y coordinate in screen space.
    /// </summary>
    public short Y { get; set; }

    /// <summary>
    /// Timestamp when event occurred.
    /// </summary>
    public DateTime Timestamp { get; set; }

    /// <summary>
    /// Whether left mouse button is pressed.
    /// </summary>
    public bool IsLeftButton => Button == 1;

    /// <summary>
    /// Whether right mouse button is pressed.
    /// </summary>
    public bool IsRightButton => Button == 2;

    /// <summary>
    /// Whether middle mouse button is pressed.
    /// </summary>
    public bool IsMiddleButton => Button == 3;
}
