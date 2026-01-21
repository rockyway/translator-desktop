namespace TextMonitor.Service.Enums;

/// <summary>
/// Indicates which method was used to retrieve selected text.
/// </summary>
public enum RetrievalMethod
{
    /// <summary>
    /// No retrieval attempted yet.
    /// </summary>
    None,

    /// <summary>
    /// Retrieved using Windows UI Automation API.
    /// </summary>
    UIAutomation,

    /// <summary>
    /// Retrieved using clipboard simulation (Ctrl+C fallback).
    /// </summary>
    ClipboardSimulation,

    /// <summary>
    /// Retrieved using SendKeys with retry logic.
    /// </summary>
    SendKeysRetry,

    /// <summary>
    /// All retrieval methods failed.
    /// </summary>
    Failed
}
