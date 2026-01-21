using Microsoft.Extensions.Logging;
using SharpHook;
using TextMonitor.Service.Interfaces;
using TextMonitor.Service.Models;

namespace TextMonitor.Service.Services;

/// <summary>
/// Implementation of global hook service using SharpHook.
/// Monitors mouse events across all applications.
/// </summary>
public class GlobalHookService : IGlobalHookService
{
    private readonly ILogger<GlobalHookService> _logger;
    private TaskPoolGlobalHook? _hook;
    private Task? _hookTask;
    private bool _isRunning;
    private bool _disposed;
    private string? _lastError;

    // Health monitoring fields
    private Task? _healthMonitorTask;
    private CancellationTokenSource? _healthMonitorCts;
    private DateTime _lastHealthCheck = DateTime.Now;

    public event EventHandler<MouseEventData>? MousePressed;
    public event EventHandler<MouseEventData>? MouseReleased;
    public event EventHandler<MouseEventData>? MouseMoved;

    public bool IsRunning => _isRunning && _hook != null;
    public string? LastError => _lastError;

    public GlobalHookService(ILogger<GlobalHookService> logger)
    {
        _logger = logger ?? throw new ArgumentNullException(nameof(logger));
    }

    /// <summary>
    /// Starts the global hook monitoring.
    /// </summary>
    public async Task<bool> StartAsync()
    {
        if (_disposed)
        {
            _lastError = "Service has been disposed";
            _logger.LogError("Cannot start GlobalHookService: already disposed");
            return false;
        }

        if (_isRunning)
        {
            _logger.LogWarning("GlobalHookService already running");
            return true;
        }

        try
        {
            _logger.LogInformation("Starting GlobalHookService...");

            // Create and configure the hook
            _hook = new TaskPoolGlobalHook();

            // Subscribe to mouse events
            _hook.MousePressed += OnMousePressed;
            _hook.MouseReleased += OnMouseReleased;
            _hook.MouseMoved += OnMouseMoved;

            // Run the hook asynchronously with comprehensive error handling
            _hookTask = Task.Run(async () =>
            {
                try
                {
                    await _hook.RunAsync();
                    _logger.LogWarning("Hook task completed normally (unexpected)");
                }
                catch (Exception ex)
                {
                    _lastError = $"Hook task failed: {ex.Message}";
                    _logger.LogError(ex, "CRITICAL: Global hook task failed");
                    _isRunning = false;
                }
            });

            // Give the hook a moment to initialize
            await Task.Delay(100);

            _isRunning = true;
            _lastError = null;

            // Start health monitoring
            _healthMonitorCts = new CancellationTokenSource();
            _healthMonitorTask = MonitorHookHealthAsync(_healthMonitorCts.Token);

            _logger.LogInformation("GlobalHookService started successfully with health monitoring");
            return true;
        }
        catch (Exception ex)
        {
            _lastError = $"Failed to start hook: {ex.Message}";
            _logger.LogError(ex, "Failed to start GlobalHookService");
            _isRunning = false;

            // Cleanup on failure
            if (_hook != null)
            {
                await CleanupHookAsync();
            }

            return false;
        }
    }

    /// <summary>
    /// Monitors hook health and performs automatic restart on failures.
    /// </summary>
    private async Task MonitorHookHealthAsync(CancellationToken ct)
    {
        _logger.LogInformation("Hook health monitoring started");

        while (!ct.IsCancellationRequested)
        {
            try
            {
                await Task.Delay(5000, ct); // Check every 5 seconds

                if (_hookTask != null && _hookTask.IsCompleted && _isRunning)
                {
                    _logger.LogWarning("Hook task unexpectedly completed - status: {Status}",
                        _hookTask.Status);

                    if (_hookTask.IsFaulted)
                    {
                        _logger.LogError(_hookTask.Exception,
                            "Hook task faulted with exception");
                    }

                    // Attempt automatic restart
                    _logger.LogInformation("Attempting automatic hook restart...");
                    try
                    {
                        await CleanupHookAsync();
                        await Task.Delay(1000, ct); // Brief pause before restart

                        // Recreate hook
                        _hook = new TaskPoolGlobalHook();
                        _hook.MousePressed += OnMousePressed;
                        _hook.MouseReleased += OnMouseReleased;
                        _hook.MouseMoved += OnMouseMoved;

                        _hookTask = Task.Run(async () =>
                        {
                            try
                            {
                                await _hook.RunAsync();
                            }
                            catch (Exception ex)
                            {
                                _logger.LogError(ex, "Hook task failed during auto-restart");
                                _isRunning = false;
                            }
                        }, ct);

                        _logger.LogInformation("Hook successfully restarted automatically");
                    }
                    catch (Exception ex)
                    {
                        _logger.LogError(ex, "Failed to auto-restart hook");
                        _isRunning = false;
                    }
                }

                _lastHealthCheck = DateTime.Now;
            }
            catch (OperationCanceledException)
            {
                // Expected during shutdown
                break;
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error in hook health monitoring");
            }
        }

        _logger.LogInformation("Hook health monitoring stopped");
    }

