using Microsoft.Extensions.Logging;
using TextMonitor.Service.Interfaces;
using TextMonitor.Service.Models;

namespace TextMonitor.Service.Events;

/// <summary>
/// Event aggregator for text selection events.
/// Implements thread-safe publish/subscribe pattern.
/// </summary>
public class SelectionEventAggregator : ISelectionEventAggregator
{
    private readonly ILogger<SelectionEventAggregator> _logger;
    private readonly object _lock = new();
    private readonly List<Action<TextSelectionEvent>> _subscribers = new();

    public int SubscriberCount
    {
        get
        {
            lock (_lock)
            {
                return _subscribers.Count;
            }
        }
    }

    public SelectionEventAggregator(ILogger<SelectionEventAggregator> logger)
    {
        _logger = logger ?? throw new ArgumentNullException(nameof(logger));
    }

    /// <summary>
    /// Subscribes to text selection events.
    /// </summary>
    public IDisposable Subscribe(Action<TextSelectionEvent> handler)
    {
        if (handler == null)
        {
            throw new ArgumentNullException(nameof(handler));
        }

        lock (_lock)
        {
            _subscribers.Add(handler);
            _logger.LogDebug("New subscription added. Total subscribers: {Count}", _subscribers.Count);
        }

        return new Subscription(this, handler);
    }

    /// <summary>
    /// Publishes a text selection event to all subscribers.
    /// </summary>
    public void Publish(TextSelectionEvent selectionEvent)
    {
        if (selectionEvent == null)
        {
            throw new ArgumentNullException(nameof(selectionEvent));
        }

        List<Action<TextSelectionEvent>> subscribersCopy;

        lock (_lock)
        {
            subscribersCopy = new List<Action<TextSelectionEvent>>(_subscribers);
        }

        _logger.LogDebug("Publishing selection event {EventId} to {Count} subscribers",
            selectionEvent.EventId, subscribersCopy.Count);

        // Invoke subscribers outside the lock to prevent deadlocks
        foreach (var subscriber in subscribersCopy)
        {
            try
            {
                subscriber(selectionEvent);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error invoking subscriber for event {EventId}", selectionEvent.EventId);
            }
        }
    }

    /// <summary>
    /// Unsubscribes a handler.
    /// </summary>
    private void Unsubscribe(Action<TextSelectionEvent> handler)
    {
        lock (_lock)
        {
            _subscribers.Remove(handler);
            _logger.LogDebug("Subscription removed. Remaining subscribers: {Count}", _subscribers.Count);
        }
    }

    /// <summary>
    /// Subscription token for unsubscribing.
    /// </summary>
    private class Subscription : IDisposable
    {
        private readonly SelectionEventAggregator _aggregator;
        private readonly Action<TextSelectionEvent> _handler;
        private bool _disposed;

        public Subscription(SelectionEventAggregator aggregator, Action<TextSelectionEvent> handler)
        {
            _aggregator = aggregator;
            _handler = handler;
        }

        public void Dispose()
        {
            if (!_disposed)
            {
                _aggregator.Unsubscribe(_handler);
                _disposed = true;
            }
        }
    }
}
