using System.IO.Pipes;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using TextMonitor.Service.Models;

namespace TextMonitor.Service.Ipc;

/// <summary>
/// Named Pipe server for receiving configuration from Tauri frontend.
/// Listens on a separate pipe from IpcServer and handles incoming configuration messages.
/// </summary>
public class ConfigurationReceiver : IHostedService, IDisposable
{
    private readonly ILogger<ConfigurationReceiver> _logger;
    private readonly JsonSerializerOptions _jsonOptions;

    private CancellationTokenSource? _receiverCts;
    private Task? _receiveTask;
    private bool _disposed;

    /// <summary>
    /// Pipe name for receiving configuration from Tauri frontend.
    /// </summary>
    public const string PipeName = "TranslatorDesktopConfig";

    /// <summary>
    /// Event raised when a configuration message is received.
    /// </summary>
    public event EventHandler<ConfigurationMessage>? ConfigurationReceived;

    /// <summary>
    /// Creates a new instance of the ConfigurationReceiver.
    /// </summary>
    /// <param name="logger">Logger instance for diagnostic output.</param>
    public ConfigurationReceiver(ILogger<ConfigurationReceiver> logger)
    {
        _logger = logger ?? throw new ArgumentNullException(nameof(logger));

        _jsonOptions = new JsonSerializerOptions
        {
            PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
            PropertyNameCaseInsensitive = true,
            DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull
        };
    }

    /// <summary>
    /// Starts the configuration receiver and begins listening for incoming messages.
    /// </summary>
    /// <param name="cancellationToken">Cancellation token for graceful shutdown.</param>
    public Task StartAsync(CancellationToken cancellationToken)
    {
        _logger.LogInformation("Starting configuration receiver on pipe: {PipeName}", PipeName);

        _receiverCts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        _receiveTask = ReceiveLoopAsync(_receiverCts.Token);

        _logger.LogInformation("Configuration receiver started");
        return Task.CompletedTask;
    }

    /// <summary>
    /// Stops the configuration receiver and closes all connections.
    /// </summary>
    /// <param name="cancellationToken">Cancellation token.</param>
    public async Task StopAsync(CancellationToken cancellationToken)
    {
        _logger.LogInformation("Stopping configuration receiver");

        _receiverCts?.Cancel();

        if (_receiveTask != null)
        {
            try
            {
                await _receiveTask.ConfigureAwait(false);
            }
            catch (OperationCanceledException)
            {
                // Expected during shutdown
            }
        }

        _receiverCts?.Dispose();
        _receiverCts = null;

        _logger.LogInformation("Configuration receiver stopped");
    }

    /// <summary>
    /// Main receive loop that continuously accepts connections and reads messages.
    /// After a client disconnects, the server reconnects to accept the next client.
    /// </summary>
    private async Task ReceiveLoopAsync(CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            NamedPipeServerStream? pipeServer = null;

            try
            {
                pipeServer = new NamedPipeServerStream(
                    PipeName,
                    PipeDirection.In,
                    1, // Single client at a time
                    PipeTransmissionMode.Byte,
                    PipeOptions.Asynchronous);

                _logger.LogDebug("Waiting for configuration client connection on pipe: {PipeName}", PipeName);

                await pipeServer.WaitForConnectionAsync(cancellationToken).ConfigureAwait(false);

                _logger.LogInformation("Configuration client connected");

                await ReadMessagesFromClientAsync(pipeServer, cancellationToken).ConfigureAwait(false);
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                // Normal shutdown
                _logger.LogDebug("Configuration receiver shutting down");
                break;
            }
            catch (IOException ex)
            {
                _logger.LogWarning("Configuration client disconnected: {Message}", ex.Message);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error in configuration receive loop");
                // Brief delay before retrying to prevent tight loop on repeated failures
                await Task.Delay(100, cancellationToken).ConfigureAwait(false);
            }
            finally
            {
                if (pipeServer != null)
                {
                    try
                    {
                        if (pipeServer.IsConnected)
                        {
                            pipeServer.Disconnect();
                        }
                        await pipeServer.DisposeAsync().ConfigureAwait(false);
                    }
                    catch (Exception ex)
                    {
                        _logger.LogWarning(ex, "Error disposing pipe server");
                    }
                }
            }
        }
    }

    /// <summary>
    /// Reads newline-delimited JSON messages from the connected client.
    /// </summary>
    private async Task ReadMessagesFromClientAsync(NamedPipeServerStream pipeServer, CancellationToken cancellationToken)
    {
        using var reader = new StreamReader(pipeServer, Encoding.UTF8, leaveOpen: true);

        while (!cancellationToken.IsCancellationRequested && pipeServer.IsConnected)
        {
            try
            {
                var line = await reader.ReadLineAsync(cancellationToken).ConfigureAwait(false);

                if (line == null)
                {
                    // Client disconnected (end of stream)
                    _logger.LogInformation("Configuration client disconnected (end of stream)");
                    break;
                }

                if (string.IsNullOrWhiteSpace(line))
                {
                    continue;
                }

                _logger.LogDebug("Received configuration message: {Message}", line);

                var message = JsonSerializer.Deserialize<ConfigurationMessage>(line, _jsonOptions);

                if (message != null)
                {
                    OnConfigurationReceived(message);
                }
                else
                {
                    _logger.LogWarning("Failed to deserialize configuration message: {Message}", line);
                }
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                break;
            }
            catch (JsonException ex)
            {
                _logger.LogWarning(ex, "Invalid JSON in configuration message");
            }
            catch (IOException ex)
            {
                _logger.LogWarning("IO error reading from configuration client: {Message}", ex.Message);
                break;
            }
        }
    }

    /// <summary>
    /// Raises the ConfigurationReceived event.
    /// </summary>
    /// <param name="message">The received configuration message.</param>
    protected virtual void OnConfigurationReceived(ConfigurationMessage message)
    {
        _logger.LogInformation("Configuration received: Type={Type}", message.Type);

        try
        {
            ConfigurationReceived?.Invoke(this, message);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error in ConfigurationReceived event handler");
        }
    }

    /// <summary>
    /// Disposes the configuration receiver and releases all resources.
    /// </summary>
    public void Dispose()
    {
        Dispose(true);
        GC.SuppressFinalize(this);
    }

    /// <summary>
    /// Disposes managed resources.
    /// </summary>
    /// <param name="disposing">True if called from Dispose(), false if from finalizer.</param>
    protected virtual void Dispose(bool disposing)
    {
        if (_disposed) return;

        if (disposing)
        {
            _receiverCts?.Cancel();
            _receiverCts?.Dispose();
            _receiverCts = null;
        }

        _disposed = true;
    }
}