    /// <summary>
    /// Stops the global hook monitoring.
    /// </summary>
    public async Task StopAsync()
    {
        if (!_isRunning || _hook == null)
        {
            _logger.LogWarning("GlobalHookService not running");
            return;
        }

        try
        {
            _logger.LogInformation("Stopping GlobalHookService...");

            // Stop health monitoring first
            if (_healthMonitorCts != null)
            {
                _healthMonitorCts.Cancel();
                if (_healthMonitorTask != null)
                {
                    try
                    {
                        await _healthMonitorTask;
                    }
                    catch (OperationCanceledException)
                    {
                        // Expected
                    }
                }
                _healthMonitorCts?.Dispose();
                _healthMonitorCts = null;
                _healthMonitorTask = null;
            }

            // Then cleanup hook
            await CleanupHookAsync();

            _isRunning = false;
            _logger.LogInformation("GlobalHookService stopped successfully");
        }
        catch (Exception ex)
        {
            _lastError = $"Error during stop: {ex.Message}";
            _logger.LogError(ex, "Error stopping GlobalHookService");
            throw;
        }
    }

    /// <summary>
    /// Handles mouse pressed events from SharpHook.
    /// </summary>
    private void OnMousePressed(object? sender, MouseHookEventArgs e)
    {
        try
        {
            var eventData = CreateMouseEventData(e);
            MousePressed?.Invoke(this, eventData);
        }
        catch (UnauthorizedAccessException ex)
        {
            _logger.LogWarning(ex, "Access denied capturing mouse event. " +
                "This may occur when elevated applications have focus.");
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error handling mouse pressed event");
        }
    }

    /// <summary>
    /// Handles mouse released events from SharpHook.
    /// </summary>
    private void OnMouseReleased(object? sender, MouseHookEventArgs e)
    {
        try
        {
            var eventData = CreateMouseEventData(e);
            MouseReleased?.Invoke(this, eventData);
        }
        catch (UnauthorizedAccessException ex)
        {
            _logger.LogWarning(ex, "Access denied capturing mouse event. " +
                "This may occur when elevated applications have focus.");
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error handling mouse released event");
        }
    }

    /// <summary>
    /// Handles mouse moved events from SharpHook.
    /// </summary>
    private void OnMouseMoved(object? sender, MouseHookEventArgs e)
    {
        try
        {
            var eventData = CreateMouseEventData(e);
            MouseMoved?.Invoke(this, eventData);
        }
        catch (UnauthorizedAccessException ex)
        {
            // Log at Debug level to avoid spam, since mouse move events are very frequent
            _logger.LogDebug(ex, "Access denied capturing mouse move event from elevated application");
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error handling mouse moved event");
        }
    }

    /// <summary>
    /// Converts SharpHook event data to our internal model.
    /// </summary>
    private static MouseEventData CreateMouseEventData(MouseHookEventArgs e)
    {
        return new MouseEventData
        {
            Button = (ushort)e.Data.Button,
            X = e.Data.X,
            Y = e.Data.Y,
            Timestamp = DateTime.UtcNow
        };
    }

    /// <summary>
    /// Cleanup hook resources.
    /// </summary>
    private async Task CleanupHookAsync()
    {
        if (_hook == null) return;

        try
        {
            // Unsubscribe from events
            _hook.MousePressed -= OnMousePressed;
            _hook.MouseReleased -= OnMouseReleased;
            _hook.MouseMoved -= OnMouseMoved;

            // Dispose the hook (this will cause RunAsync to complete)
            _hook.Dispose();

            // Wait for the hook task to complete (with timeout)
            if (_hookTask != null)
            {
                var completed = await Task.WhenAny(_hookTask, Task.Delay(2000));
                if (completed != _hookTask)
                {
                    _logger.LogWarning("Hook task did not complete within timeout");
                }
            }

            _hook = null;
            _hookTask = null;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error during hook cleanup");
        }
    }

    /// <summary>
    /// Disposes the service and releases all resources.
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
            _logger.LogInformation("Disposing GlobalHookService");

            // Stop the hook synchronously during disposal
            if (_isRunning)
            {
                Task.Run(async () => await StopAsync()).Wait(TimeSpan.FromSeconds(5));
            }

            // Ensure health monitor resources are cleaned up
            _healthMonitorCts?.Dispose();
            _healthMonitorCts = null;
            _healthMonitorTask = null;
        }

        _disposed = true;
    }
}
