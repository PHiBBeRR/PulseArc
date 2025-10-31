# Frontend Merge Readiness - Detailed Tracking

**Status:** 🔴 BLOCKERS IDENTIFIED - Build & Test Failures
**Created:** 2025-10-31
**Updated:** 2025-10-31
**Owner:** TBD
**Target:** Phase 4 API Integration
**Estimated Duration:** 2-3 days (16-24 hours)

---

## Executive Summary

The frontend codebase requires **critical fixes** before merge and Phase 4 API integration. Current state:
- ❌ **TypeScript compilation failing** (44 errors)
- ❌ **Test suite at 48% failure rate** (344/721 tests failing)
- ⚠️ **20+ incomplete feature TODOs** (Phase 3 Step 5 stubs)
- ✅ **Good architecture** (React 19, Vite 7, modern tooling)

**Why This Matters:**
- Phase 4 depends on stable frontend for API integration
- TypeScript errors block production builds
- Failing tests hide real bugs during migration
- Incomplete features create confusion about scope

**Critical Path:**
1. Fix build blockers (input-otp dependency, TypeScript config)
2. Fix test environment (vitest config, global types)
3. Resolve or remove incomplete features
4. Verify Tauri IPC contracts before Phase 4

---

## Current State Analysis

### Build Status

| Component | Status | Pass Rate | Blocker? |
|-----------|--------|-----------|----------|
| TypeScript Compilation | ❌ Failing | 0% (44 errors) | **YES** |
| Vitest Tests | ❌ Failing | 52% (221/721 pass) | **YES** |
| ESLint | ⚠️ Unknown | - | No |
| Prettier | ⚠️ Unknown | - | No |
| Production Build | ❌ Blocked | - | **YES** |

### Error Breakdown

**TypeScript Errors (44 total):**
- `global` type missing: 38 errors (test files)
- Missing `input-otp` package: 1 error
- Recharts type mismatches: 6 errors (`chart.tsx`)
- Node.js `fs` import in browser: 1 error

**Test Failures (344 total):**
- "document is not defined": ~150 errors
- Incomplete test stubs: ~20 tests (Phase 3 Step 5)
- Mock/setup issues: ~174 tests

### Technology Stack

✅ **Current Versions:**
- React: 19.2.0
- TypeScript: 5.9.3
- Vite: 7.1.11
- Vitest: 4.0.2
- Tauri: 2.9.0
- Node: v24.9.0
- pnpm: 10.17.1

---

## Phase Breakdown

### Phase 1: Fix Build (Priority: P0 - Critical)

**Duration:** 4-6 hours
**Blocker:** Must complete before any other work
**Goal:** Get `pnpm build:check` passing

#### Task 1.1: Add Missing Dependencies ✅ (30 min)

**Issue:** `input-otp` package used but not in package.json

**Action:**
```bash
pnpm add input-otp
```

**Acceptance Criteria:**
- [ ] `input-otp` package installed
- [ ] No "Cannot find module 'input-otp'" error
- [ ] Version compatible with React 19

**Files Affected:**
- `package.json`
- `pnpm-lock.yaml`

---

#### Task 1.2: Fix TypeScript Test Configuration (2-3 hours)

**Issue:** Test files can't find `global` type (38 errors)

**Root Cause:** Missing vitest types and test environment globals

**Action:**
1. Create `vitest.config.ts` in project root
2. Update `tsconfig.json` to include vitest types
3. Update test setup to properly define globals

**Implementation:**

```typescript
// vitest.config.ts
import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: './frontend/shared/test/setup.ts',
    include: ['frontend/**/*.test.{ts,tsx}'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      exclude: [
        'node_modules/',
        'frontend/shared/test/',
        '**/*.test.{ts,tsx}',
        '**/*.config.{ts,js}',
      ],
    },
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './frontend'),
      '@components': path.resolve(__dirname, './frontend/components'),
      '@features': path.resolve(__dirname, './frontend/features'),
      '@shared': path.resolve(__dirname, './frontend/shared'),
    },
  },
});
```

```json
// tsconfig.json - Update types array
{
  "compilerOptions": {
    "types": ["vite/client", "vitest/globals", "@testing-library/jest-dom"]
  }
}
```

**Acceptance Criteria:**
- [ ] `vitest.config.ts` created with proper jsdom setup
- [ ] `tsconfig.json` includes vitest types
- [ ] No "Cannot find name 'global'" errors
- [ ] `globals: true` enables `describe`, `it`, `expect` without imports

**Files Created:**
- `vitest.config.ts`

**Files Updated:**
- `tsconfig.json`

