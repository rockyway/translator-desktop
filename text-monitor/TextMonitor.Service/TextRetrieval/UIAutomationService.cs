using System.Diagnostics;
using FlaUI.Core.AutomationElements;
using FlaUI.UIA3;
using Microsoft.Extensions.Logging;
using TextMonitor.Service.Enums;
using TextMonitor.Service.Interfaces;
using TextMonitor.Service.Models;

namespace TextMonitor.Service.TextRetrieval;

/// <summary>
/// Implementation of UI Automation-based text retrieval using FlaUI.
/// Uses Windows UI Automation API to get selected text from focused elements.
/// </summary>
public class UIAutomationService : IUIAutomationService
{
    private readonly ILogger<UIAutomationService> _logger;
    private readonly UIA3Automation _automation;

    public UIAutomationService(ILogger<UIAutomationService> logger)
    {
        _logger = logger ?? throw new ArgumentNullException(nameof(logger));
        _automation = new UIA3Automation();
    }

    /// <summary>
    /// Checks if UI Automation is available on this system.
    /// </summary>
    public bool IsAvailable()
    {
        try
        {
            // Try to access the root element to verify UI Automation is available
            var root = _automation.GetDesktop();
            return root != null;
        }
        catch (Exception ex)
        {
            _logger.LogWarning(ex, "UI Automation is not available");
            return false;
        }
    }

    /// <summary>
    /// Gets information about the currently focused UI element.
    /// </summary>
    public async Task<FocusedElementInfo?> GetFocusedElementInfoAsync()
    {
        return await Task.Run(() =>
        {
            try
            {
                var focusedElement = _automation.FocusedElement();
                if (focusedElement == null)
                {
                    _logger.LogDebug("No focused element found");
                    return null;
                }

                var info = new FocusedElementInfo
                {
                    Name = GetElementName(focusedElement),
                    ControlType = GetControlTypeName(focusedElement),
                    ProcessId = GetProcessId(focusedElement),
                    ProcessName = GetProcessName(focusedElement),
                    IsPassword = IsPasswordField(focusedElement),
                    SupportsTextPattern = SupportsTextPattern(focusedElement),
                    Bounds = GetElementBounds(focusedElement)
                };

                _logger.LogDebug("Focused element: {ProcessName} ({ControlType}), SupportsText={SupportsText}",
                    info.ProcessName, info.ControlType, info.SupportsTextPattern);

                return info;
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error getting focused element info");
                return null;
            }
        });
    }

    /// <summary>
    /// Retrieves selected text from the currently focused element using UI Automation.
    /// </summary>
    public async Task<TextRetrievalResult> GetSelectedTextAsync()
    {
        var startTime = DateTime.UtcNow;
        AutomationElement? focusedElement = null;

        return await Task.Run(() =>
        {
            try
            {
                _logger.LogDebug("Attempting UI Automation text retrieval");

                focusedElement = _automation.FocusedElement();
                if (focusedElement == null)
                {
                    _logger.LogWarning("No focused element found");
                    return CreateFailureResult("No focused element", startTime);
                }

                // Check if it's a password field
                if (IsPasswordField(focusedElement))
                {
                    _logger.LogInformation("Skipping password field");
                    var result = CreateFailureResult("Protected content (password field)", startTime);
                    result.IsProtectedContent = true;
                    return result;
                }

                // Try to get TextPattern
                var textPattern = focusedElement.Patterns.Text.PatternOrDefault;
                if (textPattern == null)
                {
                    _logger.LogDebug("Element does not support TextPattern");
                    return CreateFailureResult("Element does not support TextPattern", startTime);
                }

                // Get selected text ranges
                var selections = textPattern.GetSelection();
                if (selections == null || selections.Length == 0)
                {
                    _logger.LogDebug("No text selection found");
                    return CreateFailureResult("No text selected", startTime);
                }

                // Combine all selected ranges
                var selectedText = string.Join(Environment.NewLine,
                    selections.Select(range => range.GetText(-1)));

                if (string.IsNullOrEmpty(selectedText))
                {
                    _logger.LogDebug("Selected text is empty");
                    return CreateFailureResult("Selected text is empty", startTime);
                }

                // Create success result
                var successResult = TextRetrievalResult.CreateSuccess(selectedText, RetrievalMethod.UIAutomation);
                successResult.RetrievalDuration = DateTime.UtcNow - startTime;
                successResult.SourceApplication = GetProcessName(focusedElement);
                successResult.SourceProcessId = GetProcessId(focusedElement);
                successResult.ElementType = GetControlTypeName(focusedElement);

                _logger.LogInformation("Successfully retrieved {Length} characters via UI Automation from {App}",
                    selectedText.Length, successResult.SourceApplication);

                return successResult;
            }
            catch (Exception ex) when (ex.Message.Contains("not available"))
            {
                _logger.LogWarning(ex, "Element is no longer available");
                return CreateFailureResult("Element no longer available", startTime);
            }
            catch (UnauthorizedAccessException ex)
            {
                string? processName = null;
                try
                {
                    if (focusedElement != null)
                    {
                        processName = GetProcessName(focusedElement);
                    }
                }
                catch
                {
                    // Ignore errors getting process name
                }

                _logger.LogWarning(ex, "Access denied to UI element in application: {ProcessName}",
                    processName ?? "unknown");

                return CreateFailureResult(
                    $"Access denied to {processName ?? "application"}. " +
                    "Run as Administrator to access elevated applications.",
                    startTime);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Unexpected error during UI Automation text retrieval");
                return CreateFailureResult($"Unexpected error: {ex.Message}", startTime);
            }
        });
    }

    #region Helper Methods

    private static string? GetElementName(AutomationElement element)
    {
        try
        {
            return element.Name;
        }
        catch
        {
            return null;
        }
    }

    private static string? GetControlTypeName(AutomationElement element)
    {
        try
        {
            return element.ControlType.ToString();
        }
        catch
        {
            return null;
        }
    }

    private static int GetProcessId(AutomationElement element)
    {
        try
        {
            return element.Properties.ProcessId.ValueOrDefault;
        }
        catch
        {
            return 0;
        }
    }

    private static string? GetProcessName(AutomationElement element)
    {
        try
        {
            var processId = element.Properties.ProcessId.ValueOrDefault;
            if (processId > 0)
            {
                var process = Process.GetProcessById(processId);
                return process.ProcessName;
            }
        }
        catch
        {
            // Ignore
        }

        return null;
    }

    private static bool IsPasswordField(AutomationElement element)
    {
        try
        {
            return element.Properties.IsPassword.ValueOrDefault;
        }
        catch
        {
            return false;
        }
    }

    private static bool SupportsTextPattern(AutomationElement element)
    {
        try
        {
            return element.Patterns.Text.IsSupported;
        }
        catch
        {
            return false;
        }
    }

    private static System.Drawing.Rectangle GetElementBounds(AutomationElement element)
    {
        try
        {
            var rect = element.BoundingRectangle;
            return new System.Drawing.Rectangle(
                (int)rect.Left,
                (int)rect.Top,
                (int)rect.Width,
                (int)rect.Height
            );
        }
        catch
        {
            return System.Drawing.Rectangle.Empty;
        }
    }

    private TextRetrievalResult CreateFailureResult(string errorMessage, DateTime startTime)
    {
        var result = TextRetrievalResult.CreateFailure(errorMessage, RetrievalMethod.UIAutomation);
        result.RetrievalDuration = DateTime.UtcNow - startTime;
        return result;
    }

    #endregion
}
