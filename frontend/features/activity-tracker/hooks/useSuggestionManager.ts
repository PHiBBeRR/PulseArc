import { useState, useEffect, useRef, useCallback } from 'react';
import type { SuggestionState, ActivityContext } from '../types';

interface UseSuggestionManagerProps {
  activityContext: ActivityContext | null;
  inputValue: string;
  userHasTyped: boolean;
  isTracking: boolean;
}

interface SuggestionManagerResult {
  currentSuggestion: SuggestionState | null;
  clearSuggestion: () => void;
}

const STALE_THRESHOLD_MS = 2 * 60 * 1000; // 2 minutes
const TYPING_DEBOUNCE_MS = 500; // 500ms debounce

export function useSuggestionManager({
  activityContext,
  inputValue,
  userHasTyped,
  isTracking,
}: UseSuggestionManagerProps): SuggestionManagerResult {
  const [currentSuggestion, setCurrentSuggestion] = useState<SuggestionState | null>(null);
  const previousSuggestionTextRef = useRef<string | null>(null);
  const lastActivityRef = useRef<string | null>(null);
  const typingTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const isUserTypingRef = useRef(false);

  // Debounced typing detection
  useEffect(() => {
    if (userHasTyped) {
      // User is actively typing
      isUserTypingRef.current = true;

      // Clear any existing timeout
      if (typingTimeoutRef.current) {
        clearTimeout(typingTimeoutRef.current);
      }

      // Set a timeout to detect when user stops typing
      typingTimeoutRef.current = setTimeout(() => {
        isUserTypingRef.current = false;
      }, TYPING_DEBOUNCE_MS);
    } else {
      isUserTypingRef.current = false;
    }

    return () => {
      if (typingTimeoutRef.current) {
        clearTimeout(typingTimeoutRef.current);
      }
    };
  }, [inputValue, userHasTyped]);

  // Update suggestion based on activity context
  useEffect(() => {
    if (!isTracking || !activityContext) {
      return;
    }

    const detectedActivity = activityContext.detected_activity?.trim();

    // No activity detected
    if (!detectedActivity) {
      return;
    }

    // Allow updates if:
    // 1. User hasn't typed (input is empty), OR
    // 2. User typed but stopped typing (debounce elapsed)
    const hasUserInput = inputValue.trim() !== '';
    const isCurrentlyTyping = isUserTypingRef.current;

    if (hasUserInput && isCurrentlyTyping) {
      // User is actively typing - don't update
      return;
    }

    // If user typed but stopped, allow update only if activity changed significantly
    if (hasUserInput && !isCurrentlyTyping) {
      // Check if activity matches what they typed (fuzzy match)
      const activityLower = detectedActivity.toLowerCase();
      const inputLower = inputValue.toLowerCase();
      const activityMatchesInput =
        activityLower.includes(inputLower) || inputLower.includes(activityLower);

      if (activityMatchesInput) {
        // Activity matches what they typed - don't override
        console.log('🔍 Suggestion: Activity matches user input, keeping user input');
        return;
      }
      // Activity changed significantly - allow update despite user input
      console.log('🔍 Suggestion: Activity changed significantly, updating suggestion');
    }

    // Check if activity has changed
    const activityChanged = lastActivityRef.current !== detectedActivity;

    if (activityChanged) {
      lastActivityRef.current = detectedActivity;

      // Create new suggestion
      const newSuggestion: SuggestionState = {
        text: detectedActivity,
        confidence: 0.85, // Default confidence for AI-detected activities
        timestamp: Date.now(),
        source: 'activity',
        metadata: {
          appName: activityContext.active_app.app_name,
        },
      };

      // Check if this is an update to existing suggestion
      const isUpdate =
        previousSuggestionTextRef.current !== null && previousSuggestionTextRef.current !== detectedActivity;

      if (isUpdate) {
        // Mark as updated suggestion
        newSuggestion.metadata = {
          ...newSuggestion.metadata,
          isUpdated: true,
        };
      }

      previousSuggestionTextRef.current = detectedActivity;
      setCurrentSuggestion(newSuggestion);

      // Clear the "updated" flag after animation duration (2 seconds)
      if (isUpdate) {
        setTimeout(() => {
          setCurrentSuggestion((prev) => {
            if (!prev) return null;
            return {
              ...prev,
              metadata: {
                ...prev.metadata,
                isUpdated: false,
              },
            };
          });
        }, 2000);
      }
    }
  }, [activityContext, inputValue, isTracking]);

  // Check for stale suggestions periodically
  useEffect(() => {
    if (!currentSuggestion || !activityContext) {
      return;
    }

    const checkStaleInterval = setInterval(() => {
      const age = Date.now() - currentSuggestion.timestamp;
      const oldEnough = age > STALE_THRESHOLD_MS;

      // Check if the current detected activity is different from the suggestion
      const currentActivity = activityContext.detected_activity?.trim() || '';
      const suggestionActivity = currentSuggestion.text?.trim() || '';
      const activityChanged = currentActivity !== suggestionActivity && currentActivity !== '';

      // Only mark as stale if old enough AND activity has changed
      const isNowStale = oldEnough && activityChanged;
      const wasStale = currentSuggestion.metadata?.isStale === true;

      if (isNowStale && !wasStale) {
        setCurrentSuggestion((prev) => {
          if (!prev) return null;
          return {
            ...prev,
            metadata: {
              ...prev.metadata,
              isStale: true,
            },
          };
        });
      } else if (!isNowStale && wasStale) {
        // If activity matches again, remove stale flag
        setCurrentSuggestion((prev) => {
          if (!prev) return null;
          return {
            ...prev,
            metadata: {
              ...prev.metadata,
              isStale: false,
            },
          };
        });
      }
    }, 10000); // Check every 10 seconds

    return () => clearInterval(checkStaleInterval);
  }, [currentSuggestion, activityContext]);

  const clearSuggestion = useCallback(() => {
    setCurrentSuggestion(null);
    previousSuggestionTextRef.current = null;
    lastActivityRef.current = null;
  }, []);

  return {
    currentSuggestion,
    clearSuggestion,
  };
}