---

#### Task 1.3: Fix Recharts Type Errors (1-2 hours)

**Issue:** `chart.tsx` has type mismatches with recharts (6 errors)

**Root Cause:** Recharts 3.3.0 types don't match component usage

**Action:**
1. Review [shared/components/ui/chart.tsx](../frontend/shared/components/ui/chart.tsx)
2. Add explicit type assertions or update component props
3. Option: Add `@types/recharts` if missing, or suppress specific errors with `// @ts-expect-error`

**Error Examples:**
```
chart.tsx(101,3): error TS2339: Property 'payload' does not exist on type ...
chart.tsx(106,3): error TS2339: Property 'label' does not exist on type ...
chart.tsx(164,23): error TS7006: Parameter 'item' implicitly has an 'any' type.
```

**Recommended Fix Pattern:**
```typescript
// Add explicit payload type
interface ChartPayload {
  payload?: Record<string, unknown>;
  label?: string;
}

// Or use type assertion
const payload = (props as { payload?: Record<string, unknown> }).payload;
```

**Acceptance Criteria:**
- [ ] No TypeScript errors in `chart.tsx`
- [ ] Charts render correctly in analytics views
- [ ] Types are properly documented

**Files Updated:**
- `frontend/shared/components/ui/chart.tsx`

---

#### Task 1.4: Remove Invalid Node.js Import (15 min)

**Issue:** `validation.test.ts` imports `fs` module (browser context)

**Action:**
Remove or mock the `fs` import in test file

**Files Updated:**
- `frontend/shared/types/generated/validation.test.ts`

**Acceptance Criteria:**
- [ ] No "Cannot find module 'fs'" error
- [ ] Test uses alternative method (mock data, fetch, etc.)

---

### Phase 2: Fix Test Suite (Priority: P0 - Critical)

**Duration:** 4-8 hours
**Blocker:** Must complete before Phase 4
**Goal:** Get test pass rate to >90%

#### Task 2.1: Fix Test Environment Setup (2-3 hours)

**Issue:** Many tests fail with "document is not defined" (~150 failures)

**Root Cause:** jsdom environment not properly configured

**Action:**
1. Verify `vitest.config.ts` has `environment: 'jsdom'`
2. Update test setup to ensure document/window available
3. Add missing JSDOM polyfills

**Update `frontend/shared/test/setup.ts`:**

```typescript
import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/react';
import { afterEach, vi } from 'vitest';

// Cleanup after each test
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  vi.clearAllTimers();
  vi.unstubAllGlobals();
});

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  transformCallback: vi.fn((callback) => callback),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
  emit: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: vi.fn(() => ({
    setSize: vi.fn(),
    hide: vi.fn(),
    show: vi.fn(),
    setFocus: vi.fn(),
  })),
  LogicalSize: vi.fn().mockImplementation(function (
    this: { width: number; height: number },
    width: number,
    height: number
  ) {
    this.width = width;
    this.height = height;
    return this;
  }),
}));

// Mock haptic feedback
vi.mock('@/shared/utils', () => ({
  haptic: {
    light: vi.fn(),
    medium: vi.fn(),
    heavy: vi.fn(),
  },
}));

// JSDOM environment polyfills
if (typeof global !== 'undefined') {
  // Mock ResizeObserver for Radix UI
  global.ResizeObserver = class ResizeObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
  };

  // Mock IntersectionObserver
  global.IntersectionObserver = class IntersectionObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
  } as never;

  // Mock matchMedia
  Object.defineProperty(global, 'matchMedia', {
    writable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  });
}

// Mock PointerEvent for Radix UI
if (!globalThis.PointerEvent) {
  class PointerEvent extends MouseEvent {
    height?: number;
    isPrimary?: boolean;
    pointerId?: number;
    pointerType?: string;
    pressure?: number;
    tangentialPressure?: number;
    tiltX?: number;
    tiltY?: number;
    twist?: number;
    width?: number;

    constructor(type: string, params: PointerEventInit = {}) {
      super(type, params);
      this.pointerId = params.pointerId ?? 0;
      this.width = params.width ?? 0;
      this.height = params.height ?? 0;
      this.pressure = params.pressure ?? 0;
      this.tangentialPressure = params.tangentialPressure ?? 0;
      this.tiltX = params.tiltX ?? 0;
      this.tiltY = params.tiltY ?? 0;
      this.twist = params.twist ?? 0;
      this.pointerType = params.pointerType ?? '';
      this.isPrimary = params.isPrimary ?? false;
    }
  }
  globalThis.PointerEvent = PointerEvent as never;
}

// Mock hasPointerCapture and pointer capture methods
if (typeof Element !== 'undefined') {
  Element.prototype.hasPointerCapture =
    Element.prototype.hasPointerCapture ||
    function () {
      return false;
    };
  Element.prototype.setPointerCapture = Element.prototype.setPointerCapture || function () {};
  Element.prototype.releasePointerCapture =
    Element.prototype.releasePointerCapture || function () {};
}

// Mock Audio API for audio service tests
if (typeof global !== 'undefined') {
  global.Audio = class MockAudio {
    src = '';
    volume = 1;
    play = vi.fn().mockResolvedValue(undefined);
    pause = vi.fn();
    load = vi.fn();
    addEventListener = vi.fn();
    removeEventListener = vi.fn();
  } as never;
}
```

