using System.Collections.Concurrent;
using System.IO.Pipes;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using Microsoft.Extensions.Logging;
using TextMonitor.Service.Interfaces;
using TextMonitor.Service.Models;

namespace TextMonitor.Service.Ipc;

/// <summary>
/// Named Pipe IPC server for communication with Tauri frontend.
/// Supports multiple simultaneous client connections.
/// </summary>
public class IpcServer : IDisposable
{
    private readonly ILogger<IpcServer> _logger;
    private readonly ISelectionEventAggregator _eventAggregator;
    private readonly ConcurrentDictionary<Guid, ConnectedClient> _connectedClients = new();
    private readonly JsonSerializerOptions _jsonOptions;

    private IDisposable? _subscription;
    private CancellationTokenSource? _serverCts;
    private Task? _acceptTask;
    private bool _disposed;

    /// <summary>
    /// Pipe name for communication with Tauri frontend.
    /// </summary>
    public const string PipeName = "TranslatorDesktop";

    public IpcServer(
        ILogger<IpcServer> logger,
        ISelectionEventAggregator eventAggregator)
    {
        _logger = logger ?? throw new ArgumentNullException(nameof(logger));
        _eventAggregator = eventAggregator ?? throw new ArgumentNullException(nameof(eventAggregator));

        _jsonOptions = new JsonSerializerOptions
        {
            PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
            WriteIndented = false,
            DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull
        };
    }

    /// <summary>
    /// Starts the IPC server and begins listening for client connections.
    /// </summary>
    public Task StartAsync(CancellationToken cancellationToken = default)
    {
        _logger.LogInformation("Starting IPC server on pipe: {PipeName}", PipeName);

        // Subscribe to selection events
        _subscription = _eventAggregator.Subscribe(OnTextSelected);

        // Start accepting client connections
        _serverCts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        _acceptTask = AcceptClientsAsync(_serverCts.Token);

        _logger.LogInformation("IPC server started");
        return Task.CompletedTask;
    }

    /// <summary>
    /// Stops the IPC server and closes all connections.
    /// </summary>
    public async Task StopAsync()
    {
        _logger.LogInformation("Stopping IPC server");

        _subscription?.Dispose();
        _subscription = null;

        // Cancel accept loop
        _serverCts?.Cancel();

        // Wait for accept task to complete
        if (_acceptTask != null)
        {
            try
            {
                await _acceptTask.ConfigureAwait(false);
            }
            catch (OperationCanceledException)
            {
                // Expected during shutdown
            }
        }

        // Disconnect all clients
        foreach (var client in _connectedClients.Values)
        {
            await DisconnectClientAsync(client).ConfigureAwait(false);
        }
        _connectedClients.Clear();

        _serverCts?.Dispose();
        _serverCts = null;

        _logger.LogInformation("IPC server stopped");
    }

