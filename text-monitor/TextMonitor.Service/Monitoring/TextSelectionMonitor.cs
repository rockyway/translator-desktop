using System.Runtime.InteropServices;
using Microsoft.Extensions.Logging;
using TextMonitor.Service.Enums;
using TextMonitor.Service.Interfaces;
using TextMonitor.Service.Models;

namespace TextMonitor.Service.Monitoring;

/// <summary>
/// Main coordinator for text selection monitoring.
/// Integrates hooks, state machine, cursor tracking, and text retrieval.
/// </summary>
public class TextSelectionMonitor : ITextSelectionMonitor, IDisposable
{
    private readonly IGlobalHookService _hookService;
    private readonly ITextRetrievalService _retrievalService;
    private readonly ISelectionEventAggregator _eventAggregator;
    private readonly SelectionStateMachine _stateMachine;
    private readonly CursorTracker _cursorTracker;
    private readonly ILogger<TextSelectionMonitor> _logger;

    private bool _isMonitoring;
    private bool _disposed;
    private bool _isLeftButtonPressed;
    private volatile int _activeModifierKey = VK_MENU;  // Default to Alt, thread-safe

    private const int MinimumSelectionPixels = 5;
    private const int MinimumTextLengthForReactive = 5; // Minimum characters for reactive path
    private const int TextRetrievalDelayMs = 100;
    private const int VK_CONTROL = 0x11; // Virtual key code for Ctrl
    private const int VK_SHIFT = 0x10;   // Virtual key code for Shift
    private const int VK_MENU = 0x12;    // Virtual key code for Alt

    private static readonly Dictionary<string, int> ModifierKeyMap = new()
    {
        { "ctrl", VK_CONTROL },
        { "shift", VK_SHIFT },
        { "alt", VK_MENU }
    };

    // P/Invoke for Windows API keyboard state check
    [DllImport("user32.dll")]
    private static extern short GetAsyncKeyState(int vKey);

    public event EventHandler<TextSelectionEvent>? TextSelected;
    public event EventHandler<TextSelectionEvent>? SelectionDetected;

    public bool IsMonitoring => _isMonitoring;
    public SelectionState CurrentState => _stateMachine.CurrentState;

    /// <summary>
    /// Updates the selection modifier key at runtime.
    /// Thread-safe - can be called from configuration receiver.
    /// </summary>
    /// <param name="modifier">The modifier key name: "ctrl", "shift", or "alt"</param>
    public void SetSelectionModifier(string modifier)
    {
        if (ModifierKeyMap.TryGetValue(modifier.ToLowerInvariant(), out var vkCode))
        {
            _activeModifierKey = vkCode;
            _logger.LogInformation("Selection modifier updated to: {Modifier} (VK: 0x{VkCode:X2})",
                modifier, vkCode);
        }
        else
        {
            _logger.LogWarning("Unknown modifier: {Modifier}. Valid values: ctrl, shift, alt", modifier);
        }
    }

    public TextSelectionMonitor(
        IGlobalHookService hookService,
        ITextRetrievalService retrievalService,
        ISelectionEventAggregator eventAggregator,
        ILogger<TextSelectionMonitor> logger,
        ILogger<SelectionStateMachine> stateMachineLogger,
        ILogger<CursorTracker> cursorTrackerLogger)
    {
        _hookService = hookService ?? throw new ArgumentNullException(nameof(hookService));
        _retrievalService = retrievalService ?? throw new ArgumentNullException(nameof(retrievalService));
        _eventAggregator = eventAggregator ?? throw new ArgumentNullException(nameof(eventAggregator));
        _logger = logger ?? throw new ArgumentNullException(nameof(logger));

        _stateMachine = new SelectionStateMachine(stateMachineLogger);
        _cursorTracker = new CursorTracker(cursorTrackerLogger);

        // Subscribe to hook events
        _hookService.MousePressed += OnMousePressed;
        _hookService.MouseReleased += OnMouseReleased;
        _hookService.MouseMoved += OnMouseMoved;

        _logger.LogInformation("TextSelectionMonitor initialized");
    }

    /// <summary>
    /// Starts monitoring for text selections (synchronous wrapper).
    /// </summary>
    public void Start()
    {
        _ = StartMonitoringAsync();
    }