**Acceptance Criteria:**
- [ ] All Radix UI components render in tests
- [ ] No "document is not defined" errors
- [ ] Audio service tests pass
- [ ] Test pass rate increases to >80%

**Files Updated:**
- `frontend/shared/test/setup.ts`

---

#### Task 2.2: Fix Failing Component Tests (2-4 hours)

**Issue:** 174 tests fail due to various mock/setup issues

**Action:** Systematically fix failing test files:

1. **Timer Tests** (MainTimer.test.tsx)
   - Fix global Audio mocks
   - Verify timer state transitions

2. **Activity Tracker Tests** (ActivityTrackerView.test.tsx)
   - Fix global Audio mocks
   - Update IPC invoke mocks

3. **WBS Validation Tests** (WbsValidation.test.tsx)
   - Mock document properly for rendering
   - Update SAP service mocks

4. **Settings Tests** (various)
   - Fix calendar provider mocks
   - Update sync status mocks

**Strategy:**
```bash
# Run tests file-by-file to isolate issues
pnpm test MainTimer.test.tsx
pnpm test ActivityTrackerView.test.tsx
# etc.
```

**Acceptance Criteria:**
- [ ] All timer tests passing
- [ ] All activity tracker tests passing
- [ ] All WBS validation tests passing
- [ ] Test pass rate >85%

---

### Phase 3: Complete or Remove Incomplete Features (Priority: P1 - High)

**Duration:** 4-6 hours
**Goal:** Resolve 20+ Phase 3 Step 5 TODOs

#### Task 3.1: Complete SyncStatus Component (2 hours)

**Files:**
- `frontend/features/settings/components/__tests__/SyncStatus.test.tsx`

**Current State:** All tests stubbed with "TODO(FEATURE-016): Implement during Phase 3 Step 5"

**Options:**
1. **Implement fully** - Add actual sync status UI
2. **Remove temporarily** - Delete component and tests if not in Phase 3 scope
3. **Mark as deferred** - Add clear "Post-Phase 4" comment

**Recommended Action:** Remove temporarily (not blocking Phase 4)

```bash
# Move to deferred directory
mkdir -p frontend/features/settings/components/__deferred__
git mv frontend/features/settings/components/__tests__/SyncStatus.test.tsx \
       frontend/features/settings/components/__deferred__/
```

**Acceptance Criteria:**
- [ ] Decision documented in this file
- [ ] If removed: tests no longer fail
- [ ] If implemented: 6 tests passing

---

#### Task 3.2: Complete MainApiSettings Component (1-2 hours)

**Files:**
- `frontend/features/settings/components/__tests__/MainApiSettings.test.tsx`

**Current State:** Incomplete implementation with TODOs

**Options:**
1. Implement basic API settings UI
2. Remove if not needed for Phase 4
3. Stub with minimal implementation

**Recommended Action:** Defer to Phase 4 (remove from test suite)

**Acceptance Criteria:**
- [ ] Tests removed or passing
- [ ] Component decision documented

---

#### Task 3.3: Complete Calendar Settings Tests (1-2 hours)

**Files:**
- `frontend/features/settings/components/SettingsView.calendar.test.tsx`

**Current State:** 9 test stubs with "// TODO: Implement with actual SettingsView component"

**Action:** Review calendar integration (Task 3C.5 in Phase 3) and either:
1. Implement calendar settings UI
2. Remove tests until calendar feature complete

**Note:** Calendar integration (Phase 3C.5) is marked complete, so UI should be implementable

**Acceptance Criteria:**
- [ ] Calendar settings render in SettingsView
- [ ] Tests updated with actual assertions
- [ ] All 9 tests passing or removed

---

### Phase 4: Verify Tauri IPC Contracts (Priority: P1 - High)

