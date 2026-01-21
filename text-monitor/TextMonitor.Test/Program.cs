using System.IO.Pipes;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace TextMonitor.Test;

/// <summary>
/// Simple IPC test tool for Named Pipe communication testing.
/// Starts a server and sends test messages to verify the pipe works end-to-end.
/// </summary>
public class Program
{
    private const string PipeName = "TranslatorDesktop";

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        WriteIndented = false,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull
    };

    public static async Task Main(string[] args)
    {
        Console.WriteLine("========================================");
        Console.WriteLine("  TextMonitor IPC Test Tool");
        Console.WriteLine("========================================");
        Console.WriteLine();
        Console.WriteLine($"Pipe name: {PipeName}");
        Console.WriteLine();

        if (args.Length > 0 && args[0] == "--client")
        {
            // Run as client (for testing if server is running)
            await RunClientTestAsync();
        }
        else
        {
            // Run as server (default)
            await RunServerAsync();
        }
    }

    /// <summary>
    /// Runs as a Named Pipe server, waits for a client, then sends test messages.
    /// </summary>
    private static async Task RunServerAsync()
    {
        Console.WriteLine("Starting IPC server...");
        Console.WriteLine("Waiting for Tauri app to connect...");
        Console.WriteLine("(Start the Tauri app, then press any key to send test messages)");
        Console.WriteLine();

        // Create the named pipe server
        await using var pipeServer = new NamedPipeServerStream(
            PipeName,
            PipeDirection.Out,
            NamedPipeServerStream.MaxAllowedServerInstances,
            PipeTransmissionMode.Byte,
            PipeOptions.Asynchronous);

        Console.WriteLine($"Server created. Pipe: \\\\.\\pipe\\{PipeName}");
        Console.WriteLine("Waiting for client connection...");

        // Wait for a client to connect
        await pipeServer.WaitForConnectionAsync();
        Console.WriteLine("Client connected!");
        Console.WriteLine();

        // Interactive loop to send test messages
        var messageCount = 0;
        while (true)
        {
            Console.WriteLine("Press:");
            Console.WriteLine("  [1] Send test message 'Hello from IPC test!'");
            Console.WriteLine("  [2] Send custom message");
            Console.WriteLine("  [3] Send message with special characters");
            Console.WriteLine("  [Q] Quit");
            Console.Write("> ");

            var key = Console.ReadKey(true);
            Console.WriteLine(key.KeyChar);

            if (key.Key == ConsoleKey.Q)
            {
                Console.WriteLine("Exiting...");
                break;
            }

            string testText;
            switch (key.Key)
            {
                case ConsoleKey.D1:
                case ConsoleKey.NumPad1:
                    testText = "Hello from IPC test!";
                    break;
                case ConsoleKey.D2:
                case ConsoleKey.NumPad2:
                    Console.Write("Enter custom message: ");
                    testText = Console.ReadLine() ?? "Custom test message";
                    break;
                case ConsoleKey.D3:
                case ConsoleKey.NumPad3:
                    testText = "Special chars: \"quotes\", 'apostrophe', <angle>, &ampersand, \u4e2d\u6587";
                    break;
                default:
                    continue;
            }

            messageCount++;

            if (!pipeServer.IsConnected)
            {
                Console.WriteLine("Client disconnected. Exiting...");
                break;
            }

            try
            {
                var message = CreateTestMessage(testText, messageCount);
                await SendMessageAsync(pipeServer, message);
                Console.WriteLine($"Sent message #{messageCount}: \"{testText}\"");
                Console.WriteLine();
            }
            catch (IOException ex)
            {
                Console.WriteLine($"Error sending message: {ex.Message}");
                Console.WriteLine("Client may have disconnected.");
                break;
            }
        }
    }

    /// <summary>
    /// Runs as a client to test if a server is running.
    /// </summary>
    private static async Task RunClientTestAsync()
    {
        Console.WriteLine("Running as client to test server connectivity...");

        try
        {
            await using var pipeClient = new NamedPipeClientStream(
                ".",
                PipeName,
                PipeDirection.In,
                PipeOptions.Asynchronous);

            Console.WriteLine($"Attempting to connect to pipe: \\\\.\\pipe\\{PipeName}");

            using var cts = new CancellationTokenSource(TimeSpan.FromSeconds(5));
            await pipeClient.ConnectAsync(cts.Token);

            Console.WriteLine("Connected! Server is running.");
            Console.WriteLine("Reading messages (Ctrl+C to exit)...");
            Console.WriteLine();

            using var reader = new StreamReader(pipeClient, Encoding.UTF8);
            while (!reader.EndOfStream)
            {
                var line = await reader.ReadLineAsync();
                if (!string.IsNullOrEmpty(line))
                {
                    Console.WriteLine($"Received: {line}");
                }
            }
        }
        catch (OperationCanceledException)
        {
            Console.WriteLine("Connection timed out. Server may not be running.");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Connection failed: {ex.Message}");
        }
    }

    /// <summary>
    /// Creates a test IPC message matching the expected format.
    /// </summary>
    private static IpcMessage CreateTestMessage(string text, int messageNumber)
    {
        return new IpcMessage
        {
            Type = "text_selected",
            Payload = new TextSelectedPayload
            {
                Text = text,
                CursorX = 100 + (messageNumber * 10),
                CursorY = 200 + (messageNumber * 10),
                SourceApp = "TextMonitor.Test.exe",
                WindowTitle = $"IPC Test - Message #{messageNumber}"
            },
            Timestamp = DateTime.UtcNow
        };
    }

    /// <summary>
    /// Sends a message through the pipe as newline-delimited JSON.
    /// </summary>
    private static async Task SendMessageAsync(NamedPipeServerStream pipe, IpcMessage message)
    {
        var json = JsonSerializer.Serialize(message, JsonOptions);
        var bytes = Encoding.UTF8.GetBytes(json + "\n");
        await pipe.WriteAsync(bytes);
        await pipe.FlushAsync();
    }
}

/// <summary>
/// IPC message wrapper matching the format expected by Tauri.
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
