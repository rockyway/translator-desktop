using Microsoft.Extensions.Logging;
using TextMonitor.Service.Models;

namespace TextMonitor.Service.Monitoring;

/// <summary>
/// Tracks cursor position during text selection.
/// Supports multi-monitor setups and calculates optimal context menu positions.
/// </summary>
public class CursorTracker
{
    private readonly ILogger<CursorTracker> _logger;
    private SelectionCoordinates? _currentSelection;
    private MouseEventData? _lastMousePosition;

    public CursorTracker(ILogger<CursorTracker> logger)
    {
        _logger = logger ?? throw new ArgumentNullException(nameof(logger));
    }

    /// <summary>
    /// Gets the current selection coordinates.
    /// </summary>
    public SelectionCoordinates? CurrentSelection => _currentSelection;

    /// <summary>
    /// Gets the last known mouse position.
    /// </summary>
    public MouseEventData? LastMousePosition => _lastMousePosition;

    /// <summary>
    /// Starts tracking a new selection.
    /// </summary>
    public void StartSelection(MouseEventData startPosition)
    {
        _logger.LogDebug("Starting selection at ({X}, {Y})", startPosition.X, startPosition.Y);

        _currentSelection = new SelectionCoordinates
        {
            StartX = startPosition.X,
            StartY = startPosition.Y,
            EndX = startPosition.X,
            EndY = startPosition.Y,
            SelectionStartTime = startPosition.Timestamp
        };

        _lastMousePosition = startPosition;
    }

    /// <summary>
    /// Updates selection with current mouse position.
    /// </summary>
    public void UpdateSelection(MouseEventData currentPosition)
    {
        if (_currentSelection == null)
        {
            _logger.LogWarning("Cannot update selection: no active selection");
            return;
        }

        _currentSelection.EndX = currentPosition.X;
        _currentSelection.EndY = currentPosition.Y;
        _lastMousePosition = currentPosition;

        _logger.LogTrace("Selection updated: ({StartX}, {StartY}) to ({EndX}, {EndY})",
            _currentSelection.StartX, _currentSelection.StartY,
            _currentSelection.EndX, _currentSelection.EndY);
    }

    /// <summary>
    /// Ends the current selection.
    /// </summary>
    public SelectionCoordinates? EndSelection(MouseEventData endPosition)
    {
        if (_currentSelection == null)
        {
            _logger.LogWarning("Cannot end selection: no active selection");
            return null;
        }

        _currentSelection.EndX = endPosition.X;
        _currentSelection.EndY = endPosition.Y;
        _currentSelection.SelectionEndTime = endPosition.Timestamp;
        _lastMousePosition = endPosition;

        _logger.LogDebug("Selection ended: Size={Width}x{Height}, Duration={Duration}ms",
            _currentSelection.Width, _currentSelection.Height,
            _currentSelection.SelectionDuration.TotalMilliseconds);

        var completedSelection = _currentSelection;
        _currentSelection = null;

        return completedSelection;
    }

    /// <summary>
    /// Cancels the current selection.
    /// </summary>
    public void CancelSelection()
    {
        if (_currentSelection != null)
        {
            _logger.LogDebug("Selection cancelled");
            _currentSelection = null;
        }
    }

    /// <summary>
    /// Checks if there's an active selection in progress.
    /// </summary>
    public bool IsSelecting => _currentSelection != null;

    /// <summary>
    /// Gets the optimal position for a context menu based on the selection.
    /// </summary>
    public (int X, int Y) GetOptimalContextMenuPosition()
    {
        if (_lastMousePosition == null)
        {
            _logger.LogWarning("No mouse position available for context menu");
            return (0, 0);
        }

        // Position the context menu slightly offset from the cursor
        const int offsetX = 10;
        const int offsetY = 10;

        var menuX = _lastMousePosition.X + offsetX;
        var menuY = _lastMousePosition.Y + offsetY;

        _logger.LogDebug("Context menu position: ({X}, {Y})", menuX, menuY);
        return (menuX, menuY);
    }

    /// <summary>
    /// Checks if the given selection is significant enough to process.
    /// Filters out accidental clicks.
    /// </summary>
    /// <param name="coordinates">The selection coordinates to check</param>
    /// <param name="minimumPixelMovement">Minimum pixel distance to be considered significant</param>
    /// <returns>True if the selection is significant, false otherwise</returns>
    public bool IsSignificantSelection(SelectionCoordinates? coordinates, int minimumPixelMovement = 5)
    {
        if (coordinates == null)
        {
            _logger.LogDebug("Selection significance check: Coordinates are null");
            return false;
        }

        var distance = Math.Sqrt(
            Math.Pow(coordinates.Width, 2) +
            Math.Pow(coordinates.Height, 2));

        var isSignificant = distance >= minimumPixelMovement;

        _logger.LogDebug("Selection significance check: Size={Width}x{Height}, Distance={Distance:F2}, Threshold={Threshold}, Significant={IsSignificant}",
            coordinates.Width, coordinates.Height, distance, minimumPixelMovement, isSignificant);

        return isSignificant;
    }
}