**Duration:** 2-3 hours
**Goal:** Ensure frontend IPC calls match backend commands

#### Task 4.1: Audit Tauri Command Signatures (1-2 hours)

**Current Tauri Commands** (crates/api/src/commands/):
- `calendar.rs` - Calendar integration
- `projects.rs` - Project/WBS queries
- `suggestions.rs` - Autocomplete suggestions
- `tracking.rs` - Activity tracking control

**Action:**
1. Document all command signatures
2. Compare with frontend IPC calls
3. Identify mismatches

**Frontend IPC Locations:**
- `frontend/shared/services/ipc/TauriAPI.ts`
- `frontend/features/*/services/*.ts`

**Acceptance Criteria:**
- [ ] All IPC calls documented
- [ ] No type mismatches
- [ ] Commands match 1:1 with frontend expectations

---

#### Task 4.2: Add IPC Type Safety (1 hour)

**Goal:** Generate TypeScript types from Rust commands

**Action:**
1. Use `ts-rs` or similar for type generation
2. Or: manually create type definitions matching Rust signatures

**Example:**
```typescript
// frontend/shared/services/ipc/types.ts
export interface StartTrackingRequest {
  wbs_code: string;
  project_name: string;
}

export interface TrackingStatus {
  is_active: boolean;
  elapsed_seconds: number;
  current_wbs?: string;
}

// Validate at runtime
export async function startTracking(req: StartTrackingRequest): Promise<void> {
  return invoke('start_tracking', { request: req });
}
```

**Acceptance Criteria:**
- [ ] Type-safe IPC wrapper functions
- [ ] Runtime validation (optional)
- [ ] Updated frontend services to use new types

---

### Phase 5: Final Validation (Priority: P0 - Critical)

**Duration:** 1-2 hours
**Goal:** All CI checks passing

#### Task 5.1: Run Full Build Pipeline (30 min)

**Commands:**
```bash
# TypeScript compilation
pnpm build:check

# Tests
pnpm test

# Linting
pnpm lint

# Formatting
pnpm format:check

# Tauri build (optional)
pnpm tauri build --debug
```

**Acceptance Criteria:**
- [ ] `pnpm build:check` ✅ passes
- [ ] `pnpm test` ✅ >90% pass rate
- [ ] `pnpm lint` ✅ 0 warnings
- [ ] `pnpm format:check` ✅ passes
- [ ] Tauri dev mode launches without errors

---

#### Task 5.2: Manual Smoke Test (30 min - 1 hour)

**Test Scenarios:**
1. **Timer Flow:**
   - [ ] Start timer with WBS code
   - [ ] Pause/resume works
   - [ ] Stop creates time entry

2. **Activity Tracker:**
   - [ ] Shows current app/window
   - [ ] Idle detection triggers
   - [ ] Activity history loads

3. **Settings:**
   - [ ] Calendar connection works (if implemented)
   - [ ] SAP settings save
   - [ ] Theme toggle works

4. **Time Entries:**
   - [ ] Day view loads entries
   - [ ] Week view displays correctly
   - [ ] Edit/delete works

5. **Navigation:**
   - [ ] All views accessible
   - [ ] No console errors
   - [ ] Window resizing works

**Acceptance Criteria:**
- [ ] All critical flows work
- [ ] No JavaScript errors in console
- [ ] UI renders correctly

---

## Success Criteria Summary

### Must Have (P0 - Blocking)
- [ ] TypeScript compilation passes (0 errors)
- [ ] Test suite >90% pass rate
- [ ] `pnpm build:check` succeeds
- [ ] Tauri dev mode launches
- [ ] No console errors in manual test

### Should Have (P1 - Important)
- [ ] All Phase 3 Step 5 TODOs resolved
- [ ] IPC contracts validated
- [ ] Linting passes with 0 warnings
- [ ] Code formatted consistently

### Nice to Have (P2 - Optional)
- [ ] Test coverage >80%
- [ ] Generated IPC types from Rust
- [ ] Complete calendar settings UI
- [ ] Performance benchmarks

---

## Risk Assessment

### High Risk
1. **Test Environment Issues** - If jsdom setup fails, many tests will remain broken
   - **Mitigation:** Use known-good vitest.config.ts template
   - **Fallback:** Temporarily skip failing tests with `.skip()`

2. **Recharts Type Errors** - May require library upgrade or major refactoring
   - **Mitigation:** Use type assertions as temporary fix
   - **Fallback:** Suppress errors with `@ts-expect-error` and document

3. **IPC Mismatches** - Frontend may expect commands not in new backend
   - **Mitigation:** Document all calls before Phase 4
   - **Fallback:** Add backward-compatible shims

