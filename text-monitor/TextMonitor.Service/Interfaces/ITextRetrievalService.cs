using TextMonitor.Service.Models;

namespace TextMonitor.Service.Interfaces;

/// <summary>
/// Orchestrates text retrieval using multiple strategies with fallback chain.
/// </summary>
public interface ITextRetrievalService
{
    /// <summary>
    /// Retrieves selected text using the best available method.
    /// Tries UI Automation first, then falls back to clipboard simulation.
    /// </summary>
    /// <returns>Text retrieval result with metadata about the method used.</returns>
    Task<TextRetrievalResult> RetrieveSelectedTextAsync();

    /// <summary>
    /// Gets the order of retrieval methods that will be attempted.
    /// </summary>
    IReadOnlyList<string> GetRetrievalMethodOrder();
}
