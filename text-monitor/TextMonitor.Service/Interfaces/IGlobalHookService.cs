using TextMonitor.Service.Models;

namespace TextMonitor.Service.Interfaces;

/// <summary>
/// Platform-agnostic global mouse and keyboard hook service.
/// Monitors system-wide mouse and keyboard events even when application is not focused.
/// </summary>
public interface IGlobalHookService : IDisposable
{
    /// <summary>
    /// Event raised when a mouse button is pressed.
    /// </summary>
    event EventHandler<MouseEventData>? MousePressed;

    /// <summary>
    /// Event raised when a mouse button is released.
    /// </summary>
    event EventHandler<MouseEventData>? MouseReleased;

    /// <summary>
    /// Event raised when the mouse is moved.
    /// Warning: This event fires very frequently. Use sparingly.
    /// </summary>
    event EventHandler<MouseEventData>? MouseMoved;

    /// <summary>
    /// Gets whether the hook service is currently running and monitoring events.
    /// </summary>
    bool IsRunning { get; }

    /// <summary>
    /// Gets the last error that occurred during hook operations.
    /// Null if no errors have occurred or if last operation succeeded.
    /// </summary>
    string? LastError { get; }

    /// <summary>
    /// Starts the global hook monitoring.
    /// </summary>
    /// <returns>True if hooks started successfully; false if initialization failed.</returns>
    Task<bool> StartAsync();

    /// <summary>
    /// Stops the global hook monitoring and releases system resources.
    /// </summary>
    Task StopAsync();
}
