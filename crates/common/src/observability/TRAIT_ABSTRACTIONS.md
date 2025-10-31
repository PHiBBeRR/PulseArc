# Trait Abstractions for Observability

This document explains how to use the trait abstractions in `common::observability::traits` to build portable, testable components.

## Overview

Instead of depending directly on concrete implementations like `GlobalAuditLogger` or telemetry globals, components can use trait abstractions that allow for:

- **Dependency Injection**: Pass in any implementation at runtime
- **Easy Testing**: Use no-op implementations or mocks in tests
- **Portability**: Code can work with different backends
- **Flexibility**: Swap implementations without changing component code

## Available Traits

### 1. `AuditLogger`

For logging security-relevant audit events.

```rust
#[async_trait]
pub trait AuditLogger: Send + Sync + Debug {
    async fn log(&self, event: AuditLogEntry);
    async fn entry_count(&self) -> usize;
    fn is_enabled(&self) -> bool;
}
```

**Use cases**: Privacy processing, authentication, authorization checks, data access

### 2. `MetricsCollector`

For emitting performance and operational metrics.

```rust
pub trait MetricsCollector: Send + Sync + Debug {
    fn increment_counter(&self, name: &str, labels: &[(&str, &str)]);
    fn record_gauge(&self, name: &str, value: f64, labels: &[(&str, &str)]);
    fn record_histogram(&self, name: &str, value: f64, labels: &[(&str, &str)]);
    fn record_timing(&self, name: &str, duration_ms: u64, labels: &[(&str, &str)]);
}
```

**Use cases**: Performance monitoring, queue depths, cache hit rates, processing times

### 3. `Tracer`

For distributed tracing of operations across services.

```rust
#[async_trait]
pub trait Tracer: Send + Sync + Debug {
    async fn start_span(&self, operation: &str, metadata: HashMap<String, String>) -> TraceSpan;
    fn current_span(&self) -> Option<TraceSpan>;
}
```

**Use cases**: Request tracing, cross-service calls, debugging distributed systems

## Using Trait Abstractions

### Basic Pattern

```rust
use crate::common::observability::traits::{AuditLogger, MetricsCollector};
use std::sync::Arc;

pub struct MyComponent<A, M>
where
    A: AuditLogger,
    M: MetricsCollector,
{
    audit_logger: Arc<A>,
    metrics: Arc<M>,
}

impl<A, M> MyComponent<A, M>
where
    A: AuditLogger,
    M: MetricsCollector,
{
    pub fn new(audit_logger: Arc<A>, metrics: Arc<M>) -> Self {
        Self {
            audit_logger,
            metrics,
        }
    }

    pub async fn do_work(&self) {
        // Log audit event
        self.audit_logger
            .log(AuditLogEntry::new("work_started", AuditSeverity::Info))
            .await;

        // Record metrics
        self.metrics.increment_counter("work.started", &[("component", "my_component")]);

        // ... do actual work ...
    }
}
```

### Testing with No-Op Implementations

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::observability::traits::{NoOpAuditLogger, NoOpMetricsCollector};

    #[tokio::test]
    async fn test_my_component() {
        let audit = Arc::new(NoOpAuditLogger);
        let metrics = Arc::new(NoOpMetricsCollector);

        let component = MyComponent::new(audit, metrics);
        component.do_work().await;

        // Test passes without needing real audit/metrics infrastructure
    }
}
```

### Production with Real Implementations

```rust
use crate::compliance::GlobalAuditLoggerAdapter;
use crate::telemetry::GlobalMetricsCollectorAdapter;

fn main() {
    let audit = Arc::new(GlobalAuditLoggerAdapter::default());
    let metrics = Arc::new(GlobalMetricsCollectorAdapter::new());

    let component = MyComponent::new(audit, metrics);
    // Component uses real audit logging and metrics
}
```

## Migration Guide

### Before (Concrete Dependencies)

```rust
use crate::compliance::audit::{GlobalAuditLogger, AuditEvent, AuditContext};
use crate::telemetry::collector::metrics::METRICS_COLLECTED;

pub struct OldComponent {
    audit_logger: GlobalAuditLogger,
}