    /// <summary>
    /// Stops monitoring for text selections (synchronous wrapper).
    /// </summary>
    public void Stop()
    {
        _ = StopMonitoringAsync();
    }

    /// <summary>
    /// Starts monitoring for text selections.
    /// </summary>
    public async Task<bool> StartMonitoringAsync()
    {
        if (_disposed)
        {
            _logger.LogError("Cannot start monitoring: monitor has been disposed");
            return false;
        }

        if (_isMonitoring)
        {
            _logger.LogWarning("Monitoring already active");
            return true;
        }

        _logger.LogInformation("Starting text selection monitoring...");

        var started = await _hookService.StartAsync();
        if (!started)
        {
            _logger.LogError("Failed to start hook service: {Error}", _hookService.LastError);
            return false;
        }

        _isMonitoring = true;
        _stateMachine.Reset("Monitoring started");

        _logger.LogInformation("Text selection monitoring started successfully");
        return true;
    }

    /// <summary>
    /// Stops monitoring for text selections.
    /// </summary>
    public async Task StopMonitoringAsync()
    {
        if (!_isMonitoring)
        {
            _logger.LogWarning("Monitoring not active");
            return;
        }

        _logger.LogInformation("Stopping text selection monitoring...");

        await _hookService.StopAsync();
        _isMonitoring = false;
        _stateMachine.Reset("Monitoring stopped");
        _cursorTracker.CancelSelection();

        _logger.LogInformation("Text selection monitoring stopped");
    }

    /// <summary>
    /// Handles mouse pressed events.
    /// </summary>
    private void OnMousePressed(object? sender, MouseEventData e)
    {
        if (!_isMonitoring) return;

        try
        {
            if (e.IsLeftButton)
            {
                // PERFORMANCE OPTIMIZATION: Check modifier key BEFORE starting any tracking
                // This prevents processing mouse movements for normal selections (95%+ of cases)
                // Only track when user signals intent by holding the configured modifier key
                if (!IsModifierKeyPressed())
                {
                    return; // Don't start tracking - eliminates lag completely
                }

                // Modifier key is pressed - user wants to use the assistant
                _isLeftButtonPressed = true;
                _cursorTracker.StartSelection(e);
                _stateMachine.TransitionTo(SelectionState.Selecting, "Modifier+Left mouse button pressed");

                _logger.LogDebug("Selection started at ({X}, {Y}) with modifier key held", e.X, e.Y);
            }
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error handling mouse pressed event");
        }
    }

    /// <summary>
    /// Handles mouse moved events during selection.
    /// </summary>
    private void OnMouseMoved(object? sender, MouseEventData e)
    {
        if (!_isMonitoring || !_isLeftButtonPressed) return;

        try
        {
            if (_stateMachine.IsSelecting)
            {
                _cursorTracker.UpdateSelection(e);
            }
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error handling mouse moved event");
        }
    }

    /// <summary>
    /// Checks if the configured modifier key is currently pressed.
    /// </summary>
    private bool IsModifierKeyPressed()
    {
        return (GetAsyncKeyState(_activeModifierKey) & 0x8000) != 0;
    }

