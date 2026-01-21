using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Serilog;
using TextMonitor.Service.Events;
using TextMonitor.Service.Interfaces;
using TextMonitor.Service.Ipc;
using TextMonitor.Service.Models;
using TextMonitor.Service.Monitoring;
using TextMonitor.Service.Services;
using TextMonitor.Service.TextRetrieval;

namespace TextMonitor.Service;

public class Program
{
    public static async Task Main(string[] args)
    {
        // Configure Serilog
        Log.Logger = new LoggerConfiguration()
            .MinimumLevel.Information()
            .MinimumLevel.Override("Microsoft", Serilog.Events.LogEventLevel.Warning)
            .WriteTo.Console(outputTemplate: "[{Timestamp:HH:mm:ss} {Level:u3}] {Message:lj}{NewLine}{Exception}")
            .CreateLogger();

        try
        {
            Log.Information("Starting TextMonitor.Service...");
            Log.Information("Text Monitor Build Version: {Version}", BuildInfo.BuildTimestamp);

            var host = Host.CreateDefaultBuilder(args)
                .UseSerilog()
                .ConfigureServices((context, services) =>
                {
                    // Register services
                    services.AddSingleton<IGlobalHookService, GlobalHookService>();
                    services.AddSingleton<IUIAutomationService, UIAutomationService>();
                    services.AddSingleton<IClipboardService, ClipboardService>();
                    services.AddSingleton<ITextRetrievalService, TextRetrievalEngine>();
                    services.AddSingleton<ISelectionEventAggregator, SelectionEventAggregator>();
                    services.AddSingleton<ITextSelectionMonitor, TextSelectionMonitor>();
                    services.AddSingleton<IpcServer>();
                    services.AddSingleton<ConfigurationReceiver>();

                    // Register hosted services
                    services.AddHostedService<TextMonitorHostedService>();
                    services.AddHostedService(sp => sp.GetRequiredService<ConfigurationReceiver>());
                })
                .Build();

            // Wire up configuration receiver to text selection monitor
            WireConfigurationReceiver(host.Services);

            await host.RunAsync();
        }
        catch (Exception ex)
        {
            Log.Fatal(ex, "TextMonitor.Service terminated unexpectedly");
        }
        finally
        {
            Log.CloseAndFlush();
        }
    }

    /// <summary>
    /// Wires the ConfigurationReceiver to handle configuration updates.
    /// </summary>
    private static void WireConfigurationReceiver(IServiceProvider services)
    {
        var configReceiver = services.GetRequiredService<ConfigurationReceiver>();
        var monitor = services.GetRequiredService<ITextSelectionMonitor>() as TextSelectionMonitor;

        if (monitor == null)
        {
            Log.Warning("TextSelectionMonitor not available for configuration wiring");
            return;
        }

        configReceiver.ConfigurationReceived += (sender, message) =>
        {
            if (message.Type == "update_selection_modifier" && message.Payload.HasValue)
            {
                try
                {
                    var payload = System.Text.Json.JsonSerializer.Deserialize<SelectionModifierPayload>(
                        message.Payload.Value.GetRawText(),
                        new System.Text.Json.JsonSerializerOptions
                        {
                            PropertyNameCaseInsensitive = true
                        });

                    if (payload != null && !string.IsNullOrEmpty(payload.Modifier))
                    {
                        monitor.SetSelectionModifier(payload.Modifier);
                        Log.Information("Selection modifier updated to: {Modifier}", payload.Modifier);
                    }
                }
                catch (Exception ex)
                {
                    Log.Error(ex, "Failed to process selection modifier update");
                }
            }
        };

        Log.Information("Configuration receiver wired to text selection monitor");
    }
}

/// <summary>
/// Hosted service that manages the text selection monitor lifecycle.
/// </summary>
public class TextMonitorHostedService : IHostedService
{
    private readonly ITextSelectionMonitor _monitor;
    private readonly ISelectionEventAggregator _eventAggregator;
    private readonly IpcServer _ipcServer;
    private readonly ILogger<TextMonitorHostedService> _logger;
    private IDisposable? _subscription;

    public TextMonitorHostedService(
        ITextSelectionMonitor monitor,
        ISelectionEventAggregator eventAggregator,
        IpcServer ipcServer,
        ILogger<TextMonitorHostedService> logger)
    {
        _monitor = monitor;
        _eventAggregator = eventAggregator;
        _ipcServer = ipcServer;
        _logger = logger;
    }

    public async Task StartAsync(CancellationToken cancellationToken)
    {
        _logger.LogInformation("TextMonitor service starting...");

        // Subscribe to selection events for console output
        _subscription = _eventAggregator.Subscribe(OnTextSelected);

        // Start IPC server
        await _ipcServer.StartAsync(cancellationToken);

        // Start monitoring
        var started = await _monitor.StartMonitoringAsync();
        if (started)
        {
            _logger.LogInformation("TextMonitor service started successfully");
            _logger.LogInformation("Hold Ctrl and drag to select text. Press Ctrl+C to exit.");
        }
        else
        {
            _logger.LogError("Failed to start text selection monitoring");
        }
    }

    public async Task StopAsync(CancellationToken cancellationToken)
    {
        _logger.LogInformation("TextMonitor service stopping...");

        _subscription?.Dispose();
        await _monitor.StopMonitoringAsync();
        await _ipcServer.StopAsync();

        _logger.LogInformation("TextMonitor service stopped");
    }

    private void OnTextSelected(TextSelectionEvent selectionEvent)
    {
        if (!selectionEvent.ShouldProcess) return;

        if (selectionEvent.RetrievalResult.Success)
        {
            var text = selectionEvent.RetrievalResult.Text;
            var preview = text.Length > 100 ? text[..100] + "..." : text;

            Console.WriteLine();
            Console.WriteLine("================================================================================");
            Console.WriteLine($"Text selected: [{selectionEvent.RetrievalResult.Text.Length} chars via {selectionEvent.RetrievalResult.Method}]");
            Console.WriteLine("--------------------------------------------------------------------------------");
            Console.WriteLine(preview);
            Console.WriteLine("================================================================================");
            Console.WriteLine();
        }
        else
        {
            _logger.LogWarning("Text selection failed: {Error}", selectionEvent.RetrievalResult.ErrorMessage);
        }
    }
}
