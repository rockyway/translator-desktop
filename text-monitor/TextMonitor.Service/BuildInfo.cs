using System.Reflection;

namespace TextMonitor.Service;

/// <summary>
/// Provides access to build-time information.
/// </summary>
public static class BuildInfo
{
    /// <summary>
    /// Build timestamp in yyyyMMdd-HHmmss format.
    /// </summary>
    public static string BuildTimestamp { get; } = GetBuildTimestamp();

    private static string GetBuildTimestamp()
    {
        var assembly = Assembly.GetExecutingAssembly();
        var attribute = assembly.GetCustomAttributes<AssemblyMetadataAttribute>()
            .FirstOrDefault(a => a.Key == "BuildTimestamp");
        return attribute?.Value ?? "unknown";
    }
}
