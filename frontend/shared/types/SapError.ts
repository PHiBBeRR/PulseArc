// FEATURE-020 Phase 4.4: SAP Error Type
// TypeScript representation of Rust SapError struct
//
// Note: This is a manual type definition until SapError is annotated with ts-rs
// in the Rust codebase and auto-generated.

export type SapErrorCategory =
  | 'NetworkOffline'
  | 'NetworkTimeout'
  | 'ServerUnavailable'
  | 'Authentication'
  | 'RateLimited'
  | 'Validation'
  | 'Unknown';

export type SapError = {
  category: SapErrorCategory;
  message: string;
  user_message: string;
  is_retriable: boolean;
  should_backoff: boolean;
};