    /// <summary>
    /// Handles mouse released events with hybrid modifier detection.
    /// Path 1 (Proactive): Modifier key held from start - optimal with full tracking.
    /// Path 2 (Reactive): Modifier key pressed before release - fallback for forgiveness.
    /// </summary>
    private async void OnMouseReleased(object? sender, MouseEventData e)
    {
        if (!_isMonitoring) return;

        try
        {
            if (e.IsLeftButton)
            {
                // PATH 1: PROACTIVE TRACKING (modifier key held from start)
                if (_isLeftButtonPressed)
                {
                    _isLeftButtonPressed = false;

                    if (!_stateMachine.IsSelecting)
                    {
                        _logger.LogDebug("Mouse released but not in selecting state");
                        return;
                    }

                    var coordinates = _cursorTracker.EndSelection(e);
                    if (coordinates == null)
                    {
                        _logger.LogWarning("No coordinates available after selection");
                        _stateMachine.Reset("No coordinates");
                        return;
                    }

                    // Pixel-based significance check
                    if (!_cursorTracker.IsSignificantSelection(coordinates, MinimumSelectionPixels))
                    {
                        _logger.LogDebug("Selection too small, treating as click (not drag)");
                        _stateMachine.Reset("Insignificant selection");
                        return;
                    }

                    _logger.LogInformation("Significant selection detected (proactive path), retrieving text...");
                    _stateMachine.TransitionTo(SelectionState.Selected, "Mouse released after modifier+drag");

                    await Task.Delay(TextRetrievalDelayMs);

                    var retrievalResult = await _retrievalService.RetrieveSelectedTextAsync();

                    var selectionEvent = new TextSelectionEvent
                    {
                        Coordinates = coordinates,
                        RetrievalResult = retrievalResult,
                        State = retrievalResult.Success ? SelectionState.Selected : SelectionState.Failed,
                        DetectionPath = "Proactive"
                    };

                    if (retrievalResult.Success)
                    {
                        _logger.LogInformation(
                            "Text retrieved successfully via proactive path: {Length} characters via {Method}",
                            retrievalResult.Text.Length,
                            retrievalResult.Method);
                    }
                    else
                    {
                        _logger.LogWarning("Text retrieval failed: {Error}", retrievalResult.ErrorMessage);
                        _stateMachine.TransitionTo(SelectionState.Failed, retrievalResult.ErrorMessage);
                    }

                    _eventAggregator.Publish(selectionEvent);
                    TextSelected?.Invoke(this, selectionEvent);
                    SelectionDetected?.Invoke(this, selectionEvent);

                    _stateMachine.Reset("Selection processing complete");
                    return; // Exit - don't execute Path 2
                }

                // PATH 2: REACTIVE LATE-MODIFIER DETECTION (Fallback)
                if (IsModifierKeyPressed())
                {
                    _logger.LogDebug("Late modifier key detection at mouse release - attempting text retrieval");

                    await Task.Delay(TextRetrievalDelayMs);

                    var retrievalResult = await _retrievalService.RetrieveSelectedTextAsync();

                    // Text-length based significance check (no pixel data available)
                    if (retrievalResult.Success &&
                        !string.IsNullOrWhiteSpace(retrievalResult.Text) &&
                        retrievalResult.Text.Length >= MinimumTextLengthForReactive)
                    {
                        _logger.LogInformation(
                            "Text retrieved successfully via reactive path: {Length} characters via {Method}",
                            retrievalResult.Text.Length,
                            retrievalResult.Method);

                        var selectionEvent = new TextSelectionEvent
                        {
                            Coordinates = null, // No tracking data available
                            RetrievalResult = retrievalResult,
                            State = SelectionState.Selected,
                            DetectionPath = "Reactive"
                        };

                        _eventAggregator.Publish(selectionEvent);
                        TextSelected?.Invoke(this, selectionEvent);
                        SelectionDetected?.Invoke(this, selectionEvent);
                    }
                    else if (retrievalResult.Success)
                    {
                        _logger.LogDebug(
                            "Late modifier detection ignored - text too short: {Length} chars (minimum: {Min})",
                            retrievalResult.Text?.Length ?? 0,
                            MinimumTextLengthForReactive);
                    }
                    else
                    {
                        _logger.LogDebug("Late modifier detection - text retrieval failed: {Error}",
                            retrievalResult.ErrorMessage);
                    }
                }
            }
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error handling mouse released event");
            _stateMachine.TransitionTo(SelectionState.Failed, $"Exception: {ex.Message}");
            _stateMachine.Reset("Error recovery");
        }
    }

    /// <summary>
    /// Disposes the monitor and releases all resources.
    /// </summary>
    public void Dispose()
    {
        Dispose(true);
        GC.SuppressFinalize(this);
    }

    /// <summary>
    /// Protected dispose pattern implementation.
    /// </summary>
    protected virtual void Dispose(bool disposing)
    {
        if (_disposed) return;

        if (disposing)
        {
            _logger.LogInformation("Disposing TextSelectionMonitor");

            // Stop monitoring
            if (_isMonitoring)
            {
                Task.Run(async () => await StopMonitoringAsync()).Wait(TimeSpan.FromSeconds(5));
            }

            // Unsubscribe from hook events
            _hookService.MousePressed -= OnMousePressed;
            _hookService.MouseReleased -= OnMouseReleased;
            _hookService.MouseMoved -= OnMouseMoved;

            _logger.LogInformation("TextSelectionMonitor disposed");
        }

        _disposed = true;
    }
}