    /// <summary>
    /// Continuously accepts new client connections.
    /// </summary>
    private async Task AcceptClientsAsync(CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                var pipeServer = new NamedPipeServerStream(
                    PipeName,
                    PipeDirection.Out,
                    NamedPipeServerStream.MaxAllowedServerInstances,
                    PipeTransmissionMode.Byte,
                    PipeOptions.Asynchronous);

                _logger.LogDebug("Waiting for client connection on pipe: {PipeName}", PipeName);

                await pipeServer.WaitForConnectionAsync(cancellationToken).ConfigureAwait(false);

                var clientId = Guid.NewGuid();
                var client = new ConnectedClient(clientId, pipeServer);

                if (_connectedClients.TryAdd(clientId, client))
                {
                    _logger.LogInformation("Client connected: {ClientId}. Total clients: {Count}",
                        clientId, _connectedClients.Count);

                    // Send version info to newly connected client
                    _ = SendVersionToClientAsync(client);
                }
                else
                {
                    _logger.LogWarning("Failed to add client: {ClientId}", clientId);
                    await pipeServer.DisposeAsync().ConfigureAwait(false);
                }
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                // Normal shutdown
                break;
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error accepting client connection");
                // Brief delay before retrying
                await Task.Delay(100, cancellationToken).ConfigureAwait(false);
            }
        }
    }

    /// <summary>
    /// Handles text selection events and broadcasts them to connected clients.
    /// </summary>
    private void OnTextSelected(TextSelectionEvent selectionEvent)
    {
        if (!selectionEvent.ShouldProcess) return;
        if (!selectionEvent.RetrievalResult.Success) return;

        _logger.LogDebug("IPC: Broadcasting text selection event - {Length} characters to {ClientCount} clients",
            selectionEvent.RetrievalResult.Text.Length, _connectedClients.Count);

        // Build the IPC message
        var message = new IpcMessage
        {
            Type = "text_selected",
            Payload = new TextSelectedPayload
            {
                Text = selectionEvent.RetrievalResult.Text,
                CursorX = selectionEvent.Coordinates?.EndX ?? 0,
                CursorY = selectionEvent.Coordinates?.EndY ?? 0,
                SourceApp = selectionEvent.RetrievalResult.SourceApplication ?? "unknown",
                WindowTitle = selectionEvent.RetrievalResult.ElementType
            },
            Timestamp = DateTime.UtcNow
        };

        // Broadcast to all connected clients
        _ = BroadcastAsync(message);
    }

    /// <summary>
    /// Broadcasts a message to all connected clients.
    /// </summary>
    private async Task BroadcastAsync(IpcMessage message)
    {
        var json = JsonSerializer.Serialize(message, _jsonOptions);
        var messageBytes = Encoding.UTF8.GetBytes(json + "\n"); // Newline-delimited JSON

        var disconnectedClients = new List<Guid>();

        foreach (var (clientId, client) in _connectedClients)
        {
            try
            {
                if (!client.PipeStream.IsConnected)
                {
                    disconnectedClients.Add(clientId);
                    continue;
                }

                await client.WriteLock.WaitAsync().ConfigureAwait(false);
                try
                {
                    await client.PipeStream.WriteAsync(messageBytes).ConfigureAwait(false);
                    await client.PipeStream.FlushAsync().ConfigureAwait(false);
                }
                finally
                {
                    client.WriteLock.Release();
                }

                _logger.LogDebug("Sent message to client {ClientId}", clientId);
            }
            catch (IOException ex)
            {
                _logger.LogWarning("Client {ClientId} disconnected during write: {Message}",
                    clientId, ex.Message);
                disconnectedClients.Add(clientId);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error sending message to client {ClientId}", clientId);
                disconnectedClients.Add(clientId);
            }
        }

        // Clean up disconnected clients
        foreach (var clientId in disconnectedClients)
        {
            if (_connectedClients.TryRemove(clientId, out var client))
            {
                await DisconnectClientAsync(client).ConfigureAwait(false);
                _logger.LogInformation("Removed disconnected client: {ClientId}. Remaining: {Count}",
                    clientId, _connectedClients.Count);
            }
        }
    }

    /// <summary>
    /// Sends the version message to a newly connected client.
    /// </summary>
    private async Task SendVersionToClientAsync(ConnectedClient client)
    {
        var versionMessage = new IpcMessage
        {
            Type = "version",
            Payload = new VersionPayload { Version = BuildInfo.BuildTimestamp },
            Timestamp = DateTime.UtcNow
        };

        var json = JsonSerializer.Serialize(versionMessage, _jsonOptions);
        var messageBytes = Encoding.UTF8.GetBytes(json + "\n");

        try
        {
            if (!client.PipeStream.IsConnected)
            {
                return;
            }

            await client.WriteLock.WaitAsync().ConfigureAwait(false);
            try
            {
                await client.PipeStream.WriteAsync(messageBytes).ConfigureAwait(false);
                await client.PipeStream.FlushAsync().ConfigureAwait(false);
            }
            finally
            {
                client.WriteLock.Release();
            }

            _logger.LogDebug("Sent version message to client {ClientId}: {Version}",
                client.ClientId, BuildInfo.BuildTimestamp);
        }
        catch (Exception ex)
        {
            _logger.LogWarning(ex, "Failed to send version to client {ClientId}", client.ClientId);
        }
    }

    /// <summary>
    /// Disconnects and disposes a client connection.
    /// </summary>
    private async Task DisconnectClientAsync(ConnectedClient client)
    {
        try
        {
            if (client.PipeStream.IsConnected)
            {
                client.PipeStream.Disconnect();
            }
            await client.PipeStream.DisposeAsync().ConfigureAwait(false);
            client.WriteLock.Dispose();
        }
        catch (Exception ex)
        {
            _logger.LogWarning(ex, "Error disconnecting client {ClientId}", client.ClientId);
        }
    }

    public void Dispose()
    {
        Dispose(true);
        GC.SuppressFinalize(this);
    }

    protected virtual void Dispose(bool disposing)
    {
        if (_disposed) return;

        if (disposing)
        {
            _subscription?.Dispose();
            _subscription = null;

            _serverCts?.Cancel();
            _serverCts?.Dispose();
            _serverCts = null;

            foreach (var client in _connectedClients.Values)
            {
                try
                {
                    client.PipeStream.Dispose();
                    client.WriteLock.Dispose();
                }
                catch { /* Ignore disposal errors */ }
            }
            _connectedClients.Clear();
        }

        _disposed = true;
    }

    /// <summary>
    /// Represents a connected client.
    /// </summary>
    private sealed class ConnectedClient
    {
        public Guid ClientId { get; }
        public NamedPipeServerStream PipeStream { get; }
        public SemaphoreSlim WriteLock { get; } = new(1, 1);

        public ConnectedClient(Guid clientId, NamedPipeServerStream pipeStream)
        {
            ClientId = clientId;
            PipeStream = pipeStream;
        }
    }
}

/// <summary>
/// IPC message wrapper.
/// </summary>
public class IpcMessage
{
    public string Type { get; set; } = string.Empty;
    public object? Payload { get; set; }
    public DateTime Timestamp { get; set; }
}

/// <summary>
/// Payload for text_selected events.
/// </summary>
public class TextSelectedPayload
{
    public string Text { get; set; } = string.Empty;
    public int CursorX { get; set; }
    public int CursorY { get; set; }
    public string SourceApp { get; set; } = string.Empty;
    public string? WindowTitle { get; set; }
}

/// <summary>
/// Payload for version events.
/// </summary>
public class VersionPayload
{
    public string Version { get; set; } = string.Empty;
}
