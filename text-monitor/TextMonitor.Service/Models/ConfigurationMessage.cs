using System.Text.Json;

namespace TextMonitor.Service.Models;

/// <summary>
/// Configuration message received from Tauri app via named pipe.
/// </summary>
public class ConfigurationMessage
{
    /// <summary>
    /// The type of configuration command (e.g., "update_selection_modifier").
    /// </summary>
    public string Type { get; set; } = string.Empty;

    /// <summary>
    /// The payload containing command-specific data.
    /// Uses JsonElement to allow flexible deserialization based on Type.
    /// </summary>
    public JsonElement? Payload { get; set; }

    /// <summary>
    /// Timestamp when this message was created.
    /// </summary>
    public DateTime Timestamp { get; set; }
}

/// <summary>
/// Payload for selection modifier update command.
/// Used when Type is "update_selection_modifier".
/// </summary>
public class SelectionModifierPayload
{
    /// <summary>
    /// The modifier key to use for text selection detection.
    /// Valid values: "alt", "ctrl", "shift", "meta".
    /// </summary>
    public string Modifier { get; set; } = "alt";
}
