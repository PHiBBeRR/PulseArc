/**
 * Account Menu Component - FEATURE-016 Phase 3
 *
 * Displays user authentication status and provides sign in/out actions.
 *
 * TODO(FEATURE-016): Complete implementation during Phase 3 Step 5
 */



export interface AccountMenuProps {
  className?: string;
}

export function AccountMenu({ className }: AccountMenuProps) {
  // TODO(FEATURE-016): Implement during Phase 3 Step 5
  return (
    <div className={className}>
      <p>Account Menu - Not Implemented</p>
      {/* TODO:
        - Sign in/sign out button
        - Show user email when authenticated
        - "Not signed in" state
        - Trigger OAuth flow from menu
      */}
    </div>
  );
}
