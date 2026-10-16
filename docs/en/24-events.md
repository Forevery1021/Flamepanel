# 24 · Events, notifications, and persistent tasks

> Condensed English translation. [Complete Chinese original](../../Doc/24-事件与通知系统.md) is retained unchanged. Detailed tables, examples, and historical evidence omitted here still require unabridged translation. Source baseline: `7288d9b199e1b09b529fbaae38303f4c82f38612`.

## Event architecture

An in-process EventBus decouples business actions from side effects such as notifications, audit, and coordination. `DomainEvent` defines the event vocabulary; subscribers implement event handling; the composition root connects them to concrete notifiers and persistence.

Publish through the existing service/event abstractions after examining current ordering and failure behavior. Do not instantiate an SMTP transport in each handler or create a second event bus per request.

## Reliability boundary

Broadcast delivery and later subscriber persistence are not an atomic business-transaction/outbox commit. Handle lag, bounded queues, shutdown, retry, idempotence, and duplicate processing explicitly. Stronger transactional guarantees require a separate design decision and tests.

Email notification remains feature/configuration dependent. Credentials and provider errors must be redacted; notification failure should follow the existing policy rather than silently changing business outcome.

## Extending events

Add a domain event with stable fields, connect the handler at the composition root, define allowed side effects, and test publish/consume/retry/failure behavior. Keep proposed event catalogs separate from actually implemented events. API/permission changes need their own contract synchronization.

## Persistent task center

The task center uses acknowledged persistence. Preserve enqueue/acknowledgement semantics, task state transitions, cancellation, deadlines, restart reconciliation, and shutdown drain. Writes accepted after a request disconnect must still be handled by the owning lifecycle. Do not equate an in-memory state transition with a committed task record.

Full event enums, notifier configuration, code templates, and the detailed B02b acknowledgement design remain in the Chinese edition and implementation.