impl OldComponent {
    pub async fn process(&self, user_id: &str) {
        // Tightly coupled to GlobalAuditLogger
        self.audit_logger.log_event(
            AuditEvent::UserAction { user_id: user_id.to_string() },
            AuditContext::default(),
            AuditSeverity::Info,
        ).await;

        // Tightly coupled to global static
        METRICS_COLLECTED.fetch_add(1, Ordering::Relaxed);
    }
}
```

### After (Trait Abstractions)

```rust
use crate::common::observability::traits::{AuditLogger, AuditLogEntry, AuditSeverity, MetricsCollector};
use std::sync::Arc;

pub struct NewComponent<A, M>
where
    A: AuditLogger,
    M: MetricsCollector,
{
    audit_logger: Arc<A>,
    metrics: Arc<M>,
}

impl<A, M> NewComponent<A, M>
where
    A: AuditLogger,
    M: MetricsCollector,
{
    pub fn new(audit_logger: Arc<A>, metrics: Arc<M>) -> Self {
        Self { audit_logger, metrics }
    }

    pub async fn process(&self, user_id: &str) {
        // Uses trait abstraction
        self.audit_logger
            .log(
                AuditLogEntry::new("user_action", AuditSeverity::Info)
                    .with_user(user_id)
            )
            .await;

        // Uses trait abstraction
        self.metrics.increment_counter("processed", &[("user", user_id)]);
    }
}
```

## Available Adapters

### For Production Use

- **`GlobalAuditLoggerAdapter`** (agent/compliance/adapters.rs)
  - Wraps `GlobalAuditLogger` to implement `AuditLogger` trait
  ```rust
  use crate::compliance::GlobalAuditLoggerAdapter;
  let audit = Arc::new(GlobalAuditLoggerAdapter::default());
  ```

- **`GlobalMetricsCollectorAdapter`** (agent/telemetry/adapters.rs)
  - Wraps global `METRICS_COLLECTED` to implement `MetricsCollector` trait
  ```rust
  use crate::telemetry::GlobalMetricsCollectorAdapter;
  let metrics = Arc::new(GlobalMetricsCollectorAdapter::new());
  ```

- **`SpanManagerAdapter`** (agent/telemetry/adapters.rs)
  - Wraps `SpanManager` to implement `Tracer` trait
  ```rust
  use crate::telemetry::SpanManagerAdapter;
  let tracer = Arc::new(SpanManagerAdapter::default());
  ```

### For Testing

- **`NoOpAuditLogger`** - Does nothing, always returns 0 entries
- **`NoOpMetricsCollector`** - Does nothing, no overhead
- **`NoOpTracer`** - Does nothing, returns fake spans

## Best Practices

1. **Use generics with trait bounds**:
   ```rust
   pub struct Component<A: AuditLogger, M: MetricsCollector> {
       audit: Arc<A>,
       metrics: Arc<M>,
   }
   ```

2. **Store as Arc** to allow sharing across async tasks:
   ```rust
   audit_logger: Arc<A>,  // ✅ Good
   audit_logger: A,        // ❌ Might not be Clone
   ```

3. **Don't leak trait bounds to public API** if not needed:
   ```rust
   // Internal generics
   impl<A: AuditLogger> Component<A> {
       fn internal_method(&self) { }
   }

   // Public API hides generics
   pub type ProductionComponent = Component<GlobalAuditLoggerAdapter>;
   ```

4. **Use builder pattern** for complex components:
   ```rust
   ComponentBuilder::new()
       .with_audit(Arc::new(GlobalAuditLoggerAdapter::default()))
       .with_metrics(Arc::new(GlobalMetricsCollectorAdapter::new()))
       .build()
   ```

## Example: Privacy Module Migration

See `agent/privacy/trait_based.rs` for a complete example of migrating privacy components to use trait abstractions.

## Benefits

✅ **Testability**: Use no-op implementations in unit tests
✅ **Portability**: Extract modules to separate crates
✅ **Flexibility**: Swap backends (OpenTelemetry, Prometheus, Datadog, etc.)
✅ **Performance**: Can use lightweight implementations when needed
✅ **Mocking**: Easy to create mock implementations for testing edge cases

## Next Steps

1. Gradually migrate existing components to use trait abstractions
2. Keep adapters for backward compatibility
3. Consider moving more components to use this pattern
4. Extract common modules to separate crates for wider reuse
