//! Simplified lifecycle integration tests
//!
//! Tests state management patterns from lifecycle module

use std::sync::Arc;

use pulsearc_common::lifecycle::ManagedState;

/// Validates basic ManagedState operations for thread-safe state management.
///
/// This test ensures ManagedState can safely read and write state values
/// in an async context. ManagedState provides a high-level API for managing
/// shared state with proper synchronization primitives.
///
/// # Test Steps
/// 1. Create ManagedState with initial value of 42
/// 2. Read state and verify initial value
/// 3. Write state to new value (100)
/// 4. Verify state updated correctly
/// 5. Modify state with closure (add 10)
/// 6. Verify modification applied correctly (result: 110)
#[tokio::test(flavor = "multi_thread")]
async fn test_state_manager_basic() {
    let mut state = ManagedState::new(42);

    // Read state
    let value = state.read().await;
    assert_eq!(*value, 42);
    drop(value); // Release read lock

    // Update state
    state.set(100).await;
    let value = state.read().await;
    assert_eq!(*value, 100);
    drop(value); // Release read lock

    // Modify state with function
    state.modify(|v| *v += 10).await;
    let value = state.read().await;
    assert_eq!(*value, 110);
}

/// Validates ManagedState read and write operations for concurrent access.
///
/// This test ensures ManagedState provides safe read/write access to shared
/// state, using appropriate locking mechanisms to prevent data races.
/// ManagedState offers a lower-level API compared to StateRegistry for more
/// control.
///
/// # Test Steps
/// 1. Create ManagedState with initial string value
/// 2. Read state and verify initial value ("initial")
/// 3. Write new state value ("updated")
/// 4. Read state again and verify update
/// 5. Confirm state modifications are atomic and visible
#[tokio::test(flavor = "multi_thread")]
async fn test_managed_state_operations() {
    let mut state = ManagedState::new(String::from("initial"));

    // Read state
    let value = state.read().await;
    assert_eq!(*value, "initial");
    drop(value); // Release read lock

    // Write state
    state.set(String::from("updated")).await;

    let value = state.read().await;
    assert_eq!(*value, "updated");
}

/// Validates thread-safe concurrent state modifications from multiple tasks.
///
/// This test ensures ManagedState can handle concurrent modifications from
/// multiple async tasks without data races or lost updates. Tests with 100
/// concurrent increments to verify proper synchronization and atomicity.
///
/// # Test Steps
/// 1. Create ManagedState with initial value of 0
/// 2. Wrap in Arc for sharing across tasks
/// 3. Spawn 100 concurrent tasks, each incrementing the counter
/// 4. Wait for all tasks to complete
/// 5. Verify final value is exactly 100 (no lost updates)
/// 6. Confirm all modifications were properly synchronized
#[tokio::test(flavor = "multi_thread")]
async fn test_concurrent_state_modifications() {
    let state = Arc::new(ManagedState::new(0));

    let mut handles = vec![];

    // Spawn multiple tasks that increment the state
    for _ in 0..100 {
        let state_clone = Arc::clone(&state);
        let handle = tokio::spawn(async move {
            state_clone.modify(|v| *v += 1).await;
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.expect("Task should complete");
    }

    // Verify final value
    let value = state.read().await;
    assert_eq!(*value, 100);
}

/// Validates ManagedState with complex user-defined types.
///
/// This test ensures ManagedState can handle complex types (structs with
/// multiple fields) not just primitives. Demonstrates that modification
/// closures can update specific fields while preserving others, providing
/// fine-grained state control.
///
/// # Test Steps
/// 1. Define complex UserData struct with name and count fields
/// 2. Create ManagedState with initial UserData
/// 3. Modify only the count field via closure
/// 4. Read state and verify count incremented
/// 5. Confirm name field unchanged
/// 6. Verify complex type handling works correctly
#[tokio::test(flavor = "multi_thread")]
async fn test_state_with_complex_types() {
    #[derive(Clone, Debug, PartialEq)]
    struct UserData {
        name: String,
        count: u32,
    }

    let state = ManagedState::new(UserData { name: "Alice".to_string(), count: 0 });

    // Modify complex state
    state
        .modify(|data| {
            data.count += 1;
        })
        .await;

    let value = state.read().await;
    assert_eq!(value.name, "Alice");
    assert_eq!(value.count, 1);
}
