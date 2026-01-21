using TextMonitor.Service.Models;

namespace TextMonitor.Service.Interfaces;

/// <summary>
/// Event aggregator for text selection events.
/// Implements thread-safe publish/subscribe pattern to decouple components.
/// </summary>
public interface ISelectionEventAggregator
{
    /// <summary>
    /// Subscribes to text selection events.
    /// The returned IDisposable should be disposed to unsubscribe.
    /// </summary>
    /// <param name="handler">Event handler to invoke when a text selection occurs.</param>
    /// <returns>Subscription token for unsubscribing. Dispose to unsubscribe.</returns>
    IDisposable Subscribe(Action<TextSelectionEvent> handler);

    /// <summary>
    /// Publishes a text selection event to all active subscribers.
    /// Subscribers are invoked on the thread pool to prevent blocking the publisher.
    /// Individual subscriber exceptions are caught and logged to prevent cascade failures.
    /// </summary>
    /// <param name="selectionEvent">The selection event to publish.</param>
    void Publish(TextSelectionEvent selectionEvent);

    /// <summary>
    /// Gets the current number of active subscriptions.
    /// Useful for diagnostics and detecting subscription leaks.
    /// </summary>
    int SubscriberCount { get; }
}
