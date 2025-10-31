/**
 * FEATURE-017: Calendar Types Code Generation Tests
 * Tests for TypeScript types generated from Rust via ts-rs
 *
 * Validates that calendar-related TypeScript types are correctly generated
 * from Rust structs, ensuring type safety between backend and frontend.
 *
 * Test Coverage:
 * - Type Generation: CalendarConnectionStatus includes all required fields
 * - Provider Field: Provider field exists and is string type
 * - Type Matching: TypeScript types match Rust struct definitions
 * - Field Types: Correct type conversion (i64 → number, String → string)
 * - Optional Fields: Proper handling of Option<T> → T | null
 * - Multi-Provider Support: Handles different calendar providers (google, microsoft)
 *
 * Note: Tests validate types after running `npm run codegen`
 */

import type { CalendarConnectionStatus } from '@/shared/types/generated/CalendarConnectionStatus';
import { describe, expect, it } from 'vitest';

describe('Calendar Types - Code Generation', () => {
  // ==========================================================================
  // TEST CATEGORY 1: Type Generation Validation (3 tests)
  // ==========================================================================

  it('FEATURE-017: should have CalendarConnectionStatus type with provider field', () => {
    // AC: CalendarConnectionStatus type exported from generated types
    // AC: Type includes provider field
    const status: CalendarConnectionStatus = {
      provider: 'google',
      connected: true,
      email: 'test@example.com',
      lastSync: 1705316400,
      syncEnabled: true,
    };

    expect(status.provider).toBe('google');
    expect(typeof status.provider).toBe('string');
  });

  it('FEATURE-017: should have provider field as string type', () => {
    // AC: provider field is string type (not enum)
    // AC: Accepts "google", "microsoft", and other string values
    const googleStatus: CalendarConnectionStatus = {
      provider: 'google',
      connected: true,
      email: 'user@gmail.com',
      lastSync: 1705316400,
      syncEnabled: true,
    };

    const microsoftStatus: CalendarConnectionStatus = {
      provider: 'microsoft',
      connected: true,
      email: 'user@outlook.com',
      lastSync: 1705316500,
      syncEnabled: true,
    };

    expect(typeof googleStatus.provider).toBe('string');
    expect(typeof microsoftStatus.provider).toBe('string');
    expect(googleStatus.provider).toBe('google');
    expect(microsoftStatus.provider).toBe('microsoft');
  });

  it('FEATURE-017: should match Rust CalendarConnectionStatus structure', () => {
    // AC: TypeScript type matches Rust struct field names
    // AC: All fields from Rust struct present in TS type
    // AC: Field types match (string, boolean, number | null, etc.)

    // Expected structure from Rust:
    // pub struct CalendarConnectionStatus {
    //     pub provider: String,
    //     pub connected: bool,
    //     pub email: Option<String>,
    //     pub last_sync: Option<i64>,
    //     pub sync_enabled: bool,
    // }

    const status: CalendarConnectionStatus = {
      provider: 'google', // String
      connected: true, // bool
      email: 'test@test.com', // Option<String> (nullable)
      lastSync: 1705316400, // Option<i64> (nullable number)
      syncEnabled: true, // bool
    };

    // Verify all required fields are present and correct types
    expect(typeof status.provider).toBe('string');
    expect(typeof status.connected).toBe('boolean');
    expect(typeof status.email).toBe('string'); // Can also be null
    expect(typeof status.lastSync).toBe('number'); // Can also be null
    expect(typeof status.syncEnabled).toBe('boolean');

    // Verify nullable fields work
    const statusWithNulls: CalendarConnectionStatus = {
      provider: 'microsoft',
      connected: false,
      email: null,
      lastSync: null,
      syncEnabled: false,
    };

    expect(statusWithNulls.email).toBeNull();
    expect(statusWithNulls.lastSync).toBeNull();
  });
});