### Medium Risk
1. **Incomplete Features** - Phase 3 Step 5 TODOs may be critical
   - **Mitigation:** Review with product owner
   - **Fallback:** Defer to Phase 4

2. **Performance Issues** - React 19 + Tauri 2 may have regressions
   - **Mitigation:** Benchmark before merge
   - **Fallback:** Document known issues

---

## Dependencies & Blockers

### External Dependencies
- **Phase 3 Infrastructure** (partially blocking)
  - Phase 3C (SAP integration) - frontend expects SAP commands
  - Phase 3C (Calendar) - settings UI expects calendar API

### Internal Dependencies
- None - frontend work can proceed independently

### Blockers
- **Build failures** - Must fix before any testing
- **Test environment** - Must configure before test fixes

---

## Timeline Estimate

### Optimistic (16 hours / 2 days)
- Phase 1 (Fix Build): 4 hours
- Phase 2 (Fix Tests): 4 hours
- Phase 3 (Complete Features): 4 hours
- Phase 4 (Verify IPC): 2 hours
- Phase 5 (Validation): 2 hours

### Realistic (20 hours / 2.5 days)
- Phase 1 (Fix Build): 6 hours
- Phase 2 (Fix Tests): 6 hours
- Phase 3 (Complete Features): 4 hours
- Phase 4 (Verify IPC): 2 hours
- Phase 5 (Validation): 2 hours

### Pessimistic (24+ hours / 3 days)
- Phase 1 (Fix Build): 6 hours
- Phase 2 (Fix Tests): 8 hours (jsdom issues)
- Phase 3 (Complete Features): 6 hours (must implement)
- Phase 4 (Verify IPC): 3 hours (mismatches found)
- Phase 5 (Validation): 1-2 hours

---

## Recommendations

### Immediate Actions (Today)
1. ✅ Create this tracking document
2. ⏳ Start Phase 1: Add `input-otp` dependency
3. ⏳ Create `vitest.config.ts`

### Short-Term (This Week)
1. Complete Phase 1 & 2 (build + tests)
2. Make decisions on Phase 3 Step 5 features
3. Document IPC contracts

### Before Phase 4 (Next Week)
1. All tests passing (>90%)
2. Clean build
3. IPC contracts verified
4. Manual smoke test complete

---

## Open Questions

1. **Phase 3 Step 5 Features:**
   - Are SyncStatus, MainApiSettings, Calendar tests required for Phase 4?
   - Can we defer to post-Phase 4?
   - Who is the product owner to confirm scope?

2. **Calendar Integration:**
   - Phase 3C.5 shows calendar as complete (2,831 LOC)
   - Should frontend calendar UI be fully implemented now?
   - Or is it feature-flagged for later release?

3. **IPC Versioning:**
   - How will we handle IPC changes during Phase 4?
   - Do we need versioning or compatibility layer?

4. **Testing Strategy:**
   - Target test coverage for Phase 4?
   - Integration test plan for Tauri commands?

---

## Document Change Log

### Version 1.0 (2025-10-31) - Initial Creation

**Created by:** Assessment of frontend codebase (Oct 31, 2025)

**Key Findings:**
- 44 TypeScript errors blocking build
- 344/721 tests failing (48% failure rate)
- 20+ incomplete Phase 3 Step 5 TODOs
- Missing vitest configuration
- Good architecture foundation

**Recommendations:**
- Fix build as P0 (critical blocker)
- Fix test suite as P0 (critical blocker)
- Make decisions on incomplete features
- Verify IPC contracts before Phase 4

**Status:** Ready for execution. Awaiting team assignment and Phase 3 Step 5 scope confirmation.

---

## Next Steps

### For Developer Starting This Work:
1. Read this entire document
2. Set up environment: `pnpm install`
3. Reproduce build failure: `pnpm build:check`
4. Start with Phase 1, Task 1.1
5. Update task statuses in this document as you progress

### For Code Reviewers:
1. Verify each phase acceptance criteria
2. Manual smoke test all critical flows
3. Check for new test failures introduced
4. Validate IPC contracts match backend

### For Project Manager:
1. Confirm Phase 3 Step 5 feature scope
2. Assign developer(s) to frontend work
3. Schedule Phase 4 kickoff after frontend merge
4. Review open questions section

---

**Document Status:** ✅ Ready for Use
**Last Reviewed:** 2025-10-31
**Next Review:** After Phase 1 completion

---

**END OF FRONTEND MERGE READINESS TRACKING DOCUMENT**
