using TextMonitor.Service.Enums;

namespace TextMonitor.Service.Models;

/// <summary>
/// Result of a text retrieval operation with metadata.
/// </summary>
public class TextRetrievalResult
{
    /// <summary>
    /// The retrieved text content. Empty if retrieval failed.
    /// </summary>
    public string Text { get; set; } = string.Empty;

    /// <summary>
    /// Whether text was successfully retrieved.
    /// </summary>
    public bool Success { get; set; }

    /// <summary>
    /// Method used to retrieve the text.
    /// </summary>
    public RetrievalMethod Method { get; set; }

    /// <summary>
    /// Error message if retrieval failed.
    /// </summary>
    public string? ErrorMessage { get; set; }

    /// <summary>
    /// Name of the source application (if available).
    /// </summary>
    public string? SourceApplication { get; set; }

    /// <summary>
    /// Process ID of the source application.
    /// </summary>
    public int? SourceProcessId { get; set; }

    /// <summary>
    /// Type of UI element where text was selected (e.g., "Edit", "Document").
    /// </summary>
    public string? ElementType { get; set; }

    /// <summary>
    /// Timestamp when retrieval was attempted.
    /// </summary>
    public DateTime RetrievalTime { get; set; }

    /// <summary>
    /// Time taken to retrieve the text.
    /// </summary>
    public TimeSpan RetrievalDuration { get; set; }

    /// <summary>
    /// Whether the source is a password field or protected content.
    /// </summary>
    public bool IsProtectedContent { get; set; }

    /// <summary>
    /// Creates a successful result.
    /// </summary>
    public static TextRetrievalResult CreateSuccess(string text, RetrievalMethod method)
    {
        return new TextRetrievalResult
        {
            Text = text,
            Success = true,
            Method = method,
            RetrievalTime = DateTime.UtcNow
        };
    }

    /// <summary>
    /// Creates a failed result.
    /// </summary>
    public static TextRetrievalResult CreateFailure(string errorMessage, RetrievalMethod attemptedMethod)
    {
        return new TextRetrievalResult
        {
            Text = string.Empty,
            Success = false,
            Method = attemptedMethod,
            ErrorMessage = errorMessage,
            RetrievalTime = DateTime.UtcNow
        };
    }
}
