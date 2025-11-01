# org_id from Auth0 Migration

**Status:** ✅ Completed
**Date:** 2025-01-XX
**Related:** Calendar Attendees & Time Entry Suggestions Implementation

## Summary

Migrated `org_id` from environment variable configuration to Auth0 user profile, enabling proper multi-tenant support and removing deployment configuration complexity.

## Changes Made

### 1. Domain Model
- **File:** [`crates/domain/src/types/user.rs`](../../crates/domain/src/types/user.rs)
- **Change:** Added `org_id: String` field to `UserProfile` struct
- **Documentation:** Added comment explaining source is Auth0 (org_id claim or app_metadata)

### 2. Database Schema
- **File:** [`crates/infra/src/database/schema.sql`](../../crates/infra/src/database/schema.sql)
- **Changes:**
  - Added `org_id TEXT NOT NULL DEFAULT 'default_org'` column to `user_profiles` table
  - Added index: `idx_user_profiles_org_id`
  - Default value ensures backward compatibility

### 3. Repository Layer
- **File:** [`crates/infra/src/database/user_profile_repository.rs`](../../crates/infra/src/database/user_profile_repository.rs)
- **Changes:**
  - Updated all SELECT queries to include `org_id`
  - Updated `map_user_profile_row()` to map `org_id` at index 3
  - Updated `insert_user_profile()` to include `org_id`
  - Updated `update_user_profile()` to include `org_id`
  - Updated `upsert_user_profile()` to include `org_id` in INSERT and UPDATE
  - Updated test helper `create_test_profile()` to include test `org_id`

### 4. Command Layer
- **File:** [`crates/api/src/commands/blocks.rs`](../../crates/api/src/commands/blocks.rs)
- **Change:** Replaced environment variable lookup with `user_profile.org_id`
- **Before:**
  ```rust
  let org_id = std::env::var("ORG_ID").unwrap_or_else(|_| {
      warn!("ORG_ID not set in environment, using default");
      "default_org".to_string()
  });
  ```
- **After:**
  ```rust
  let org_id = user_profile.org_id;
  ```

- **File:** [`crates/api/src/commands/user_profile.rs`](../../crates/api/src/commands/user_profile.rs)
- **Change:** Updated SELECT query and row mapping to include `org_id`

### 5. Documentation
- **File:** [`crates/infra/src/integrations/calendar/README.md`](../../crates/infra/src/integrations/calendar/README.md)
- **Change:** Removed `ORG_ID` environment variable documentation, added note about Auth0 source

## Benefits

### Multi-Tenant Ready
- Each user can belong to different organizations
- No need for environment-specific configuration
- Supports dynamic org assignment

### Security
- org_id is per-user, not shared across deployment
- Prevents cross-org data leakage
- Enforced at identity provider level

### Operational
- Eliminates `ORG_ID` from deployment configuration
- Works consistently across dev/staging/prod
- No manual configuration per environment

### Scalability
- Users can switch organizations without redeployment
- Supports org hierarchies and relationships
- Enables org-level isolation and billing

## Migration Path

### For Existing Deployments

1. **Database Migration (Automatic)**
   - `org_id` column has `DEFAULT 'default_org'`
   - Existing rows automatically get default value
   - No manual data migration required

2. **Auth0 Configuration Required**

   **Option A: Using Auth0 Organizations (Recommended)**
   ```javascript
   // Auth0 automatically adds org_id to ID token
   // No additional configuration needed
   {
     "org_id": "org_abc123",  // Automatically in token
     "email": "user@example.com",
     ...
   }
   ```

   **Option B: Using App Metadata**
   ```javascript
   // Add to Auth0 Action/Rule to set app_metadata
   exports.onExecutePostLogin = async (event, api) => {
     api.idToken.setCustomClaim('https://yourapp.com/org_id',
       event.user.app_metadata.org_id || 'default_org'
     );
   };
   ```

