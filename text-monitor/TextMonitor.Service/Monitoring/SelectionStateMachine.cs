using Microsoft.Extensions.Logging;
using TextMonitor.Service.Enums;

namespace TextMonitor.Service.Monitoring;

/// <summary>
/// State machine for managing text selection states.
/// Transitions: Idle -> Selecting -> Selected -> Idle
/// </summary>
public class SelectionStateMachine
{
    private readonly ILogger<SelectionStateMachine> _logger;
    private SelectionState _currentState = SelectionState.Idle;
    private readonly object _stateLock = new();

    public event EventHandler<SelectionState>? StateChanged;

    public SelectionStateMachine(ILogger<SelectionStateMachine> logger)
    {
        _logger = logger ?? throw new ArgumentNullException(nameof(logger));
    }

    /// <summary>
    /// Gets the current state.
    /// </summary>
    public SelectionState CurrentState
    {
        get
        {
            lock (_stateLock)
            {
                return _currentState;
            }
        }
    }

    /// <summary>
    /// Transitions to a new state if the transition is valid.
    /// </summary>
    public bool TransitionTo(SelectionState newState, string? reason = null)
    {
        lock (_stateLock)
        {
            if (!IsValidTransition(_currentState, newState))
            {
                _logger.LogWarning("Invalid state transition: {From} -> {To}. Reason: {Reason}",
                    _currentState, newState, reason ?? "None");
                return false;
            }

            var oldState = _currentState;
            _currentState = newState;

            _logger.LogInformation("State transition: {From} -> {To}. Reason: {Reason}",
                oldState, newState, reason ?? "Normal flow");

            // Raise event outside the lock to prevent potential deadlocks
            Task.Run(() => StateChanged?.Invoke(this, newState));

            return true;
        }
    }

    /// <summary>
    /// Resets the state machine to Idle.
    /// </summary>
    public void Reset(string? reason = null)
    {
        TransitionTo(SelectionState.Idle, reason ?? "Reset");
    }

    /// <summary>
    /// Determines if a state transition is valid.
    /// </summary>
    private static bool IsValidTransition(SelectionState from, SelectionState to)
    {
        // Define valid transitions
        return (from, to) switch
        {
            // From Idle
            (SelectionState.Idle, SelectionState.Selecting) => true,

            // From Selecting
            (SelectionState.Selecting, SelectionState.Selected) => true,
            (SelectionState.Selecting, SelectionState.Idle) => true, // Cancelled
            (SelectionState.Selecting, SelectionState.Failed) => true,

            // From Selected
            (SelectionState.Selected, SelectionState.Idle) => true,
            (SelectionState.Selected, SelectionState.Failed) => true,

            // From Failed
            (SelectionState.Failed, SelectionState.Idle) => true,
            (SelectionState.Failed, SelectionState.Selecting) => true,

            // Self-transition (no-op but valid)
            _ when from == to => true,

            // All other transitions are invalid
            _ => false
        };
    }

    /// <summary>
    /// Checks if currently in a specific state.
    /// </summary>
    public bool IsInState(SelectionState state)
    {
        return CurrentState == state;
    }

    /// <summary>
    /// Checks if currently idle.
    /// </summary>
    public bool IsIdle => IsInState(SelectionState.Idle);

    /// <summary>
    /// Checks if currently selecting.
    /// </summary>
    public bool IsSelecting => IsInState(SelectionState.Selecting);

    /// <summary>
    /// Checks if selection is complete.
    /// </summary>
    public bool IsSelected => IsInState(SelectionState.Selected);

    /// <summary>
    /// Checks if in failed state.
    /// </summary>
    public bool IsFailed => IsInState(SelectionState.Failed);
}
