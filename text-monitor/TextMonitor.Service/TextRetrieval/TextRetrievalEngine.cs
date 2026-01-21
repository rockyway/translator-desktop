using Microsoft.Extensions.Logging;
using TextMonitor.Service.Enums;
using TextMonitor.Service.Interfaces;
using TextMonitor.Service.Models;

namespace TextMonitor.Service.TextRetrieval;

/// <summary>
/// Orchestrates text retrieval using multiple strategies with automatic fallback.
/// Tries UI Automation first, then falls back to clipboard-based retrieval.
/// </summary>
public class TextRetrievalEngine : ITextRetrievalService
{
    private readonly IUIAutomationService _uiAutomationService;
    private readonly IClipboardService _clipboardService;
    private readonly ILogger<TextRetrievalEngine> _logger;

    private static readonly IReadOnlyList<string> MethodOrder = new[]
    {
        "UI Automation API (Primary)",
        "Clipboard Simulation (Secondary)"
    };

    public TextRetrievalEngine(
        IUIAutomationService uiAutomationService,
        IClipboardService clipboardService,
        ILogger<TextRetrievalEngine> logger)
    {
        _uiAutomationService = uiAutomationService ?? throw new ArgumentNullException(nameof(uiAutomationService));
        _clipboardService = clipboardService ?? throw new ArgumentNullException(nameof(clipboardService));
        _logger = logger ?? throw new ArgumentNullException(nameof(logger));
    }

    /// <summary>
    /// Gets the order of retrieval methods.
    /// </summary>
    public IReadOnlyList<string> GetRetrievalMethodOrder() => MethodOrder;

    /// <summary>
    /// Retrieves selected text using the best available method with automatic fallback.
    /// </summary>
    public async Task<TextRetrievalResult> RetrieveSelectedTextAsync()
    {
        _logger.LogInformation("Starting text retrieval with fallback chain");

        // Try UI Automation first (Primary method)
        if (_uiAutomationService.IsAvailable())
        {
            _logger.LogDebug("Attempting primary method: UI Automation");
            var uiAutomationResult = await _uiAutomationService.GetSelectedTextAsync();

            if (uiAutomationResult.Success && !string.IsNullOrEmpty(uiAutomationResult.Text))
            {
                _logger.LogInformation("Primary method succeeded: UI Automation returned {Length} characters",
                    uiAutomationResult.Text.Length);
                return uiAutomationResult;
            }

            _logger.LogDebug("Primary method returned no text: {Error}", uiAutomationResult.ErrorMessage);
        }
        else
        {
            _logger.LogWarning("Primary method unavailable: UI Automation not accessible");
        }

        // Try Clipboard-based retrieval (Secondary method)
        if (_clipboardService.IsAvailable())
        {
            _logger.LogDebug("Attempting secondary method: Clipboard retrieval");
            var clipboardResult = await _clipboardService.GetSelectedTextViaClipboardAsync();

            if (clipboardResult.Success && !string.IsNullOrEmpty(clipboardResult.Text))
            {
                _logger.LogInformation("Secondary method succeeded: Clipboard returned {Length} characters",
                    clipboardResult.Text.Length);
                return clipboardResult;
            }

            _logger.LogDebug("Secondary method returned no text: {Error}", clipboardResult.ErrorMessage);
        }
        else
        {
            _logger.LogWarning("Secondary method unavailable: Clipboard not accessible");
        }

        // All methods failed
        _logger.LogError("All text retrieval methods failed");
        return TextRetrievalResult.CreateFailure(
            "All retrieval methods failed (UI Automation and Clipboard)",
            RetrievalMethod.Failed);
    }
}