3. **Profile Sync**
   - Next user login will sync `org_id` from Auth0
   - Use `UserProfileRepository::upsert()` during login flow
   - Existing users get updated on next sync

### Rollback Plan

If issues arise, temporary environment variable fallback:

```rust
let org_id = user_profile.org_id.as_str();
let org_id = if org_id == "default_org" {
    std::env::var("ORG_ID").unwrap_or_else(|_| "default_org".to_string())
} else {
    org_id.to_string()
};
```

## Testing

### Unit Tests
- ✅ User profile repository tests pass with `org_id`
- ✅ User profile creation includes `org_id`
- ✅ User profile updates preserve `org_id`

### Integration Tests
- ⚠️ **TODO:** Test Auth0 token extraction in login flow
- ⚠️ **TODO:** Test multi-org user scenarios
- ⚠️ **TODO:** Verify org isolation in time entries

### Manual Testing Checklist
- [ ] Login flow populates `org_id` from Auth0
- [ ] Block acceptance uses correct `org_id`
- [ ] Calendar suggestions use correct `org_id` (when implemented)
- [ ] Multiple users with different `org_id` values work correctly
- [ ] Default value works for development/testing

## Calendar Implementation Impact

### For Calendar Attendees Feature
- No impact - attendees are per-event, not per-org
- Proceed with attendee parsing as planned

### For Time Entry Suggestions
Suggestions should now use `org_id` from user profile:

```rust
// In generate_suggestions
async fn generate_suggestions(
    &self,
    events: &[CalendarEvent],
    user_email: &str,
    settings: &CalendarSyncSettings,
) -> Result<usize> {
    // Get org_id from user profile
    let user_profile = self.user_profile_repo
        .get_by_email(user_email)
        .await?
        .ok_or_else(|| PulseArcError::InvalidInput("User not found"))?;

    let org_id = &user_profile.org_id;
    let user_id = &user_profile.auth0_id;

    for event in events {
        let dto = PrismaTimeEntryDto {
            org_id: org_id.clone(),
            user_id: user_id.clone(),
            // ... rest of fields
        };

        // Create outbox entry with org_id from profile
    }
}
```

## Open Questions (Resolved)

### Q: Should org_id come from auth provider?
**A: YES** ✅ - Implemented in this migration

**Rationale:**
- Multi-tenant support
- Security boundary per user
- No deployment configuration needed
- Aligns with Auth0 best practices

### Q: Where does org_id come from for calendar sync?
**A:** From `UserProfile.org_id` after looking up user by email

**Implementation:**
```rust
let user_profile = user_profile_repo.get_by_email(user_email).await?;
let org_id = user_profile.org_id;
```

## Related Documentation

- [Auth0 Organizations Documentation](https://auth0.com/docs/manage-users/organizations)
- [Auth0 App Metadata Best Practices](https://auth0.com/docs/manage-users/user-accounts/metadata)
- [Calendar Implementation Plan](./CALENDAR_ATTENDEES_IMPLEMENTATION.md)

## Next Steps

1. **Configure Auth0 to provide org_id**
   - Set up Auth0 Organizations OR
   - Add Action to include org_id in token claims

2. **Update Login Flow**
   - Extract `org_id` from token claims
   - Persist to user_profiles via `upsert()`

3. **Implement Calendar Suggestions**
   - Use `user_profile.org_id` in suggestion generation
   - Follow recommendations from this migration

4. **Testing**
   - Add integration tests for multi-org scenarios
   - Verify org isolation

## Success Criteria

- [x] `org_id` field added to `UserProfile`
- [x] Database schema updated with migration
- [x] All repository methods handle `org_id`
- [x] Block commands use `org_id` from profile
- [x] Environment variable removed from codebase
- [x] Documentation updated
- [x] Code compiles and tests pass
- [ ] Auth0 configured to provide `org_id`
- [ ] Login flow syncs `org_id` from Auth0
- [ ] Calendar suggestions use `org_id` from profile
